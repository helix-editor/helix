use crate::{events::PostInsertChar, key};

use super::*;
#[allow(dead_code)]
pub type Hook = fn(&Rope, &Selection, char) -> Option<Transaction>;

/// Exclude the cursor in range.
fn exclude_cursor(text: RopeSlice, range: Range, cursor: Range) -> Range {
    if range.to() == cursor.to() && text.len_chars() != cursor.to() {
        Range::new(
            range.from(),
            graphemes::prev_grapheme_boundary(text, cursor.to()),
        )
    } else {
        range
    }
}

// The default insert hook: simply insert the character
#[allow(clippy::unnecessary_wraps)] // need to use Option<> because of the Hook signature
fn insert(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
    let cursors = selection.clone().cursors(doc.slice(..));
    let mut t = Tendril::new();
    t.push(ch);
    let transaction = Transaction::insert(doc, &cursors, t);
    Some(transaction)
}

use silicon_core::auto_pairs;
use silicon_view::editor::SmartTabConfig;

pub fn insert_char(cx: &mut Context, c: char) {
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text();
    let selection = doc.selection(view.id);

    let loader: &silicon_core::syntax::Loader = &cx.editor.syn_loader.load();
    let auto_pairs = doc.auto_pairs(cx.editor, loader, view);

    let transaction = auto_pairs
        .as_ref()
        .and_then(|ap| auto_pairs::hook(text, selection, c, ap))
        .or_else(|| insert(text, selection, c));

    let (view, doc) = current!(cx.editor);
    if let Some(t) = transaction {
        doc.apply(&t, view.id);
    }

    silicon_event::dispatch(PostInsertChar { c, cx });
}

pub fn smart_tab(cx: &mut Context) {
    let (view, doc) = current_ref!(cx.editor);
    let view_id = view.id;

    if matches!(
        cx.editor.config().smart_tab,
        Some(SmartTabConfig { enable: true, .. })
    ) {
        let cursors_after_whitespace = doc.selection(view_id).ranges().iter().all(|range| {
            let cursor = range.cursor(doc.text().slice(..));
            let current_line_num = doc.text().char_to_line(cursor);
            let current_line_start = doc.text().line_to_char(current_line_num);
            let left = doc.text().slice(current_line_start..cursor);
            left.chars().all(|c| c.is_whitespace())
        });

        if !cursors_after_whitespace {
            if doc.active_snippet.is_some() {
                goto_next_tabstop(cx);
            } else {
                move_parent_node_end(cx);
            }
            return;
        }
    }

    insert_tab(cx);
}

pub fn insert_tab(cx: &mut Context) {
    insert_tab_impl(cx, 1)
}

fn insert_tab_impl(cx: &mut Context, count: usize) {
    let (view, doc) = current!(cx.editor);
    // TODO: round out to nearest indentation level (for example a line with 3 spaces should
    // indent by one to reach 4 spaces).

    let indent = Tendril::from(doc.indent_style.as_str().repeat(count));
    let transaction = Transaction::insert(
        doc.text(),
        &doc.selection(view.id).clone().cursors(doc.text().slice(..)),
        indent,
    );
    doc.apply(&transaction, view.id);
}

pub fn append_char_interactive(cx: &mut Context) {
    // Save the current mode, so we can restore it later.
    let mode = cx.editor.mode;
    append_mode(cx);
    insert_selection_interactive(cx, mode);
}

pub fn insert_char_interactive(cx: &mut Context) {
    let mode = cx.editor.mode;
    insert_mode(cx);
    insert_selection_interactive(cx, mode);
}

fn insert_selection_interactive(cx: &mut Context, old_mode: Mode) {
    let count = cx.count();

    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        match event {
            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => {
                for _ in 0..count {
                    insert::insert_char(cx, ch)
                }
            }
            key!(Enter) => {
                if count != 1 {
                    cx.editor
                        .set_error("inserting multiple newlines not yet supported");
                    return;
                }
                insert_newline(cx)
            }
            key!(Tab) => insert_tab_impl(cx, count),
            _ => (),
        };
        // Restore the old mode.
        cx.editor.mode = old_mode;
    });
}

pub fn insert_newline(cx: &mut Context) {
    let config = cx.editor.config();
    let (view, doc) = current_ref!(cx.editor);
    let loader = cx.editor.syn_loader.load();
    let text = doc.text().slice(..);
    let line_ending = doc.line_ending.as_str();

    let contents = doc.text();
    let selection = doc.selection(view.id);
    let mut ranges = SmallVec::with_capacity(selection.len());

    // TODO: this is annoying, but we need to do it to properly calculate pos after edits
    let mut global_offs = 0;
    let mut new_text = String::new();

    let continue_comment_tokens = if config.continue_comments {
        doc.language_config()
            .and_then(|config| config.comment_tokens.as_ref())
    } else {
        None
    };

    let mut last_pos = 0;
    let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
        // Tracks the number of trailing whitespace characters deleted by this selection.
        let mut chars_deleted = 0;
        let pos = range.cursor(text);

        let prev = if pos == 0 {
            ' '
        } else {
            contents.char(pos - 1)
        };
        let curr = contents.get_char(pos).unwrap_or(' ');

        let current_line = text.char_to_line(pos);
        let line_start = text.line_to_char(current_line);

        let continue_comment_token = continue_comment_tokens
            .and_then(|tokens| comment::get_comment_token(text, tokens, current_line));

        let (from, to, local_offs) = if let Some(idx) =
            text.slice(line_start..pos).last_non_whitespace_char()
        {
            let first_trailing_whitespace_char = (line_start + idx + 1).clamp(last_pos, pos);
            last_pos = pos;
            let line = text.line(current_line);

            let indent = match line.first_non_whitespace_char() {
                Some(pos) if continue_comment_token.is_some() => line.slice(..pos).to_string(),
                _ => core_indent::indent_for_newline(
                    &loader,
                    doc.syntax(),
                    &config.indent_heuristic,
                    &doc.indent_style,
                    doc.tab_width(),
                    text,
                    current_line,
                    pos,
                    current_line,
                ),
            };

            let loader: &silicon_core::syntax::Loader = &cx.editor.syn_loader.load();
            // If we are between pairs (such as brackets), we want to
            // insert an additional line which is indented one level
            // more and place the cursor there
            let on_auto_pair = doc
                .auto_pairs(cx.editor, loader, view)
                .and_then(|pairs| pairs.get(prev))
                .is_some_and(|pair| pair.open == prev && pair.close == curr);

            let local_offs = if let Some(token) = continue_comment_token {
                new_text.reserve_exact(line_ending.len() + indent.len() + token.len() + 1);
                new_text.push_str(line_ending);
                new_text.push_str(&indent);
                new_text.push_str(token);
                new_text.push(' ');
                new_text.chars().count()
            } else if on_auto_pair {
                // line where the cursor will be
                let inner_indent = indent.clone() + doc.indent_style.as_str();
                new_text
                    .reserve_exact(line_ending.len() * 2 + indent.len() + inner_indent.len());
                new_text.push_str(line_ending);
                new_text.push_str(&inner_indent);

                // line where the matching pair will be
                let local_offs = new_text.chars().count();
                new_text.push_str(line_ending);
                new_text.push_str(&indent);

                local_offs
            } else {
                new_text.reserve_exact(line_ending.len() + indent.len());
                new_text.push_str(line_ending);
                new_text.push_str(&indent);

                new_text.chars().count()
            };

            // Note that `first_trailing_whitespace_char` is at least `pos` so this unsigned
            // subtraction cannot underflow.
            chars_deleted = pos - first_trailing_whitespace_char;

            (
                first_trailing_whitespace_char,
                pos,
                local_offs as isize - chars_deleted as isize,
            )
        } else {
            // If the current line is all whitespace, insert a line ending at the beginning of
            // the current line. This makes the current line empty and the new line contain the
            // indentation of the old line.
            new_text.push_str(line_ending);

            (line_start, line_start, new_text.chars().count() as isize)
        };

        let new_range = if range.cursor(text) > range.anchor {
            // when appending, extend the range by local_offs
            Range::new(
                (range.anchor as isize + global_offs) as usize,
                (range.head as isize + local_offs + global_offs) as usize,
            )
        } else {
            // when inserting, slide the range by local_offs
            Range::new(
                (range.anchor as isize + local_offs + global_offs) as usize,
                (range.head as isize + local_offs + global_offs) as usize,
            )
        };

        // TODO: range replace or extend
        // range.replace(|range| range.is_empty(), head); -> fn extend if cond true, new head pos
        // can be used with cx.mode to do replace or extend on most changes
        ranges.push(new_range);
        global_offs += new_text.chars().count() as isize - chars_deleted as isize;
        let tendril = Tendril::from(&new_text);
        new_text.clear();

        (from, to, Some(tendril))
    });

    transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));

    let (view, doc) = current!(cx.editor);
    doc.apply(&transaction, view.id);
}

pub fn delete_char_backward(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);
    let tab_width = doc.tab_width();
    let indent_width = doc.indent_width();

    let loader: &silicon_core::syntax::Loader = &cx.editor.syn_loader.load();
    let auto_pairs = doc.auto_pairs(cx.editor, loader, view);

    let transaction =
        Transaction::delete_by_selection(doc.text(), doc.selection(view.id), |range| {
            let pos = range.cursor(text);
            if pos == 0 {
                return (pos, pos);
            }
            let line_start_pos = text.line_to_char(range.cursor_line(text));
            // consider to delete by indent level if all characters before `pos` are indent units.
            let fragment = Cow::from(text.slice(line_start_pos..pos));
            if !fragment.is_empty() && fragment.chars().all(|ch| ch == ' ' || ch == '\t') {
                if text.get_char(pos.saturating_sub(1)) == Some('\t') {
                    // fast path, delete one char
                    (graphemes::nth_prev_grapheme_boundary(text, pos, 1), pos)
                } else {
                    let width: usize = fragment
                        .chars()
                        .map(|ch| {
                            if ch == '\t' {
                                tab_width
                            } else {
                                // it can be none if it still meet control characters other than '\t'
                                // here just set the width to 1 (or some value better?).
                                ch.width().unwrap_or(1)
                            }
                        })
                        .sum();
                    let mut drop = width % indent_width; // round down to nearest unit
                    if drop == 0 {
                        drop = indent_width
                    }; // if it's already at a unit, consume a whole unit
                    let mut chars = fragment.chars().rev();
                    let mut start = pos;
                    for _ in 0..drop {
                        // delete up to `drop` spaces
                        match chars.next() {
                            Some(' ') => start -= 1,
                            _ => break,
                        }
                    }
                    (start, pos) // delete!
                }
            } else {
                match (
                    text.get_char(pos.saturating_sub(1)),
                    text.get_char(pos),
                    auto_pairs,
                ) {
                    (Some(_x), Some(_y), Some(ap))
                        if range.is_single_grapheme(text)
                            && ap.get(_x).is_some()
                            && ap.get(_x).unwrap().open == _x
                            && ap.get(_x).unwrap().close == _y =>
                    // delete both autopaired characters
                    {
                        (
                            graphemes::nth_prev_grapheme_boundary(text, pos, count),
                            graphemes::nth_next_grapheme_boundary(text, pos, count),
                        )
                    }
                    _ =>
                    // delete 1 char
                    {
                        (graphemes::nth_prev_grapheme_boundary(text, pos, count), pos)
                    }
                }
            }
        });
    let (view, doc) = current!(cx.editor);
    doc.apply(&transaction, view.id);
}

pub fn delete_char_forward(cx: &mut Context) {
    let count = cx.count();
    delete_by_selection_insert_mode(
        cx,
        |text, range| {
            let pos = range.cursor(text);
            (pos, graphemes::nth_next_grapheme_boundary(text, pos, count))
        },
        Direction::Forward,
    )
}

pub fn delete_word_backward(cx: &mut Context) {
    let count = cx.count();
    delete_by_selection_insert_mode(
        cx,
        |text, range| {
            let anchor = core_movement::move_prev_word_start(text, *range, count).from();
            let next = Range::new(anchor, range.cursor(text));
            let range = exclude_cursor(text, next, *range);
            (range.from(), range.to())
        },
        Direction::Backward,
    );
}

pub fn delete_word_forward(cx: &mut Context) {
    let count = cx.count();
    delete_by_selection_insert_mode(
        cx,
        |text, range| {
            let head = core_movement::move_next_word_end(text, *range, count).to();
            (range.cursor(text), head)
        },
        Direction::Forward,
    );
}
