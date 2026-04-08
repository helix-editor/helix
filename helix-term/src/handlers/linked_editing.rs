use helix_core::syntax::config::LanguageServerFeature;
use helix_core::{Assoc, ChangeSet, Range, Tendril, Transaction};
use helix_event::{cancelable_future, register_hook};
use helix_lsp::util::lsp_range_to_range;
use helix_view::document::{LinkedEditingState, Mode};
use helix_view::events::{DocumentDidChange, SelectionDidChange};
use helix_view::{DocumentId, Editor, ViewId};

use crate::events::OnModeSwitch;
use crate::job;

#[derive(Debug, Clone)]
struct Edit {
    from: usize,
    to: usize,
    insert: Option<Tendril>,
}

fn changeset_to_edits(changes: &ChangeSet) -> Vec<Edit> {
    use helix_core::Operation::*;

    let mut edits: Vec<Edit> = Vec::new();
    let mut old_pos = 0;

    for op in changes.changes() {
        match op {
            Retain(n) => old_pos += n,
            Insert(text) => {
                // Coalesce insertions/deletions that share the same anchor into a single edit.
                let insert = text.clone();
                if let Some(last) = edits.last_mut() {
                    if last.from == old_pos {
                        if let Some(existing) = &mut last.insert {
                            existing.push_str(&insert);
                            continue;
                        }
                        if last.insert.is_none() && last.to > old_pos {
                            last.insert = Some(insert);
                            continue;
                        }
                        if last.to == old_pos {
                            last.insert = Some(insert);
                            continue;
                        }
                    }
                }
                edits.push(Edit {
                    from: old_pos,
                    to: old_pos,
                    insert: Some(insert),
                });
            }
            Delete(n) => {
                let to = old_pos + n;
                if let Some(last) = edits.last_mut() {
                    if last.from == old_pos {
                        if last.to == old_pos {
                            last.to = to;
                            old_pos = to;
                            continue;
                        }
                        if last.insert.is_none() {
                            last.to = to;
                            old_pos = to;
                            continue;
                        }
                    }
                }
                edits.push(Edit {
                    from: old_pos,
                    to,
                    insert: None,
                });
                old_pos = to;
            }
        }
    }

    edits
}

fn edit_inside_range(edit: &Edit, range: &Range) -> bool {
    edit.from >= range.from() && edit.to <= range.to()
}

fn edit_overlaps_range(edit: &Edit, range: &Range) -> bool {
    if edit.from == edit.to {
        edit.from >= range.from() && edit.from <= range.to()
    } else {
        edit.from < range.to() && edit.to > range.from()
    }
}

fn update_ranges(ranges: &mut [Range], changes: &ChangeSet) {
    changes.update_positions(ranges.iter_mut().flat_map(|range| {
        [
            (&mut range.anchor, Assoc::After),
            (&mut range.head, Assoc::After),
        ]
    }));
}

fn cursor_in_ranges(cursor: usize, ranges: &[Range]) -> bool {
    ranges
        .iter()
        .any(|range| range.contains(cursor) || range.to() == cursor)
}

fn clear_state(doc: &mut helix_view::Document, view_id: ViewId) {
    doc.linked_editing.remove(&view_id);
}

fn reset_state(doc: &mut helix_view::Document, view_id: ViewId) {
    clear_state(doc, view_id);
    doc.linked_editing_changes.remove(&view_id);
    doc.linked_editing_controller(view_id).cancel();
}

fn active_linked_range_idx(edits: &[Edit], old_ranges: &[Range]) -> Option<usize> {
    let mut active_range_idx = None;

    for edit in edits {
        let mut edit_range_idx = None;
        let mut contained = false;

        for (idx, range) in old_ranges.iter().enumerate() {
            if edit_overlaps_range(edit, range) {
                if edit_range_idx.is_some() {
                    return None;
                }
                edit_range_idx = Some(idx);
                contained = edit_inside_range(edit, range);
            }
        }

        let idx = edit_range_idx?;
        if !contained {
            return None;
        }

        if let Some(active_idx) = active_range_idx {
            if active_idx != idx {
                return None;
            }
        } else {
            active_range_idx = Some(idx);
        }
    }

    active_range_idx
}

fn rebase_linked_ranges(ranges: &mut [Range], pending_changes: &ChangeSet) -> Option<usize> {
    let edits = changeset_to_edits(pending_changes);
    let active_range_idx = active_linked_range_idx(&edits, ranges)?;
    update_ranges(ranges, pending_changes);
    Some(active_range_idx)
}

fn apply_linked_edits(
    editor: &mut Editor,
    doc_id: DocumentId,
    view_id: ViewId,
    new_ranges: Vec<Range>,
    active_range_idx: usize,
    expected_version: i32,
) {
    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };
    if doc.version() != expected_version {
        doc.linked_editing
            .entry(view_id)
            .and_modify(|state| state.suppress = false);
        return;
    }
    if new_ranges.len() <= 1 || active_range_idx >= new_ranges.len() {
        clear_state(doc, view_id);
        doc.linked_editing
            .entry(view_id)
            .and_modify(|state| state.suppress = false);
        return;
    }

    let active_range = &new_ranges[active_range_idx];
    let active_text = doc.text().slice(active_range.from()..active_range.to());
    let active_text = Tendril::from(active_text.to_string());

    let mut changes = Vec::new();
    for (idx, range) in new_ranges.iter().enumerate() {
        if idx == active_range_idx {
            continue;
        }

        // Replace each linked range with the full active range text to keep content in sync.
        changes.push((range.from(), range.to(), Some(active_text.clone())));
    }

    if changes.is_empty() {
        return;
    }

    changes.sort_by_key(|(from, _, _)| *from);
    let transaction = Transaction::change(doc.text(), changes.into_iter());
    doc.apply(&transaction, view_id);
    doc.linked_editing
        .entry(view_id)
        .and_modify(|state| state.suppress = false);
}

fn apply_linked_editing_response(
    editor: &mut Editor,
    doc_id: DocumentId,
    view_id: ViewId,
    text: helix_core::Rope,
    offset_encoding: helix_lsp::OffsetEncoding,
    version: i32,
    response: Option<helix_lsp::lsp::LinkedEditingRanges>,
) {
    if editor.mode != Mode::Insert || !editor.config().lsp.linked_editing {
        if let Some(doc) = editor.document_mut(doc_id) {
            reset_state(doc, view_id);
        }
        return;
    }

    let Some(linked_ranges) = response else {
        if let Some(doc) = editor.document_mut(doc_id) {
            clear_state(doc, view_id);
            doc.linked_editing_changes.remove(&view_id);
        }
        return;
    };

    let mut ranges = Vec::new();
    for range in linked_ranges.ranges {
        if let Some(range) = lsp_range_to_range(&text, range, offset_encoding) {
            ranges.push(range);
        }
    }

    if ranges.len() <= 1 {
        if let Some(doc) = editor.document_mut(doc_id) {
            clear_state(doc, view_id);
            doc.linked_editing_changes.remove(&view_id);
        }
        return;
    }

    let mut replay = None;

    {
        let Some(doc) = editor.document_mut(doc_id) else {
            return;
        };
        let current_version = doc.version();
        let pending_changes = doc
            .linked_editing_changes
            .remove(&view_id)
            .unwrap_or_else(|| ChangeSet::new(text.slice(..)));

        if current_version != version {
            let Some(active_range_idx) = rebase_linked_ranges(&mut ranges, &pending_changes) else {
                clear_state(doc, view_id);
                return;
            };
            let cursor = doc
                .selection(view_id)
                .primary()
                .cursor(doc.text().slice(..));
            if !cursor_in_ranges(cursor, &ranges) {
                clear_state(doc, view_id);
                return;
            }

            doc.linked_editing.insert(
                view_id,
                LinkedEditingState {
                    ranges: ranges.clone(),
                    suppress: true,
                },
            );
            replay = Some((ranges, active_range_idx, current_version));
        } else {
            let cursor = doc
                .selection(view_id)
                .primary()
                .cursor(doc.text().slice(..));
            if !cursor_in_ranges(cursor, &ranges) {
                clear_state(doc, view_id);
                return;
            }

            doc.linked_editing.insert(
                view_id,
                LinkedEditingState {
                    ranges,
                    suppress: false,
                },
            );
        }
    }

    if let Some((ranges, active_range_idx, current_version)) = replay {
        apply_linked_edits(
            editor,
            doc_id,
            view_id,
            ranges,
            active_range_idx,
            current_version,
        );
    }
}

fn request_linked_editing_range(editor: &mut Editor, view_id: ViewId, doc_id: DocumentId) {
    if editor.mode != Mode::Insert || !editor.config().lsp.linked_editing {
        if let Some(doc) = editor.document_mut(doc_id) {
            reset_state(doc, view_id);
        }
        return;
    }

    // Gather request data while borrowing the document, then release borrows before async work.
    let (server_id, offset_encoding, position, text, version, identifier) = {
        let Some(doc) = editor.document_mut(doc_id) else {
            return;
        };

        let Some(language_server) = doc
            .language_servers_with_feature(LanguageServerFeature::LinkedEditingRange)
            .next()
        else {
            reset_state(doc, view_id);
            return;
        };

        let server_id = language_server.id();
        let offset_encoding = language_server.offset_encoding();
        let position = doc.position(view_id, offset_encoding);
        let text = doc.text().clone();
        let version = doc.version();
        let identifier = doc.identifier();
        doc.linked_editing_changes
            .insert(view_id, ChangeSet::new(text.slice(..)));
        (
            server_id,
            offset_encoding,
            position,
            text,
            version,
            identifier,
        )
    };

    let cancel = match editor.document_mut(doc_id) {
        Some(doc) => doc.linked_editing_controller(view_id).restart(),
        None => return,
    };

    let Some(language_server) = editor.language_server_by_id(server_id) else {
        if let Some(doc) = editor.document_mut(doc_id) {
            reset_state(doc, view_id);
        }
        return;
    };

    let Some(future) =
        language_server.text_document_linked_editing_range(identifier, position, None)
    else {
        if let Some(doc) = editor.document_mut(doc_id) {
            reset_state(doc, view_id);
        }
        return;
    };

    tokio::spawn(async move {
        let response = cancelable_future(future, &cancel).await;
        let response = match response {
            Some(Ok(result)) => result,
            Some(Err(err)) => {
                log::warn!("linked editing range request failed: {err}");
                return;
            }
            None => return,
        };

        job::dispatch(move |editor, _| {
            apply_linked_editing_response(
                editor,
                doc_id,
                view_id,
                text,
                offset_encoding,
                version,
                response,
            );
        })
        .await;
    });
}

pub(super) fn register_hooks(_handlers: &helix_view::handlers::Handlers) {
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        if event.new_mode == Mode::Insert {
            let (view, doc) = current_ref!(event.cx.editor);
            request_linked_editing_range(event.cx.editor, view.id, doc.id());
        } else if event.old_mode == Mode::Insert {
            let (view, doc) = current!(event.cx.editor);
            reset_state(doc, view.id);
        }
        Ok(())
    });

    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        if !event.doc.config.load().lsp.linked_editing {
            reset_state(event.doc, event.view);
            return Ok(());
        }

        if let Some(state) = event.doc.linked_editing.get(&event.view) {
            let cursor = event
                .doc
                .selection(event.view)
                .primary()
                .cursor(event.doc.text().slice(..));
            if cursor_in_ranges(cursor, &state.ranges) {
                return Ok(());
            }
            clear_state(event.doc, event.view);
            return Ok(());
        }

        let doc_id = event.doc.id();
        let view_id = event.view;
        job::dispatch_blocking(move |editor, _| {
            request_linked_editing_range(editor, view_id, doc_id);
        });
        Ok(())
    });

    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if !event.doc.config.load().lsp.linked_editing {
            reset_state(event.doc, event.view);
            return Ok(());
        }

        if let Some(changes) = event.doc.linked_editing_changes.get_mut(&event.view) {
            let pending = std::mem::take(changes);
            *changes = pending.compose(event.changes.clone());
        }

        let (old_ranges, new_ranges, suppress) = {
            let Some(state) = event.doc.linked_editing.get_mut(&event.view) else {
                return Ok(());
            };

            let old_ranges = state.ranges.clone();
            let suppress = state.suppress;
            update_ranges(&mut state.ranges, event.changes);
            let new_ranges = state.ranges.clone();
            (old_ranges, new_ranges, suppress)
        };

        if suppress {
            return Ok(());
        }
        if event.ghost_transaction {
            return Ok(());
        }

        let edits = changeset_to_edits(event.changes);
        if edits.is_empty() {
            return Ok(());
        }

        // Only mirror edits that touch a single linked range; anything else resets the state.
        let Some(active_range_idx) = active_linked_range_idx(&edits, &old_ranges) else {
            clear_state(event.doc, event.view);
            return Ok(());
        };

        if let Some(state) = event.doc.linked_editing.get_mut(&event.view) {
            state.suppress = true;
        }

        let doc_id = event.doc.id();
        let view_id = event.view;
        let expected_version = event.doc.version();
        // Apply the mirrored edits after this change finishes to avoid composing while
        // we're inside the original transaction.
        job::dispatch_blocking(move |editor, _| {
            apply_linked_edits(
                editor,
                doc_id,
                view_id,
                new_ranges,
                active_range_idx,
                expected_version,
            );
        });

        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_core::Rope;

    fn edits_from_changes(
        changes: impl Iterator<Item = (usize, usize, Option<Tendril>)>,
    ) -> Vec<Edit> {
        let text = Rope::from("abcdef");
        let transaction = Transaction::change(&text, changes);
        changeset_to_edits(transaction.changes())
    }

    #[test]
    fn changeset_to_edits_insertion() {
        let edits = edits_from_changes([(2, 2, Some(Tendril::from("X")))].into_iter());

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].from, 2);
        assert_eq!(edits[0].to, 2);
        assert_eq!(edits[0].insert.as_deref(), Some("X"));
    }

    #[test]
    fn changeset_to_edits_deletion() {
        let edits = edits_from_changes([(1, 4, None)].into_iter());

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].from, 1);
        assert_eq!(edits[0].to, 4);
        assert!(edits[0].insert.is_none());
    }

    #[test]
    fn changeset_to_edits_replacement() {
        let edits = edits_from_changes([(1, 3, Some(Tendril::from("Z")))].into_iter());

        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].from, 1);
        assert_eq!(edits[0].to, 3);
        assert_eq!(edits[0].insert.as_deref(), Some("Z"));
    }

    #[test]
    fn edit_range_helpers() {
        let range = Range::new(2, 6);
        let inside = Edit {
            from: 3,
            to: 5,
            insert: None,
        };
        let overlapping = Edit {
            from: 5,
            to: 7,
            insert: None,
        };
        let disjoint = Edit {
            from: 0,
            to: 2,
            insert: None,
        };

        assert!(edit_inside_range(&inside, &range));
        assert!(edit_overlaps_range(&inside, &range));
        assert!(edit_overlaps_range(&overlapping, &range));
        assert!(!edit_inside_range(&overlapping, &range));
        assert!(!edit_overlaps_range(&disjoint, &range));
    }

    #[test]
    fn active_linked_range_idx_rejects_multi_range_edits() {
        let ranges = vec![Range::new(2, 6), Range::new(8, 12)];
        let edits = vec![
            Edit {
                from: 3,
                to: 3,
                insert: Some(Tendril::from("x")),
            },
            Edit {
                from: 9,
                to: 9,
                insert: Some(Tendril::from("y")),
            },
        ];

        assert_eq!(active_linked_range_idx(&edits, &ranges), None);
    }

    #[test]
    fn rebase_linked_ranges_tracks_single_range_insert() {
        let text = Rope::from("<foo></foo>");
        let pending_changes =
            Transaction::change(&text, [(2, 2, Some(Tendril::from("x")))].into_iter());
        let mut ranges = vec![Range::new(1, 4), Range::new(7, 10)];

        let active_idx = rebase_linked_ranges(&mut ranges, pending_changes.changes());

        assert_eq!(active_idx, Some(0));
        assert_eq!(ranges, vec![Range::new(1, 5), Range::new(8, 11)]);
    }

    #[test]
    fn rebase_linked_ranges_rejects_cross_range_edit() {
        let text = Rope::from("<foo></foo>");
        let pending_changes =
            Transaction::change(&text, [(3, 8, Some(Tendril::from("x")))].into_iter());
        let mut ranges = vec![Range::new(1, 4), Range::new(7, 10)];

        assert_eq!(
            rebase_linked_ranges(&mut ranges, pending_changes.changes()),
            None
        );
    }
}
