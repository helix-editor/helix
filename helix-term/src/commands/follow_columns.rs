use super::{split, Context};
use helix_core::{char_idx_at_visual_offset, Selection};
use helix_view::{document::Mode, editor::Action, view::ViewPosition, DocumentId, Editor, ViewId};
use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::{
    args::aio,
    compositor,
    ui::{self, PromptEvent},
};

const FOLLOW_COLUMNS_MAX: usize = 9;
const FOLLOW_COLUMNS_MIN_WIDTH: u16 = 36;

#[derive(Clone)]
struct FollowColumnsState {
    doc: DocumentId,
    master: ViewId,
    views: Vec<ViewId>,
    base: ViewPosition,
}

static FOLLOW_COLUMNS: Lazy<Mutex<Option<FollowColumnsState>>> = Lazy::new(|| Mutex::new(None));

fn state() -> Option<FollowColumnsState> {
    FOLLOW_COLUMNS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn set_state(value: Option<FollowColumnsState>) {
    *FOLLOW_COLUMNS
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = value;
}

fn valid(editor: &Editor, current: &FollowColumnsState) -> bool {
    editor.documents.contains_key(&current.doc)
        && current.views.first() == Some(&current.master)
        && editor.tree.views().count() == current.views.len()
        && current.views.iter().all(|view_id| {
            editor
                .tree
                .try_get(*view_id)
                .is_some_and(|view| view.doc == current.doc)
        })
}

fn close_follow_views(editor: &mut Editor, current: &FollowColumnsState) {
    for view_id in current.views.iter().skip(1).rev().copied() {
        if editor.tree.contains(view_id) {
            editor.close(view_id);
        }
    }
    if editor.tree.contains(current.master) {
        editor.tree.focus = current.master;
    }
}

fn visual_shift(
    editor: &Editor,
    view_id: ViewId,
    position: ViewPosition,
    rows: isize,
) -> Option<ViewPosition> {
    let view = editor.tree.try_get(view_id)?;
    let doc = editor.documents.get(&view.doc)?;
    let viewport = view.inner_area(doc);
    let text = doc.text().slice(..);
    let text_format = doc.text_format(viewport.width, None);
    let annotations = view.text_annotations(doc, None);
    let target_rows = position.vertical_offset as isize + rows;

    let (anchor, vertical_offset) = char_idx_at_visual_offset(
        text,
        position.anchor,
        target_rows,
        0,
        &text_format,
        &annotations,
    );

    Some(ViewPosition {
        anchor,
        horizontal_offset: if text_format.soft_wrap {
            0
        } else {
            position.horizontal_offset
        },
        vertical_offset,
    })
}

fn pane_height(editor: &Editor, view_id: ViewId) -> Option<usize> {
    let view = editor.tree.try_get(view_id)?;
    let doc = editor.documents.get(&view.doc)?;
    Some(view.inner_area(doc).height as usize)
}

fn advance(editor: &Editor, view_id: ViewId, position: ViewPosition) -> Option<ViewPosition> {
    let height = pane_height(editor, view_id)?;
    visual_shift(editor, view_id, position, height as isize)
}

fn retreat(editor: &Editor, view_id: ViewId, position: ViewPosition) -> Option<ViewPosition> {
    let height = pane_height(editor, view_id)?;
    visual_shift(editor, view_id, position, -(height as isize))
}

fn pane_positions(
    editor: &Editor,
    current: &FollowColumnsState,
    base: ViewPosition,
) -> Option<Vec<ViewPosition>> {
    let mut result = Vec::with_capacity(current.views.len());
    let mut position = base;

    for (index, view_id) in current.views.iter().copied().enumerate() {
        result.push(position);
        if index + 1 < current.views.len() {
            position = advance(editor, view_id, position)?;
        }
    }

    Some(result)
}

fn base_from_active(
    editor: &Editor,
    current: &FollowColumnsState,
    active_index: usize,
    active_position: ViewPosition,
) -> Option<ViewPosition> {
    let mut position = active_position;

    for index in (0..active_index).rev() {
        position = retreat(editor, current.views[index], position)?;
    }

    Some(position)
}

fn end_visible(editor: &Editor, view_id: ViewId) -> Option<bool> {
    let view = editor.tree.try_get(view_id)?;
    let doc = editor.documents.get(&view.doc)?;
    let text = doc.text().slice(..);

    if text.len_chars() == 0 {
        return Some(true);
    }

    Some(
        view.screen_coords_at_pos(doc, text, text.len_chars().saturating_sub(1))
            .is_some(),
    )
}

fn layout(
    editor: &mut Editor,
    current: &FollowColumnsState,
    base: ViewPosition,
    move_cursors: bool,
) -> Option<bool> {
    let positions = pane_positions(editor, current, base)?;
    let mut covered = false;

    for (view_id, position) in current.views.iter().copied().zip(positions.iter().copied()) {
        let len = editor.documents.get(&current.doc)?.text().len_chars();

        {
            let doc = editor.documents.get_mut(&current.doc)?;
            doc.set_view_offset(view_id, position);

            if move_cursors {
                let selection = Selection::point(position.anchor.min(len));
                if doc.selection(view_id) != &selection {
                    doc.set_selection(view_id, selection);
                }
            }
        }

        covered |= end_visible(editor, view_id)?;
    }

    Some(covered)
}

fn shift_spread(
    editor: &Editor,
    current: &FollowColumnsState,
    forward: bool,
    half: bool,
) -> Option<ViewPosition> {
    let mut remaining = current
        .views
        .iter()
        .copied()
        .map(|view_id| pane_height(editor, view_id))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .sum::<usize>();

    if half {
        remaining = (remaining / 2).max(1);
    }

    let mut position = current.base;

    if forward {
        for view_id in current.views.iter().copied() {
            if remaining == 0 {
                break;
            }

            let height = pane_height(editor, view_id)?;
            let rows = remaining.min(height);
            position = visual_shift(editor, view_id, position, rows as isize)?;
            remaining -= rows;
        }
    } else {
        for view_id in current.views.iter().copied().rev() {
            if remaining == 0 {
                break;
            }

            let height = pane_height(editor, view_id)?;
            let rows = remaining.min(height);
            position = visual_shift(editor, view_id, position, -(rows as isize))?;
            remaining -= rows;
        }
    }

    Some(position)
}

fn navigate_spread(
    editor: &mut Editor,
    mut current: FollowColumnsState,
    forward: bool,
    half: bool,
) {
    if forward {
        if let Some(last) = current.views.last().copied() {
            if end_visible(editor, last) == Some(true) {
                editor.set_status("Follow columns: end of file");
                return;
            }
        }
    } else if current.base.anchor == 0 && current.base.vertical_offset == 0 {
        editor.set_status("Follow columns: start of file");
        return;
    }

    let Some(new_base) = shift_spread(editor, &current, forward, half) else {
        return;
    };

    if new_base == current.base {
        editor.set_status(if forward {
            "Follow columns: end of file"
        } else {
            "Follow columns: start of file"
        });
        return;
    }

    current.base = new_base;
    editor.tree.focus = current.master;
    let _ = layout(editor, &current, current.base, true);
    set_state(Some(current));

    editor.set_status(if half {
        if forward {
            "Follow columns: half spread forward"
        } else {
            "Follow columns: half spread backward"
        }
    } else if forward {
        "Follow columns: next spread"
    } else {
        "Follow columns: previous spread"
    });
}

pub(super) fn prepare_follow_columns(_editor: &mut Editor) {}

pub(crate) fn sync_follow_columns(editor: &mut Editor) {
    if let Some(activation) = aio::activate_pending(editor) {
        set_state(None);
        let (count, _) =
            enable_follow_columns_editor(editor, activation.columns, ViewPosition::default());
        if let Some(revision) = activation.git_revision {
            editor.set_status(format!(
                "AIO Git diff {revision}: {count} pane{}",
                if count == 1 { "" } else { "s" }
            ));
        } else {
            editor.set_status(format!(
                "AIO: {} files across {} pane{}",
                activation.files,
                count,
                if count == 1 { "" } else { "s" }
            ));
        }
    }

    aio::sync_syntax_mode(editor);

    let Some(mut current) = state() else {
        return;
    };

    if !valid(editor, &current) {
        set_state(None);
        return;
    }

    if editor.mode() == Mode::Insert {
        return;
    }

    let focused = editor.tree.focus;
    let Some(active_index) = current.views.iter().position(|view_id| *view_id == focused) else {
        return;
    };

    let Some(expected) = pane_positions(editor, &current, current.base) else {
        return;
    };

    let Some(active_position) = editor
        .documents
        .get(&current.doc)
        .map(|doc| doc.view_offset(focused))
    else {
        return;
    };

    if active_position != expected[active_index] {
        if let Some(new_base) = base_from_active(editor, &current, active_index, active_position) {
            current.base = new_base;
        }
    }

    if layout(editor, &current, current.base, false).is_some() {
        set_state(Some(current));
    }
}

fn enable_follow_columns_editor(
    editor: &mut Editor,
    requested_columns: Option<usize>,
    base: ViewPosition,
) -> (usize, bool) {
    let master = editor.tree.focus;
    let Some(doc) = editor.tree.try_get(master).map(|view| view.doc) else {
        return (0, true);
    };

    let other_views = editor
        .tree
        .views()
        .filter_map(|(view, _)| (view.id != master).then_some(view.id))
        .collect::<Vec<_>>();

    for view_id in other_views.into_iter().rev() {
        editor.close(view_id);
    }
    editor.tree.focus = master;

    let width = editor.tree.area().width as usize;
    let max_by_width = (width / FOLLOW_COLUMNS_MIN_WIDTH as usize).max(1);
    let auto_limit = FOLLOW_COLUMNS_MAX.min(max_by_width);
    let target_columns = requested_columns
        .map(|count| count.clamp(1, FOLLOW_COLUMNS_MAX))
        .unwrap_or(auto_limit);

    let mut current = FollowColumnsState {
        doc,
        master,
        views: vec![master],
        base,
    };

    let mut covered = layout(editor, &current, current.base, true).unwrap_or(true);

    while current.views.len() < target_columns && (requested_columns.is_some() || !covered) {
        editor.tree.focus = *current.views.last().unwrap();
        let previous_focus = editor.tree.focus;
        split(editor, Action::VerticalSplit);
        let new_focus = editor.tree.focus;

        if new_focus == previous_focus || current.views.contains(&new_focus) {
            break;
        }

        current.views.push(new_focus);
        covered = layout(editor, &current, current.base, true).unwrap_or(true);
    }

    editor.tree.focus = master;
    let count = current.views.len();
    set_state(Some(current.clone()));
    let _ = layout(editor, &current, current.base, false);

    (count, covered)
}

fn enable_follow_columns(cx: &mut Context, requested_columns: Option<usize>, base: ViewPosition) {
    let (count, covered) = enable_follow_columns_editor(cx.editor, requested_columns, base);

    match requested_columns {
        Some(requested) if count != requested => cx.editor.set_status(format!(
            "Follow columns: requested {requested}, created {count}"
        )),
        Some(_) => cx
            .editor
            .set_status(format!("Follow columns: fixed {count} panes")),
        None if covered => cx.editor.set_status(format!(
            "Follow columns: {count} pane{}, whole file visible",
            if count == 1 { "" } else { "s" }
        )),
        None => cx
            .editor
            .set_status(format!("Follow columns: {count} panes")),
    }
}

fn save_current(cx: &mut Context) {
    match aio::save_active(cx.editor) {
        Ok(Some(result)) => {
            cx.editor.set_status(format!(
                "AIO saved: {} written, {} renamed, {} deleted; {} files",
                result.written, result.renamed, result.deleted, result.files
            ));
        }
        Ok(None) => {
            let Some(command) = super::typed::TYPABLE_COMMAND_MAP.get("write") else {
                cx.editor.set_error("write command is unavailable");
                return;
            };

            let mut typed_context = compositor::Context {
                editor: &mut *cx.editor,
                jobs: &mut *cx.jobs,
                scroll: None,
            };

            if let Err(error) = super::typed::execute_command(
                &mut typed_context,
                command,
                "",
                PromptEvent::Validate,
            ) {
                typed_context.editor.set_error(error.to_string());
            }
        }
        Err(error) => cx.editor.set_error(error.to_string()),
    }
}

fn aio_new_prompt(cx: &mut Context) {
    ui::prompt(
        cx,
        "AIO new file: ".into(),
        None,
        |_, _| Vec::new(),
        |cx, input, event| {
            if event != PromptEvent::Validate {
                return;
            }

            match aio::new_file(cx.editor, input) {
                Ok(path) => cx.editor.set_status(format!("AIO new: {path}")),
                Err(error) => cx.editor.set_error(error.to_string()),
            }
        },
    );
}

fn aio_delete_current(cx: &mut Context) {
    match aio::delete_current(cx.editor) {
        Ok(path) => cx.editor.set_status(format!("AIO delete pending: {path}")),
        Err(error) => cx.editor.set_error(error.to_string()),
    }
}

fn aio_rename_prompt(cx: &mut Context) {
    let initial = match aio::current_relative(cx.editor) {
        Ok(path) => path,
        Err(error) => {
            cx.editor.set_error(error.to_string());
            return;
        }
    };

    ui::prompt_with_input(
        cx,
        "AIO rename: ".into(),
        initial,
        None,
        |_, _| Vec::new(),
        |cx, input, event| {
            if event != PromptEvent::Validate {
                return;
            }

            match aio::rename_current(cx.editor, input) {
                Ok((old, new)) => cx
                    .editor
                    .set_status(format!("AIO rename pending: {old} -> {new}")),
                Err(error) => cx.editor.set_error(error.to_string()),
            }
        },
    );
}

fn aio_refresh(cx: &mut Context) {
    match aio::refresh(cx.editor) {
        Ok(files) => {
            if let Some(revision) = aio::git_revision() {
                cx.editor
                    .set_status(format!("AIO Git diff refreshed: {revision}"));
            } else {
                cx.editor
                    .set_status(format!("AIO refreshed: {files} files"));
            }
        }
        Err(error) => cx.editor.set_error(error.to_string()),
    }
}

fn take_aio_signal(editor: &mut Editor) -> Option<String> {
    let signal = editor
        .registers
        .first('!', editor)
        .map(|value| value.into_owned());

    if signal
        .as_deref()
        .is_some_and(|value| value.starts_with("__hx_aio_"))
    {
        editor.registers.remove('!');
        signal
    } else {
        None
    }
}

pub(super) fn follow_columns(cx: &mut Context) {
    if let Some(signal) = take_aio_signal(cx.editor) {
        match signal.as_str() {
            "__hx_aio_new__" => aio_new_prompt(cx),
            "__hx_aio_delete__" => aio_delete_current(cx),
            "__hx_aio_rename__" => aio_rename_prompt(cx),
            "__hx_aio_refresh__" => aio_refresh(cx),
            _ => {}
        }
        return;
    }

    let count = cx.count.map(|count| count.get());

    match count {
        Some(15) => {
            save_current(cx);
            return;
        }
        Some(16) => {
            aio_new_prompt(cx);
            return;
        }
        Some(17) => {
            aio_delete_current(cx);
            return;
        }
        Some(18) => {
            aio_rename_prompt(cx);
            return;
        }
        Some(19) => {
            aio_refresh(cx);
            return;
        }
        _ => {}
    }

    if let Some(current) = state() {
        match count {
            None => {
                set_state(None);
                close_follow_views(cx.editor, &current);
                cx.editor.set_status("Follow columns off");
            }
            Some(columns @ 1..=9) => {
                let base = current.base;
                set_state(None);
                close_follow_views(cx.editor, &current);
                enable_follow_columns(cx, Some(columns), base);
            }
            Some(11) => navigate_spread(cx.editor, current, true, false),
            Some(12) => navigate_spread(cx.editor, current, false, false),
            Some(13) => navigate_spread(cx.editor, current, true, true),
            Some(14) => navigate_spread(cx.editor, current, false, true),
            Some(_) => cx
                .editor
                .set_status("Follow columns: use 1-9 panes, 11-14 navigation, 15 save, 16-19 AIO"),
        }
        return;
    }

    match count {
        None => enable_follow_columns(cx, None, ViewPosition::default()),
        Some(columns @ 1..=9) => enable_follow_columns(cx, Some(columns), ViewPosition::default()),
        Some(11..=14) => cx
            .editor
            .set_status("Enable follow columns first with Space F or 1-9 Space F"),
        Some(_) => cx
            .editor
            .set_status("Follow columns: pane count must be between 1 and 9"),
    }
}
