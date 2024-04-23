use crate::ui::{
    editor::{gutter_coords_and_view, pos_and_view},
    EditorView,
};
use anyhow::anyhow;
use helix_core::{movement::Direction, Range, Selection};
use helix_view::{input::MouseEvent, Document, ViewId};

use super::{
    move_next_long_word_end, move_next_word_end, move_prev_long_word_start, move_prev_word_start,
    paste_primary_clipboard_before, yank_primary_selection_impl, Context,
};

#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Debug, Copy)]
pub struct StaticMouseCommand {
    pub name: &'static str,
    pub(crate) fun: fn(&mut Context, &MouseEvent, &mut EditorView),
    pub doc: &'static str,
}

impl std::str::FromStr for StaticMouseCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        StaticMouseCommand::STATIC_COMMAND_LIST
            .iter()
            .find(|cmd| cmd.name() == s)
            .cloned()
            .ok_or_else(|| anyhow!("No command named '{}'", s))
    }
}

impl StaticMouseCommand {
    pub fn execute(&self, cx: &mut Context, event: &MouseEvent, editor_view: &mut EditorView) {
        (self.fun)(cx, event, editor_view);
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn doc(&self) -> &str {
        self.doc
    }
}

fn handle_selection_in_buffer(
    cx: &mut Context,
    evt: &MouseEvent,
    ev: &mut EditorView,
    callback: impl Fn(&mut Document, &ViewId, usize),
) -> bool {
    let editor = &mut cx.editor;

    if let Some((pos, view_id)) = pos_and_view(editor, evt.row, evt.column, true) {
        let prev_view_id = view!(editor).id;
        let doc = doc_mut!(editor, &view!(editor, view_id).doc);

        callback(doc, &view_id, pos);

        if view_id != prev_view_id {
            ev.clear_completion(editor);
        }

        editor.focus(view_id);
        editor.ensure_cursor_in_view(view_id);

        return true;
    }
    false
}

pub fn handle_main_button_mouse(cx: &mut Context, evt: &MouseEvent, ev: &mut EditorView) {
    if !handle_selection_in_buffer(
        cx,
        evt,
        ev,
        |doc: &mut Document, view_id: &ViewId, pos: usize| {
            doc.set_selection(*view_id, Selection::point(pos));
        },
    ) {
        add_breakpoint_mouse(cx, evt, ev);
    }
}

pub fn set_mouse_selection(cx: &mut Context, evt: &MouseEvent, ev: &mut EditorView) {
    handle_selection_in_buffer(
        cx,
        evt,
        ev,
        |doc: &mut Document, view_id: &ViewId, pos: usize| {
            doc.set_selection(*view_id, Selection::point(pos));
        },
    );
}

pub fn select_word_mouse(cx: &mut Context, _: &MouseEvent, _: &mut EditorView) {
    move_prev_word_start(cx);
    move_next_word_end(cx);
}

pub fn select_long_word_mouse(cx: &mut Context, _: &MouseEvent, _: &mut EditorView) {
    move_prev_long_word_start(cx);
    move_next_long_word_end(cx);
}

pub fn scroll_mouse_impl(cx: &mut Context, evt: &MouseEvent, dir: Direction, _: &mut EditorView) {
    let current_view = cx.editor.tree.focus;
    match pos_and_view(cx.editor, evt.row, evt.column, false) {
        Some((_, view_id)) => cx.editor.tree.focus = view_id,
        None => return,
    }
    super::scroll(cx, cx.editor.config().scroll_lines.unsigned_abs(), dir);
    cx.editor.tree.focus = current_view;
    cx.editor.ensure_cursor_in_view(current_view);
}

pub fn scroll_up_mouse(cx: &mut Context, evt: &MouseEvent, ev: &mut EditorView) {
    scroll_mouse_impl(cx, evt, Direction::Backward, ev)
}

pub fn scroll_down_mouse(cx: &mut Context, evt: &MouseEvent, ev: &mut EditorView) {
    scroll_mouse_impl(cx, evt, Direction::Forward, ev)
}

pub fn paste_primary_clipboard_before_mouse(
    cx: &mut Context,
    evt: &MouseEvent,
    _: &mut EditorView,
) {
    // if !config.middle_click_paste {
    //     return;
    // }
    let editor = &mut cx.editor;
    if let Some((pos, view_id)) = pos_and_view(editor, evt.row, evt.column, true) {
        let doc = doc_mut!(editor, &view!(editor, view_id).doc);
        doc.set_selection(view_id, Selection::point(pos));
        cx.editor.focus(view_id);
        paste_primary_clipboard_before(cx)
    }
}

pub fn yank_main_selection_to_primary_clipboard_mouse(
    cx: &mut Context,
    _: &MouseEvent,
    _: &mut EditorView,
) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);
    if doc
        .selection(view.id)
        .primary()
        .slice(doc.text().slice(..))
        .len_chars()
        <= 1
    {
        return;
    }

    yank_primary_selection_impl(cx.editor, '*');
}

pub fn add_breakpoint_mouse(cx: &mut Context, evt: &MouseEvent, _: &mut EditorView) {
    log::info!("called breakpoint adder");
    let editor = &mut cx.editor;
    if let Some((coords, view_id)) = gutter_coords_and_view(editor, evt.row, evt.column) {
        editor.focus(view_id);

        let (view, doc) = current!(cx.editor);

        let path = match doc.path() {
            Some(path) => path.clone(),
            None => return,
        };

        if let Some(char_idx) =
            view.pos_at_visual_coords(doc, coords.row as u16, coords.col as u16, true)
        {
            let line = doc.text().char_to_line(char_idx);
            super::dap_toggle_breakpoint_impl(cx, path, line);
            return;
        }
    }
}

pub fn add_selection_mouse(cx: &mut Context, evt: &MouseEvent, ev: &mut EditorView) {
    handle_selection_in_buffer(
        cx,
        evt,
        ev,
        |doc: &mut Document, view_id: &ViewId, pos: usize| {
            let selection = doc.selection(*view_id).clone();
            doc.set_selection(*view_id, selection.push(Range::point(pos)));
        },
    );
}

fn dap_impl_mouse(cx: &mut Context, evt: &MouseEvent, callback: impl Fn(&mut Context)) {
    let editor = &mut cx.editor;
    if let Some((coords, view_id)) = gutter_coords_and_view(editor, evt.row, evt.column) {
        editor.focus(view_id);

        let (view, doc) = current!(editor);
        if let Some(pos) =
            view.pos_at_visual_coords(doc, coords.row as u16, coords.col as u16, true)
        {
            doc.set_selection(view_id, Selection::point(pos));
            callback(cx);
        }
    }
}

pub fn dap_edit_condition_mouse(cx: &mut Context, evt: &MouseEvent, _: &mut EditorView) {
    dap_impl_mouse(cx, evt, |cx| {
        super::dap_edit_condition(cx);
    })
}

pub fn dap_edit_log_mouse(cx: &mut Context, evt: &MouseEvent, _: &mut EditorView) {
    dap_impl_mouse(cx, evt, |cx: &mut Context| {
        super::dap_edit_log(cx);
    })
}
