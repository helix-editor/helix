use helix_core::{
    comment, coords_at_pos, find_first_non_whitespace_char, find_root, graphemes,
    history::UndoKind,
    indent,
    indent::IndentStyle,
    line_ending::{get_line_ending_of_str, line_end_char_index, str_is_line_ending},
    match_brackets,
    movement::{self, Direction},
    numbers::NumberIncrementor,
    object, pos_at_coords,
    regex::{self, Regex, RegexBuilder},
    search, selection, surround, textobject,
    unicode::width::UnicodeWidthChar,
    LineEnding, Position, Range, Rope, RopeGraphemes, RopeSlice, Selection, SmallVec, Tendril,
    Transaction,
};
use helix_view::{
    clipboard::ClipboardType,
    document::{Mode, SCRATCH_BUFFER_NAME},
    editor::{Action, Motion},
    input::KeyEvent,
    keyboard::KeyCode,
    view::View,
    Document, DocumentId, Editor, ViewId,
};

use anyhow::{anyhow, bail, Context as _};
use helix_lsp::{
    block_on, lsp,
    util::{lsp_pos_to_pos, lsp_range_to_range, pos_to_lsp_pos, range_to_lsp_range},
    OffsetEncoding,
};
use insert::*;
use movement::Movement;

use crate::{
    compositor::{self, Component, Compositor},
    ui::{self, FilePicker, Picker, Popup, Prompt, PromptEvent},
};

use crate::job::{self, Job, Jobs};
use futures_util::{FutureExt, StreamExt};
use std::num::NonZeroUsize;
use std::{fmt, future::Future};

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use serde::de::{self, Deserialize, Deserializer};

use grep_regex::RegexMatcherBuilder;
use grep_searcher::{sinks, BinaryDetection, SearcherBuilder};
use ignore::{DirEntry, WalkBuilder, WalkState};
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct Context<'a> {
    pub register: Option<char>,
    pub count: Option<NonZeroUsize>,
    pub editor: &'a mut Editor,

    pub callback: Option<crate::compositor::Callback>,
    pub on_next_key_callback: Option<Box<dyn FnOnce(&mut Context, KeyEvent)>>,
    pub jobs: &'a mut Jobs,
}

impl<'a> Context<'a> {
    /// Push a new component onto the compositor.
    pub fn push_layer(&mut self, component: Box<dyn Component>) {
        self.callback = Some(Box::new(|compositor: &mut Compositor| {
            compositor.push(component)
        }));
    }

    #[inline]
    pub fn on_next_key(
        &mut self,
        on_next_key_callback: impl FnOnce(&mut Context, KeyEvent) + 'static,
    ) {
        self.on_next_key_callback = Some(Box::new(on_next_key_callback));
    }

    #[inline]
    pub fn callback<T, F>(
        &mut self,
        call: impl Future<Output = helix_lsp::Result<serde_json::Value>> + 'static + Send,
        callback: F,
    ) where
        T: for<'de> serde::Deserialize<'de> + Send + 'static,
        F: FnOnce(&mut Editor, &mut Compositor, T) + Send + 'static,
    {
        let callback = Box::pin(async move {
            let json = call.await?;
            let response = serde_json::from_value(json)?;
            let call: job::Callback =
                Box::new(move |editor: &mut Editor, compositor: &mut Compositor| {
                    callback(editor, compositor, response)
                });
            Ok(call)
        });
        self.jobs.callback(callback);
    }

    /// Returns 1 if no explicit count was provided
    #[inline]
    pub fn count(&self) -> usize {
        self.count.map_or(1, |v| v.get())
    }
}

enum Align {
    Top,
    Center,
    Bottom,
}

fn align_view(doc: &Document, view: &mut View, align: Align) {
    let pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));
    let line = doc.text().char_to_line(pos);

    let height = view.inner_area().height as usize;

    let relative = match align {
        Align::Center => height / 2,
        Align::Top => 0,
        Align::Bottom => height,
    };

    view.offset.row = line.saturating_sub(relative);
}

/// A command is composed of a static name, and a function that takes the current state plus a count,
/// and does a side-effect on the state (usually by creating and applying a transaction).
#[derive(Copy, Clone)]
pub struct Command {
    name: &'static str,
    fun: fn(cx: &mut Context),
    doc: &'static str,
}

macro_rules! commands {
    ( $($name:ident, $doc:literal,)* ) => {
        $(
            #[allow(non_upper_case_globals)]
            pub const $name: Self = Self {
                name: stringify!($name),
                fun: $name,
                doc: $doc
            };
        )*

        pub const COMMAND_LIST: &'static [Self] = &[
            $( Self::$name, )*
        ];
    }
}

impl Command {
    pub fn execute(&self, cx: &mut Context) {
        (self.fun)(cx);
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn doc(&self) -> &'static str {
        self.doc
    }

    #[rustfmt::skip]
    commands!(
        no_op, "Do nothing",
        move_char_left, "Move left",
        move_char_right, "Move right",
        move_line_up, "Move up",
        move_line_down, "Move down",
        extend_char_left, "Extend left",
        extend_char_right, "Extend right",
        extend_line_up, "Extend up",
        extend_line_down, "Extend down",
        copy_selection_on_next_line, "Copy selection on next line",
        copy_selection_on_prev_line, "Copy selection on previous line",
        move_next_word_start, "Move to beginning of next word",
        move_prev_word_start, "Move to beginning of previous word",
        move_prev_word_end, "Move to end of previous word",
        move_next_word_end, "Move to end of next word",
        move_next_long_word_start, "Move to beginning of next long word",
        move_prev_long_word_start, "Move to beginning of previous long word",
        move_next_long_word_end, "Move to end of next long word",
        extend_next_word_start, "Extend to beginning of next word",
        extend_prev_word_start, "Extend to beginning of previous word",
        extend_next_long_word_start, "Extend to beginning of next long word",
        extend_prev_long_word_start, "Extend to beginning of previous long word",
        extend_next_long_word_end, "Extend to end of next long word",
        extend_next_word_end, "Extend to end of next word",
        find_till_char, "Move till next occurance of char",
        find_next_char, "Move to next occurance of char",
        extend_till_char, "Extend till next occurance of char",
        extend_next_char, "Extend to next occurance of char",
        till_prev_char, "Move till previous occurance of char",
        find_prev_char, "Move to previous occurance of char",
        extend_till_prev_char, "Extend till previous occurance of char",
        extend_prev_char, "Extend to previous occurance of char",
        repeat_last_motion, "repeat last motion(extend_next_char, extend_till_char, find_next_char, find_till_char...)",
        replace, "Replace with new char",
        switch_case, "Switch (toggle) case",
        switch_to_uppercase, "Switch to uppercase",
        switch_to_lowercase, "Switch to lowercase",
        page_up, "Move page up",
        page_down, "Move page down",
        half_page_up, "Move half page up",
        half_page_down, "Move half page down",
        select_all, "Select whole document",
        select_regex, "Select all regex matches inside selections",
        split_selection, "Split selection into subselections on regex matches",
        split_selection_on_newline, "Split selection on newlines",
        search, "Search for regex pattern",
        rsearch, "Reverse search for regex pattern",
        search_next, "Select next search match",
        search_prev, "Select previous search match",
        extend_search_next, "Add next search match to selection",
        extend_search_prev, "Add previous search match to selection",
        search_selection, "Use current selection as search pattern",
        global_search, "Global Search in workspace folder",
        extend_line, "Select current line, if already selected, extend to next line",
        extend_to_line_bounds, "Extend selection to line bounds (line-wise selection)",
        delete_selection, "Delete selection",
        delete_selection_noyank, "Delete selection, without yanking",
        change_selection, "Change selection (delete and enter insert mode)",
        change_selection_noyank, "Change selection (delete and enter insert mode, without yanking)",
        collapse_selection, "Collapse selection onto a single cursor",
        flip_selections, "Flip selection cursor and anchor",
        insert_mode, "Insert before selection",
        append_mode, "Insert after selection (append)",
        command_mode, "Enter command mode",
        file_picker, "Open file picker",
        code_action, "Perform code action",
        buffer_picker, "Open buffer picker",
        symbol_picker, "Open symbol picker",
        workspace_symbol_picker, "Open workspace symbol picker",
        last_picker, "Open last picker",
        prepend_to_line, "Insert at start of line",
        append_to_line, "Insert at end of line",
        open_below, "Open new line below selection",
        open_above, "Open new line above selection",
        normal_mode, "Enter normal mode",
        select_mode, "Enter selection extend mode",
        exit_select_mode, "Exit selection mode",
        goto_definition, "Goto definition",
        add_newline_above, "Add newline above",
        add_newline_below, "Add newline below",
        goto_type_definition, "Goto type definition",
        goto_implementation, "Goto implementation",
        goto_file_start, "Goto file start/line",
        goto_file_end, "Goto file end",
        goto_reference, "Goto references",
        goto_window_top, "Goto window top",
        goto_window_middle, "Goto window middle",
        goto_window_bottom, "Goto window bottom",
        goto_last_accessed_file, "Goto last accessed file",
        goto_last_modification, "Goto last modification",
        goto_line, "Goto line",
        goto_last_line, "Goto last line",
        goto_first_diag, "Goto first diagnostic",
        goto_last_diag, "Goto last diagnostic",
        goto_next_diag, "Goto next diagnostic",
        goto_prev_diag, "Goto previous diagnostic",
        goto_line_start, "Goto line start",
        goto_line_end, "Goto line end",
        goto_next_buffer, "Goto next buffer",
        goto_previous_buffer, "Goto previous buffer",
        // TODO: different description ?
        goto_line_end_newline, "Goto line end",
        goto_first_nonwhitespace, "Goto first non-blank in line",
        trim_selections, "Trim whitespace from selections",
        extend_to_line_start, "Extend to line start",
        extend_to_line_end, "Extend to line end",
        extend_to_line_end_newline, "Extend to line end",
        signature_help, "Show signature help",
        insert_tab, "Insert tab char",
        insert_newline, "Insert newline char",
        delete_char_backward, "Delete previous char",
        delete_char_forward, "Delete next char",
        delete_word_backward, "Delete previous word",
        delete_word_forward, "Delete next word",
        kill_to_line_start, "Delete content till the start of the line",
        kill_to_line_end, "Delete content till the end of the line",
        undo, "Undo change",
        redo, "Redo change",
        earlier, "Move backward in history",
        later, "Move forward in history",
        yank, "Yank selection",
        yank_joined_to_clipboard, "Join and yank selections to clipboard",
        yank_main_selection_to_clipboard, "Yank main selection to clipboard",
        yank_joined_to_primary_clipboard, "Join and yank selections to primary clipboard",
        yank_main_selection_to_primary_clipboard, "Yank main selection to primary clipboard",
        replace_with_yanked, "Replace with yanked text",
        replace_selections_with_clipboard, "Replace selections by clipboard content",
        replace_selections_with_primary_clipboard, "Replace selections by primary clipboard content",
        paste_after, "Paste after selection",
        paste_before, "Paste before selection",
        paste_clipboard_after, "Paste clipboard after selections",
        paste_clipboard_before, "Paste clipboard before selections",
        paste_primary_clipboard_after, "Paste primary clipboard after selections",
        paste_primary_clipboard_before, "Paste primary clipboard before selections",
        indent, "Indent selection",
        unindent, "Unindent selection",
        format_selections, "Format selection",
        join_selections, "Join lines inside selection",
        keep_selections, "Keep selections matching regex",
        remove_selections, "Remove selections matching regex",
        align_selections, "Align selections in column",
        keep_primary_selection, "Keep primary selection",
        remove_primary_selection, "Remove primary selection",
        completion, "Invoke completion popup",
        hover, "Show docs for item under cursor",
        toggle_comments, "Comment/uncomment selections",
        rotate_selections_forward, "Rotate selections forward",
        rotate_selections_backward, "Rotate selections backward",
        rotate_selection_contents_forward, "Rotate selection contents forward",
        rotate_selection_contents_backward, "Rotate selections contents backward",
        expand_selection, "Expand selection to parent syntax node",
        jump_forward, "Jump forward on jumplist",
        jump_backward, "Jump backward on jumplist",
        jump_view_right, "Jump to the split to the right",
        jump_view_left, "Jump to the split to the left",
        jump_view_up, "Jump to the split above",
        jump_view_down, "Jump to the split below",
        rotate_view, "Goto next window",
        hsplit, "Horizontal bottom split",
        vsplit, "Vertical right split",
        wclose, "Close window",
        wonly, "Current window only",
        select_register, "Select register",
        insert_register, "Insert register",
        align_view_middle, "Align view middle",
        align_view_top, "Align view top",
        align_view_center, "Align view center",
        align_view_bottom, "Align view bottom",
        scroll_up, "Scroll view up",
        scroll_down, "Scroll view down",
        match_brackets, "Goto matching bracket",
        surround_add, "Surround add",
        surround_replace, "Surround replace",
        surround_delete, "Surround delete",
        select_textobject_around, "Select around object",
        select_textobject_inner, "Select inside object",
        shell_pipe, "Pipe selections through shell command",
        shell_pipe_to, "Pipe selections into shell command, ignoring command output",
        shell_insert_output, "Insert output of shell command before each selection",
        shell_append_output, "Append output of shell command after each selection",
        shell_keep_pipe, "Filter selections with shell predicate",
        suspend, "Suspend",
        rename_symbol, "Rename symbol",
        increment, "Increment",
        decrement, "Decrement",
    );
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Command { name, .. } = self;
        f.debug_tuple("Command").field(name).finish()
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Command { name, .. } = self;
        f.write_str(name)
    }
}

impl std::str::FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Command::COMMAND_LIST
            .iter()
            .copied()
            .find(|cmd| cmd.name == s)
            .ok_or_else(|| anyhow!("No command named '{}'", s))
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

fn no_op(_cx: &mut Context) {}

fn move_impl<F>(cx: &mut Context, move_fn: F, dir: Direction, behaviour: Movement)
where
    F: Fn(RopeSlice, Range, Direction, usize, Movement) -> Range,
{
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| move_fn(text, range, dir, count, behaviour));
    doc.set_selection(view.id, selection);
}

use helix_core::movement::{move_horizontally, move_vertically};

fn move_char_left(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Backward, Movement::Move)
}

fn move_char_right(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Forward, Movement::Move)
}

fn move_line_up(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Backward, Movement::Move)
}

fn move_line_down(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Forward, Movement::Move)
}

fn extend_char_left(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Backward, Movement::Extend)
}

fn extend_char_right(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Forward, Movement::Extend)
}

fn extend_line_up(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Backward, Movement::Extend)
}

fn extend_line_down(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Forward, Movement::Extend)
}

fn goto_line_end_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line);

        let pos = graphemes::prev_grapheme_boundary(text, line_end_char_index(&text, line))
            .max(line_start);

        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(
        view,
        doc,
        if doc.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

fn extend_to_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(view, doc, Movement::Extend)
}

fn goto_line_end_newline_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);

        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_end_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_newline_impl(
        view,
        doc,
        if doc.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

fn extend_to_line_end_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_newline_impl(view, doc, Movement::Extend)
}

fn goto_line_start_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_start_impl(
        view,
        doc,
        if doc.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

fn goto_next_buffer(cx: &mut Context) {
    goto_buffer(cx, Direction::Forward);
}

fn goto_previous_buffer(cx: &mut Context) {
    goto_buffer(cx, Direction::Backward);
}

fn goto_buffer(cx: &mut Context, direction: Direction) {
    let current = view!(cx.editor).doc;

    let id = match direction {
        Direction::Forward => {
            let iter = cx.editor.documents.keys();
            let mut iter = iter.skip_while(|id| *id != &current);
            iter.next(); // skip current item
            iter.next().or_else(|| cx.editor.documents.keys().next())
        }
        Direction::Backward => {
            let iter = cx.editor.documents.keys();
            let mut iter = iter.rev().skip_while(|id| *id != &current);
            iter.next(); // skip current item
            iter.next()
                .or_else(|| cx.editor.documents.keys().rev().next())
        }
    }
    .unwrap();

    let id = *id;

    cx.editor.switch(id, Action::Replace);
}

fn extend_to_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_start_impl(view, doc, Movement::Extend)
}

fn kill_to_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        range.put_cursor(text, text.line_to_char(line), true)
    });
    delete_selection_insert_mode(doc, view, &selection);
}

fn kill_to_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);
        range.put_cursor(text, pos, true)
    });
    delete_selection_insert_mode(doc, view, &selection);
}

fn goto_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        if let Some(pos) = find_first_non_whitespace_char(text.line(line)) {
            let pos = pos + text.line_to_char(line);
            range.put_cursor(text, pos, doc.mode == Mode::Select)
        } else {
            range
        }
    });
    doc.set_selection(view.id, selection);
}

fn trim_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let ranges: SmallVec<[Range; 1]> = doc
        .selection(view.id)
        .iter()
        .filter_map(|range| {
            if range.is_empty() || range.fragment(text).chars().all(|ch| ch.is_whitespace()) {
                return None;
            }
            let mut start = range.from();
            let mut end = range.to();
            start = movement::skip_while(text, start, |x| x.is_whitespace()).unwrap_or(start);
            end = movement::backwards_skip_while(text, end, |x| x.is_whitespace()).unwrap_or(end);
            if range.anchor < range.head {
                Some(Range::new(start, end))
            } else {
                Some(Range::new(end, start))
            }
        })
        .collect();

    if !ranges.is_empty() {
        let primary = doc.selection(view.id).primary();
        let idx = ranges
            .iter()
            .position(|range| range.overlaps(&primary))
            .unwrap_or(ranges.len() - 1);
        doc.set_selection(view.id, Selection::new(ranges, idx));
    } else {
        collapse_selection(cx);
        keep_primary_selection(cx);
    };
}

// align text in selection
fn align_selections(cx: &mut Context) {
    let align_style = cx.count();
    if align_style > 3 {
        cx.editor.set_error(
            "align only accept 1,2,3 as count to set left/center/right align".to_string(),
        );
        return;
    }

    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);
    let mut column_widths = vec![];
    let mut last_line = text.len_lines();
    let mut column = 0;
    // first of all, we need compute all column's width, let use max width of the selections in a column
    for sel in selection {
        let (l1, l2) = sel.line_range(text);
        if l1 != l2 {
            cx.editor
                .set_error("align cannot work with multi line selections".to_string());
            return;
        }
        // if the selection is not in the same line with last selection, we set the column to 0
        column = if l1 != last_line { 0 } else { column + 1 };
        last_line = l1;

        if column < column_widths.len() {
            if sel.to() - sel.from() > column_widths[column] {
                column_widths[column] = sel.to() - sel.from();
            }
        } else {
            // a new column, current selection width is the temp width of the column
            column_widths.push(sel.to() - sel.from());
        }
    }
    last_line = text.len_lines();
    // once we get the with of each column, we transform each selection with to it's column width based on the align style
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        let l = range.cursor_line(text);
        column = if l != last_line { 0 } else { column + 1 };
        last_line = l;

        (
            range.from(),
            range.to(),
            Some(
                align_fragment_to_width(&range.fragment(text), column_widths[column], align_style)
                    .into(),
            ),
        )
    });

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn align_fragment_to_width(fragment: &str, width: usize, align_style: usize) -> String {
    let trimed = fragment.trim_matches(|c| c == ' ');
    let mut s = " ".repeat(width - trimed.chars().count());
    match align_style {
        1 => s.insert_str(0, trimed),           // left align
        2 => s.insert_str(s.len() / 2, trimed), // center align
        3 => s.push_str(trimed),                // right align
        n => unimplemented!("{}", n),
    }
    s
}

fn goto_window(cx: &mut Context, align: Align) {
    let (view, doc) = current!(cx.editor);

    let height = view.inner_area().height as usize;

    // - 1 so we have at least one gap in the middle.
    // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
    // as we type
    let scrolloff = cx.editor.config.scrolloff.min(height.saturating_sub(1) / 2);

    let last_line = view.last_line(doc);

    let line = match align {
        Align::Top => (view.offset.row + scrolloff),
        Align::Center => (view.offset.row + (height / 2)),
        Align::Bottom => last_line.saturating_sub(scrolloff),
    }
    .min(last_line.saturating_sub(scrolloff));

    let pos = doc.text().line_to_char(line);

    doc.set_selection(view.id, Selection::point(pos));
}

fn goto_window_top(cx: &mut Context) {
    goto_window(cx, Align::Top)
}

fn goto_window_middle(cx: &mut Context) {
    goto_window(cx, Align::Center)
}

fn goto_window_bottom(cx: &mut Context) {
    goto_window(cx, Align::Bottom)
}

fn move_word_impl<F>(cx: &mut Context, move_fn: F)
where
    F: Fn(RopeSlice, Range, usize) -> Range,
{
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| move_fn(text, range, count));
    doc.set_selection(view.id, selection);
}

fn move_next_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_word_start)
}

fn move_prev_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_word_start)
}

fn move_prev_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_word_end)
}

fn move_next_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_word_end)
}

fn move_next_long_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_long_word_start)
}

fn move_prev_long_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_long_word_start)
}

fn move_next_long_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_long_word_end)
}

fn goto_file_start(cx: &mut Context) {
    if cx.count.is_some() {
        goto_line(cx);
    } else {
        push_jump(cx.editor);
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, 0, doc.mode == Mode::Select));
        doc.set_selection(view.id, selection);
    }
}

fn goto_file_end(cx: &mut Context) {
    push_jump(cx.editor);
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let pos = doc.text().len_chars();
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, doc.mode == Mode::Select));
    doc.set_selection(view.id, selection);
}

fn extend_word_impl<F>(cx: &mut Context, extend_fn: F)
where
    F: Fn(RopeSlice, Range, usize) -> Range,
{
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let word = extend_fn(text, range, count);
        let pos = word.cursor(text);
        range.put_cursor(text, pos, true)
    });
    doc.set_selection(view.id, selection);
}

fn extend_next_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_start)
}

fn extend_prev_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_word_start)
}

fn extend_next_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_end)
}

fn extend_next_long_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_start)
}

fn extend_prev_long_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_long_word_start)
}

fn extend_next_long_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_end)
}

fn will_find_char<F>(cx: &mut Context, search_fn: F, inclusive: bool, extend: bool)
where
    F: Fn(RopeSlice, char, usize, usize, bool) -> Option<usize> + 'static,
{
    // TODO: count is reset to 1 before next key so we move it into the closure here.
    // Would be nice to carry over.
    let count = cx.count();

    // need to wait for next key
    // TODO: should this be done by grapheme rather than char?  For example,
    // we can't properly handle the line-ending CRLF case here in terms of char.
    cx.on_next_key(move |cx, event| {
        let ch = match event {
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } =>
            // TODO: this isn't quite correct when CRLF is involved.
            // This hack will work in most cases, since documents don't
            // usually mix line endings.  But we should fix it eventually
            // anyway.
            {
                doc!(cx.editor).line_ending.as_str().chars().next().unwrap()
            }

            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => ch,
            _ => return,
        };

        find_char_impl(cx.editor, &search_fn, inclusive, extend, ch, count);
        cx.editor.last_motion = Some(Motion(Box::new(move |editor: &mut Editor| {
            find_char_impl(editor, &search_fn, inclusive, true, ch, 1);
        })));
    })
}

//

#[inline]
fn find_char_impl<F>(
    editor: &mut Editor,
    search_fn: &F,
    inclusive: bool,
    extend: bool,
    ch: char,
    count: usize,
) where
    F: Fn(RopeSlice, char, usize, usize, bool) -> Option<usize> + 'static,
{
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        // TODO: use `Range::cursor()` here instead.  However, that works in terms of
        // graphemes, whereas this function doesn't yet.  So we're doing the same logic
        // here, but just in terms of chars instead.
        let search_start_pos = if range.anchor < range.head {
            range.head - 1
        } else {
            range.head
        };

        search_fn(text, ch, search_start_pos, count, inclusive).map_or(range, |pos| {
            if extend {
                range.put_cursor(text, pos, true)
            } else {
                Range::point(range.cursor(text)).put_cursor(text, pos, true)
            }
        })
    });
    doc.set_selection(view.id, selection);
}

fn find_next_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    let pos = (pos + 1).min(text.len_chars());
    if inclusive {
        search::find_nth_next(text, ch, pos, n)
    } else {
        let n = match text.get_char(pos) {
            Some(next_ch) if next_ch == ch => n + 1,
            _ => n,
        };
        search::find_nth_next(text, ch, pos, n).map(|n| n.saturating_sub(1))
    }
}

fn find_prev_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    if inclusive {
        search::find_nth_prev(text, ch, pos, n)
    } else {
        let n = match text.get_char(pos.saturating_sub(1)) {
            Some(next_ch) if next_ch == ch => n + 1,
            _ => n,
        };
        search::find_nth_prev(text, ch, pos, n).map(|n| (n + 1).min(text.len_chars()))
    }
}

fn find_till_char(cx: &mut Context) {
    will_find_char(cx, find_next_char_impl, false, false)
}

fn find_next_char(cx: &mut Context) {
    will_find_char(cx, find_next_char_impl, true, false)
}

fn extend_till_char(cx: &mut Context) {
    will_find_char(cx, find_next_char_impl, false, true)
}

fn extend_next_char(cx: &mut Context) {
    will_find_char(cx, find_next_char_impl, true, true)
}

fn till_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, false, false)
}

fn find_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, true, false)
}

fn extend_till_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, false, true)
}

fn extend_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, true, true)
}

fn repeat_last_motion(cx: &mut Context) {
    let last_motion = cx.editor.last_motion.take();
    if let Some(m) = &last_motion {
        m.run(cx.editor);
        cx.editor.last_motion = last_motion;
    }
}

fn replace(cx: &mut Context) {
    let mut buf = [0u8; 4]; // To hold utf8 encoded char.

    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        let (view, doc) = current!(cx.editor);
        let ch = match event {
            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => Some(&ch.encode_utf8(&mut buf[..])[..]),
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => Some(doc.line_ending.as_str()),
            _ => None,
        };

        let selection = doc.selection(view.id);

        if let Some(ch) = ch {
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                if !range.is_empty() {
                    let text: String =
                        RopeGraphemes::new(doc.text().slice(range.from()..range.to()))
                            .map(|g| {
                                let cow: Cow<str> = g.into();
                                if str_is_line_ending(&cow) {
                                    cow
                                } else {
                                    ch.into()
                                }
                            })
                            .collect();

                    (range.from(), range.to(), Some(text.into()))
                } else {
                    // No change.
                    (range.from(), range.to(), None)
                }
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    })
}

fn switch_case_impl<F>(cx: &mut Context, change_fn: F)
where
    F: Fn(Cow<str>) -> Tendril,
{
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        let text: Tendril = change_fn(range.fragment(doc.text().slice(..)));

        (range.from(), range.to(), Some(text))
    });

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn switch_case(cx: &mut Context) {
    switch_case_impl(cx, |string| {
        string
            .chars()
            .flat_map(|ch| {
                if ch.is_lowercase() {
                    ch.to_uppercase().collect()
                } else if ch.is_uppercase() {
                    ch.to_lowercase().collect()
                } else {
                    vec![ch]
                }
            })
            .collect()
    });
}

fn switch_to_uppercase(cx: &mut Context) {
    switch_case_impl(cx, |string| string.to_uppercase().into());
}

fn switch_to_lowercase(cx: &mut Context) {
    switch_case_impl(cx, |string| string.to_lowercase().into());
}

pub fn scroll(cx: &mut Context, offset: usize, direction: Direction) {
    use Direction::*;
    let (view, doc) = current!(cx.editor);

    let range = doc.selection(view.id).primary();
    let text = doc.text().slice(..);

    let cursor = coords_at_pos(text, range.cursor(text));
    let doc_last_line = doc.text().len_lines().saturating_sub(1);

    let last_line = view.last_line(doc);

    if direction == Backward && view.offset.row == 0
        || direction == Forward && last_line == doc_last_line
    {
        return;
    }

    let height = view.inner_area().height;

    let scrolloff = cx.editor.config.scrolloff.min(height as usize / 2);

    view.offset.row = match direction {
        Forward => view.offset.row + offset,
        Backward => view.offset.row.saturating_sub(offset),
    }
    .min(doc_last_line);

    // recalculate last line
    let last_line = view.last_line(doc);

    // clamp into viewport
    let line = cursor
        .row
        .max(view.offset.row + scrolloff)
        .min(last_line.saturating_sub(scrolloff));

    let head = pos_at_coords(text, Position::new(line, cursor.col), true); // this func will properly truncate to line end

    let anchor = if doc.mode == Mode::Select {
        range.anchor
    } else {
        head
    };

    // TODO: only manipulate main selection
    doc.set_selection(view.id, Selection::single(anchor, head));
}

fn page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_area().height as usize;
    scroll(cx, offset, Direction::Backward);
}

fn page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_area().height as usize;
    scroll(cx, offset, Direction::Forward);
}

fn half_page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_area().height as usize / 2;
    scroll(cx, offset, Direction::Backward);
}

fn half_page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_area().height as usize / 2;
    scroll(cx, offset, Direction::Forward);
}

fn copy_selection_on_line(cx: &mut Context, direction: Direction) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);
    let mut ranges = SmallVec::with_capacity(selection.ranges().len() * (count + 1));
    ranges.extend_from_slice(selection.ranges());
    let mut primary_index = 0;
    for range in selection.iter() {
        let is_primary = *range == selection.primary();
        let head_pos = coords_at_pos(text, range.head);
        let anchor_pos = coords_at_pos(text, range.anchor);
        let height = std::cmp::max(head_pos.row, anchor_pos.row)
            - std::cmp::min(head_pos.row, anchor_pos.row)
            + 1;

        if is_primary {
            primary_index = ranges.len();
        }
        ranges.push(*range);

        let mut sels = 0;
        let mut i = 0;
        while sels < count {
            let offset = (i + 1) * height;

            let anchor_row = match direction {
                Direction::Forward => anchor_pos.row + offset,
                Direction::Backward => anchor_pos.row.saturating_sub(offset),
            };

            let head_row = match direction {
                Direction::Forward => head_pos.row + offset,
                Direction::Backward => head_pos.row.saturating_sub(offset),
            };

            if anchor_row >= text.len_lines() || head_row >= text.len_lines() {
                break;
            }

            let anchor = pos_at_coords(text, Position::new(anchor_row, anchor_pos.col), true);
            let head = pos_at_coords(text, Position::new(head_row, head_pos.col), true);

            // skip lines that are too short
            if coords_at_pos(text, anchor).col == anchor_pos.col
                && coords_at_pos(text, head).col == head_pos.col
            {
                if is_primary {
                    primary_index = ranges.len();
                }
                ranges.push(Range::new(anchor, head));
                sels += 1;
            }

            i += 1;
        }
    }

    let selection = Selection::new(ranges, primary_index);
    doc.set_selection(view.id, selection);
}

fn copy_selection_on_prev_line(cx: &mut Context) {
    copy_selection_on_line(cx, Direction::Backward)
}

fn copy_selection_on_next_line(cx: &mut Context) {
    copy_selection_on_line(cx, Direction::Forward)
}

fn select_all(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let end = doc.text().len_chars();
    doc.set_selection(view.id, Selection::single(0, end))
}

fn select_regex(cx: &mut Context) {
    let reg = cx.register.unwrap_or('/');
    let prompt = ui::regex_prompt(
        cx,
        "select:".into(),
        Some(reg),
        |_input: &str| Vec::new(),
        move |view, doc, regex, event| {
            if event != PromptEvent::Update {
                return;
            }
            let text = doc.text().slice(..);
            if let Some(selection) =
                selection::select_on_matches(text, doc.selection(view.id), &regex)
            {
                doc.set_selection(view.id, selection);
            }
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn split_selection(cx: &mut Context) {
    let reg = cx.register.unwrap_or('/');
    let prompt = ui::regex_prompt(
        cx,
        "split:".into(),
        Some(reg),
        |_input: &str| Vec::new(),
        move |view, doc, regex, event| {
            if event != PromptEvent::Update {
                return;
            }
            let text = doc.text().slice(..);
            let selection = selection::split_on_matches(text, doc.selection(view.id), &regex);
            doc.set_selection(view.id, selection);
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn split_selection_on_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\r\n|[\n\r\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}]").unwrap());
    let selection = selection::split_on_matches(text, doc.selection(view.id), &REGEX);
    doc.set_selection(view.id, selection);
}

fn search_impl(
    doc: &mut Document,
    view: &mut View,
    contents: &str,
    regex: &Regex,
    movement: Movement,
    direction: Direction,
    scrolloff: usize,
) {
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);

    // Get the right side of the primary block cursor for forward search, or the
    //grapheme before the start of the selection for reverse search.
    let start = match direction {
        Direction::Forward => text.char_to_byte(graphemes::next_grapheme_boundary(
            text,
            selection.primary().to(),
        )),
        Direction::Backward => text.char_to_byte(graphemes::prev_grapheme_boundary(
            text,
            selection.primary().from(),
        )),
    };

    //A regex::Match returns byte-positions in the str. In the case where we
    //do a reverse search and wraparound to the end, we don't need to search
    //the text before the current cursor position for matches, but by slicing
    //it out, we need to add it back to the position of the selection.
    let mut offset = 0;

    // use find_at to find the next match after the cursor, loop around the end
    // Careful, `Regex` uses `bytes` as offsets, not character indices!
    let mat = match direction {
        Direction::Forward => regex
            .find_at(contents, start)
            .or_else(|| regex.find(contents)),
        Direction::Backward => regex.find_iter(&contents[..start]).last().or_else(|| {
            offset = start;
            regex.find_iter(&contents[start..]).last()
        }),
    };
    // TODO: message on wraparound
    if let Some(mat) = mat {
        let start = text.byte_to_char(mat.start() + offset);
        let end = text.byte_to_char(mat.end() + offset);

        if end == 0 {
            // skip empty matches that don't make sense
            return;
        }

        // Determine range direction based on the primary range
        let primary = selection.primary();
        let range = if primary.head < primary.anchor {
            Range::new(end, start)
        } else {
            Range::new(start, end)
        };

        let selection = match movement {
            Movement::Extend => selection.clone().push(range),
            Movement::Move => selection.clone().replace(selection.primary_index(), range),
        };

        doc.set_selection(view.id, selection);
        if view.is_cursor_in_view(doc, 0) {
            view.ensure_cursor_in_view(doc, scrolloff);
        } else {
            align_view(doc, view, Align::Center)
        }
    };
}

fn search_completions(cx: &mut Context, reg: Option<char>) -> Vec<String> {
    let mut items = reg
        .and_then(|reg| cx.editor.registers.get(reg))
        .map_or(Vec::new(), |reg| reg.read().iter().take(200).collect());
    items.sort_unstable();
    items.dedup();
    items.into_iter().cloned().collect()
}

// TODO: use one function for search vs extend
fn search(cx: &mut Context) {
    searcher(cx, Direction::Forward)
}

fn rsearch(cx: &mut Context) {
    searcher(cx, Direction::Backward)
}
// TODO: use one function for search vs extend
fn searcher(cx: &mut Context, direction: Direction) {
    let reg = cx.register.unwrap_or('/');
    let scrolloff = cx.editor.config.scrolloff;

    let (_, doc) = current!(cx.editor);

    // TODO: could probably share with select_on_matches?

    // HAXX: sadly we can't avoid allocating a single string for the whole buffer since we can't
    // feed chunks into the regex yet
    let contents = doc.text().slice(..).to_string();
    let completions = search_completions(cx, Some(reg));

    let prompt = ui::regex_prompt(
        cx,
        "search:".into(),
        Some(reg),
        move |input: &str| {
            completions
                .iter()
                .filter(|comp| comp.starts_with(input))
                .map(|comp| (0.., std::borrow::Cow::Owned(comp.clone())))
                .collect()
        },
        move |view, doc, regex, event| {
            if event != PromptEvent::Update {
                return;
            }
            search_impl(
                doc,
                view,
                &contents,
                &regex,
                Movement::Move,
                direction,
                scrolloff,
            );
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn search_next_or_prev_impl(cx: &mut Context, movement: Movement, direction: Direction) {
    let scrolloff = cx.editor.config.scrolloff;
    let (view, doc) = current!(cx.editor);
    let registers = &cx.editor.registers;
    if let Some(query) = registers.read('/') {
        let query = query.last().unwrap();
        let contents = doc.text().slice(..).to_string();
        let case_insensitive = if cx.editor.config.smart_case {
            !query.chars().any(char::is_uppercase)
        } else {
            false
        };
        if let Ok(regex) = RegexBuilder::new(query)
            .case_insensitive(case_insensitive)
            .build()
        {
            search_impl(doc, view, &contents, &regex, movement, direction, scrolloff);
        } else {
            // get around warning `mutable_borrow_reservation_conflict`
            // which will be a hard error in the future
            // see: https://github.com/rust-lang/rust/issues/59159
            let query = query.clone();
            cx.editor.set_error(format!("Invalid regex: {}", query));
        }
    }
}

fn search_next(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Move, Direction::Forward);
}

fn search_prev(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Move, Direction::Backward);
}
fn extend_search_next(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Extend, Direction::Forward);
}

fn extend_search_prev(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Extend, Direction::Backward);
}

fn search_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let contents = doc.text().slice(..);
    let query = doc.selection(view.id).primary().fragment(contents);
    let regex = regex::escape(&query);
    cx.editor.registers.get_mut('/').push(regex);
    let msg = format!("register '{}' set to '{}'", '\\', query);
    cx.editor.set_status(msg);
}

fn global_search(cx: &mut Context) {
    let (all_matches_sx, all_matches_rx) =
        tokio::sync::mpsc::unbounded_channel::<(usize, PathBuf)>();
    let smart_case = cx.editor.config.smart_case;
    let file_picker_config = cx.editor.config.file_picker.clone();

    let completions = search_completions(cx, None);
    let prompt = ui::regex_prompt(
        cx,
        "global-search:".into(),
        None,
        move |input: &str| {
            completions
                .iter()
                .filter(|comp| comp.starts_with(input))
                .map(|comp| (0.., std::borrow::Cow::Owned(comp.clone())))
                .collect()
        },
        move |_view, _doc, regex, event| {
            if event != PromptEvent::Validate {
                return;
            }

            if let Ok(matcher) = RegexMatcherBuilder::new()
                .case_smart(smart_case)
                .build(regex.as_str())
            {
                let searcher = SearcherBuilder::new()
                    .binary_detection(BinaryDetection::quit(b'\x00'))
                    .build();

                let search_root = std::env::current_dir()
                    .expect("Global search error: Failed to get current dir");
                WalkBuilder::new(search_root)
                    .hidden(file_picker_config.hidden)
                    .parents(file_picker_config.parents)
                    .ignore(file_picker_config.ignore)
                    .git_ignore(file_picker_config.git_ignore)
                    .git_global(file_picker_config.git_global)
                    .git_exclude(file_picker_config.git_exclude)
                    .max_depth(file_picker_config.max_depth)
                    .build_parallel()
                    .run(|| {
                        let mut searcher_cl = searcher.clone();
                        let matcher_cl = matcher.clone();
                        let all_matches_sx_cl = all_matches_sx.clone();
                        Box::new(move |dent: Result<DirEntry, ignore::Error>| -> WalkState {
                            let dent = match dent {
                                Ok(dent) => dent,
                                Err(_) => return WalkState::Continue,
                            };

                            match dent.file_type() {
                                Some(fi) => {
                                    if !fi.is_file() {
                                        return WalkState::Continue;
                                    }
                                }
                                None => return WalkState::Continue,
                            }

                            let result_sink = sinks::UTF8(|line_num, _| {
                                match all_matches_sx_cl
                                    .send((line_num as usize - 1, dent.path().to_path_buf()))
                                {
                                    Ok(_) => Ok(true),
                                    Err(_) => Ok(false),
                                }
                            });
                            let result =
                                searcher_cl.search_path(&matcher_cl, dent.path(), result_sink);

                            if let Err(err) = result {
                                log::error!(
                                    "Global search error: {}, {}",
                                    dent.path().display(),
                                    err
                                );
                            }
                            WalkState::Continue
                        })
                    });
            } else {
                // Otherwise do nothing
                // log::warn!("Global Search Invalid Pattern")
            }
        },
    );

    cx.push_layer(Box::new(prompt));

    let current_path = doc_mut!(cx.editor).path().cloned();

    let show_picker = async move {
        let all_matches: Vec<(usize, PathBuf)> =
            UnboundedReceiverStream::new(all_matches_rx).collect().await;
        let call: job::Callback =
            Box::new(move |editor: &mut Editor, compositor: &mut Compositor| {
                if all_matches.is_empty() {
                    editor.set_status("No matches found".to_string());
                    return;
                }

                let picker = FilePicker::new(
                    all_matches,
                    move |(_line_num, path)| {
                        let relative_path = helix_core::path::get_relative_path(path)
                            .to_str()
                            .unwrap()
                            .to_owned();
                        if current_path.as_ref().map(|p| p == path).unwrap_or(false) {
                            format!("{} (*)", relative_path).into()
                        } else {
                            relative_path.into()
                        }
                    },
                    move |editor: &mut Editor, (line_num, path), action| {
                        match editor.open(path.into(), action) {
                            Ok(_) => {}
                            Err(e) => {
                                editor.set_error(format!(
                                    "Failed to open file '{}': {}",
                                    path.display(),
                                    e
                                ));
                                return;
                            }
                        }

                        let line_num = *line_num;
                        let (view, doc) = current!(editor);
                        let text = doc.text();
                        let start = text.line_to_char(line_num);
                        let end = text.line_to_char((line_num + 1).min(text.len_lines()));

                        doc.set_selection(view.id, Selection::single(start, end));
                        align_view(doc, view, Align::Center);
                    },
                    |_editor, (line_num, path)| Some((path.clone(), Some((*line_num, *line_num)))),
                );
                compositor.push(Box::new(picker));
            });
        Ok(call)
    };
    cx.jobs.callback(show_picker);
}

fn extend_line(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

    let text = doc.text();
    let range = doc.selection(view.id).primary();

    let (start_line, end_line) = range.line_range(text.slice(..));
    let start = text.line_to_char(start_line);
    let mut end = text.line_to_char((end_line + count).min(text.len_lines()));

    if range.from() == start && range.to() == end {
        end = text.line_to_char((end_line + count + 1).min(text.len_lines()));
    }

    doc.set_selection(view.id, Selection::single(start, end));
}

fn extend_to_line_bounds(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    doc.set_selection(
        view.id,
        doc.selection(view.id).clone().transform(|range| {
            let text = doc.text();

            let (start_line, end_line) = range.line_range(text.slice(..));
            let start = text.line_to_char(start_line);
            let end = text.line_to_char((end_line + 1).min(text.len_lines()));

            if range.anchor <= range.head {
                Range::new(start, end)
            } else {
                Range::new(end, start)
            }
        }),
    );
}

enum Operation {
    Delete,
    Change,
}

fn delete_selection_impl(cx: &mut Context, op: Operation) {
    let (view, doc) = current!(cx.editor);

    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);

    if cx.register != Some('_') {
        // first yank the selection
        let values: Vec<String> = selection.fragments(text).map(Cow::into_owned).collect();
        let reg_name = cx.register.unwrap_or('"');
        let registers = &mut cx.editor.registers;
        let reg = registers.get_mut(reg_name);
        reg.write(values);
    };

    // then delete
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        (range.from(), range.to(), None)
    });
    doc.apply(&transaction, view.id);

    match op {
        Operation::Delete => {
            doc.append_changes_to_history(view.id);
            // exit select mode, if currently in select mode
            exit_select_mode(cx);
        }
        Operation::Change => {
            enter_insert_mode(doc);
        }
    }
}

#[inline]
fn delete_selection_insert_mode(doc: &mut Document, view: &View, selection: &Selection) {
    let view_id = view.id;

    // then delete
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        (range.from(), range.to(), None)
    });
    doc.apply(&transaction, view_id);
}

fn delete_selection(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Delete);
}

fn delete_selection_noyank(cx: &mut Context) {
    cx.register = Some('_');
    delete_selection_impl(cx, Operation::Delete);
}

fn change_selection(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Change);
}

fn change_selection_noyank(cx: &mut Context) {
    cx.register = Some('_');
    delete_selection_impl(cx, Operation::Change);
}

fn collapse_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let pos = range.cursor(text);
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
}

fn flip_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| Range::new(range.head, range.anchor));
    doc.set_selection(view.id, selection);
}

fn enter_insert_mode(doc: &mut Document) {
    doc.mode = Mode::Insert;
}

// inserts at the start of each selection
fn insert_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| Range::new(range.to(), range.from()));
    doc.set_selection(view.id, selection);
}

// inserts at the end of each selection
fn append_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);
    doc.restore_cursor = true;
    let text = doc.text().slice(..);

    // Make sure there's room at the end of the document if the last
    // selection butts up against it.
    let end = text.len_chars();
    let last_range = doc.selection(view.id).iter().last().unwrap();
    if !last_range.is_empty() && last_range.head == end {
        let transaction = Transaction::change(
            doc.text(),
            std::array::IntoIter::new([(end, end, Some(doc.line_ending.as_str().into()))]),
        );
        doc.apply(&transaction, view.id);
    }

    let selection = doc.selection(view.id).clone().transform(|range| {
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(doc.text().slice(..), range.to()),
        )
    });
    doc.set_selection(view.id, selection);
}

mod cmd {
    use super::*;
    use std::collections::HashMap;

    use helix_view::editor::Action;
    use ui::completers::{self, Completer};

    #[derive(Clone)]
    pub struct TypableCommand {
        pub name: &'static str,
        pub aliases: &'static [&'static str],
        pub doc: &'static str,
        // params, flags, helper, completer
        pub fun: fn(&mut compositor::Context, &[&str], PromptEvent) -> anyhow::Result<()>,
        pub completer: Option<Completer>,
    }

    fn quit(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        // last view and we have unsaved changes
        if cx.editor.tree.views().count() == 1 {
            buffers_remaining_impl(cx.editor)?
        }

        cx.editor.close(view!(cx.editor).id);

        Ok(())
    }

    fn force_quit(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        cx.editor.close(view!(cx.editor).id);

        Ok(())
    }

    fn open(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let path = args.get(0).context("wrong argument count")?;
        let _ = cx.editor.open(path.into(), Action::Replace)?;
        Ok(())
    }

    fn buffer_close(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let view = view!(cx.editor);
        let doc_id = view.doc;
        cx.editor.close_document(doc_id, false)?;
        Ok(())
    }

    fn force_buffer_close(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let view = view!(cx.editor);
        let doc_id = view.doc;
        cx.editor.close_document(doc_id, true)?;
        Ok(())
    }

    fn write_impl<P: AsRef<Path>>(
        cx: &mut compositor::Context,
        path: Option<P>,
    ) -> anyhow::Result<()> {
        let jobs = &mut cx.jobs;
        let (_, doc) = current!(cx.editor);

        if let Some(path) = path {
            doc.set_path(Some(path.as_ref()))
                .context("invalid filepath")?;
        }
        if doc.path().is_none() {
            bail!("cannot write a buffer without a filename");
        }
        let fmt = doc.auto_format().map(|fmt| {
            let shared = fmt.shared();
            let callback = make_format_callback(
                doc.id(),
                doc.version(),
                Modified::SetUnmodified,
                shared.clone(),
            );
            jobs.callback(callback);
            shared
        });
        let future = doc.format_and_save(fmt);
        cx.jobs.add(Job::new(future).wait_before_exiting());
        Ok(())
    }

    fn write(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_impl(cx, args.first())
    }

    fn new_file(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        cx.editor.new_file(Action::Replace);

        Ok(())
    }

    fn format(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);

        if let Some(format) = doc.format() {
            let callback =
                make_format_callback(doc.id(), doc.version(), Modified::LeaveModified, format);
            cx.jobs.callback(callback);
        }

        Ok(())
    }
    fn set_indent_style(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        use IndentStyle::*;

        // If no argument, report current indent style.
        if args.is_empty() {
            let style = doc!(cx.editor).indent_style;
            cx.editor.set_status(match style {
                Tabs => "tabs".into(),
                Spaces(1) => "1 space".into(),
                Spaces(n) if (2..=8).contains(&n) => format!("{} spaces", n),
                _ => "error".into(), // Shouldn't happen.
            });
            return Ok(());
        }

        // Attempt to parse argument as an indent style.
        let style = match args.get(0) {
            Some(arg) if "tabs".starts_with(&arg.to_lowercase()) => Some(Tabs),
            Some(&"0") => Some(Tabs),
            Some(arg) => arg
                .parse::<u8>()
                .ok()
                .filter(|n| (1..=8).contains(n))
                .map(Spaces),
            _ => None,
        };

        let style = style.context("invalid indent style")?;
        let doc = doc_mut!(cx.editor);
        doc.indent_style = style;

        Ok(())
    }

    /// Sets or reports the current document's line ending setting.
    fn set_line_ending(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        use LineEnding::*;

        // If no argument, report current line ending setting.
        if args.is_empty() {
            let line_ending = doc!(cx.editor).line_ending;
            cx.editor.set_status(match line_ending {
                Crlf => "crlf".into(),
                LF => "line feed".into(),
                FF => "form feed".into(),
                CR => "carriage return".into(),
                Nel => "next line".into(),

                // These should never be a document's default line ending.
                VT | LS | PS => "error".into(),
            });

            return Ok(());
        }

        let arg = args
            .get(0)
            .context("argument missing")?
            .to_ascii_lowercase();

        // Attempt to parse argument as a line ending.
        let line_ending = match arg {
            // We check for CR first because it shares a common prefix with CRLF.
            arg if arg.starts_with("cr") => CR,
            arg if arg.starts_with("crlf") => Crlf,
            arg if arg.starts_with("lf") => LF,
            arg if arg.starts_with("ff") => FF,
            arg if arg.starts_with("nel") => Nel,
            _ => bail!("invalid line ending"),
        };

        doc_mut!(cx.editor).line_ending = line_ending;
        Ok(())
    }

    fn earlier(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let uk = args.join(" ").parse::<UndoKind>().map_err(|s| anyhow!(s))?;

        let (view, doc) = current!(cx.editor);
        let success = doc.earlier(view.id, uk);
        if !success {
            cx.editor.set_status("Already at oldest change".to_owned());
        }

        Ok(())
    }

    fn later(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let uk = args.join(" ").parse::<UndoKind>().map_err(|s| anyhow!(s))?;
        let (view, doc) = current!(cx.editor);
        let success = doc.later(view.id, uk);
        if !success {
            cx.editor.set_status("Already at newest change".to_owned());
        }

        Ok(())
    }

    fn write_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_impl(cx, args.first())?;
        quit(cx, &[], event)
    }

    fn force_write_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_impl(cx, args.first())?;
        force_quit(cx, &[], event)
    }

    /// Results an error if there are modified buffers remaining and sets editor error,
    /// otherwise returns `Ok(())`
    pub(super) fn buffers_remaining_impl(editor: &mut Editor) -> anyhow::Result<()> {
        let modified: Vec<_> = editor
            .documents()
            .filter(|doc| doc.is_modified())
            .map(|doc| {
                doc.relative_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into())
            })
            .collect();
        if !modified.is_empty() {
            bail!(
                "{} unsaved buffer(s) remaining: {:?}",
                modified.len(),
                modified
            );
        }
        Ok(())
    }

    fn write_all_impl(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
        quit: bool,
        force: bool,
    ) -> anyhow::Result<()> {
        let mut errors = String::new();

        // save all documents
        for doc in &mut cx.editor.documents.values_mut() {
            if doc.path().is_none() {
                errors.push_str("cannot write a buffer without a filename\n");
                continue;
            }

            // TODO: handle error.
            let handle = doc.save();
            cx.jobs.add(Job::new(handle).wait_before_exiting());
        }

        if quit {
            if !force {
                buffers_remaining_impl(cx.editor)?;
            }

            // close all views
            let views: Vec<_> = cx.editor.tree.views().map(|(view, _)| view.id).collect();
            for view_id in views {
                cx.editor.close(view_id);
            }
        }

        bail!(errors)
    }

    fn write_all(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_all_impl(cx, args, event, false, false)
    }

    fn write_all_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_all_impl(cx, args, event, true, false)
    }

    fn force_write_all_quit(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        write_all_impl(cx, args, event, true, true)
    }

    fn quit_all_impl(
        editor: &mut Editor,
        _args: &[&str],
        _event: PromptEvent,
        force: bool,
    ) -> anyhow::Result<()> {
        if !force {
            buffers_remaining_impl(editor)?;
        }

        // close all views
        let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
        for view_id in views {
            editor.close(view_id);
        }

        Ok(())
    }

    fn quit_all(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        quit_all_impl(&mut cx.editor, args, event, false)
    }

    fn force_quit_all(
        cx: &mut compositor::Context,
        args: &[&str],
        event: PromptEvent,
    ) -> anyhow::Result<()> {
        quit_all_impl(&mut cx.editor, args, event, true)
    }

    fn cquit(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let exit_code = args
            .first()
            .and_then(|code| code.parse::<i32>().ok())
            .unwrap_or(1);
        cx.editor.exit_code = exit_code;

        let views: Vec<_> = cx.editor.tree.views().map(|(view, _)| view.id).collect();
        for view_id in views {
            cx.editor.close(view_id);
        }

        Ok(())
    }

    fn theme(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let theme = args.first().context("theme not provided")?;
        cx.editor.set_theme_from_name(theme)
    }

    fn yank_main_selection_to_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Clipboard)
    }

    fn yank_joined_to_clipboard(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);
        let separator = args
            .first()
            .copied()
            .unwrap_or_else(|| doc.line_ending.as_str());
        yank_joined_to_clipboard_impl(&mut cx.editor, separator, ClipboardType::Clipboard)
    }

    fn yank_main_selection_to_primary_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Selection)
    }

    fn yank_joined_to_primary_clipboard(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);
        let separator = args
            .first()
            .copied()
            .unwrap_or_else(|| doc.line_ending.as_str());
        yank_joined_to_clipboard_impl(&mut cx.editor, separator, ClipboardType::Selection)
    }

    fn paste_clipboard_after(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard)
    }

    fn paste_clipboard_before(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard)
    }

    fn paste_primary_clipboard_after(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection)
    }

    fn paste_primary_clipboard_before(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection)
    }

    fn replace_selections_with_clipboard_impl(
        cx: &mut compositor::Context,
        clipboard_type: ClipboardType,
    ) -> anyhow::Result<()> {
        let (view, doc) = current!(cx.editor);

        match cx.editor.clipboard_provider.get_contents(clipboard_type) {
            Ok(contents) => {
                let selection = doc.selection(view.id);
                let transaction =
                    Transaction::change_by_selection(doc.text(), selection, |range| {
                        (range.from(), range.to(), Some(contents.as_str().into()))
                    });

                doc.apply(&transaction, view.id);
                doc.append_changes_to_history(view.id);
                Ok(())
            }
            Err(e) => Err(e.context("Couldn't get system clipboard contents")),
        }
    }

    fn replace_selections_with_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        replace_selections_with_clipboard_impl(cx, ClipboardType::Clipboard)
    }

    fn replace_selections_with_primary_clipboard(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        replace_selections_with_clipboard_impl(cx, ClipboardType::Selection)
    }

    fn show_clipboard_provider(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        cx.editor
            .set_status(cx.editor.clipboard_provider.name().to_string());
        Ok(())
    }

    fn change_current_directory(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let dir = helix_core::path::expand_tilde(
            args.first()
                .context("target directory not provided")?
                .as_ref(),
        );

        if let Err(e) = std::env::set_current_dir(dir) {
            bail!("Couldn't change the current working directory: {}", e);
        }

        let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
        cx.editor.set_status(format!(
            "Current working directory is now {}",
            cwd.display()
        ));
        Ok(())
    }

    fn show_current_directory(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let cwd = std::env::current_dir().context("Couldn't get the new working directory")?;
        cx.editor
            .set_status(format!("Current working directory is {}", cwd.display()));
        Ok(())
    }

    /// Sets the [`Document`]'s encoding..
    fn set_encoding(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (_, doc) = current!(cx.editor);
        if let Some(label) = args.first() {
            doc.set_encoding(label)
        } else {
            let encoding = doc.encoding().name().to_string();
            cx.editor.set_status(encoding);
            Ok(())
        }
    }

    /// Reload the [`Document`] from its source file.
    fn reload(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (view, doc) = current!(cx.editor);
        doc.reload(view.id)
    }

    fn tree_sitter_scopes(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let pos = doc.selection(view.id).primary().cursor(text);
        let scopes = indent::get_scopes(doc.syntax(), text, pos);
        cx.editor.set_status(format!("scopes: {:?}", &scopes));
        Ok(())
    }

    fn vsplit(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let id = view!(cx.editor).doc;

        if let Some(path) = args.get(0) {
            cx.editor.open(path.into(), Action::VerticalSplit)?;
        } else {
            cx.editor.switch(id, Action::VerticalSplit);
        }

        Ok(())
    }

    fn hsplit(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let id = view!(cx.editor).doc;

        if let Some(path) = args.get(0) {
            cx.editor.open(path.into(), Action::HorizontalSplit)?;
        } else {
            cx.editor.switch(id, Action::HorizontalSplit);
        }

        Ok(())
    }

    fn tutor(
        cx: &mut compositor::Context,
        _args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        let path = helix_core::runtime_dir().join("tutor.txt");
        cx.editor.open(path, Action::Replace)?;
        // Unset path to prevent accidentally saving to the original tutor file.
        doc_mut!(cx.editor).set_path(None)?;
        Ok(())
    }

    pub(super) fn goto_line_number(
        cx: &mut compositor::Context,
        args: &[&str],
        _event: PromptEvent,
    ) -> anyhow::Result<()> {
        if args.is_empty() {
            bail!("Line number required");
        }

        let line = args[0].parse::<usize>()?;

        goto_line_impl(&mut cx.editor, NonZeroUsize::new(line));

        let (view, doc) = current!(cx.editor);

        view.ensure_cursor_in_view(doc, line);

        Ok(())
    }

    pub const TYPABLE_COMMAND_LIST: &[TypableCommand] = &[
        TypableCommand {
            name: "quit",
            aliases: &["q"],
            doc: "Close the current view.",
            fun: quit,
            completer: None,
        },
        TypableCommand {
            name: "quit!",
            aliases: &["q!"],
            doc: "Close the current view forcefully (ignoring unsaved changes).",
            fun: force_quit,
            completer: None,
        },
        TypableCommand {
            name: "open",
            aliases: &["o"],
            doc: "Open a file from disk into the current view.",
            fun: open,
            completer: Some(completers::filename),
        },
        TypableCommand {
          name: "buffer-close",
          aliases: &["bc", "bclose"],
          doc: "Close the current buffer.",
          fun: buffer_close,
          completer: None, // FIXME: buffer completer
        },
        TypableCommand {
          name: "buffer-close!",
          aliases: &["bc!", "bclose!"],
          doc: "Close the current buffer forcefully (ignoring unsaved changes).",
          fun: force_buffer_close,
          completer: None, // FIXME: buffer completer
        },
        TypableCommand {
            name: "write",
            aliases: &["w"],
            doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)",
            fun: write,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "new",
            aliases: &["n"],
            doc: "Create a new scratch buffer.",
            fun: new_file,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "format",
            aliases: &["fmt"],
            doc: "Format the file using a formatter.",
            fun: format,
            completer: None,
        },
        TypableCommand {
            name: "indent-style",
            aliases: &[],
            doc: "Set the indentation style for editing. ('t' for tabs or 1-8 for number of spaces.)",
            fun: set_indent_style,
            completer: None,
        },
        TypableCommand {
            name: "line-ending",
            aliases: &[],
            doc: "Set the document's default line ending. Options: crlf, lf, cr, ff, nel.",
            fun: set_line_ending,
            completer: None,
        },
        TypableCommand {
            name: "earlier",
            aliases: &["ear"],
            doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
            fun: earlier,
            completer: None,
        },
        TypableCommand {
            name: "later",
            aliases: &["lat"],
            doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.",
            fun: later,
            completer: None,
        },
        TypableCommand {
            name: "write-quit",
            aliases: &["wq", "x"],
            doc: "Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt)",
            fun: write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-quit!",
            aliases: &["wq!", "x!"],
            doc: "Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt)",
            fun: force_write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-all",
            aliases: &["wa"],
            doc: "Write changes from all views to disk.",
            fun: write_all,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all",
            aliases: &["wqa", "xa"],
            doc: "Write changes from all views to disk and close all views.",
            fun: write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all!",
            aliases: &["wqa!", "xa!"],
            doc: "Write changes from all views to disk and close all views forcefully (ignoring unsaved changes).",
            fun: force_write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "quit-all",
            aliases: &["qa"],
            doc: "Close all views.",
            fun: quit_all,
            completer: None,
        },
        TypableCommand {
            name: "quit-all!",
            aliases: &["qa!"],
            doc: "Close all views forcefully (ignoring unsaved changes).",
            fun: force_quit_all,
            completer: None,
        },
        TypableCommand {
            name: "cquit",
            aliases: &["cq"],
            doc: "Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2).",
            fun: cquit,
            completer: None,
        },
        TypableCommand {
            name: "theme",
            aliases: &[],
            doc: "Change the theme of current view. Requires theme name as argument (:theme <name>)",
            fun: theme,
            completer: Some(completers::theme),
        },
        TypableCommand {
            name: "clipboard-yank",
            aliases: &[],
            doc: "Yank main selection into system clipboard.",
            fun: yank_main_selection_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-yank-join",
            aliases: &[],
            doc: "Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank",
            aliases: &[],
            doc: "Yank main selection into system primary clipboard.",
            fun: yank_main_selection_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-yank-join",
            aliases: &[],
            doc: "Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-after",
            aliases: &[],
            doc: "Paste system clipboard after selections.",
            fun: paste_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-before",
            aliases: &[],
            doc: "Paste system clipboard before selections.",
            fun: paste_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-replace",
            aliases: &[],
            doc: "Replace selections with content of system clipboard.",
            fun: replace_selections_with_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-after",
            aliases: &[],
            doc: "Paste primary clipboard after selections.",
            fun: paste_primary_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-before",
            aliases: &[],
            doc: "Paste primary clipboard before selections.",
            fun: paste_primary_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "primary-clipboard-paste-replace",
            aliases: &[],
            doc: "Replace selections with content of system primary clipboard.",
            fun: replace_selections_with_primary_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "show-clipboard-provider",
            aliases: &[],
            doc: "Show clipboard provider name in status bar.",
            fun: show_clipboard_provider,
            completer: None,
        },
        TypableCommand {
            name: "change-current-directory",
            aliases: &["cd"],
            doc: "Change the current working directory (:cd <dir>).",
            fun: change_current_directory,
            completer: Some(completers::directory),
        },
        TypableCommand {
            name: "show-directory",
            aliases: &["pwd"],
            doc: "Show the current working directory.",
            fun: show_current_directory,
            completer: None,
        },
        TypableCommand {
            name: "encoding",
            aliases: &[],
            doc: "Set encoding based on `https://encoding.spec.whatwg.org`",
            fun: set_encoding,
            completer: None,
        },
        TypableCommand {
            name: "reload",
            aliases: &[],
            doc: "Discard changes and reload from the source file.",
            fun: reload,
            completer: None,
        },
        TypableCommand {
            name: "tree-sitter-scopes",
            aliases: &[],
            doc: "Display tree sitter scopes, primarily for theming and development.",
            fun: tree_sitter_scopes,
            completer: None,
        },
        TypableCommand {
            name: "vsplit",
            aliases: &["vs"],
            doc: "Open the file in a vertical split.",
            fun: vsplit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "hsplit",
            aliases: &["hs", "sp"],
            doc: "Open the file in a horizontal split.",
            fun: hsplit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "tutor",
            aliases: &[],
            doc: "Open the tutorial.",
            fun: tutor,
            completer: None,
        },
        TypableCommand {
            name: "goto",
            aliases: &["g"],
            doc: "Go to line number.",
            fun: goto_line_number,
            completer: None,
        }
    ];

    pub static COMMANDS: Lazy<HashMap<&'static str, &'static TypableCommand>> = Lazy::new(|| {
        TYPABLE_COMMAND_LIST
            .iter()
            .flat_map(|cmd| {
                std::iter::once((cmd.name, cmd))
                    .chain(cmd.aliases.iter().map(move |&alias| (alias, cmd)))
            })
            .collect()
    });
}

fn command_mode(cx: &mut Context) {
    let mut prompt = Prompt::new(
        ":".into(),
        Some(':'),
        |input: &str| {
            // we use .this over split_whitespace() because we care about empty segments
            let parts = input.split(' ').collect::<Vec<&str>>();

            // simple heuristic: if there's no just one part, complete command name.
            // if there's a space, per command completion kicks in.
            if parts.len() <= 1 {
                let end = 0..;
                cmd::TYPABLE_COMMAND_LIST
                    .iter()
                    .filter(|command| command.name.contains(input))
                    .map(|command| (end.clone(), Cow::Borrowed(command.name)))
                    .collect()
            } else {
                let part = parts.last().unwrap();

                if let Some(cmd::TypableCommand {
                    completer: Some(completer),
                    ..
                }) = cmd::COMMANDS.get(parts[0])
                {
                    completer(part)
                        .into_iter()
                        .map(|(range, file)| {
                            // offset ranges to input
                            let offset = input.len() - part.len();
                            let range = (range.start + offset)..;
                            (range, file)
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }, // completion
        move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
            if event != PromptEvent::Validate {
                return;
            }

            let parts = input.split_whitespace().collect::<Vec<&str>>();
            if parts.is_empty() {
                return;
            }

            // If command is numeric, interpret as line number and go there.
            if parts.len() == 1 && parts[0].parse::<usize>().ok().is_some() {
                if let Err(e) = cmd::goto_line_number(cx, &parts[0..], event) {
                    cx.editor.set_error(format!("{}", e));
                }
                return;
            }

            // Handle typable commands
            if let Some(cmd) = cmd::COMMANDS.get(parts[0]) {
                if let Err(e) = (cmd.fun)(cx, &parts[1..], event) {
                    cx.editor.set_error(format!("{}", e));
                }
            } else {
                cx.editor
                    .set_error(format!("no such command: '{}'", parts[0]));
            };
        },
    );
    prompt.doc_fn = Box::new(|input: &str| {
        let part = input.split(' ').next().unwrap_or_default();

        if let Some(cmd::TypableCommand { doc, .. }) = cmd::COMMANDS.get(part) {
            return Some(doc);
        }

        None
    });

    cx.push_layer(Box::new(prompt));
}

fn file_picker(cx: &mut Context) {
    let root = find_root(None).unwrap_or_else(|| PathBuf::from("./"));
    let picker = ui::file_picker(root, &cx.editor.config);
    cx.push_layer(Box::new(picker));
}

fn buffer_picker(cx: &mut Context) {
    let current = view!(cx.editor).doc;

    struct BufferMeta {
        id: DocumentId,
        path: Option<PathBuf>,
        is_modified: bool,
        is_current: bool,
    }

    impl BufferMeta {
        fn format(&self) -> Cow<str> {
            let path = self
                .path
                .as_deref()
                .map(helix_core::path::get_relative_path);
            let path = match path.as_deref().and_then(Path::to_str) {
                Some(path) => path,
                None => return Cow::Borrowed(SCRATCH_BUFFER_NAME),
            };

            let mut flags = Vec::new();
            if self.is_modified {
                flags.push("+");
            }
            if self.is_current {
                flags.push("*");
            }

            let flag = if flags.is_empty() {
                "".into()
            } else {
                format!(" ({})", flags.join(""))
            };
            Cow::Owned(format!("{}{}", path, flag))
        }
    }

    let new_meta = |doc: &Document| BufferMeta {
        id: doc.id(),
        path: doc.path().cloned(),
        is_modified: doc.is_modified(),
        is_current: doc.id() == current,
    };

    let picker = FilePicker::new(
        cx.editor
            .documents
            .iter()
            .map(|(_, doc)| new_meta(doc))
            .collect(),
        BufferMeta::format,
        |editor: &mut Editor, meta, _action| {
            editor.switch(meta.id, Action::Replace);
        },
        |editor, meta| {
            let doc = &editor.documents.get(&meta.id)?;
            let &view_id = doc.selections().keys().next()?;
            let line = doc
                .selection(view_id)
                .primary()
                .cursor_line(doc.text().slice(..));
            Some((meta.path.clone()?, Some((line, line))))
        },
    );
    cx.push_layer(Box::new(picker));
}

fn symbol_picker(cx: &mut Context) {
    fn nested_to_flat(
        list: &mut Vec<lsp::SymbolInformation>,
        file: &lsp::TextDocumentIdentifier,
        symbol: lsp::DocumentSymbol,
    ) {
        #[allow(deprecated)]
        list.push(lsp::SymbolInformation {
            name: symbol.name,
            kind: symbol.kind,
            tags: symbol.tags,
            deprecated: symbol.deprecated,
            location: lsp::Location::new(file.uri.clone(), symbol.selection_range),
            container_name: None,
        });
        for child in symbol.children.into_iter().flatten() {
            nested_to_flat(list, file, child);
        }
    }
    let (_, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };
    let offset_encoding = language_server.offset_encoding();

    let future = language_server.document_symbols(doc.identifier());

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::DocumentSymbolResponse>| {
            if let Some(symbols) = response {
                // lsp has two ways to represent symbols (flat/nested)
                // convert the nested variant to flat, so that we have a homogeneous list
                let symbols = match symbols {
                    lsp::DocumentSymbolResponse::Flat(symbols) => symbols,
                    lsp::DocumentSymbolResponse::Nested(symbols) => {
                        let (_view, doc) = current!(editor);
                        let mut flat_symbols = Vec::new();
                        for symbol in symbols {
                            nested_to_flat(&mut flat_symbols, &doc.identifier(), symbol)
                        }
                        flat_symbols
                    }
                };

                let mut picker = FilePicker::new(
                    symbols,
                    |symbol| (&symbol.name).into(),
                    move |editor: &mut Editor, symbol, _action| {
                        push_jump(editor);
                        let (view, doc) = current!(editor);

                        if let Some(range) =
                            lsp_range_to_range(doc.text(), symbol.location.range, offset_encoding)
                        {
                            // we flip the range so that the cursor sits on the start of the symbol
                            // (for example start of the function).
                            doc.set_selection(view.id, Selection::single(range.head, range.anchor));
                            align_view(doc, view, Align::Center);
                        }
                    },
                    move |_editor, symbol| {
                        let path = symbol.location.uri.to_file_path().unwrap();
                        let line = Some((
                            symbol.location.range.start.line as usize,
                            symbol.location.range.end.line as usize,
                        ));
                        Some((path, line))
                    },
                );
                picker.truncate_start = false;
                compositor.push(Box::new(picker))
            }
        },
    )
}

fn workspace_symbol_picker(cx: &mut Context) {
    let (_, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };
    let offset_encoding = language_server.offset_encoding();

    let future = language_server.workspace_symbols("".to_string());

    let current_path = doc_mut!(cx.editor).path().cloned();
    cx.callback(
        future,
        move |_editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<Vec<lsp::SymbolInformation>>| {
            if let Some(symbols) = response {
                let mut picker = FilePicker::new(
                    symbols,
                    move |symbol| {
                        let path = symbol.location.uri.to_file_path().unwrap();
                        if current_path.as_ref().map(|p| p == &path).unwrap_or(false) {
                            (&symbol.name).into()
                        } else {
                            let relative_path = helix_core::path::get_relative_path(path.as_path())
                                .to_str()
                                .unwrap()
                                .to_owned();
                            format!("{} ({})", &symbol.name, relative_path).into()
                        }
                    },
                    move |editor: &mut Editor, symbol, action| {
                        let path = symbol.location.uri.to_file_path().unwrap();
                        editor.open(path, action).expect("editor.open failed");
                        let (view, doc) = current!(editor);

                        if let Some(range) =
                            lsp_range_to_range(doc.text(), symbol.location.range, offset_encoding)
                        {
                            // we flip the range so that the cursor sits on the start of the symbol
                            // (for example start of the function).
                            doc.set_selection(view.id, Selection::single(range.head, range.anchor));
                            align_view(doc, view, Align::Center);
                        }
                    },
                    move |_editor, symbol| {
                        let path = symbol.location.uri.to_file_path().unwrap();
                        let line = Some((
                            symbol.location.range.start.line as usize,
                            symbol.location.range.end.line as usize,
                        ));
                        Some((path, line))
                    },
                );
                picker.truncate_start = false;
                compositor.push(Box::new(picker))
            }
        },
    )
}

pub fn code_action(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let range = range_to_lsp_range(
        doc.text(),
        doc.selection(view.id).primary(),
        language_server.offset_encoding(),
    );

    let future = language_server.code_actions(doc.identifier(), range);
    let offset_encoding = language_server.offset_encoding();

    cx.callback(
        future,
        move |_editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::CodeActionResponse>| {
            if let Some(actions) = response {
                let picker = Picker::new(
                    true,
                    actions,
                    |action| match action {
                        lsp::CodeActionOrCommand::CodeAction(action) => {
                            action.title.as_str().into()
                        }
                        lsp::CodeActionOrCommand::Command(command) => command.title.as_str().into(),
                    },
                    move |editor, code_action, _action| match code_action {
                        lsp::CodeActionOrCommand::Command(command) => {
                            log::debug!("code action command: {:?}", command);
                            editor.set_error(String::from("Handling code action command is not implemented yet, see https://github.com/helix-editor/helix/issues/183"));
                        }
                        lsp::CodeActionOrCommand::CodeAction(code_action) => {
                            log::debug!("code action: {:?}", code_action);
                            if let Some(ref workspace_edit) = code_action.edit {
                                apply_workspace_edit(editor, offset_encoding, workspace_edit)
                            }
                        }
                    },
                );
                compositor.push(Box::new(picker))
            }
        },
    )
}

pub fn apply_document_resource_op(op: &lsp::ResourceOp) -> std::io::Result<()> {
    use lsp::ResourceOp;
    use std::fs;
    match op {
        ResourceOp::Create(op) => {
            let path = op.uri.to_file_path().unwrap();
            let ignore_if_exists = if let Some(options) = &op.options {
                !options.overwrite.unwrap_or(false) && options.ignore_if_exists.unwrap_or(false)
            } else {
                false
            };
            if ignore_if_exists && path.exists() {
                Ok(())
            } else {
                fs::write(&path, [])
            }
        }
        ResourceOp::Delete(op) => {
            let path = op.uri.to_file_path().unwrap();
            if path.is_dir() {
                let recursive = if let Some(options) = &op.options {
                    options.recursive.unwrap_or(false)
                } else {
                    false
                };
                if recursive {
                    fs::remove_dir_all(&path)
                } else {
                    fs::remove_dir(&path)
                }
            } else if path.is_file() {
                fs::remove_file(&path)
            } else {
                Ok(())
            }
        }
        ResourceOp::Rename(op) => {
            let from = op.old_uri.to_file_path().unwrap();
            let to = op.new_uri.to_file_path().unwrap();
            let ignore_if_exists = if let Some(options) = &op.options {
                !options.overwrite.unwrap_or(false) && options.ignore_if_exists.unwrap_or(false)
            } else {
                false
            };
            if ignore_if_exists && to.exists() {
                Ok(())
            } else {
                fs::rename(&from, &to)
            }
        }
    }
}

fn apply_workspace_edit(
    editor: &mut Editor,
    offset_encoding: OffsetEncoding,
    workspace_edit: &lsp::WorkspaceEdit,
) {
    let mut apply_edits = |uri: &helix_lsp::Url, text_edits: Vec<lsp::TextEdit>| {
        let path = uri
            .to_file_path()
            .expect("unable to convert URI to filepath");

        let current_view_id = view!(editor).id;
        let doc_id = editor.open(path, Action::Load).unwrap();
        let doc = editor
            .document_mut(doc_id)
            .expect("Document for document_changes not found");

        // Need to determine a view for apply/append_changes_to_history
        let selections = doc.selections();
        let view_id = if selections.contains_key(&current_view_id) {
            // use current if possible
            current_view_id
        } else {
            // Hack: we take the first available view_id
            selections
                .keys()
                .next()
                .copied()
                .expect("No view_id available")
        };

        let transaction = helix_lsp::util::generate_transaction_from_edits(
            doc.text(),
            text_edits,
            offset_encoding,
        );
        doc.apply(&transaction, view_id);
        doc.append_changes_to_history(view_id);
    };

    if let Some(ref changes) = workspace_edit.changes {
        log::debug!("workspace changes: {:?}", changes);
        for (uri, text_edits) in changes {
            let text_edits = text_edits.to_vec();
            apply_edits(uri, text_edits);
        }
        return;
        // Not sure if it works properly, it'll be safer to just panic here to avoid breaking some parts of code on which code actions will be used
        // TODO: find some example that uses workspace changes, and test it
        // for (url, edits) in changes.iter() {
        //     let file_path = url.origin().ascii_serialization();
        //     let file_path = std::path::PathBuf::from(file_path);
        //     let file = std::fs::File::open(file_path).unwrap();
        //     let mut text = Rope::from_reader(file).unwrap();
        //     let transaction = edits_to_changes(&text, edits);
        //     transaction.apply(&mut text);
        // }
    }

    if let Some(ref document_changes) = workspace_edit.document_changes {
        match document_changes {
            lsp::DocumentChanges::Edits(document_edits) => {
                for document_edit in document_edits {
                    let edits = document_edit
                        .edits
                        .iter()
                        .map(|edit| match edit {
                            lsp::OneOf::Left(text_edit) => text_edit,
                            lsp::OneOf::Right(annotated_text_edit) => {
                                &annotated_text_edit.text_edit
                            }
                        })
                        .cloned()
                        .collect();
                    apply_edits(&document_edit.text_document.uri, edits);
                }
            }
            lsp::DocumentChanges::Operations(operations) => {
                log::debug!("document changes - operations: {:?}", operations);
                for operateion in operations {
                    match operateion {
                        lsp::DocumentChangeOperation::Op(op) => {
                            apply_document_resource_op(op).unwrap();
                        }

                        lsp::DocumentChangeOperation::Edit(document_edit) => {
                            let edits = document_edit
                                .edits
                                .iter()
                                .map(|edit| match edit {
                                    lsp::OneOf::Left(text_edit) => text_edit,
                                    lsp::OneOf::Right(annotated_text_edit) => {
                                        &annotated_text_edit.text_edit
                                    }
                                })
                                .cloned()
                                .collect();
                            apply_edits(&document_edit.text_document.uri, edits);
                        }
                    }
                }
            }
        }
    }
}

fn last_picker(cx: &mut Context) {
    // TODO: last picker does not seem to work well with buffer_picker
    cx.callback = Some(Box::new(|compositor: &mut Compositor| {
        if let Some(picker) = compositor.last_picker.take() {
            compositor.push(picker);
        }
        // XXX: figure out how to show error when no last picker lifetime
        // cx.editor.set_error("no last picker".to_owned())
    }));
}

// I inserts at the first nonwhitespace character of each line with a selection
fn prepend_to_line(cx: &mut Context) {
    goto_first_nonwhitespace(cx);
    let doc = doc_mut!(cx.editor);
    enter_insert_mode(doc);
}

// A inserts at the end of each line with a selection
fn append_to_line(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let text = doc.text().slice(..);
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
}

/// Sometimes when applying formatting changes we want to mark the buffer as unmodified, for
/// example because we just applied the same changes while saving.
enum Modified {
    SetUnmodified,
    LeaveModified,
}

// Creates an LspCallback that waits for formatting changes to be computed. When they're done,
// it applies them, but only if the doc hasn't changed.
//
// TODO: provide some way to cancel this, probably as part of a more general job cancellation
// scheme
async fn make_format_callback(
    doc_id: DocumentId,
    doc_version: i32,
    modified: Modified,
    format: impl Future<Output = helix_lsp::util::LspFormatting> + Send + 'static,
) -> anyhow::Result<job::Callback> {
    let format = format.await;
    let call: job::Callback = Box::new(move |editor: &mut Editor, _compositor: &mut Compositor| {
        let view_id = view!(editor).id;
        if let Some(doc) = editor.document_mut(doc_id) {
            if doc.version() == doc_version {
                doc.apply(&Transaction::from(format), view_id);
                doc.append_changes_to_history(view_id);
                if let Modified::SetUnmodified = modified {
                    doc.reset_modified();
                }
            } else {
                log::info!("discarded formatting changes because the document changed");
            }
        }
    });
    Ok(call)
}

enum Open {
    Below,
    Above,
}

fn open(cx: &mut Context, open: Open) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);

    let text = doc.text().slice(..);
    let contents = doc.text();
    let selection = doc.selection(view.id);

    let mut ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
        let line = range.cursor_line(text);

        let line = match open {
            // adjust position to the end of the line (next line - 1)
            Open::Below => line + 1,
            // adjust position to the end of the previous line (current line - 1)
            Open::Above => line,
        };

        // Index to insert newlines after, as well as the char width
        // to use to compensate for those inserted newlines.
        let (line_end_index, line_end_offset_width) = if line == 0 {
            (0, 0)
        } else {
            (
                line_end_char_index(&doc.text().slice(..), line.saturating_sub(1)),
                doc.line_ending.len_chars(),
            )
        };

        // TODO: share logic with insert_newline for indentation
        let indent_level = indent::suggested_indent_for_pos(
            doc.language_config(),
            doc.syntax(),
            text,
            line_end_index,
            true,
        );
        let indent = doc.indent_unit().repeat(indent_level);
        let indent_len = indent.len();
        let mut text = String::with_capacity(1 + indent_len);
        text.push_str(doc.line_ending.as_str());
        text.push_str(&indent);
        let text = text.repeat(count);

        // calculate new selection ranges
        let pos = offs + line_end_index + line_end_offset_width;
        for i in 0..count {
            // pos                    -> beginning of reference line,
            // + (i * (1+indent_len)) -> beginning of i'th line from pos
            // + indent_len ->        -> indent for i'th line
            ranges.push(Range::point(pos + (i * (1 + indent_len)) + indent_len));
        }

        offs += text.chars().count();

        (line_end_index, line_end_index, Some(text.into()))
    });

    transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));

    doc.apply(&transaction, view.id);
}

// o inserts a new line after each line with a selection
fn open_below(cx: &mut Context) {
    open(cx, Open::Below)
}

// O inserts a new line before each line with a selection
fn open_above(cx: &mut Context) {
    open(cx, Open::Above)
}

fn normal_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    if doc.mode == Mode::Normal {
        return;
    }

    doc.mode = Mode::Normal;

    doc.append_changes_to_history(view.id);

    // if leaving append mode, move cursor back by 1
    if doc.restore_cursor {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id).clone().transform(|range| {
            Range::new(
                range.from(),
                graphemes::prev_grapheme_boundary(text, range.to()),
            )
        });
        doc.set_selection(view.id, selection);

        doc.restore_cursor = false;
    }
}

// Store a jump on the jumplist.
fn push_jump(editor: &mut Editor) {
    let (view, doc) = current!(editor);
    let jump = (doc.id(), doc.selection(view.id).clone());
    view.jumps.push(jump);
}

fn goto_line(cx: &mut Context) {
    goto_line_impl(&mut cx.editor, cx.count)
}

fn goto_line_impl(editor: &mut Editor, count: Option<NonZeroUsize>) {
    if let Some(count) = count {
        push_jump(editor);

        let (view, doc) = current!(editor);
        let max_line = if doc.text().line(doc.text().len_lines() - 1).len_chars() == 0 {
            // If the last line is blank, don't jump to it.
            doc.text().len_lines().saturating_sub(2)
        } else {
            doc.text().len_lines() - 1
        };
        let line_idx = std::cmp::min(count.get() - 1, max_line);
        let text = doc.text().slice(..);
        let pos = doc.text().line_to_char(line_idx);
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, pos, doc.mode == Mode::Select));
        doc.set_selection(view.id, selection);
    }
}

fn goto_last_line(cx: &mut Context) {
    push_jump(cx.editor);

    let (view, doc) = current!(cx.editor);
    let line_idx = if doc.text().line(doc.text().len_lines() - 1).len_chars() == 0 {
        // If the last line is blank, don't jump to it.
        doc.text().len_lines().saturating_sub(2)
    } else {
        doc.text().len_lines() - 1
    };
    let text = doc.text().slice(..);
    let pos = doc.text().line_to_char(line_idx);
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, doc.mode == Mode::Select));
    doc.set_selection(view.id, selection);
}

fn goto_last_accessed_file(cx: &mut Context) {
    let alternate_file = view!(cx.editor).last_accessed_doc;
    if let Some(alt) = alternate_file {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last accessed buffer".to_owned())
    }
}

fn goto_last_modification(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let pos = doc.history.get_mut().last_edit_pos();
    let text = doc.text().slice(..);
    if let Some(pos) = pos {
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, pos, doc.mode == Mode::Select));
        doc.set_selection(view.id, selection);
    }
}

fn select_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    // Make sure end-of-document selections are also 1-width.
    // (With the exception of being in an empty document, of course.)
    let selection = doc.selection(view.id).clone().transform(|range| {
        if range.is_empty() && range.head == text.len_chars() {
            Range::new(
                graphemes::prev_grapheme_boundary(text, range.anchor),
                range.head,
            )
        } else {
            range
        }
    });
    doc.set_selection(view.id, selection);

    doc_mut!(cx.editor).mode = Mode::Select;
}

fn exit_select_mode(cx: &mut Context) {
    let doc = doc_mut!(cx.editor);
    if doc.mode == Mode::Select {
        doc.mode = Mode::Normal;
    }
}

fn goto_impl(
    editor: &mut Editor,
    compositor: &mut Compositor,
    locations: Vec<lsp::Location>,
    offset_encoding: OffsetEncoding,
) {
    push_jump(editor);

    fn jump_to(
        editor: &mut Editor,
        location: &lsp::Location,
        offset_encoding: OffsetEncoding,
        action: Action,
    ) {
        let path = location
            .uri
            .to_file_path()
            .expect("unable to convert URI to filepath");
        let _id = editor.open(path, action).expect("editor.open failed");
        let (view, doc) = current!(editor);
        let definition_pos = location.range.start;
        // TODO: convert inside server
        let new_pos =
            if let Some(new_pos) = lsp_pos_to_pos(doc.text(), definition_pos, offset_encoding) {
                new_pos
            } else {
                return;
            };
        doc.set_selection(view.id, Selection::point(new_pos));
        align_view(doc, view, Align::Center);
    }

    let cwdir = std::env::current_dir().expect("couldn't determine current directory");

    match locations.as_slice() {
        [location] => {
            jump_to(editor, location, offset_encoding, Action::Replace);
        }
        [] => {
            editor.set_error("No definition found.".to_string());
        }
        _locations => {
            let picker = FilePicker::new(
                locations,
                move |location| {
                    let file: Cow<'_, str> = (location.uri.scheme() == "file")
                        .then(|| {
                            location
                                .uri
                                .to_file_path()
                                .map(|path| {
                                    // strip root prefix
                                    path.strip_prefix(&cwdir)
                                        .map(|path| path.to_path_buf())
                                        .unwrap_or(path)
                                })
                                .ok()
                                .and_then(|path| path.to_str().map(|path| path.to_owned().into()))
                        })
                        .flatten()
                        .unwrap_or_else(|| location.uri.as_str().into());
                    let line = location.range.start.line;
                    format!("{}:{}", file, line).into()
                },
                move |editor: &mut Editor, location, action| {
                    jump_to(editor, location, offset_encoding, action)
                },
                |_editor, location| {
                    let path = location.uri.to_file_path().unwrap();
                    let line = Some((
                        location.range.start.line as usize,
                        location.range.end.line as usize,
                    ));
                    Some((path, line))
                },
            );
            compositor.push(Box::new(picker));
        }
    }
}

fn goto_definition(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_definition(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::GotoDefinitionResponse>| {
            let items = match response {
                Some(lsp::GotoDefinitionResponse::Scalar(location)) => vec![location],
                Some(lsp::GotoDefinitionResponse::Array(locations)) => locations,
                Some(lsp::GotoDefinitionResponse::Link(locations)) => locations
                    .into_iter()
                    .map(|location_link| lsp::Location {
                        uri: location_link.target_uri,
                        range: location_link.target_range,
                    })
                    .collect(),
                None => Vec::new(),
            };

            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

fn goto_type_definition(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_type_definition(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::GotoDefinitionResponse>| {
            let items = match response {
                Some(lsp::GotoDefinitionResponse::Scalar(location)) => vec![location],
                Some(lsp::GotoDefinitionResponse::Array(locations)) => locations,
                Some(lsp::GotoDefinitionResponse::Link(locations)) => locations
                    .into_iter()
                    .map(|location_link| lsp::Location {
                        uri: location_link.target_uri,
                        range: location_link.target_range,
                    })
                    .collect(),
                None => Vec::new(),
            };

            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

fn goto_implementation(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_implementation(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::GotoDefinitionResponse>| {
            let items = match response {
                Some(lsp::GotoDefinitionResponse::Scalar(location)) => vec![location],
                Some(lsp::GotoDefinitionResponse::Array(locations)) => locations,
                Some(lsp::GotoDefinitionResponse::Link(locations)) => locations
                    .into_iter()
                    .map(|location_link| lsp::Location {
                        uri: location_link.target_uri,
                        range: location_link.target_range,
                    })
                    .collect(),
                None => Vec::new(),
            };

            goto_impl(editor, compositor, items, offset_encoding);
        },
    );
}

fn goto_reference(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        offset_encoding,
    );

    let future = language_server.goto_reference(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              items: Option<Vec<lsp::Location>>| {
            goto_impl(
                editor,
                compositor,
                items.unwrap_or_default(),
                offset_encoding,
            );
        },
    );
}

fn goto_pos(editor: &mut Editor, pos: usize) {
    push_jump(editor);

    let (view, doc) = current!(editor);

    doc.set_selection(view.id, Selection::point(pos));
    align_view(doc, view, Align::Center);
}

fn goto_first_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (_, doc) = current!(editor);

    let pos = match doc.diagnostics().first() {
        Some(diag) => diag.range.start,
        None => return,
    };

    goto_pos(editor, pos);
}

fn goto_last_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (_, doc) = current!(editor);

    let pos = match doc.diagnostics().last() {
        Some(diag) => diag.range.start,
        None => return,
    };

    goto_pos(editor, pos);
}

fn goto_next_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);

    let cursor_pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));

    let diag = doc
        .diagnostics()
        .iter()
        .find(|diag| diag.range.start > cursor_pos)
        .or_else(|| doc.diagnostics().first());

    let pos = match diag {
        Some(diag) => diag.range.start,
        None => return,
    };

    goto_pos(editor, pos);
}

fn goto_prev_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);

    let cursor_pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));

    let diag = doc
        .diagnostics()
        .iter()
        .rev()
        .find(|diag| diag.range.start < cursor_pos)
        .or_else(|| doc.diagnostics().last());

    let pos = match diag {
        Some(diag) => diag.range.start,
        None => return,
    };

    goto_pos(editor, pos);
}

fn signature_help(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        language_server.offset_encoding(),
    );

    let future = language_server.text_document_signature_help(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |_editor: &mut Editor,
              _compositor: &mut Compositor,
              response: Option<lsp::SignatureHelp>| {
            if let Some(signature_help) = response {
                log::info!("{:?}", signature_help);
                // signatures
                // active_signature
                // active_parameter
                // render as:

                // signature
                // ----------
                // doc

                // with active param highlighted
            }
        },
    );
}

// NOTE: Transactions in this module get appended to history when we switch back to normal mode.
pub mod insert {
    use super::*;
    pub type Hook = fn(&Rope, &Selection, char) -> Option<Transaction>;
    pub type PostHook = fn(&mut Context, char);

    // It trigger completion when idle timer reaches deadline
    // Only trigger completion if the word under cursor is longer than n characters
    pub fn idle_completion(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let cursor = doc.selection(view.id).primary().cursor(text);

        use helix_core::chars::char_is_word;
        let mut iter = text.chars_at(cursor);
        iter.reverse();
        for _ in 0..cx.editor.config.completion_trigger_len {
            match iter.next() {
                Some(c) if char_is_word(c) => {}
                _ => return,
            }
        }
        super::completion(cx);
    }

    fn language_server_completion(cx: &mut Context, ch: char) {
        // if ch matches completion char, trigger completion
        let doc = doc_mut!(cx.editor);
        let language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };

        let capabilities = language_server.capabilities();

        if let Some(lsp::CompletionOptions {
            trigger_characters: Some(triggers),
            ..
        }) = &capabilities.completion_provider
        {
            // TODO: what if trigger is multiple chars long
            if triggers.iter().any(|trigger| trigger.contains(ch)) {
                cx.editor.clear_idle_timer();
                super::completion(cx);
            }
        }
    }

    fn signature_help(cx: &mut Context, ch: char) {
        // if ch matches signature_help char, trigger
        let doc = doc_mut!(cx.editor);
        let language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };

        let capabilities = language_server.capabilities();

        if let lsp::ServerCapabilities {
            signature_help_provider:
                Some(lsp::SignatureHelpOptions {
                    trigger_characters: Some(triggers),
                    // TODO: retrigger_characters
                    ..
                }),
            ..
        } = capabilities
        {
            // TODO: what if trigger is multiple chars long
            let is_trigger = triggers.iter().any(|trigger| trigger.contains(ch));

            if is_trigger {
                super::signature_help(cx);
            }
        }

        // SignatureHelp {
        // signatures: [
        //  SignatureInformation {
        //      label: "fn open(&mut self, path: PathBuf, action: Action) -> Result<DocumentId, Error>",
        //      documentation: None,
        //      parameters: Some(
        //          [ParameterInformation { label: Simple("path: PathBuf"), documentation: None },
        //          ParameterInformation { label: Simple("action: Action"), documentation: None }]
        //      ),
        //      active_parameter: Some(0)
        //  }
        // ],
        // active_signature: None, active_parameter: Some(0)
        // }
    }

    // The default insert hook: simply insert the character
    #[allow(clippy::unnecessary_wraps)] // need to use Option<> because of the Hook signature
    fn insert(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
        let t = Tendril::from_char(ch);
        let transaction = Transaction::insert(doc, selection, t);
        Some(transaction)
    }

    use helix_core::auto_pairs;

    pub fn insert_char(cx: &mut Context, c: char) {
        let (view, doc) = current!(cx.editor);

        let hooks: &[Hook] = match cx.editor.config.auto_pairs {
            true => &[auto_pairs::hook, insert],
            false => &[insert],
        };

        let text = doc.text();
        let selection = doc.selection(view.id).clone().cursors(text.slice(..));

        // run through insert hooks, stopping on the first one that returns Some(t)
        for hook in hooks {
            if let Some(transaction) = hook(text, &selection, c) {
                doc.apply(&transaction, view.id);
                break;
            }
        }

        // TODO: need a post insert hook too for certain triggers (autocomplete, signature help, etc)
        // this could also generically look at Transaction, but it's a bit annoying to look at
        // Operation instead of Change.
        for hook in &[language_server_completion, signature_help] {
            // for hook in &[signature_help] {
            hook(cx, c);
        }
    }

    pub fn insert_tab(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        // TODO: round out to nearest indentation level (for example a line with 3 spaces should
        // indent by one to reach 4 spaces).

        let indent = Tendril::from(doc.indent_unit());
        let transaction = Transaction::insert(
            doc.text(),
            &doc.selection(view.id).clone().cursors(doc.text().slice(..)),
            indent,
        );
        doc.apply(&transaction, view.id);
    }

    pub fn insert_newline(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let contents = doc.text();
        let selection = doc.selection(view.id).clone().cursors(text);
        let mut ranges = SmallVec::with_capacity(selection.len());

        // TODO: this is annoying, but we need to do it to properly calculate pos after edits
        let mut offs = 0;

        let mut transaction = Transaction::change_by_selection(contents, &selection, |range| {
            let pos = range.head;

            let prev = if pos == 0 {
                ' '
            } else {
                contents.char(pos - 1)
            };
            let curr = contents.get_char(pos).unwrap_or(' ');

            // TODO: offset range.head by 1? when calculating?
            let indent_level = indent::suggested_indent_for_pos(
                doc.language_config(),
                doc.syntax(),
                text,
                pos.saturating_sub(1),
                true,
            );
            let indent = doc.indent_unit().repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push_str(doc.line_ending.as_str());
            text.push_str(&indent);

            let head = pos + offs + text.chars().count();

            // TODO: range replace or extend
            // range.replace(|range| range.is_empty(), head); -> fn extend if cond true, new head pos
            // can be used with cx.mode to do replace or extend on most changes
            ranges.push(Range::new(
                if range.is_empty() {
                    head
                } else {
                    range.anchor + offs
                },
                head,
            ));

            // if between a bracket pair
            if helix_core::auto_pairs::PAIRS.contains(&(prev, curr)) {
                // another newline, indent the end bracket one level less
                let indent = doc.indent_unit().repeat(indent_level.saturating_sub(1));
                text.push_str(doc.line_ending.as_str());
                text.push_str(&indent);
            }

            offs += text.chars().count();

            (pos, pos, Some(text.into()))
        });

        transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));
        //

        doc.apply(&transaction, view.id);
    }

    pub fn delete_char_backward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let indent_unit = doc.indent_unit();
        let tab_size = doc.tab_width();

        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.cursor(text);
                let line_start_pos = text.line_to_char(range.cursor_line(text));
                // considier to delete by indent level if all characters before `pos` are indent units.
                let fragment = Cow::from(text.slice(line_start_pos..pos));
                if !fragment.is_empty() && fragment.chars().all(|ch| ch.is_whitespace()) {
                    if text.get_char(pos.saturating_sub(1)) == Some('\t') {
                        // fast path, delete one char
                        (
                            graphemes::nth_prev_grapheme_boundary(text, pos, 1),
                            pos,
                            None,
                        )
                    } else {
                        let unit_len = indent_unit.chars().count();
                        // NOTE: indent_unit always contains 'only spaces' or 'only tab' according to `IndentStyle` definition.
                        let unit_size = if indent_unit.starts_with('\t') {
                            tab_size * unit_len
                        } else {
                            unit_len
                        };
                        let width: usize = fragment
                            .chars()
                            .map(|ch| {
                                if ch == '\t' {
                                    tab_size
                                } else {
                                    // it can be none if it still meet control characters other than '\t'
                                    // here just set the width to 1 (or some value better?).
                                    ch.width().unwrap_or(1)
                                }
                            })
                            .sum();
                        let mut drop = width % unit_size; // round down to nearest unit
                        if drop == 0 {
                            drop = unit_size
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
                        (start, pos, None) // delete!
                    }
                } else {
                    // delete char
                    (
                        graphemes::nth_prev_grapheme_boundary(text, pos, count),
                        pos,
                        None,
                    )
                }
            });
        doc.apply(&transaction, view.id);
    }

    pub fn delete_char_forward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.cursor(text);
                (
                    pos,
                    graphemes::nth_next_grapheme_boundary(text, pos, count),
                    None,
                )
            });
        doc.apply(&transaction, view.id);
    }

    pub fn delete_word_backward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| movement::move_prev_word_start(text, range, count));
        delete_selection_insert_mode(doc, view, &selection);
    }

    pub fn delete_word_forward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| movement::move_next_word_start(text, range, count));
        delete_selection_insert_mode(doc, view, &selection);
    }
}

// Undo / Redo

// TODO: each command could simply return a Option<transaction>, then the higher level handles
// storing it?

fn undo(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        if !doc.undo(view.id) {
            cx.editor.set_status("Already at oldest change".to_owned());
            break;
        }
    }
}

fn redo(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        if !doc.redo(view.id) {
            cx.editor.set_status("Already at newest change".to_owned());
            break;
        }
    }
}

fn earlier(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        // rather than doing in batch we do this so get error halfway
        if !doc.earlier(view.id, UndoKind::Steps(1)) {
            cx.editor.set_status("Already at oldest change".to_owned());
            break;
        }
    }
}

fn later(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        // rather than doing in batch we do this so get error halfway
        if !doc.later(view.id, UndoKind::Steps(1)) {
            cx.editor.set_status("Already at newest change".to_owned());
            break;
        }
    }
}

// Yank / Paste

fn yank(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "yanked {} selection(s) to register {}",
        values.len(),
        cx.register.unwrap_or('"')
    );

    cx.editor
        .registers
        .write(cx.register.unwrap_or('"'), values);

    cx.editor.set_status(msg);
    exit_select_mode(cx);
}

fn yank_joined_to_clipboard_impl(
    editor: &mut Editor,
    separator: &str,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "joined and yanked {} selection(s) to system clipboard",
        values.len(),
    );

    let joined = values.join(separator);

    editor
        .clipboard_provider
        .set_contents(joined, clipboard_type)
        .context("Couldn't set system clipboard content")?;

    editor.set_status(msg);

    Ok(())
}

fn yank_joined_to_clipboard(cx: &mut Context) {
    let line_ending = doc!(cx.editor).line_ending;
    let _ = yank_joined_to_clipboard_impl(
        &mut cx.editor,
        line_ending.as_str(),
        ClipboardType::Clipboard,
    );
    exit_select_mode(cx);
}

fn yank_main_selection_to_clipboard_impl(
    editor: &mut Editor,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let value = doc.selection(view.id).primary().fragment(text);

    if let Err(e) = editor
        .clipboard_provider
        .set_contents(value.into_owned(), clipboard_type)
    {
        bail!("Couldn't set system clipboard content: {}", e);
    }

    editor.set_status("yanked main selection to system clipboard".to_owned());
    Ok(())
}

fn yank_main_selection_to_clipboard(cx: &mut Context) {
    let _ = yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Clipboard);
}

fn yank_joined_to_primary_clipboard(cx: &mut Context) {
    let line_ending = doc!(cx.editor).line_ending;
    let _ = yank_joined_to_clipboard_impl(
        &mut cx.editor,
        line_ending.as_str(),
        ClipboardType::Selection,
    );
}

fn yank_main_selection_to_primary_clipboard(cx: &mut Context) {
    let _ = yank_main_selection_to_clipboard_impl(&mut cx.editor, ClipboardType::Selection);
    exit_select_mode(cx);
}

#[derive(Copy, Clone)]
enum Paste {
    Before,
    After,
}

fn paste_impl(
    values: &[String],
    doc: &mut Document,
    view: &View,
    action: Paste,
) -> Option<Transaction> {
    let repeat = std::iter::repeat(
        values
            .last()
            .map(|value| Tendril::from_slice(value))
            .unwrap(),
    );

    // if any of values ends with a line ending, it's linewise paste
    let linewise = values
        .iter()
        .any(|value| get_line_ending_of_str(value).is_some());

    // Only compiled once.
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\r\n|\r|\n").unwrap());
    let mut values = values
        .iter()
        .map(|value| REGEX.replace_all(value, doc.line_ending.as_str()))
        .map(|value| Tendril::from(value.as_ref()))
        .chain(repeat);

    let text = doc.text();
    let selection = doc.selection(view.id);

    let transaction = Transaction::change_by_selection(text, selection, |range| {
        let pos = match (action, linewise) {
            // paste linewise before
            (Paste::Before, true) => text.line_to_char(text.char_to_line(range.from())),
            // paste linewise after
            (Paste::After, true) => {
                let line = range.line_range(text.slice(..)).1;
                text.line_to_char((line + 1).min(text.len_lines()))
            }
            // paste insert
            (Paste::Before, false) => range.from(),
            // paste append
            (Paste::After, false) => range.to(),
        };
        (pos, pos, Some(values.next().unwrap()))
    });

    Some(transaction)
}

fn paste_clipboard_impl(
    editor: &mut Editor,
    action: Paste,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);

    match editor
        .clipboard_provider
        .get_contents(clipboard_type)
        .map(|contents| paste_impl(&[contents], doc, view, action))
    {
        Ok(Some(transaction)) => {
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(e) => Err(e.context("Couldn't get system clipboard contents")),
    }
}

fn paste_clipboard_after(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Clipboard);
}

fn paste_clipboard_before(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::Before, ClipboardType::Clipboard);
}

fn paste_primary_clipboard_after(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::After, ClipboardType::Selection);
}

fn paste_primary_clipboard_before(cx: &mut Context) {
    let _ = paste_clipboard_impl(&mut cx.editor, Paste::Before, ClipboardType::Selection);
}

fn replace_with_yanked(cx: &mut Context) {
    let reg_name = cx.register.unwrap_or('"');
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(values) = registers.read(reg_name) {
        if !values.is_empty() {
            let repeat = std::iter::repeat(
                values
                    .last()
                    .map(|value| Tendril::from_slice(value))
                    .unwrap(),
            );
            let mut values = values
                .iter()
                .map(|value| Tendril::from_slice(value))
                .chain(repeat);
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                if !range.is_empty() {
                    (range.from(), range.to(), Some(values.next().unwrap()))
                } else {
                    (range.from(), range.to(), None)
                }
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    }
}

fn replace_selections_with_clipboard_impl(
    editor: &mut Editor,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);

    match editor.clipboard_provider.get_contents(clipboard_type) {
        Ok(contents) => {
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                (range.from(), range.to(), Some(contents.as_str().into()))
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
            Ok(())
        }
        Err(e) => Err(e.context("Couldn't get system clipboard contents")),
    }
}

fn replace_selections_with_clipboard(cx: &mut Context) {
    let _ = replace_selections_with_clipboard_impl(&mut cx.editor, ClipboardType::Clipboard);
}

fn replace_selections_with_primary_clipboard(cx: &mut Context) {
    let _ = replace_selections_with_clipboard_impl(&mut cx.editor, ClipboardType::Selection);
}

fn paste_after(cx: &mut Context) {
    let reg_name = cx.register.unwrap_or('"');
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(transaction) = registers
        .read(reg_name)
        .and_then(|values| paste_impl(values, doc, view, Paste::After))
    {
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view.id);
    }
}

fn paste_before(cx: &mut Context) {
    let reg_name = cx.register.unwrap_or('"');
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(transaction) = registers
        .read(reg_name)
        .and_then(|values| paste_impl(values, doc, view, Paste::Before))
    {
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view.id);
    }
}

fn get_lines(doc: &Document, view_id: ViewId) -> Vec<usize> {
    let mut lines = Vec::new();

    // Get all line numbers
    for range in doc.selection(view_id) {
        let (start, end) = range.line_range(doc.text().slice(..));

        for line in start..=end {
            lines.push(line)
        }
    }
    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();
    lines
}

fn indent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let lines = get_lines(doc, view.id);

    // Indent by one level
    let indent = Tendril::from(doc.indent_unit().repeat(count));

    let transaction = Transaction::change(
        doc.text(),
        lines.into_iter().map(|line| {
            let pos = doc.text().line_to_char(line);
            (pos, pos, Some(indent.clone()))
        }),
    );
    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn unindent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let lines = get_lines(doc, view.id);
    let mut changes = Vec::with_capacity(lines.len());
    let tab_width = doc.tab_width();
    let indent_width = count * tab_width;

    for line_idx in lines {
        let line = doc.text().line(line_idx);
        let mut width = 0;
        let mut pos = 0;

        for ch in line.chars() {
            match ch {
                ' ' => width += 1,
                '\t' => width = (width / tab_width + 1) * tab_width,
                _ => break,
            }

            pos += 1;

            if width >= indent_width {
                break;
            }
        }

        // now delete from start to first non-blank
        if pos > 0 {
            let start = doc.text().line_to_char(line_idx);
            changes.push((start, start + pos, None))
        }
    }

    let transaction = Transaction::change(doc.text(), changes.into_iter());

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn format_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    // via lsp if available
    // else via tree-sitter indentation calculations

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let ranges: Vec<lsp::Range> = doc
        .selection(view.id)
        .iter()
        .map(|range| range_to_lsp_range(doc.text(), *range, language_server.offset_encoding()))
        .collect();

    // TODO: all of the TODO's and commented code inside the loop,
    // to make this actually work.
    for _range in ranges {
        let _language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };
        // TODO: handle fails
        // TODO: concurrent map

        // TODO: need to block to get the formatting

        // let edits = block_on(language_server.text_document_range_formatting(
        //     doc.identifier(),
        //     range,
        //     lsp::FormattingOptions::default(),
        // ))
        // .unwrap_or_default();

        // let transaction = helix_lsp::util::generate_transaction_from_edits(
        //     doc.text(),
        //     edits,
        //     language_server.offset_encoding(),
        // );

        // doc.apply(&transaction, view.id);
    }

    doc.append_changes_to_history(view.id);
}

fn join_selections(cx: &mut Context) {
    use movement::skip_while;
    let (view, doc) = current!(cx.editor);
    let text = doc.text();
    let slice = doc.text().slice(..);

    let mut changes = Vec::new();
    let fragment = Tendril::from(" ");

    for selection in doc.selection(view.id) {
        let (start, mut end) = selection.line_range(slice);
        if start == end {
            end = (end + 1).min(text.len_lines() - 1);
        }
        let lines = start..end;

        changes.reserve(lines.len());

        for line in lines {
            let start = line_end_char_index(&slice, line);
            let mut end = text.line_to_char(line + 1);
            end = skip_while(slice, end, |ch| matches!(ch, ' ' | '\t')).unwrap_or(end);

            // need to skip from start, not end
            let change = (start, end, Some(fragment.clone()));
            changes.push(change);
        }
    }

    changes.sort_unstable_by_key(|(from, _to, _text)| *from);
    changes.dedup();

    // TODO: joining multiple empty lines should be replaced by a single space.
    // need to merge change ranges that touch

    let transaction = Transaction::change(doc.text(), changes.into_iter());
    // TODO: select inserted spaces
    // .with_selection(selection);

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn keep_or_remove_selections_impl(cx: &mut Context, remove: bool) {
    // keep or remove selections matching regex
    let reg = cx.register.unwrap_or('/');
    let prompt = ui::regex_prompt(
        cx,
        if !remove { "keep:" } else { "remove:" }.into(),
        Some(reg),
        |_input: &str| Vec::new(),
        move |view, doc, regex, event| {
            if event != PromptEvent::Update {
                return;
            }
            let text = doc.text().slice(..);

            if let Some(selection) =
                selection::keep_or_remove_matches(text, doc.selection(view.id), &regex, remove)
            {
                doc.set_selection(view.id, selection);
            }
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn keep_selections(cx: &mut Context) {
    keep_or_remove_selections_impl(cx, false)
}

fn remove_selections(cx: &mut Context) {
    keep_or_remove_selections_impl(cx, true)
}

fn keep_primary_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    // TODO: handle count

    let range = doc.selection(view.id).primary();
    doc.set_selection(view.id, Selection::single(range.anchor, range.head));
}

fn remove_primary_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    // TODO: handle count

    let selection = doc.selection(view.id);
    if selection.len() == 1 {
        cx.editor.set_error("no selections remaining".to_owned());
        return;
    }
    let index = selection.primary_index();
    let selection = selection.clone().remove(index);

    doc.set_selection(view.id, selection);
}

pub fn completion(cx: &mut Context) {
    // trigger on trigger char, or if user calls it
    // (or on word char typing??)
    // after it's triggered, if response marked is_incomplete, update on every subsequent keypress
    //
    // lsp calls are done via a callback: it sends a request and doesn't block.
    // when we get the response similarly to notification, trigger a call to the completion popup
    //
    // language_server.completion(params, |cx: &mut Context, _meta, response| {
    //    // called at response time
    //    // compositor, lookup completion layer
    //    // downcast dyn Component to Completion component
    //    // emit response to completion (completion.complete/handle(response))
    // })
    //
    // typing after prompt opens: usually start offset is tracked and everything between
    // start_offset..cursor is replaced. For our purposes we could keep the start state (doc,
    // selection) and revert to them before applying. This needs to properly reset changes/history
    // though...
    //
    // company-mode does this by matching the prefix of the completion and removing it.

    // ignore isIncomplete for now
    // keep state while typing
    // the behavior should be, filter the menu based on input
    // if items returns empty at any point, remove the popup
    // if backspace past initial offset point, remove the popup
    //
    // debounce requests!
    //
    // need an idle timeout thing.
    // https://github.com/company-mode/company-mode/blob/master/company.el#L620-L622
    //
    //  "The idle delay in seconds until completion starts automatically.
    // The prefix still has to satisfy `company-minimum-prefix-length' before that
    // happens.  The value of nil means no idle completion."

    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();
    let text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(text);

    let pos = pos_to_lsp_pos(doc.text(), cursor, offset_encoding);

    let future = language_server.completion(doc.identifier(), pos, None);

    let trigger_offset = cursor;

    // TODO: trigger_offset should be the cursor offset but we also need a starting offset from where we want to apply
    // completion filtering. For example logger.te| should filter the initial suggestion list with "te".

    use helix_core::chars;
    let mut iter = text.chars_at(cursor);
    iter.reverse();
    let offset = iter.take_while(|ch| chars::char_is_word(*ch)).count();
    let start_offset = cursor.saturating_sub(offset);
    let prefix = text.slice(start_offset..cursor).to_string();

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::CompletionResponse>| {
            let (_, doc) = current!(editor);
            if doc.mode() != Mode::Insert {
                // we're not in insert mode anymore
                return;
            }

            let mut items = match response {
                Some(lsp::CompletionResponse::Array(items)) => items,
                // TODO: do something with is_incomplete
                Some(lsp::CompletionResponse::List(lsp::CompletionList {
                    is_incomplete: _is_incomplete,
                    items,
                })) => items,
                None => Vec::new(),
            };

            if !prefix.is_empty() {
                items = items
                    .into_iter()
                    .filter(|item| {
                        item.filter_text
                            .as_ref()
                            .unwrap_or(&item.label)
                            .starts_with(&prefix)
                    })
                    .collect();
            }

            if items.is_empty() {
                // editor.set_error("No completion available".to_string());
                return;
            }
            let size = compositor.size();
            let ui = compositor.find::<ui::EditorView>().unwrap();
            ui.set_completion(
                editor,
                items,
                offset_encoding,
                start_offset,
                trigger_offset,
                size,
            );
        },
    );
}

fn hover(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: factor out a doc.position_identifier() that returns lsp::TextDocumentPositionIdentifier

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
        language_server.offset_encoding(),
    );

    let future = language_server.text_document_hover(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor, compositor: &mut Compositor, response: Option<lsp::Hover>| {
            if let Some(hover) = response {
                // hover.contents / .range <- used for visualizing

                fn marked_string_to_markdown(contents: lsp::MarkedString) -> String {
                    match contents {
                        lsp::MarkedString::String(contents) => contents,
                        lsp::MarkedString::LanguageString(string) => {
                            if string.language == "markdown" {
                                string.value
                            } else {
                                format!("```{}\n{}\n```", string.language, string.value)
                            }
                        }
                    }
                }

                let contents = match hover.contents {
                    lsp::HoverContents::Scalar(contents) => marked_string_to_markdown(contents),
                    lsp::HoverContents::Array(contents) => contents
                        .into_iter()
                        .map(marked_string_to_markdown)
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                    lsp::HoverContents::Markup(contents) => contents.value,
                };

                // skip if contents empty

                let contents = ui::Markdown::new(contents, editor.syn_loader.clone());
                let popup = Popup::new(contents);
                compositor.push(Box::new(popup));
            }
        },
    );
}

// comments
fn toggle_comments(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let token = doc
        .language_config()
        .and_then(|lc| lc.comment_token.as_ref())
        .map(|tc| tc.as_ref());
    let transaction = comment::toggle_line_comments(doc.text(), doc.selection(view.id), token);

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
    exit_select_mode(cx);
}

fn rotate_selections(cx: &mut Context, direction: Direction) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let mut selection = doc.selection(view.id).clone();
    let index = selection.primary_index();
    let len = selection.len();
    selection.set_primary_index(match direction {
        Direction::Forward => (index + count) % len,
        Direction::Backward => (index + (len.saturating_sub(count) % len)) % len,
    });
    doc.set_selection(view.id, selection);
}
fn rotate_selections_forward(cx: &mut Context) {
    rotate_selections(cx, Direction::Forward)
}
fn rotate_selections_backward(cx: &mut Context) {
    rotate_selections(cx, Direction::Backward)
}

fn rotate_selection_contents(cx: &mut Context, direction: Direction) {
    let count = cx.count;
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id);
    let mut fragments: Vec<_> = selection
        .fragments(text)
        .map(|fragment| Tendril::from_slice(&fragment))
        .collect();

    let group = count
        .map(|count| count.get())
        .unwrap_or(fragments.len()) // default to rotating everything as one group
        .min(fragments.len());

    for chunk in fragments.chunks_mut(group) {
        // TODO: also modify main index
        match direction {
            Direction::Forward => chunk.rotate_right(1),
            Direction::Backward => chunk.rotate_left(1),
        };
    }

    let transaction = Transaction::change(
        doc.text(),
        selection
            .ranges()
            .iter()
            .zip(fragments)
            .map(|(range, fragment)| (range.from(), range.to(), Some(fragment))),
    );

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}
fn rotate_selection_contents_forward(cx: &mut Context) {
    rotate_selection_contents(cx, Direction::Forward)
}
fn rotate_selection_contents_backward(cx: &mut Context) {
    rotate_selection_contents(cx, Direction::Backward)
}

// tree sitter node selection

fn expand_selection(cx: &mut Context) {
    let motion = |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let selection = object::expand_selection(syntax, text, doc.selection(view.id));
            doc.set_selection(view.id, selection);
        }
    };
    motion(&mut cx.editor);
    cx.editor.last_motion = Some(Motion(Box::new(motion)));
}

fn match_brackets(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id).clone().transform(|range| {
            if let Some(pos) =
                match_brackets::find_matching_bracket_fuzzy(syntax, doc.text(), range.anchor)
            {
                range.put_cursor(text, pos, doc.mode == Mode::Select)
            } else {
                range
            }
        });
        doc.set_selection(view.id, selection);
    }
}

//

fn jump_forward(cx: &mut Context) {
    let count = cx.count();
    let view = view_mut!(cx.editor);

    if let Some((id, selection)) = view.jumps.forward(count) {
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = current!(cx.editor); // refetch doc
        doc.set_selection(view.id, selection);

        align_view(doc, view, Align::Center);
    };
}

fn jump_backward(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

    if let Some((id, selection)) = view.jumps.backward(view.id, doc, count) {
        // manually set the alternate_file as we cannot use the Editor::switch function here.
        if view.doc != *id {
            view.last_accessed_doc = Some(view.doc)
        }
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = current!(cx.editor); // refetch doc
        doc.set_selection(view.id, selection);

        align_view(doc, view, Align::Center);
    };
}

fn rotate_view(cx: &mut Context) {
    cx.editor.focus_next()
}

fn jump_view_right(cx: &mut Context) {
    cx.editor.focus_right()
}

fn jump_view_left(cx: &mut Context) {
    cx.editor.focus_left()
}

fn jump_view_up(cx: &mut Context) {
    cx.editor.focus_up()
}

fn jump_view_down(cx: &mut Context) {
    cx.editor.focus_down()
}

// split helper, clear it later
fn split(cx: &mut Context, action: Action) {
    let (view, doc) = current!(cx.editor);
    let id = doc.id();
    let selection = doc.selection(view.id).clone();
    let offset = view.offset;

    cx.editor.switch(id, action);

    // match the selection in the previous view
    let (view, doc) = current!(cx.editor);
    view.offset = offset;
    doc.set_selection(view.id, selection);
}

fn hsplit(cx: &mut Context) {
    split(cx, Action::HorizontalSplit);
}

fn vsplit(cx: &mut Context) {
    split(cx, Action::VerticalSplit);
}

fn wclose(cx: &mut Context) {
    if cx.editor.tree.views().count() == 1 {
        if let Err(err) = cmd::buffers_remaining_impl(cx.editor) {
            cx.editor.set_error(err.to_string());
            return;
        }
    }
    let view_id = view!(cx.editor).id;
    // close current split
    cx.editor.close(view_id);
}

fn wonly(cx: &mut Context) {
    let views = cx
        .editor
        .tree
        .views()
        .map(|(v, focus)| (v.id, focus))
        .collect::<Vec<_>>();
    for (view_id, focus) in views {
        if !focus {
            cx.editor.close(view_id);
        }
    }
}

fn select_register(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            cx.editor.selected_register = Some(ch);
        }
    })
}

fn insert_register(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            cx.editor.selected_register = Some(ch);
            paste_before(cx);
        }
    })
}

fn align_view_top(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Top);
}

fn align_view_center(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Center);
}

fn align_view_bottom(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Bottom);
}

fn align_view_middle(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let pos = doc.selection(view.id).primary().cursor(text);
    let pos = coords_at_pos(text, pos);

    view.offset.col = pos
        .col
        .saturating_sub((view.inner_area().width as usize) / 2);
}

fn scroll_up(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Backward);
}

fn scroll_down(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Forward);
}

fn select_textobject_around(cx: &mut Context) {
    select_textobject(cx, textobject::TextObject::Around);
}

fn select_textobject_inner(cx: &mut Context) {
    select_textobject(cx, textobject::TextObject::Inside);
}

fn select_textobject(cx: &mut Context, objtype: textobject::TextObject) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            let textobject = move |editor: &mut Editor| {
                let (view, doc) = current!(editor);
                let text = doc.text().slice(..);

                let textobject_treesitter = |obj_name: &str, range: Range| -> Range {
                    let (lang_config, syntax) = match doc.language_config().zip(doc.syntax()) {
                        Some(t) => t,
                        None => return range,
                    };
                    textobject::textobject_treesitter(
                        text,
                        range,
                        objtype,
                        obj_name,
                        syntax.tree().root_node(),
                        lang_config,
                        count,
                    )
                };

                let selection = doc.selection(view.id).clone().transform(|range| {
                    match ch {
                        'w' => textobject::textobject_word(text, range, objtype, count, false),
                        'W' => textobject::textobject_word(text, range, objtype, count, true),
                        'c' => textobject_treesitter("class", range),
                        'f' => textobject_treesitter("function", range),
                        'p' => textobject_treesitter("parameter", range),
                        'm' => {
                            let ch = text.char(range.cursor(text));
                            if !ch.is_ascii_alphanumeric() {
                                textobject::textobject_surround(text, range, objtype, ch, count)
                            } else {
                                range
                            }
                        }
                        // TODO: cancel new ranges if inconsistent surround matches across lines
                        ch if !ch.is_ascii_alphanumeric() => {
                            textobject::textobject_surround(text, range, objtype, ch, count)
                        }
                        _ => range,
                    }
                });
                doc.set_selection(view.id, selection);
            };
            textobject(&mut cx.editor);
            cx.editor.last_motion = Some(Motion(Box::new(textobject)));
        }
    })
}

fn surround_add(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            let (view, doc) = current!(cx.editor);
            let selection = doc.selection(view.id);
            let (open, close) = surround::get_pair(ch);

            let mut changes = Vec::with_capacity(selection.len() * 2);
            for range in selection.iter() {
                changes.push((range.from(), range.from(), Some(Tendril::from_char(open))));
                changes.push((range.to(), range.to(), Some(Tendril::from_char(close))));
            }

            let transaction = Transaction::change(doc.text(), changes.into_iter());
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    })
}

fn surround_replace(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        if let Some(from) = event.char() {
            cx.on_next_key(move |cx, event| {
                if let Some(to) = event.char() {
                    let (view, doc) = current!(cx.editor);
                    let text = doc.text().slice(..);
                    let selection = doc.selection(view.id);

                    let change_pos = match surround::get_surround_pos(text, selection, from, count)
                    {
                        Some(c) => c,
                        None => return,
                    };

                    let (open, close) = surround::get_pair(to);
                    let transaction = Transaction::change(
                        doc.text(),
                        change_pos.iter().enumerate().map(|(i, &pos)| {
                            (
                                pos,
                                pos + 1,
                                Some(Tendril::from_char(if i % 2 == 0 { open } else { close })),
                            )
                        }),
                    );
                    doc.apply(&transaction, view.id);
                    doc.append_changes_to_history(view.id);
                }
            });
        }
    })
}

fn surround_delete(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            let (view, doc) = current!(cx.editor);
            let text = doc.text().slice(..);
            let selection = doc.selection(view.id);

            let change_pos = match surround::get_surround_pos(text, selection, ch, count) {
                Some(c) => c,
                None => return,
            };

            let transaction =
                Transaction::change(doc.text(), change_pos.into_iter().map(|p| (p, p + 1, None)));
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    })
}

#[derive(Eq, PartialEq)]
enum ShellBehavior {
    Replace,
    Ignore,
    Insert,
    Append,
}

fn shell_pipe(cx: &mut Context) {
    shell(cx, "pipe:".into(), ShellBehavior::Replace);
}

fn shell_pipe_to(cx: &mut Context) {
    shell(cx, "pipe-to:".into(), ShellBehavior::Ignore);
}

fn shell_insert_output(cx: &mut Context) {
    shell(cx, "insert-output:".into(), ShellBehavior::Insert);
}

fn shell_append_output(cx: &mut Context) {
    shell(cx, "append-output:".into(), ShellBehavior::Append);
}

fn shell_keep_pipe(cx: &mut Context) {
    let prompt = Prompt::new(
        "keep-pipe:".into(),
        Some('|'),
        |_input: &str| Vec::new(),
        move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
            let shell = &cx.editor.config.shell;
            if event != PromptEvent::Validate {
                return;
            }
            if input.is_empty() {
                return;
            }
            let (view, doc) = current!(cx.editor);
            let selection = doc.selection(view.id);

            let mut ranges = SmallVec::with_capacity(selection.len());
            let old_index = selection.primary_index();
            let mut index: Option<usize> = None;
            let text = doc.text().slice(..);

            for (i, range) in selection.ranges().iter().enumerate() {
                let fragment = range.fragment(text);
                let (_output, success) = match shell_impl(shell, input, Some(fragment.as_bytes())) {
                    Ok(result) => result,
                    Err(err) => {
                        cx.editor.set_error(err.to_string());
                        return;
                    }
                };

                // if the process exits successfully, keep the selection
                if success {
                    ranges.push(*range);
                    if i >= old_index && index.is_none() {
                        index = Some(ranges.len() - 1);
                    }
                }
            }

            if ranges.is_empty() {
                cx.editor.set_error("No selections remaining".to_string());
                return;
            }

            let index = index.unwrap_or_else(|| ranges.len() - 1);
            doc.set_selection(view.id, Selection::new(ranges, index));
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn shell_impl(
    shell: &[String],
    cmd: &str,
    input: Option<&[u8]>,
) -> anyhow::Result<(Tendril, bool)> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    if shell.is_empty() {
        bail!("No shell set");
    }

    let mut process = match Command::new(&shell[0])
        .args(&shell[1..])
        .arg(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(process) => process,
        Err(e) => {
            log::error!("Failed to start shell: {}", e);
            return Err(e.into());
        }
    };
    if let Some(input) = input {
        let mut stdin = process.stdin.take().unwrap();
        stdin.write_all(input)?;
    }
    let output = process.wait_with_output()?;

    if !output.stderr.is_empty() {
        log::error!("Shell error: {}", String::from_utf8_lossy(&output.stderr));
    }

    let tendril = Tendril::try_from_byte_slice(&output.stdout)
        .map_err(|_| anyhow!("Process did not output valid UTF-8"))?;
    Ok((tendril, output.status.success()))
}

fn shell(cx: &mut Context, prompt: Cow<'static, str>, behavior: ShellBehavior) {
    let pipe = match behavior {
        ShellBehavior::Replace | ShellBehavior::Ignore => true,
        ShellBehavior::Insert | ShellBehavior::Append => false,
    };
    let prompt = Prompt::new(
        prompt,
        Some('|'),
        |_input: &str| Vec::new(),
        move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
            let shell = &cx.editor.config.shell;
            if event != PromptEvent::Validate {
                return;
            }
            if input.is_empty() {
                return;
            }
            let (view, doc) = current!(cx.editor);
            let selection = doc.selection(view.id);

            let mut changes = Vec::with_capacity(selection.len());
            let text = doc.text().slice(..);

            for range in selection.ranges() {
                let fragment = range.fragment(text);
                let (output, success) =
                    match shell_impl(shell, input, pipe.then(|| fragment.as_bytes())) {
                        Ok(result) => result,
                        Err(err) => {
                            cx.editor.set_error(err.to_string());
                            return;
                        }
                    };

                if !success {
                    cx.editor.set_error("Command failed".to_string());
                    return;
                }

                let (from, to) = match behavior {
                    ShellBehavior::Replace => (range.from(), range.to()),
                    ShellBehavior::Insert => (range.from(), range.from()),
                    ShellBehavior::Append => (range.to(), range.to()),
                    _ => (range.from(), range.from()),
                };
                changes.push((from, to, Some(output)));
            }

            if behavior != ShellBehavior::Ignore {
                let transaction = Transaction::change(doc.text(), changes.into_iter());
                doc.apply(&transaction, view.id);
                doc.append_changes_to_history(view.id);
            }

            // after replace cursor may be out of bounds, do this to
            // make sure cursor is in view and update scroll as well
            view.ensure_cursor_in_view(doc, cx.editor.config.scrolloff);
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn suspend(_cx: &mut Context) {
    #[cfg(not(windows))]
    signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
}

fn add_newline_above(cx: &mut Context) {
    add_newline_impl(cx, Open::Above);
}

fn add_newline_below(cx: &mut Context) {
    add_newline_impl(cx, Open::Below)
}

fn add_newline_impl(cx: &mut Context, open: Open) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let text = doc.text();
    let slice = text.slice(..);

    let changes = selection.into_iter().map(|range| {
        let (start, end) = range.line_range(slice);
        let line = match open {
            Open::Above => start,
            Open::Below => end + 1,
        };
        let pos = text.line_to_char(line);
        (
            pos,
            pos,
            Some(doc.line_ending.as_str().repeat(count).into()),
        )
    });

    let transaction = Transaction::change(text, changes);
    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

fn rename_symbol(cx: &mut Context) {
    let prompt = Prompt::new(
        "rename-to:".into(),
        None,
        |_input: &str| Vec::new(),
        move |cx: &mut compositor::Context, input: &str, event: PromptEvent| {
            if event != PromptEvent::Validate {
                return;
            }

            log::debug!("renaming to: {:?}", input);

            let (view, doc) = current!(cx.editor);
            let language_server = match doc.language_server() {
                Some(language_server) => language_server,
                None => return,
            };

            let offset_encoding = language_server.offset_encoding();

            let pos = pos_to_lsp_pos(
                doc.text(),
                doc.selection(view.id)
                    .primary()
                    .cursor(doc.text().slice(..)),
                offset_encoding,
            );

            let task = language_server.rename_symbol(doc.identifier(), pos, input.to_string());
            let edits = block_on(task).unwrap_or_default();
            log::debug!("Edits from LSP: {:?}", edits);
            apply_workspace_edit(&mut cx.editor, offset_encoding, &edits);
        },
    );
    cx.push_layer(Box::new(prompt));
}

/// Increment object under cursor by count.
fn increment(cx: &mut Context) {
    increment_impl(cx, cx.count() as i64);
}

/// Decrement object under cursor by count.
fn decrement(cx: &mut Context) {
    increment_impl(cx, -(cx.count() as i64));
}

/// Decrement object under cursor by `amount`.
fn increment_impl(cx: &mut Context, amount: i64) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let text = doc.text();

    let changes = selection.ranges().iter().filter_map(|range| {
        let incrementor = NumberIncrementor::from_range(text.slice(..), *range)?;
        let new_text = incrementor.incremented_text(amount);
        Some((
            incrementor.range.from(),
            incrementor.range.to(),
            Some(new_text),
        ))
    });

    if changes.clone().count() > 0 {
        let transaction = Transaction::change(doc.text(), changes);
        let transaction = transaction.with_selection(selection.clone());

        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view.id);
    }
}
