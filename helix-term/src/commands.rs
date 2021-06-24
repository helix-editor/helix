use helix_core::{
    comment, coords_at_pos, find_first_non_whitespace_char, find_root, graphemes, indent,
    line_ending::{
        get_line_ending, get_line_ending_of_str, line_end_char_index, rope_end_without_line_ending,
        str_is_line_ending,
    },
    match_brackets,
    movement::{self, Direction},
    object, pos_at_coords,
    regex::{self, Regex},
    register::{self, Register, Registers},
    search, selection, Change, ChangeSet, LineEnding, Position, Range, Rope, RopeGraphemes,
    RopeSlice, Selection, SmallVec, Tendril, Transaction, DEFAULT_LINE_ENDING,
};

use helix_view::{
    document::{IndentStyle, Mode},
    input::{KeyCode, KeyEvent},
    view::{View, PADDING},
    Document, DocumentId, Editor, ViewId,
};

use anyhow::anyhow;
use helix_lsp::{
    lsp,
    util::{lsp_pos_to_pos, lsp_range_to_range, pos_to_lsp_pos, range_to_lsp_range},
    LspProgressMap, OffsetEncoding,
};
use insert::*;
use movement::Movement;

use crate::{
    compositor::{self, Callback, Component, Compositor},
    ui::{self, Completion, Picker, Popup, Prompt, PromptEvent},
};

use crate::application::{LspCallbackWrapper, LspCallbacks};
use futures_util::FutureExt;
use std::{fmt, future::Future, path::Display, str::FromStr};

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use serde::de::{self, Deserialize, Deserializer};

pub struct Context<'a> {
    pub selected_register: helix_view::RegisterSelection,
    pub count: Option<std::num::NonZeroUsize>,
    pub editor: &'a mut Editor,

    pub callback: Option<crate::compositor::Callback>,
    pub on_next_key_callback: Option<Box<dyn FnOnce(&mut Context, KeyEvent)>>,
    pub callbacks: &'a mut LspCallbacks,
}

impl<'a> Context<'a> {
    /// Push a new component onto the compositor.
    pub fn push_layer(&mut self, mut component: Box<dyn Component>) {
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
            let call: LspCallbackWrapper =
                Box::new(move |editor: &mut Editor, compositor: &mut Compositor| {
                    callback(editor, compositor, response)
                });
            Ok(call)
        });
        self.callbacks.push(callback);
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
    let pos = doc.selection(view.id).cursor();
    let line = doc.text().char_to_line(pos);

    let relative = match align {
        Align::Center => view.area.height as usize / 2,
        Align::Top => 0,
        Align::Bottom => view.area.height as usize,
    };

    view.first_line = line.saturating_sub(relative);
}

/// A command is composed of a static name, and a function that takes the current state plus a count,
/// and does a side-effect on the state (usually by creating and applying a transaction).
#[derive(Copy, Clone)]
pub struct Command(&'static str, fn(cx: &mut Context));

macro_rules! commands {
    ( $($name:ident),* ) => {
        $(
            #[allow(non_upper_case_globals)]
            pub const $name: Self = Self(stringify!($name), $name);
        )*

        pub const COMMAND_LIST: &'static [Self] = &[
            $( Self::$name, )*
        ];
    }
}

impl Command {
    pub fn execute(&self, cx: &mut Context) {
        (self.1)(cx);
    }

    pub fn name(&self) -> &'static str {
        self.0
    }

    commands!(
        move_char_left,
        move_char_right,
        move_line_up,
        move_line_down,
        move_line_end,
        move_line_start,
        move_first_nonwhitespace,
        move_next_word_start,
        move_prev_word_start,
        move_next_word_end,
        move_file_start,
        move_file_end,
        extend_next_word_start,
        extend_prev_word_start,
        extend_next_word_end,
        find_till_char,
        find_next_char,
        extend_till_char,
        extend_next_char,
        till_prev_char,
        find_prev_char,
        extend_till_prev_char,
        extend_prev_char,
        extend_first_nonwhitespace,
        replace,
        page_up,
        page_down,
        half_page_up,
        half_page_down,
        extend_char_left,
        extend_char_right,
        extend_line_up,
        extend_line_down,
        extend_line_end,
        extend_line_start,
        select_all,
        select_regex,
        split_selection,
        split_selection_on_newline,
        search,
        search_next,
        extend_search_next,
        search_selection,
        extend_line,
        delete_selection,
        change_selection,
        collapse_selection,
        flip_selections,
        insert_mode,
        append_mode,
        command_mode,
        file_picker,
        buffer_picker,
        symbol_picker,
        prepend_to_line,
        append_to_line,
        open_below,
        open_above,
        normal_mode,
        goto_mode,
        select_mode,
        exit_select_mode,
        goto_definition,
        goto_type_definition,
        goto_implementation,
        goto_reference,
        goto_first_diag,
        goto_last_diag,
        goto_next_diag,
        goto_prev_diag,
        signature_help,
        insert_tab,
        insert_newline,
        delete_char_backward,
        delete_char_forward,
        delete_word_backward,
        undo,
        redo,
        yank,
        yank_joined_to_clipboard,
        yank_main_selection_to_clipboard,
        replace_with_yanked,
        replace_selections_with_clipboard,
        paste_after,
        paste_before,
        paste_clipboard_after,
        paste_clipboard_before,
        indent,
        unindent,
        format_selections,
        join_selections,
        keep_selections,
        keep_primary_selection,
        completion,
        hover,
        toggle_comments,
        expand_selection,
        match_brackets,
        jump_forward,
        jump_backward,
        window_mode,
        rotate_view,
        hsplit,
        vsplit,
        wclose,
        select_register,
        space_mode,
        view_mode,
        left_bracket_mode,
        right_bracket_mode,
        match_mode
    );
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Command(name, _) = self;
        f.debug_tuple("Command").field(name).finish()
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Command(name, _) = self;
        f.write_str(name)
    }
}

impl std::str::FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Command::COMMAND_LIST
            .iter()
            .copied()
            .find(|cmd| cmd.0 == s)
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

fn move_char_left(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Backward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn move_char_right(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Forward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn move_line_up(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Backward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn move_line_down(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Forward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

fn move_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        let pos = line_end_char_index(&text.slice(..), line);
        let pos = graphemes::nth_prev_grapheme_boundary(text.slice(..), pos, 1);
        let pos = range.head.max(pos).max(text.line_to_char(line));

        Range::new(pos, pos)
    });

    doc.set_selection(view.id, selection);
}

fn move_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        Range::new(pos, pos)
    });

    doc.set_selection(view.id, selection);
}

fn move_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line_idx = text.char_to_line(range.head);

        if let Some(pos) = find_first_non_whitespace_char(text.line(line_idx)) {
            let pos = pos + text.line_to_char(line_idx);
            Range::new(pos, pos)
        } else {
            range
        }
    });

    doc.set_selection(view.id, selection);
}

// TODO: move vs extend could take an extra type Extend/Move that would
// Range::new(if Move { pos } if Extend { range.anchor }, pos)
// since these all really do the same thing

fn move_next_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .transform(|range| movement::move_next_word_start(text, range, count));

    doc.set_selection(view.id, selection);
}

fn move_prev_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .transform(|range| movement::move_prev_word_start(text, range, count));

    doc.set_selection(view.id, selection);
}

fn move_next_word_end(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .transform(|range| movement::move_next_word_end(text, range, count));

    doc.set_selection(view.id, selection);
}

fn move_file_start(cx: &mut Context) {
    push_jump(cx.editor);
    let (view, doc) = current!(cx.editor);
    doc.set_selection(view.id, Selection::point(0));
}

fn move_file_end(cx: &mut Context) {
    push_jump(cx.editor);
    let (view, doc) = current!(cx.editor);
    let text = doc.text();
    let last_line = text.line_to_char(text.len_lines().saturating_sub(2));
    doc.set_selection(view.id, Selection::point(last_line));
}

fn extend_next_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_next_word_start(text, range, count);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

fn extend_prev_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_prev_word_start(text, range, count);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });
    doc.set_selection(view.id, selection);
}

fn extend_next_word_end(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_next_word_end(text, range, count);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

#[inline]
fn find_char_impl<F>(cx: &mut Context, search_fn: F, inclusive: bool, extend: bool)
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
                current!(cx.editor)
                    .1
                    .line_ending
                    .as_str()
                    .chars()
                    .next()
                    .unwrap()
            }

            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => ch,
            _ => return,
        };

        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let selection = doc.selection(view.id).transform(|mut range| {
            search_fn(text, ch, range.head, count, inclusive).map_or(range, |pos| {
                if extend {
                    Range::new(range.anchor, pos)
                } else {
                    // select
                    Range::new(range.head, pos)
                }
                // or (pos, pos) to move to found val
            })
        });

        doc.set_selection(view.id, selection);
    })
}

fn find_till_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_next,
        false, /* inclusive */
        false, /* extend */
    )
}

fn find_next_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_next,
        true,  /* inclusive */
        false, /* extend */
    )
}

fn extend_till_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_next,
        false, /* inclusive */
        true,  /* extend */
    )
}

fn extend_next_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_next,
        true, /* inclusive */
        true, /* extend */
    )
}

fn till_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_prev,
        false, /* inclusive */
        false, /* extend */
    )
}

fn find_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_prev,
        true,  /* inclusive */
        false, /* extend */
    )
}

fn extend_till_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_prev,
        false, /* inclusive */
        true,  /* extend */
    )
}

fn extend_prev_char(cx: &mut Context) {
    find_char_impl(
        cx,
        search::find_nth_prev,
        true, /* inclusive */
        true, /* extend */
    )
}

fn extend_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line_idx = text.char_to_line(range.head);

        if let Some(pos) = find_first_non_whitespace_char(text.line(line_idx)) {
            let pos = pos + text.line_to_char(line_idx);
            Range::new(range.anchor, pos)
        } else {
            range
        }
    });

    doc.set_selection(view.id, selection);
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

        if let Some(ch) = ch {
            let transaction =
                Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                    let max_to = rope_end_without_line_ending(&doc.text().slice(..));
                    let to = std::cmp::min(max_to, range.to() + 1);
                    let text: String = RopeGraphemes::new(doc.text().slice(range.from()..to))
                        .map(|g| {
                            let cow: Cow<str> = g.into();
                            if str_is_line_ending(&cow) {
                                cow
                            } else {
                                ch.into()
                            }
                        })
                        .collect();

                    (range.from(), to, Some(text.into()))
                });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    })
}

fn scroll(cx: &mut Context, offset: usize, direction: Direction) {
    use Direction::*;
    let (view, doc) = current!(cx.editor);
    let cursor = coords_at_pos(doc.text().slice(..), doc.selection(view.id).cursor());
    let doc_last_line = doc.text().len_lines() - 1;

    let last_line = view.last_line(doc);

    if direction == Backward && view.first_line == 0
        || direction == Forward && last_line == doc_last_line
    {
        return;
    }

    let scrolloff = PADDING.min(view.area.height as usize / 2); // TODO: user pref

    view.first_line = match direction {
        Forward => view.first_line + offset,
        Backward => view.first_line.saturating_sub(offset),
    }
    .min(doc_last_line);

    // recalculate last line
    let last_line = view.last_line(doc);

    // clamp into viewport
    let line = cursor
        .row
        .max(view.first_line + scrolloff)
        .min(last_line.saturating_sub(scrolloff));

    let text = doc.text().slice(..);
    let pos = pos_at_coords(text, Position::new(line, cursor.col)); // this func will properly truncate to line end

    // TODO: only manipulate main selection
    doc.set_selection(view.id, Selection::point(pos));
}

fn page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize;
    scroll(cx, offset, Direction::Backward);
}

fn page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize;
    scroll(cx, offset, Direction::Forward);
}

fn half_page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize / 2;
    scroll(cx, offset, Direction::Backward);
}

fn half_page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.area.height as usize / 2;
    scroll(cx, offset, Direction::Forward);
}

fn extend_char_left(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Backward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn extend_char_right(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Forward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn extend_line_up(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Backward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn extend_line_down(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Forward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn extend_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        let pos = line_end_char_index(&text.slice(..), line);
        let pos = graphemes::nth_prev_grapheme_boundary(text.slice(..), pos, 1);
        let pos = range.head.max(pos).max(text.line_to_char(line));

        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

fn extend_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

fn select_all(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let end = rope_end_without_line_ending(&doc.text().slice(..));
    doc.set_selection(view.id, Selection::single(0, end))
}

fn select_regex(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "select:".to_string(), move |view, doc, _, regex| {
        let text = doc.text().slice(..);
        if let Some(selection) = selection::select_on_matches(text, doc.selection(view.id), &regex)
        {
            doc.set_selection(view.id, selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

fn split_selection(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "split:".to_string(), move |view, doc, _, regex| {
        let text = doc.text().slice(..);
        let selection = selection::split_on_matches(text, doc.selection(view.id), &regex);
        doc.set_selection(view.id, selection);
    });

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

fn search_impl(doc: &mut Document, view: &mut View, contents: &str, regex: &Regex, extend: bool) {
    let text = doc.text();
    let selection = doc.selection(view.id);
    let start = text.char_to_byte(selection.cursor());

    // use find_at to find the next match after the cursor, loop around the end
    // Careful, `Regex` uses `bytes` as offsets, not character indices!
    let mat = regex
        .find_at(contents, start)
        .or_else(|| regex.find(contents));
    // TODO: message on wraparound
    if let Some(mat) = mat {
        let start = text.byte_to_char(mat.start());
        let end = text.byte_to_char(mat.end());

        if end == 0 {
            // skip empty matches that don't make sense
            return;
        }

        let head = end - 1;

        let selection = if extend {
            selection.clone().push(Range::new(start, head))
        } else {
            Selection::single(start, head)
        };

        doc.set_selection(view.id, selection);
        align_view(doc, view, Align::Center);
    };
}

// TODO: use one function for search vs extend
fn search(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    // TODO: could probably share with select_on_matches?

    // HAXX: sadly we can't avoid allocating a single string for the whole buffer since we can't
    // feed chunks into the regex yet
    let contents = doc.text().slice(..).to_string();

    let view_id = view.id;
    let prompt = ui::regex_prompt(
        cx,
        "search:".to_string(),
        move |view, doc, registers, regex| {
            search_impl(doc, view, &contents, &regex, false);
            // TODO: only store on enter (accept), not update
            registers.write('\\', vec![regex.as_str().to_string()]);
        },
    );

    cx.push_layer(Box::new(prompt));
}

fn search_next_impl(cx: &mut Context, extend: bool) {
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;
    if let Some(query) = registers.read('\\') {
        let query = query.first().unwrap();
        let contents = doc.text().slice(..).to_string();
        let regex = Regex::new(query).unwrap();
        search_impl(doc, view, &contents, &regex, extend);
    }
}

fn search_next(cx: &mut Context) {
    search_next_impl(cx, false);
}

fn extend_search_next(cx: &mut Context) {
    search_next_impl(cx, true);
}

fn search_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let contents = doc.text().slice(..);
    let query = doc.selection(view.id).primary().fragment(contents);
    let regex = regex::escape(&query);
    cx.editor.registers.write('\\', vec![regex]);
    search_next(cx);
}

fn extend_line(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

    let pos = doc.selection(view.id).primary();
    let text = doc.text();

    let line_start = text.char_to_line(pos.anchor);
    let start = text.line_to_char(line_start);
    let line_end = text.char_to_line(pos.head);
    let mut end = line_end_char_index(&text.slice(..), line_end);

    if pos.anchor == start && pos.head == end && line_end < (text.len_lines() - 2) {
        end = line_end_char_index(&text.slice(..), line_end + 1);
    }

    doc.set_selection(view.id, Selection::single(start, end));
}

fn delete_selection_impl(reg: &mut Register, doc: &mut Document, view_id: ViewId) {
    // first yank the selection
    let values: Vec<String> = doc
        .selection(view_id)
        .fragments(doc.text().slice(..))
        .map(Cow::into_owned)
        .collect();

    reg.write(values);

    // then delete
    let transaction =
        Transaction::change_by_selection(doc.text(), doc.selection(view_id), |range| {
            let alltext = doc.text().slice(..);
            let line = alltext.char_to_line(range.head);
            let max_to = rope_end_without_line_ending(&alltext);
            let to = std::cmp::min(max_to, range.to() + 1);
            (range.from(), to, None)
        });
    doc.apply(&transaction, view_id);
}

fn delete_selection(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;
    let reg = registers.get_or_insert(reg_name);
    delete_selection_impl(reg, doc, view.id);

    doc.append_changes_to_history(view.id);

    // exit select mode, if currently in select mode
    exit_select_mode(cx);
}

fn change_selection(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;
    let reg = registers.get_or_insert(reg_name);
    delete_selection_impl(reg, doc, view.id);
    enter_insert_mode(doc);
}

fn collapse_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = doc
        .selection(view.id)
        .transform(|range| Range::new(range.head, range.head));

    doc.set_selection(view.id, selection);
}

fn flip_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = doc
        .selection(view.id)
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
        .transform(|range| Range::new(range.to(), range.from()));
    doc.set_selection(view.id, selection);
}

// inserts at the end of each selection
fn append_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);
    doc.restore_cursor = true;

    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(text, range.to()), // to() + next char
        )
    });

    let end = text.len_chars();

    if selection.iter().any(|range| range.head == end) {
        let transaction = Transaction::change(
            doc.text(),
            std::array::IntoIter::new([(end, end, Some(doc.line_ending.as_str().into()))]),
        );
        doc.apply(&transaction, view.id);
    }

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
        pub alias: Option<&'static str>,
        pub doc: &'static str,
        // params, flags, helper, completer
        pub fun: fn(&mut compositor::Context, &[&str], PromptEvent),
        pub completer: Option<Completer>,
    }

    fn quit(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        // last view and we have unsaved changes
        if cx.editor.tree.views().count() == 1 && buffers_remaining_impl(cx.editor) {
            return;
        }
        cx.editor
            .close(view!(cx.editor).id, /* close_buffer */ false);
    }

    fn force_quit(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        cx.editor
            .close(view!(cx.editor).id, /* close_buffer */ false);
    }

    fn open(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        match args.get(0) {
            Some(path) => {
                // TODO: handle error
                cx.editor.open(path.into(), Action::Replace);
            }
            None => {
                cx.editor.set_error("wrong argument count".to_string());
            }
        };
    }

    fn write_impl<P: AsRef<Path>>(
        view: &View,
        doc: &mut Document,
        path: Option<P>,
    ) -> Result<tokio::task::JoinHandle<Result<(), anyhow::Error>>, anyhow::Error> {
        use anyhow::anyhow;

        if let Some(path) = path {
            if let Err(err) = doc.set_path(path.as_ref()) {
                return Err(anyhow!("invalid filepath: {}", err));
            };
        }
        if doc.path().is_none() {
            return Err(anyhow!("cannot write a buffer without a filename"));
        }
        let autofmt = doc
            .language_config()
            .map(|config| config.auto_format)
            .unwrap_or_default();
        if autofmt {
            doc.format(view.id); // TODO: merge into save
        }
        Ok(tokio::spawn(doc.save()))
    }

    fn write(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        let (view, doc) = current!(cx.editor);
        if let Err(e) = write_impl(view, doc, args.first()) {
            cx.editor.set_error(e.to_string());
        };
    }

    fn new_file(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        cx.editor.new_file(Action::Replace);
    }

    fn format(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        let (view, doc) = current!(cx.editor);

        doc.format(view.id)
    }

    fn set_indent_style(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        use IndentStyle::*;

        // If no argument, report current indent style.
        if args.is_empty() {
            let style = current!(cx.editor).1.indent_style;
            cx.editor.set_status(match style {
                Tabs => "tabs".into(),
                Spaces(1) => "1 space".into(),
                Spaces(n) if (2..=8).contains(&n) => format!("{} spaces", n),
                _ => "error".into(), // Shouldn't happen.
            });
            return;
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

        if let Some(s) = style {
            let doc = doc_mut!(cx.editor);
            doc.indent_style = s;
        } else {
            // Invalid argument.
            cx.editor
                .set_error(format!("invalid indent style '{}'", args[0],));
        }
    }

    /// Sets or reports the current document's line ending setting.
    fn set_line_ending(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        use LineEnding::*;

        // If no argument, report current line ending setting.
        if args.is_empty() {
            let line_ending = current!(cx.editor).1.line_ending;
            cx.editor.set_status(match line_ending {
                Crlf => "crlf".into(),
                LF => "line feed".into(),
                FF => "form feed".into(),
                CR => "carriage return".into(),
                Nel => "next line".into(),

                // These should never be a document's default line ending.
                VT | LS | PS => "error".into(),
            });
            return;
        }

        // Attempt to parse argument as a line ending.
        let line_ending = match args.get(0) {
            // We check for CR first because it shares a common prefix with CRLF.
            Some(arg) if "cr".starts_with(&arg.to_lowercase()) => Some(CR),
            Some(arg) if "crlf".starts_with(&arg.to_lowercase()) => Some(Crlf),
            Some(arg) if "lf".starts_with(&arg.to_lowercase()) => Some(LF),
            Some(arg) if "ff".starts_with(&arg.to_lowercase()) => Some(FF),
            Some(arg) if "nel".starts_with(&arg.to_lowercase()) => Some(Nel),
            _ => None,
        };

        if let Some(le) = line_ending {
            doc_mut!(cx.editor).line_ending = le;
        } else {
            // Invalid argument.
            cx.editor
                .set_error(format!("invalid line ending '{}'", args[0],));
        }
    }

    fn earlier(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        let uk = match args.join(" ").parse::<helix_core::history::UndoKind>() {
            Ok(uk) => uk,
            Err(msg) => {
                cx.editor.set_error(msg);
                return;
            }
        };
        let (view, doc) = current!(cx.editor);
        doc.earlier(view.id, uk)
    }

    fn later(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        let uk = match args.join(" ").parse::<helix_core::history::UndoKind>() {
            Ok(uk) => uk,
            Err(msg) => {
                cx.editor.set_error(msg);
                return;
            }
        };
        let (view, doc) = current!(cx.editor);
        doc.later(view.id, uk)
    }

    fn write_quit(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        let (view, doc) = current!(cx.editor);
        match write_impl(view, doc, args.first()) {
            Ok(handle) => {
                if let Err(e) = helix_lsp::block_on(handle) {
                    cx.editor.set_error(e.to_string());
                } else {
                    quit(cx, &[], event);
                }
            }
            Err(e) => {
                cx.editor.set_error(e.to_string());
            }
        }
    }

    fn force_write_quit(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        let (view, doc) = current!(cx.editor);
        match write_impl(view, doc, args.first()) {
            Ok(handle) => {
                if let Err(e) = helix_lsp::block_on(handle) {
                    cx.editor.set_error(e.to_string());
                } else {
                    force_quit(cx, &[], event);
                }
            }
            Err(e) => {
                cx.editor.set_error(e.to_string());
            }
        }
    }

    /// Returns `true` if there are modified buffers remaining and sets editor error,
    /// otherwise returns `false`
    fn buffers_remaining_impl(editor: &mut Editor) -> bool {
        let modified: Vec<_> = editor
            .documents()
            .filter(|doc| doc.is_modified())
            .map(|doc| {
                doc.relative_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| "[scratch]".into())
            })
            .collect();
        if !modified.is_empty() {
            let err = format!(
                "{} unsaved buffer(s) remaining: {:?}",
                modified.len(),
                modified
            );
            editor.set_error(err);
            true
        } else {
            false
        }
    }

    fn write_all_impl(
        editor: &mut Editor,
        args: &[&str],
        event: PromptEvent,
        quit: bool,
        force: bool,
    ) {
        let mut errors = String::new();

        // save all documents
        for (id, mut doc) in &mut editor.documents {
            if doc.path().is_none() {
                errors.push_str("cannot write a buffer without a filename\n");
                continue;
            }
            helix_lsp::block_on(tokio::spawn(doc.save()));
        }
        editor.set_error(errors);

        if quit {
            if !force && buffers_remaining_impl(editor) {
                return;
            }

            // close all views
            let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
            for view_id in views {
                editor.close(view_id, false);
            }
        }
    }

    fn write_all(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        write_all_impl(&mut cx.editor, args, event, false, false)
    }

    fn write_all_quit(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        write_all_impl(&mut cx.editor, args, event, true, false)
    }

    fn force_write_all_quit(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        write_all_impl(&mut cx.editor, args, event, true, true)
    }

    fn quit_all_impl(editor: &mut Editor, args: &[&str], event: PromptEvent, force: bool) {
        if !force && buffers_remaining_impl(editor) {
            return;
        }

        // close all views
        let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
        for view_id in views {
            editor.close(view_id, false);
        }
    }

    fn quit_all(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        quit_all_impl(&mut cx.editor, args, event, false)
    }

    fn force_quit_all(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        quit_all_impl(&mut cx.editor, args, event, true)
    }

    fn theme(cx: &mut compositor::Context, args: &[&str], event: PromptEvent) {
        let theme = if let Some(theme) = args.first() {
            theme
        } else {
            cx.editor.set_error("theme name not provided".into());
            return;
        };

        cx.editor.set_theme_from_name(theme);
    }

    fn yank_main_selection_to_clipboard(cx: &mut compositor::Context, _: &[&str], _: PromptEvent) {
        yank_main_selection_to_clipboard_impl(&mut cx.editor);
    }

    fn yank_joined_to_clipboard(cx: &mut compositor::Context, args: &[&str], _: PromptEvent) {
        let (_, doc) = current!(cx.editor);
        let separator = args
            .first()
            .copied()
            .unwrap_or_else(|| doc.line_ending.as_str());
        yank_joined_to_clipboard_impl(&mut cx.editor, separator);
    }

    fn paste_clipboard_after(cx: &mut compositor::Context, _: &[&str], _: PromptEvent) {
        paste_clipboard_impl(&mut cx.editor, Paste::After);
    }

    fn paste_clipboard_before(cx: &mut compositor::Context, _: &[&str], _: PromptEvent) {
        paste_clipboard_impl(&mut cx.editor, Paste::After);
    }

    fn replace_selections_with_clipboard(cx: &mut compositor::Context, _: &[&str], _: PromptEvent) {
        let (view, doc) = current!(cx.editor);

        match cx.editor.clipboard_provider.get_contents() {
            Ok(contents) => {
                let transaction =
                    Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                        let max_to = rope_end_without_line_ending(&doc.text().slice(..));
                        let to = std::cmp::min(max_to, range.to() + 1);
                        (range.from(), to, Some(contents.as_str().into()))
                    });

                doc.apply(&transaction, view.id);
                doc.append_changes_to_history(view.id);
            }
            Err(e) => log::error!("Couldn't get system clipboard contents: {:?}", e),
        }
    }

    fn show_clipboard_provider(cx: &mut compositor::Context, _: &[&str], _: PromptEvent) {
        cx.editor
            .set_status(cx.editor.clipboard_provider.name().into());
    }

    fn change_current_directory(cx: &mut compositor::Context, args: &[&str], _: PromptEvent) {
        let dir = match args.first() {
            Some(dir) => dir,
            None => {
                cx.editor.set_error("target directory not provided".into());
                return;
            }
        };

        if let Err(e) = std::env::set_current_dir(dir) {
            cx.editor.set_error(format!(
                "Couldn't change the current working directory: {:?}",
                e
            ));
            return;
        }

        match std::env::current_dir() {
            Ok(cwd) => cx.editor.set_status(format!(
                "Current working directory is now {}",
                cwd.display()
            )),
            Err(e) => cx
                .editor
                .set_error(format!("Couldn't get the new working directory: {}", e)),
        }
    }

    fn show_current_directory(cx: &mut compositor::Context, args: &[&str], _: PromptEvent) {
        match std::env::current_dir() {
            Ok(cwd) => cx
                .editor
                .set_status(format!("Current working directory is {}", cwd.display())),
            Err(e) => cx
                .editor
                .set_error(format!("Couldn't get the current working directory: {}", e)),
        }
    }

    pub const TYPABLE_COMMAND_LIST: &[TypableCommand] = &[
        TypableCommand {
            name: "quit",
            alias: Some("q"),
            doc: "Close the current view.",
            fun: quit,
            completer: None,
        },
        TypableCommand {
            name: "quit!",
            alias: Some("q!"),
            doc: "Close the current view.",
            fun: force_quit,
            completer: None,
        },
        TypableCommand {
            name: "open",
            alias: Some("o"),
            doc: "Open a file from disk into the current view.",
            fun: open,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write",
            alias: Some("w"),
            doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)",
            fun: write,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "new",
            alias: Some("n"),
            doc: "Create a new scratch buffer.",
            fun: new_file,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "format",
            alias: Some("fmt"),
            doc: "Format the file using a formatter.",
            fun: format,
            completer: None,
        },
        TypableCommand {
            name: "indent-style",
            alias: None,
            doc: "Set the indentation style for editing. ('t' for tabs or 1-8 for number of spaces.)",
            fun: set_indent_style,
            completer: None,
        },
        TypableCommand {
            name: "line-ending",
            alias: None,
            doc: "Set the document's default line ending. Options: crlf, lf, cr, ff, nel.",
            fun: set_line_ending,
            completer: None,
        },
        TypableCommand {
            name: "earlier",
            alias: Some("ear"),
            doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
            fun: earlier,
            completer: None,
        },
        TypableCommand {
            name: "later",
            alias: Some("lat"),
            doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.",
            fun: later,
            completer: None,
        },
        TypableCommand {
            name: "write-quit",
            alias: Some("wq"),
            doc: "Writes changes to disk and closes the current view. Accepts an optional path (:wq some/path.txt)",
            fun: write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-quit!",
            alias: Some("wq!"),
            doc: "Writes changes to disk and closes the current view forcefully. Accepts an optional path (:wq! some/path.txt)",
            fun: force_write_quit,
            completer: Some(completers::filename),
        },
        TypableCommand {
            name: "write-all",
            alias: Some("wa"),
            doc: "Writes changes from all views to disk.",
            fun: write_all,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all",
            alias: Some("wqa"),
            doc: "Writes changes from all views to disk and close all views.",
            fun: write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "write-quit-all!",
            alias: Some("wqa!"),
            doc: "Writes changes from all views to disk and close all views forcefully (ignoring unsaved changes).",
            fun: force_write_all_quit,
            completer: None,
        },
        TypableCommand {
            name: "quit-all",
            alias: Some("qa"),
            doc: "Close all views.",
            fun: quit_all,
            completer: None,
        },
        TypableCommand {
            name: "quit-all!",
            alias: Some("qa!"),
            doc: "Close all views forcefully (ignoring unsaved changes).",
            fun: force_quit_all,
            completer: None,
        },
        TypableCommand {
            name: "theme",
            alias: None,
            doc: "Change the theme of current view. Requires theme name as argument (:theme <name>)",
            fun: theme,
            completer: Some(completers::theme),
        },
        TypableCommand {
            name: "clipboard-yank",
            alias: None,
            doc: "Yank main selection into system clipboard.",
            fun: yank_main_selection_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-yank-join",
            alias: None,
            doc: "Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.", // FIXME: current UI can't display long doc.
            fun: yank_joined_to_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-after",
            alias: None,
            doc: "Paste system clipboard after selections.",
            fun: paste_clipboard_after,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-before",
            alias: None,
            doc: "Paste system clipboard before selections.",
            fun: paste_clipboard_before,
            completer: None,
        },
        TypableCommand {
            name: "clipboard-paste-replace",
            alias: None,
            doc: "Replace selections with content of system clipboard.",
            fun: replace_selections_with_clipboard,
            completer: None,
        },
        TypableCommand {
            name: "show-clipboard-provider",
            alias: None,
            doc: "Show clipboard provider name in status bar.",
            fun: show_clipboard_provider,
            completer: None,
        },
        TypableCommand {
            name: "change-current-directory",
            alias: Some("cd"),
            doc: "Change the current working directory (:cd <dir>).",
            fun: change_current_directory,
            completer: Some(completers::directory),
        },
        TypableCommand {
            name: "show-directory",
            alias: Some("pwd"),
            doc: "Show the current working directory.",
            fun: show_current_directory,
            completer: None,
        },
    ];

    pub static COMMANDS: Lazy<HashMap<&'static str, &'static TypableCommand>> = Lazy::new(|| {
        let mut map = HashMap::new();

        for cmd in TYPABLE_COMMAND_LIST {
            map.insert(cmd.name, cmd);
            if let Some(alias) = cmd.alias {
                map.insert(alias, cmd);
            }
        }

        map
    });
}

fn command_mode(cx: &mut Context) {
    let mut prompt = Prompt::new(
        ":".to_owned(),
        |input: &str| {
            // we use .this over split_whitespace() because we care about empty segments
            let parts = input.split(' ').collect::<Vec<&str>>();

            // simple heuristic: if there's no just one part, complete command name.
            // if there's a space, per command completion kicks in.
            if parts.len() <= 1 {
                use std::{borrow::Cow, ops::Range};
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
            use helix_view::editor::Action;

            if event != PromptEvent::Validate {
                return;
            }

            let parts = input.split_whitespace().collect::<Vec<&str>>();
            if parts.is_empty() {
                return;
            }

            if let Some(cmd) = cmd::COMMANDS.get(parts[0]) {
                (cmd.fun)(cx, &parts[1..], event);
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
    let picker = ui::file_picker(root);
    cx.push_layer(Box::new(picker));
}

fn buffer_picker(cx: &mut Context) {
    use std::path::{Path, PathBuf};
    let current = view!(cx.editor).doc;

    let picker = Picker::new(
        cx.editor
            .documents
            .iter()
            .map(|(id, doc)| (id, doc.relative_path()))
            .collect(),
        move |(id, path): &(DocumentId, Option<PathBuf>)| {
            // format_fn
            match path.as_ref().and_then(|path| path.to_str()) {
                Some(path) => {
                    if *id == current {
                        format!("{} (*)", path).into()
                    } else {
                        path.into()
                    }
                }
                None => "[scratch buffer]".into(),
            }
        },
        |editor: &mut Editor, (id, _path): &(DocumentId, Option<PathBuf>), _action| {
            use helix_view::editor::Action;
            editor.switch(*id, Action::Replace);
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
    let (view, doc) = current!(cx.editor);

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

                let picker = Picker::new(
                    symbols,
                    |symbol| (&symbol.name).into(),
                    move |editor: &mut Editor, symbol, _action| {
                        push_jump(editor);
                        let (view, doc) = current!(editor);

                        if let Some(range) =
                            lsp_range_to_range(doc.text(), symbol.location.range, offset_encoding)
                        {
                            doc.set_selection(view.id, Selection::single(range.to(), range.from()));
                            align_view(doc, view, Align::Center);
                        }
                    },
                );
                compositor.push(Box::new(picker))
            }
        },
    )
}

// I inserts at the first nonwhitespace character of each line with a selection
fn prepend_to_line(cx: &mut Context) {
    move_first_nonwhitespace(cx);
    let doc = doc_mut!(cx.editor);
    enter_insert_mode(doc);
}

// A inserts at the end of each line with a selection
fn append_to_line(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    enter_insert_mode(doc);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);
        let pos = line_end_char_index(&text.slice(..), line);
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
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
        let line = text.char_to_line(range.head);

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

    doc.mode = Mode::Normal;

    doc.append_changes_to_history(view.id);

    // if leaving append mode, move cursor back by 1
    if doc.restore_cursor {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id).transform(|range| {
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

fn switch_to_last_accessed_file(cx: &mut Context) {
    let alternate_file = view!(cx.editor).last_accessed_doc;
    if let Some(alt) = alternate_file {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last accessed buffer".to_owned())
    }
}

fn goto_mode(cx: &mut Context) {
    if let Some(count) = cx.count {
        push_jump(cx.editor);

        let (view, doc) = current!(cx.editor);
        let line_idx = std::cmp::min(count.get() - 1, doc.text().len_lines().saturating_sub(2));
        let pos = doc.text().line_to_char(line_idx);
        doc.set_selection(view.id, Selection::point(pos));
        return;
    }

    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            // TODO: temporarily show GOTO in the mode list
            let doc = doc_mut!(cx.editor);
            match (doc.mode, ch) {
                (_, 'g') => move_file_start(cx),
                (_, 'e') => move_file_end(cx),
                (_, 'a') => switch_to_last_accessed_file(cx),
                (Mode::Normal, 'h') => move_line_start(cx),
                (Mode::Normal, 'l') => move_line_end(cx),
                (Mode::Select, 'h') => extend_line_start(cx),
                (Mode::Select, 'l') => extend_line_end(cx),
                (_, 'd') => goto_definition(cx),
                (_, 'y') => goto_type_definition(cx),
                (_, 'r') => goto_reference(cx),
                (_, 'i') => goto_implementation(cx),
                (Mode::Normal, 's') => move_first_nonwhitespace(cx),
                (Mode::Select, 's') => extend_first_nonwhitespace(cx),

                (_, 't') | (_, 'm') | (_, 'b') => {
                    let (view, doc) = current!(cx.editor);

                    let pos = doc.selection(view.id).cursor();
                    let line = doc.text().char_to_line(pos);

                    let scrolloff = PADDING.min(view.area.height as usize / 2); // TODO: user pref

                    let last_line = view.last_line(doc);

                    let line = match ch {
                        't' => (view.first_line + scrolloff),
                        'm' => (view.first_line + (view.area.height as usize / 2)),
                        'b' => last_line.saturating_sub(scrolloff),
                        _ => unreachable!(),
                    }
                    .min(last_line.saturating_sub(scrolloff));

                    let pos = doc.text().line_to_char(line);

                    doc.set_selection(view.id, Selection::point(pos));
                }
                _ => (),
            }
        }
    })
}

fn select_mode(cx: &mut Context) {
    doc_mut!(cx.editor).mode = Mode::Select;
}

fn exit_select_mode(cx: &mut Context) {
    doc_mut!(cx.editor).mode = Mode::Normal;
}

fn goto_impl(
    editor: &mut Editor,
    compositor: &mut Compositor,
    locations: Vec<lsp::Location>,
    offset_encoding: OffsetEncoding,
) {
    use helix_view::editor::Action;

    push_jump(editor);

    fn jump_to(
        editor: &mut Editor,
        location: &lsp::Location,
        offset_encoding: OffsetEncoding,
        action: Action,
    ) {
        let id = editor
            .open(PathBuf::from(location.uri.path()), action)
            .expect("editor.open failed");
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

    match locations.as_slice() {
        [location] => {
            jump_to(editor, location, offset_encoding, Action::Replace);
        }
        [] => {
            editor.set_error("No definition found.".to_string());
        }
        _locations => {
            let mut picker = ui::Picker::new(
                locations,
                |location| {
                    let file = location.uri.as_str();
                    let line = location.range.start.line;
                    format!("{}:{}", file, line).into()
                },
                move |editor: &mut Editor, location, action| {
                    jump_to(editor, location, offset_encoding, action)
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

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
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

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
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

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
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

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
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
    let (view, doc) = current!(editor);

    let cursor_pos = doc.selection(view.id).cursor();
    let diag = if let Some(diag) = doc.diagnostics().first() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn goto_last_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);

    let cursor_pos = doc.selection(view.id).cursor();
    let diag = if let Some(diag) = doc.diagnostics().last() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn goto_next_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);

    let cursor_pos = doc.selection(view.id).cursor();
    let diag = if let Some(diag) = doc
        .diagnostics()
        .iter()
        .map(|diag| diag.range.start)
        .find(|&pos| pos > cursor_pos)
    {
        diag
    } else if let Some(diag) = doc.diagnostics().first() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn goto_prev_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);

    let cursor_pos = doc.selection(view.id).cursor();
    let diag = if let Some(diag) = doc
        .diagnostics()
        .iter()
        .rev()
        .map(|diag| diag.range.start)
        .find(|&pos| pos < cursor_pos)
    {
        diag
    } else if let Some(diag) = doc.diagnostics().last() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

fn signature_help(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id).cursor(),
        language_server.offset_encoding(),
    );

    // TODO: handle fails
    let future = language_server.text_document_signature_help(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
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

    fn completion(cx: &mut Context, ch: char) {
        // if ch matches completion char, trigger completion
        let doc = doc_mut!(cx.editor);
        let language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };

        let capabilities = language_server.capabilities();

        if let lsp::ServerCapabilities {
            completion_provider:
                Some(lsp::CompletionOptions {
                    trigger_characters: Some(triggers),
                    ..
                }),
            ..
        } = capabilities
        {
            // TODO: what if trigger is multiple chars long
            let is_trigger = triggers.iter().any(|trigger| trigger.contains(ch));

            if is_trigger {
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
    const HOOKS: &[Hook] = &[auto_pairs::hook, insert];
    const POST_HOOKS: &[PostHook] = &[completion, signature_help];

    pub fn insert_char(cx: &mut Context, c: char) {
        let (view, doc) = current!(cx.editor);

        let text = doc.text();
        let selection = doc.selection(view.id);

        // run through insert hooks, stopping on the first one that returns Some(t)
        for hook in HOOKS {
            if let Some(transaction) = hook(text, selection, c) {
                doc.apply(&transaction, view.id);
                break;
            }
        }

        // TODO: need a post insert hook too for certain triggers (autocomplete, signature help, etc)
        // this could also generically look at Transaction, but it's a bit annoying to look at
        // Operation instead of Change.
        for hook in POST_HOOKS {
            hook(cx, c);
        }
    }

    pub fn insert_tab(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        // TODO: round out to nearest indentation level (for example a line with 3 spaces should
        // indent by one to reach 4 spaces).

        let indent = Tendril::from(doc.indent_unit());
        let transaction = Transaction::insert(doc.text(), doc.selection(view.id), indent);
        doc.apply(&transaction, view.id);
    }

    pub fn insert_newline(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let contents = doc.text();
        let selection = doc.selection(view.id);
        let mut ranges = SmallVec::with_capacity(selection.len());

        // TODO: this is annoying, but we need to do it to properly calculate pos after edits
        let mut offs = 0;

        let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
            let pos = range.head;

            let prev = if pos == 0 {
                ' '
            } else {
                contents.char(pos - 1)
            };
            let curr = contents.char(pos);

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

    // TODO: handle indent-aware delete
    pub fn delete_char_backward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                (
                    graphemes::nth_prev_grapheme_boundary(text, range.head, count),
                    range.head,
                    None,
                )
            });
        doc.apply(&transaction, view.id);
    }

    pub fn delete_char_forward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                (
                    range.head,
                    graphemes::nth_next_grapheme_boundary(text, range.head, count),
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
            .transform(|range| movement::move_prev_word_start(text, range, count));
        doc.set_selection(view.id, selection);
        delete_selection(cx)
    }
}

// Undo / Redo

// TODO: each command could simply return a Option<transaction>, then the higher level handles
// storing it?

fn undo(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    doc.undo(view_id);
}

fn redo(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    doc.redo(view_id);
}

// Yank / Paste

fn yank(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(doc.text().slice(..))
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "yanked {} selection(s) to register {}",
        values.len(),
        cx.selected_register.name()
    );

    cx.editor
        .registers
        .write(cx.selected_register.name(), values);

    cx.editor.set_status(msg)
}

fn yank_joined_to_clipboard_impl(editor: &mut Editor, separator: &str) {
    let (view, doc) = current!(editor);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(doc.text().slice(..))
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "joined and yanked {} selection(s) to system clipboard",
        values.len(),
    );

    let joined = values.join(separator);

    if let Err(e) = editor.clipboard_provider.set_contents(joined) {
        log::error!("Couldn't set system clipboard content: {:?}", e);
    }

    editor.set_status(msg);
}

fn yank_joined_to_clipboard(cx: &mut Context) {
    let line_ending = current!(cx.editor).1.line_ending;
    yank_joined_to_clipboard_impl(&mut cx.editor, line_ending.as_str());
}

fn yank_main_selection_to_clipboard_impl(editor: &mut Editor) {
    let (view, doc) = current!(editor);

    let value = doc
        .selection(view.id)
        .primary()
        .fragment(doc.text().slice(..));

    if let Err(e) = editor.clipboard_provider.set_contents(value.into_owned()) {
        log::error!("Couldn't set system clipboard content: {:?}", e);
    }

    editor.set_status("yanked main selection to system clipboard".to_owned());
}

fn yank_main_selection_to_clipboard(cx: &mut Context) {
    yank_main_selection_to_clipboard_impl(&mut cx.editor);
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

    let mut values = values.iter().cloned().map(Tendril::from).chain(repeat);

    let text = doc.text();

    let transaction = Transaction::change_by_selection(text, doc.selection(view.id), |range| {
        let pos = match (action, linewise) {
            // paste linewise before
            (Paste::Before, true) => text.line_to_char(text.char_to_line(range.from())),
            // paste linewise after
            (Paste::After, true) => text.line_to_char(text.char_to_line(range.to()) + 1),
            // paste insert
            (Paste::Before, false) => range.from(),
            // paste append
            (Paste::After, false) => range.to() + 1,
        };
        (pos, pos, Some(values.next().unwrap()))
    });

    Some(transaction)
}

fn paste_clipboard_impl(editor: &mut Editor, action: Paste) {
    let (view, doc) = current!(editor);

    match editor
        .clipboard_provider
        .get_contents()
        .map(|contents| paste_impl(&[contents], doc, view, action))
    {
        Ok(Some(transaction)) => {
            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
        Ok(None) => {}
        Err(e) => log::error!("Couldn't get system clipboard contents: {:?}", e),
    }
}

fn paste_clipboard_after(cx: &mut Context) {
    paste_clipboard_impl(&mut cx.editor, Paste::After);
}

fn paste_clipboard_before(cx: &mut Context) {
    paste_clipboard_impl(&mut cx.editor, Paste::Before);
}

fn replace_with_yanked(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(values) = registers.read(reg_name) {
        if let Some(yank) = values.first() {
            let transaction =
                Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                    let max_to = rope_end_without_line_ending(&doc.text().slice(..));
                    let to = std::cmp::min(max_to, range.to() + 1);
                    (range.from(), to, Some(yank.as_str().into()))
                });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    }
}

fn replace_selections_with_clipboard_impl(editor: &mut Editor) {
    let (view, doc) = current!(editor);

    match editor.clipboard_provider.get_contents() {
        Ok(contents) => {
            let transaction =
                Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                    let max_to = rope_end_without_line_ending(&doc.text().slice(..));
                    let to = std::cmp::min(max_to, range.to() + 1);
                    (range.from(), to, Some(contents.as_str().into()))
                });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
        Err(e) => log::error!("Couldn't get system clipboard contents: {:?}", e),
    }
}

fn replace_selections_with_clipboard(cx: &mut Context) {
    replace_selections_with_clipboard_impl(&mut cx.editor);
}

fn paste_after(cx: &mut Context) {
    let reg_name = cx.selected_register.name();
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
    let reg_name = cx.selected_register.name();
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
        let start = doc.text().char_to_line(range.from());
        let end = doc.text().char_to_line(range.to());

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

    for range in ranges {
        let language_server = match doc.language_server() {
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
        let start = text.char_to_line(selection.from());
        let mut end = text.char_to_line(selection.to());
        if start == end {
            end += 1
        }
        let lines = start..end;

        changes.reserve(lines.len());

        for line in lines {
            let mut start = line_end_char_index(&slice, line);
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

fn keep_selections(cx: &mut Context) {
    // keep selections matching regex
    let prompt = ui::regex_prompt(cx, "keep:".to_string(), move |view, doc, _, regex| {
        let text = doc.text().slice(..);

        if let Some(selection) = selection::keep_matches(text, doc.selection(view.id), &regex) {
            doc.set_selection(view.id, selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

fn keep_primary_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let range = doc.selection(view.id).primary();
    let selection = Selection::single(range.anchor, range.head);
    doc.set_selection(view.id, selection);
}

fn completion(cx: &mut Context) {
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

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
    let future = language_server.completion(doc.identifier(), pos, None);

    let trigger_offset = doc.selection(view.id).cursor();

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              response: Option<lsp::CompletionResponse>| {
            let items = match response {
                Some(lsp::CompletionResponse::Array(items)) => items,
                // TODO: do something with is_incomplete
                Some(lsp::CompletionResponse::List(lsp::CompletionList {
                    is_incomplete: _is_incomplete,
                    items,
                })) => items,
                None => Vec::new(),
            };

            // TODO: if no completion, show some message or something
            if items.is_empty() {
                return;
            }
            use crate::compositor::AnyComponent;
            let size = compositor.size();
            let ui = compositor
                .find(std::any::type_name::<ui::EditorView>())
                .unwrap();
            if let Some(ui) = ui.as_any_mut().downcast_mut::<ui::EditorView>() {
                ui.set_completion(items, offset_encoding, trigger_offset, size);
            };
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
        doc.selection(view.id).cursor(),
        language_server.offset_encoding(),
    );

    // TODO: handle fails
    let future = language_server.text_document_hover(doc.identifier(), pos, None);

    cx.callback(
        future,
        move |editor: &mut Editor, compositor: &mut Compositor, response: Option<lsp::Hover>| {
            if let Some(hover) = response {
                // hover.contents / .range <- used for visualizing
                let contents = match hover.contents {
                    lsp::HoverContents::Scalar(contents) => {
                        // markedstring(string/languagestring to be highlighted)
                        // TODO
                        log::error!("hover contents {:?}", contents);
                        return;
                    }
                    lsp::HoverContents::Array(contents) => {
                        log::error!("hover contents {:?}", contents);
                        return;
                    }
                    // TODO: render markdown
                    lsp::HoverContents::Markup(contents) => contents.value,
                };

                // skip if contents empty

                let contents = ui::Markdown::new(contents, editor.syn_loader.clone());
                let mut popup = Popup::new(contents);
                compositor.push(Box::new(popup));
            }
        },
    );
}

// comments
fn toggle_comments(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let transaction = comment::toggle_line_comments(doc.text(), doc.selection(view.id));

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

// tree sitter node selection

fn expand_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let selection = object::expand_selection(syntax, text, doc.selection(view.id));
        doc.set_selection(view.id, selection);
    }
}

fn match_brackets(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    if let Some(syntax) = doc.syntax() {
        let pos = doc.selection(view.id).cursor();
        if let Some(pos) = match_brackets::find(syntax, doc.text(), pos) {
            let selection = Selection::point(pos);
            doc.set_selection(view.id, selection);
        };
    }
}

//

fn jump_forward(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

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

fn window_mode(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            match ch {
                'w' => rotate_view(cx),
                'h' => hsplit(cx),
                'v' => vsplit(cx),
                'q' => wclose(cx),
                _ => {}
            }
        }
    })
}

fn rotate_view(cx: &mut Context) {
    cx.editor.focus_next()
}

// split helper, clear it later
use helix_view::editor::Action;

use self::cmd::TypableCommand;
fn split(cx: &mut Context, action: Action) {
    use helix_view::editor::Action;
    let (view, doc) = current!(cx.editor);
    let id = doc.id();
    let selection = doc.selection(view.id).clone();
    let first_line = view.first_line;

    cx.editor.switch(id, action);

    // match the selection in the previous view
    let (view, doc) = current!(cx.editor);
    view.first_line = first_line;
    doc.set_selection(view.id, selection);
}

fn hsplit(cx: &mut Context) {
    split(cx, Action::HorizontalSplit);
}

fn vsplit(cx: &mut Context) {
    split(cx, Action::VerticalSplit);
}

fn wclose(cx: &mut Context) {
    let view_id = view!(cx.editor).id;
    // close current split
    cx.editor.close(view_id, /* close_buffer */ false);
}

fn select_register(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            cx.editor.selected_register.select(ch);
        }
    })
}

fn space_mode(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            // TODO: temporarily show SPC in the mode list
            match ch {
                'f' => file_picker(cx),
                'b' => buffer_picker(cx),
                's' => symbol_picker(cx),
                'w' => window_mode(cx),
                'y' => yank_joined_to_clipboard(cx),
                'Y' => yank_main_selection_to_clipboard(cx),
                'p' => paste_clipboard_after(cx),
                'P' => paste_clipboard_before(cx),
                'R' => replace_selections_with_clipboard(cx),
                // ' ' => toggle_alternate_buffer(cx),
                // TODO: temporary since space mode took its old key
                ' ' => keep_primary_selection(cx),
                _ => (),
            }
        }
    })
}

fn view_mode(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            // if lock, call cx again
            // TODO: temporarily show VIE in the mode list
            match ch {
                // center
                'z' | 'c'
                // top
                | 't'
                // bottom
                | 'b' => {
                    let (view, doc) = current!(cx.editor);

                    align_view(doc, view, match ch {
                        'z' | 'c' => Align::Center,
                        't' => Align::Top,
                        'b' => Align::Bottom,
                        _ => unreachable!()
                    });
                }
                'm' => {
                    let (view, doc) = current!(cx.editor);
                    let pos = doc.selection(view.id).cursor();
                    let pos = coords_at_pos(doc.text().slice(..), pos);

                    const OFFSET: usize = 7; // gutters
                    view.first_col = pos.col.saturating_sub(((view.area.width as usize).saturating_sub(OFFSET)) / 2);
                },
                'h' => (),
                'j' => scroll(cx, 1, Direction::Forward),
                'k' => scroll(cx, 1, Direction::Backward),
                'l' => (),
                _ => (),
            }
        }
    })
}

fn left_bracket_mode(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            match ch {
                'd' => goto_prev_diag(cx),
                'D' => goto_first_diag(cx),
                _ => (),
            }
        }
    })
}

fn right_bracket_mode(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            match ch {
                'd' => goto_next_diag(cx),
                'D' => goto_last_diag(cx),
                _ => (),
            }
        }
    })
}

fn match_mode(cx: &mut Context) {
    let count = cx.count;
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            // FIXME: count gets reset because of cx.on_next_key()
            cx.count = count;
            match ch {
                'm' => match_brackets(cx),
                's' => surround_add(cx),
                'r' => surround_replace(cx),
                'd' => {
                    surround_delete(cx);
                    let (view, doc) = current!(cx.editor);
                }
                _ => (),
            }
        }
    })
}

use helix_core::surround;

fn surround_add(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            let (view, doc) = current!(cx.editor);
            let text = doc.text().slice(..);
            let selection = doc.selection(view.id);
            let (open, close) = surround::get_pair(ch);

            let mut changes = Vec::new();
            for (i, range) in selection.iter().enumerate() {
                let from = range.from();
                let line = text.char_to_line(range.to());
                let max_to = rope_end_without_line_ending(&text);
                let to = std::cmp::min(range.to() + 1, max_to);

                changes.push((from, from, Some(Tendril::from_char(open))));
                changes.push((to, to, Some(Tendril::from_char(close))));
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
        if let KeyEvent {
            code: KeyCode::Char(from),
            ..
        } = event
        {
            cx.on_next_key(move |cx, event| {
                if let KeyEvent {
                    code: KeyCode::Char(to),
                    ..
                } = event
                {
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
                            let ch = if i % 2 == 0 { open } else { close };
                            (pos, pos + 1, Some(Tendril::from_char(ch)))
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
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
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
