use helix_core::{
    comment, coords_at_pos, find_first_non_whitespace_char, find_root, graphemes, indent,
    match_brackets,
    movement::{self, Direction},
    object, pos_at_coords,
    regex::{self, Regex},
    register, search, selection, Change, ChangeSet, Position, Range, Rope, RopeSlice, Selection,
    SmallVec, Tendril, Transaction,
};

use helix_view::{
    document::Mode,
    view::{View, PADDING},
    Document, DocumentId, Editor, ViewId,
};

use helix_lsp::{
    lsp,
    util::{lsp_pos_to_pos, lsp_range_to_range, pos_to_lsp_pos, range_to_lsp_range},
    OffsetEncoding,
};
use movement::Movement;

use crate::{
    compositor::{Callback, Component, Compositor},
    ui::{self, Completion, Picker, Popup, Prompt, PromptEvent},
};

use crate::application::{LspCallbackWrapper, LspCallbacks};
use futures_util::FutureExt;
use std::future::Future;

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent};
use once_cell::sync::Lazy;

pub struct Context<'a> {
    pub register: helix_view::RegisterSelection,
    pub _count: Option<std::num::NonZeroUsize>,
    pub editor: &'a mut Editor,

    pub callback: Option<crate::compositor::Callback>,
    pub on_next_key_callback: Option<Box<dyn FnOnce(&mut Context, KeyEvent)>>,
    pub callbacks: &'a mut LspCallbacks,
}

impl<'a> Context<'a> {
    #[inline]
    pub fn view(&mut self) -> &mut View {
        self.editor.view_mut()
    }

    #[inline]
    pub fn doc(&mut self) -> &mut Document {
        let id = self.editor.view().doc;
        &mut self.editor.documents[id]
    }

    #[inline]
    pub fn current(&mut self) -> (&mut View, &mut Document) {
        self.editor.current()
    }

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

    #[inline]
    pub fn count(&self) -> usize {
        self._count.map_or(1, |v| v.get())
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

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(cx: &mut Context);

pub fn move_char_left(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Backward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

pub fn move_char_right(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Forward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

pub fn move_line_up(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Backward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

pub fn move_line_down(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Forward, count, Movement::Move)
    });
    doc.set_selection(view.id, selection);
}

pub fn move_line_end(cx: &mut Context) {
    let (view, doc) = cx.current();

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        // Line end is pos at the start of next line - 1
        // subtract another 1 because the line ends with \n
        let pos = text.line_to_char(line + 1).saturating_sub(2);
        Range::new(pos, pos)
    });

    doc.set_selection(view.id, selection);
}

pub fn move_line_start(cx: &mut Context) {
    let (view, doc) = cx.current();

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        Range::new(pos, pos)
    });

    doc.set_selection(view.id, selection);
}

pub fn move_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = cx.current();

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

pub fn move_next_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .transform(|range| movement::move_next_word_start(text, range, count));

    doc.set_selection(view.id, selection);
}

pub fn move_prev_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .transform(|range| movement::move_prev_word_start(text, range, count));

    doc.set_selection(view.id, selection);
}

pub fn move_next_word_end(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .transform(|range| movement::move_next_word_end(text, range, count));

    doc.set_selection(view.id, selection);
}

pub fn move_file_start(cx: &mut Context) {
    push_jump(cx.editor);
    let (view, doc) = cx.current();
    doc.set_selection(view.id, Selection::point(0));
}

pub fn move_file_end(cx: &mut Context) {
    push_jump(cx.editor);
    let (view, doc) = cx.current();
    let text = doc.text();
    let last_line = text.line_to_char(text.len_lines().saturating_sub(2));
    doc.set_selection(view.id, Selection::point(last_line));
}

pub fn extend_next_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_next_word_start(text, range, count);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

pub fn extend_prev_word_start(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_prev_word_start(text, range, count);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_next_word_end(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_next_word_end(text, range, count);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

#[inline]
fn _find_char<F>(cx: &mut Context, search_fn: F, inclusive: bool, extend: bool)
where
    // TODO: make an options struct for and abstract this Fn into a searcher type
    // use the definition for w/b/e too
    F: Fn(RopeSlice, char, usize, usize, bool) -> Option<usize> + 'static,
{
    // TODO: count is reset to 1 before next key so we move it into the closure here.
    // Would be nice to carry over.
    let count = cx.count();

    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        let ch = match event {
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => '\n',
            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => ch,
            _ => return,
        };

        let (view, doc) = cx.current();
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

pub fn find_till_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_next,
        false, /* inclusive */
        false, /* extend */
    )
}

pub fn find_next_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_next,
        true,  /* inclusive */
        false, /* extend */
    )
}

pub fn extend_till_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_next,
        false, /* inclusive */
        true,  /* extend */
    )
}

pub fn extend_next_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_next,
        true, /* inclusive */
        true, /* extend */
    )
}

pub fn till_prev_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_prev,
        false, /* inclusive */
        false, /* extend */
    )
}

pub fn find_prev_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_prev,
        true,  /* inclusive */
        false, /* extend */
    )
}

pub fn extend_till_prev_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_prev,
        false, /* inclusive */
        true,  /* extend */
    )
}

pub fn extend_prev_char(cx: &mut Context) {
    _find_char(
        cx,
        search::find_nth_prev,
        true, /* inclusive */
        true, /* extend */
    )
}

pub fn extend_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = cx.current();

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

pub fn replace(cx: &mut Context) {
    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        let ch = match event {
            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => Some(ch),
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => Some('\n'),
            _ => None,
        };

        if let Some(ch) = ch {
            let (view, doc) = cx.current();

            let transaction =
                Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                    let max_to = doc.text().len_chars().saturating_sub(1);
                    let to = std::cmp::min(max_to, range.to() + 1);
                    let text: String = doc
                        .text()
                        .slice(range.from()..to)
                        .chars()
                        .map(|c| if c == '\n' { '\n' } else { ch })
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
    let (view, doc) = cx.current();
    let cursor = coords_at_pos(doc.text().slice(..), doc.selection(view.id).cursor());
    let doc_last_line = doc.text().len_lines() - 1;

    let last_line = view.last_line(doc);

    if direction == Backward && view.first_line == 0
        || direction == Forward && last_line == doc_last_line
    {
        return;
    }

    let scrolloff = PADDING.min(view.area.height as usize / 2); // TODO: user pref

    // cursor visual offset
    // TODO: only if dragging via mouse?
    // let cursor_off = cursor.row - view.first_line;

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

pub fn page_up(cx: &mut Context) {
    let view = cx.view();
    let offset = view.area.height as usize;
    scroll(cx, offset, Direction::Backward);
}

pub fn page_down(cx: &mut Context) {
    let view = cx.view();
    let offset = view.area.height as usize;
    scroll(cx, offset, Direction::Forward);
}

pub fn half_page_up(cx: &mut Context) {
    let view = cx.view();
    let offset = view.area.height as usize / 2;
    scroll(cx, offset, Direction::Backward);
}

pub fn half_page_down(cx: &mut Context) {
    let view = cx.view();
    let offset = view.area.height as usize / 2;
    scroll(cx, offset, Direction::Forward);
}

pub fn extend_char_left(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Backward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_char_right(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(text, range, Direction::Forward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_line_up(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Backward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_line_down(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(text, range, Direction::Forward, count, Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_line_end(cx: &mut Context) {
    let (view, doc) = cx.current();

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        // Line end is pos at the start of next line - 1
        // subtract another 1 because the line ends with \n
        let pos = text.line_to_char(line + 1).saturating_sub(2);
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

pub fn extend_line_start(cx: &mut Context) {
    let (view, doc) = cx.current();

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

pub fn select_all(cx: &mut Context) {
    let (view, doc) = cx.current();

    let end = doc.text().len_chars().saturating_sub(1);
    doc.set_selection(view.id, Selection::single(0, end))
}

pub fn select_regex(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "select:".to_string(), move |view, doc, regex| {
        let text = doc.text().slice(..);
        if let Some(selection) = selection::select_on_matches(text, doc.selection(view.id), &regex)
        {
            doc.set_selection(view.id, selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

pub fn split_selection(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "split:".to_string(), move |view, doc, regex| {
        let text = doc.text().slice(..);
        let selection = selection::split_on_matches(text, doc.selection(view.id), &regex);
        doc.set_selection(view.id, selection);
    });

    cx.push_layer(Box::new(prompt));
}

pub fn split_selection_on_newline(cx: &mut Context) {
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
    let selection = selection::split_on_matches(text, doc.selection(view.id), &REGEX);
    doc.set_selection(view.id, selection);
}

// search: searches for the first occurence in file, provides a prompt
// search_next: reuses the last search regex and searches for the next match. The next match becomes the main selection.
// -> we always search from after the cursor.head
// TODO: be able to use selection as search query (*/alt *)
// I'd probably collect all the matches right now and store the current index. The cache needs
// wiping if input happens.

fn _search(doc: &mut Document, view: &mut View, contents: &str, regex: &Regex, extend: bool) {
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

        // TODO: (first_match, regex) stuff in register?
        doc.set_selection(view.id, selection);
        align_view(doc, view, Align::Center);
    };
}

// TODO: use one function for search vs extend
pub fn search(cx: &mut Context) {
    let (view, doc) = cx.current();

    // TODO: could probably share with select_on_matches?

    // HAXX: sadly we can't avoid allocating a single string for the whole buffer since we can't
    // feed chunks into the regex yet
    let contents = doc.text().slice(..).to_string();

    let view_id = view.id;
    let prompt = ui::regex_prompt(cx, "search:".to_string(), move |view, doc, regex| {
        let text = doc.text();
        let start = doc.selection(view.id).cursor();
        _search(doc, view, &contents, &regex, false);

        // TODO: only store on enter (accept), not update
        register::set('\\', vec![regex.as_str().to_string()]);
    });

    cx.push_layer(Box::new(prompt));
}
// can't search next for ""compose"" for some reason

pub fn _search_next(cx: &mut Context, extend: bool) {
    if let Some(query) = register::get('\\') {
        let query = query.first().unwrap();
        let (view, doc) = cx.current();
        let contents = doc.text().slice(..).to_string();
        let regex = Regex::new(query).unwrap();
        _search(doc, view, &contents, &regex, extend);
    }
}

pub fn search_next(cx: &mut Context) {
    _search_next(cx, false);
}

pub fn extend_search_next(cx: &mut Context) {
    _search_next(cx, true);
}

pub fn search_selection(cx: &mut Context) {
    let (view, doc) = cx.current();
    let contents = doc.text().slice(..);
    let query = doc.selection(view.id).primary().fragment(contents);
    let regex = regex::escape(&query);
    register::set('\\', vec![regex]);
    search_next(cx);
}

// TODO: N -> search_prev
// need to loop around buffer also and show a message
// same for no matches

//

pub fn select_line(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();

    let pos = doc.selection(view.id).primary();
    let text = doc.text();

    let line = text.char_to_line(pos.head);
    let start = text.line_to_char(line);
    let end = text
        .line_to_char(std::cmp::min(doc.text().len_lines(), line + count))
        .saturating_sub(1);

    doc.set_selection(view.id, Selection::single(start, end));
}
pub fn extend_line(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();

    let pos = doc.selection(view.id).primary();
    let text = doc.text();

    let line_start = text.char_to_line(pos.anchor);
    let mut line = text.char_to_line(pos.head);
    let line_end = text.line_to_char(line + 1).saturating_sub(1);
    if line_start <= pos.anchor && pos.head == line_end && line != text.len_lines() {
        line += 1;
    }

    let start = text.line_to_char(line_start);
    let end = text.line_to_char(line + 1).saturating_sub(1);

    doc.set_selection(view.id, Selection::single(start, end));
}

// heuristic: append changes to history after each command, unless we're in insert mode

fn _delete_selection(reg: char, doc: &mut Document, view_id: ViewId) {
    // first yank the selection
    let values: Vec<String> = doc
        .selection(view_id)
        .fragments(doc.text().slice(..))
        .map(Cow::into_owned)
        .collect();

    register::set(reg, values);

    // then delete
    let transaction =
        Transaction::change_by_selection(doc.text(), doc.selection(view_id), |range| {
            let max_to = doc.text().len_chars().saturating_sub(1);
            let to = std::cmp::min(max_to, range.to() + 1);
            (range.from(), to, None)
        });
    doc.apply(&transaction, view_id);
}

pub fn delete_selection(cx: &mut Context) {
    let reg = cx.register.name();
    let (view, doc) = cx.current();
    _delete_selection(reg, doc, view.id);

    doc.append_changes_to_history(view.id);

    // exit select mode, if currently in select mode
    exit_select_mode(cx);
}

pub fn change_selection(cx: &mut Context) {
    let reg = cx.register.name();
    let (view, doc) = cx.current();
    _delete_selection(reg, doc, view.id);
    enter_insert_mode(doc);
}

pub fn collapse_selection(cx: &mut Context) {
    let (view, doc) = cx.current();
    let selection = doc
        .selection(view.id)
        .transform(|range| Range::new(range.head, range.head));

    doc.set_selection(view.id, selection);
}

pub fn flip_selections(cx: &mut Context) {
    let (view, doc) = cx.current();
    let selection = doc
        .selection(view.id)
        .transform(|range| Range::new(range.head, range.anchor));

    doc.set_selection(view.id, selection);
}

fn enter_insert_mode(doc: &mut Document) {
    doc.mode = Mode::Insert;
}

// inserts at the start of each selection
pub fn insert_mode(cx: &mut Context) {
    let (view, doc) = cx.current();
    enter_insert_mode(doc);

    let selection = doc
        .selection(view.id)
        .transform(|range| Range::new(range.to(), range.from()));
    doc.set_selection(view.id, selection);
}

// inserts at the end of each selection
pub fn append_mode(cx: &mut Context) {
    let (view, doc) = cx.current();
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
            std::array::IntoIter::new([(end, end, Some(Tendril::from_char('\n')))]),
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
    pub struct Command {
        pub name: &'static str,
        pub alias: Option<&'static str>,
        pub doc: &'static str,
        // params, flags, helper, completer
        pub fun: fn(&mut Editor, &[&str], PromptEvent),
        pub completer: Option<Completer>,
    }

    fn quit(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        // last view and we have unsaved changes
        if editor.tree.views().count() == 1 && _buffers_remaining(editor) {
            return;
        }
        editor.close(editor.view().id, /* close_buffer */ false);
    }

    fn force_quit(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        editor.close(editor.view().id, /* close_buffer */ false);
    }

    fn open(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        match args.get(0) {
            Some(path) => {
                // TODO: handle error
                editor.open(path.into(), Action::Replace);
            }
            None => {
                editor.set_error("wrong argument count".to_string());
            }
        };
    }

    fn _write<P: AsRef<Path>>(
        view: &View,
        doc: &mut Document,
        path: Option<P>,
    ) -> Result<(), anyhow::Error> {
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
        tokio::spawn(doc.save());
        Ok(())
    }

    fn write(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        let (view, doc) = editor.current();
        if let Err(e) = _write(view, doc, args.first()) {
            editor.set_error(e.to_string());
        };
    }

    fn new_file(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        editor.new_file(Action::Replace);
    }

    fn format(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        let (view, doc) = editor.current();

        doc.format(view.id)
    }

    fn earlier(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        let uk = match args.join(" ").parse::<helix_core::history::UndoKind>() {
            Ok(uk) => uk,
            Err(msg) => {
                editor.set_error(msg);
                return;
            }
        };
        let (view, doc) = editor.current();
        doc.earlier(view.id, uk)
    }

    fn later(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        let uk = match args.join(" ").parse::<helix_core::history::UndoKind>() {
            Ok(uk) => uk,
            Err(msg) => {
                editor.set_error(msg);
                return;
            }
        };
        let (view, doc) = editor.current();
        doc.later(view.id, uk)
    }

    fn write_quit(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        let (view, doc) = editor.current();
        if let Err(e) = _write(view, doc, args.first()) {
            editor.set_error(e.to_string());
            return;
        };
        quit(editor, &[], event)
    }

    fn force_write_quit(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        write(editor, args, event);
        force_quit(editor, &[], event);
    }

    /// Returns `true` if there are modified buffers remaining and sets editor error,
    /// otherwise returns `false`
    fn _buffers_remaining(editor: &mut Editor) -> bool {
        let modified: Vec<_> = editor
            .documents()
            .filter(|doc| doc.is_modified())
            .map(|doc| {
                doc.relative_path()
                    .and_then(|path| path.to_str())
                    .unwrap_or("[scratch]")
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

    fn _write_all(editor: &mut Editor, args: &[&str], event: PromptEvent, quit: bool, force: bool) {
        let mut errors = String::new();

        // save all documents
        for (id, mut doc) in &mut editor.documents {
            if doc.path().is_none() {
                errors.push_str("cannot write a buffer without a filename\n");
                continue;
            }
            tokio::spawn(doc.save());
        }
        editor.set_error(errors);

        if quit {
            if !force && _buffers_remaining(editor) {
                return;
            }

            // close all views
            let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
            for view_id in views {
                editor.close(view_id, false);
            }
        }
    }

    fn write_all(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        _write_all(editor, args, event, false, false)
    }

    fn write_all_quit(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        _write_all(editor, args, event, true, false)
    }

    fn force_write_all_quit(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        _write_all(editor, args, event, true, true)
    }

    fn _quit_all(editor: &mut Editor, args: &[&str], event: PromptEvent, force: bool) {
        if !force && _buffers_remaining(editor) {
            return;
        }

        // close all views
        let views: Vec<_> = editor.tree.views().map(|(view, _)| view.id).collect();
        for view_id in views {
            editor.close(view_id, false);
        }
    }

    fn quit_all(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        _quit_all(editor, args, event, false)
    }

    fn force_quit_all(editor: &mut Editor, args: &[&str], event: PromptEvent) {
        _quit_all(editor, args, event, true)
    }

    pub const COMMAND_LIST: &[Command] = &[
        Command {
            name: "quit",
            alias: Some("q"),
            doc: "Close the current view.",
            fun: quit,
            completer: None,
        },
        Command {
            name: "quit!",
            alias: Some("q!"),
            doc: "Close the current view.",
            fun: force_quit,
            completer: None,
        },
        Command {
            name: "open",
            alias: Some("o"),
            doc: "Open a file from disk into the current view.",
            fun: open,
            completer: Some(completers::filename),
        },
        Command {
            name: "write",
            alias: Some("w"),
            doc: "Write changes to disk. Accepts an optional path (:write some/path.txt)",
            fun: write,
            completer: Some(completers::filename),
        },
        Command {
            name: "new",
            alias: Some("n"),
            doc: "Create a new scratch buffer.",
            fun: new_file,
            completer: Some(completers::filename),
        },
        Command {
            name: "format",
            alias: Some("fmt"),
            doc: "Format the file using a formatter.",
            fun: format,
            completer: None,
        },
        Command {
            name: "earlier",
            alias: Some("ear"),
            doc: "Jump back to an earlier point in edit history. Accepts a number of steps or a time span.",
            fun: earlier,
            completer: None,
        },
        Command {
            name: "later",
            alias: Some("lat"),
            doc: "Jump to a later point in edit history. Accepts a number of steps or a time span.",
            fun: later,
            completer: None,
        },
        Command {
            name: "write-quit",
            alias: Some("wq"),
            doc: "Writes changes to disk and closes the current view. Accepts an optional path (:wq some/path.txt)",
            fun: write_quit,
            completer: Some(completers::filename),
        },
        Command {
            name: "write-quit!",
            alias: Some("wq!"),
            doc: "Writes changes to disk and closes the current view forcefully. Accepts an optional path (:wq! some/path.txt)",
            fun: force_write_quit,
            completer: Some(completers::filename),
        },
        Command {
            name: "write-all",
            alias: Some("wa"),
            doc: "Writes changes from all views to disk.",
            fun: write_all,
            completer: None,
        },
        Command {
            name: "write-quit-all",
            alias: Some("wqa"),
            doc: "Writes changes from all views to disk and close all views.",
            fun: write_all_quit,
            completer: None,
        },
        Command {
            name: "write-quit-all!",
            alias: Some("wqa!"),
            doc: "Writes changes from all views to disk and close all views forcefully (ignoring unsaved changes).",
            fun: force_write_all_quit,
            completer: None,
        },
        Command {
            name: "quit-all",
            alias: Some("qa"),
            doc: "Close all views.",
            fun: quit_all,
            completer: None,
        },
        Command {
            name: "quit-all!",
            alias: Some("qa!"),
            doc: "Close all views forcefully (ignoring unsaved changes).",
            fun: force_quit_all,
            completer: None,
        },

    ];

    pub static COMMANDS: Lazy<HashMap<&'static str, &'static Command>> = Lazy::new(|| {
        let mut map = HashMap::new();

        for cmd in COMMAND_LIST {
            map.insert(cmd.name, cmd);
            if let Some(alias) = cmd.alias {
                map.insert(alias, cmd);
            }
        }

        map
    });
}

pub fn command_mode(cx: &mut Context) {
    // TODO: completion items should have a info section that would get displayed in
    // a popup above the prompt when items are tabbed over

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
                cmd::COMMAND_LIST
                    .iter()
                    .filter(|command| command.name.contains(input))
                    .map(|command| (end.clone(), Cow::Borrowed(command.name)))
                    .collect()
            } else {
                let part = parts.last().unwrap();

                if let Some(cmd::Command {
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
        move |editor: &mut Editor, input: &str, event: PromptEvent| {
            use helix_view::editor::Action;

            if event != PromptEvent::Validate {
                return;
            }

            let parts = input.split_whitespace().collect::<Vec<&str>>();
            if parts.is_empty() {
                return;
            }

            if let Some(cmd) = cmd::COMMANDS.get(parts[0]) {
                (cmd.fun)(editor, &parts[1..], event);
            } else {
                editor.set_error(format!("no such command: '{}'", parts[0]));
            };
        },
    );
    prompt.doc_fn = Box::new(|input: &str| {
        let part = input.split(' ').next().unwrap_or_default();

        if let Some(cmd::Command { doc, .. }) = cmd::COMMANDS.get(part) {
            return Some(doc);
        }

        None
    });

    cx.push_layer(Box::new(prompt));
}

pub fn file_picker(cx: &mut Context) {
    let root = find_root(None).unwrap_or_else(|| PathBuf::from("./"));
    let picker = ui::file_picker(root);
    cx.push_layer(Box::new(picker));
}

pub fn buffer_picker(cx: &mut Context) {
    use std::path::{Path, PathBuf};
    let current = cx.editor.view().doc;

    let picker = Picker::new(
        cx.editor
            .documents
            .iter()
            .map(|(id, doc)| (id, doc.relative_path().map(Path::to_path_buf)))
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

pub fn symbol_picker(cx: &mut Context) {
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
    let (view, doc) = cx.current();

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
                        let (_view, doc) = editor.current();
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
                        let (view, doc) = editor.current();

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
pub fn prepend_to_line(cx: &mut Context) {
    move_first_nonwhitespace(cx);
    let doc = cx.doc();
    enter_insert_mode(doc);
}

// A inserts at the end of each line with a selection
pub fn append_to_line(cx: &mut Context) {
    let (view, doc) = cx.current();
    enter_insert_mode(doc);

    let selection = doc.selection(view.id).transform(|range| {
        let text = doc.text();
        let line = text.char_to_line(range.head);
        // we can't use line_to_char(line + 1) - 2 because the last line might not contain \n
        let pos = (text.line_to_char(line) + text.line(line).len_chars()).saturating_sub(1);
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
    let (view, doc) = cx.current();
    enter_insert_mode(doc);

    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);

    let mut ranges = SmallVec::with_capacity(selection.len());

    let changes: Vec<Change> = selection
        .iter()
        .map(|range| {
            let line = text.char_to_line(range.head);

            let line = match open {
                // adjust position to the end of the line (next line - 1)
                Open::Below => line + 1,
                // adjust position to the end of the previous line (current line - 1)
                Open::Above => line,
            };

            let index = doc.text().line_to_char(line).saturating_sub(1);

            // TODO: share logic with insert_newline for indentation
            let indent_level = indent::suggested_indent_for_pos(
                doc.language_config(),
                doc.syntax(),
                text,
                index,
                true,
            );
            let indent = doc.indent_unit().repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push('\n');
            text.push_str(&indent);
            let text = text.repeat(count);

            // calculate new selection range
            let pos = index + text.chars().count();
            ranges.push(Range::new(pos, pos));

            (index, index, Some(text.into()))
        })
        .collect();

    // TODO: count actually inserts "n" new lines and starts editing on all of them.
    // TODO: append "count" newlines and modify cursors to those lines

    let selection = Selection::new(ranges, 0);

    let transaction =
        Transaction::change(doc.text(), changes.into_iter()).with_selection(selection);

    doc.apply(&transaction, view.id);
}

// o inserts a new line after each line with a selection
pub fn open_below(cx: &mut Context) {
    open(cx, Open::Below)
}

// O inserts a new line before each line with a selection
pub fn open_above(cx: &mut Context) {
    open(cx, Open::Above)
}

pub fn normal_mode(cx: &mut Context) {
    let (view, doc) = cx.current();

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
    let (view, doc) = editor.current();
    let jump = (doc.id(), doc.selection(view.id).clone());
    view.jumps.push(jump);
}

fn switch_to_last_accessed_file(cx: &mut Context) {
    let alternate_file = cx.view().last_accessed_doc;
    if let Some(alt) = alternate_file {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last accessed buffer".to_owned())
    }
}

pub fn goto_mode(cx: &mut Context) {
    if let Some(count) = cx._count {
        push_jump(cx.editor);

        let (view, doc) = cx.current();
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
            match (cx.doc().mode, ch) {
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
                    let (view, doc) = cx.current();

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

pub fn select_mode(cx: &mut Context) {
    cx.doc().mode = Mode::Select;
}

pub fn exit_select_mode(cx: &mut Context) {
    cx.doc().mode = Mode::Normal;
}

fn _goto(
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
        let (view, doc) = editor.current();
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

pub fn goto_definition(cx: &mut Context) {
    let (view, doc) = cx.current();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
    let future = language_server.goto_definition(doc.identifier(), pos);

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

            _goto(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_type_definition(cx: &mut Context) {
    let (view, doc) = cx.current();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
    let future = language_server.goto_type_definition(doc.identifier(), pos);

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

            _goto(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_implementation(cx: &mut Context) {
    let (view, doc) = cx.current();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
    let future = language_server.goto_implementation(doc.identifier(), pos);

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

            _goto(editor, compositor, items, offset_encoding);
        },
    );
}

pub fn goto_reference(cx: &mut Context) {
    let (view, doc) = cx.current();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
    let future = language_server.goto_reference(doc.identifier(), pos);

    cx.callback(
        future,
        move |editor: &mut Editor,
              compositor: &mut Compositor,
              items: Option<Vec<lsp::Location>>| {
            _goto(
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

    let (view, doc) = editor.current();

    doc.set_selection(view.id, Selection::point(pos));
    align_view(doc, view, Align::Center);
}

pub fn goto_first_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = editor.current();

    let cursor_pos = doc.selection(view.id).cursor();
    let diag = if let Some(diag) = doc.diagnostics().first() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

pub fn goto_last_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = editor.current();

    let cursor_pos = doc.selection(view.id).cursor();
    let diag = if let Some(diag) = doc.diagnostics().last() {
        diag.range.start
    } else {
        return;
    };

    goto_pos(editor, diag);
}

pub fn goto_next_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = editor.current();

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

pub fn goto_prev_diag(cx: &mut Context) {
    let editor = &mut cx.editor;
    let (view, doc) = editor.current();

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

pub fn signature_help(cx: &mut Context) {
    let (view, doc) = cx.current();

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
    let future = language_server.text_document_signature_help(doc.identifier(), pos);

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
        let doc = cx.doc();
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
        let doc = cx.doc();
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
        let (view, doc) = cx.current();

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
        let (view, doc) = cx.current();
        // TODO: round out to nearest indentation level (for example a line with 3 spaces should
        // indent by one to reach 4 spaces).

        let indent = Tendril::from(doc.indent_unit());
        let transaction = Transaction::insert(doc.text(), doc.selection(view.id), indent);
        doc.apply(&transaction, view.id);
    }

    pub fn insert_newline(cx: &mut Context) {
        let (view, doc) = cx.current();
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
            text.push('\n');
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
                text.push('\n');
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
        let (view, doc) = cx.current();
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
        let (view, doc) = cx.current();
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
        let (view, doc) = cx.current();
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

pub fn undo(cx: &mut Context) {
    let view_id = cx.view().id;
    cx.doc().undo(view_id);
}

pub fn redo(cx: &mut Context) {
    let view_id = cx.view().id;
    cx.doc().redo(view_id);
}

// Yank / Paste

pub fn yank(cx: &mut Context) {
    // TODO: should selections be made end inclusive?
    let (view, doc) = cx.current();
    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(doc.text().slice(..))
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "yanked {} selection(s) to register {}",
        values.len(),
        cx.register.name()
    );

    register::set(cx.register.name(), values);

    cx.editor.set_status(msg)
}

#[derive(Copy, Clone)]
enum Paste {
    Before,
    After,
}

fn _paste(reg: char, doc: &mut Document, view: &View, action: Paste) -> Option<Transaction> {
    if let Some(values) = register::get(reg) {
        let repeat = std::iter::repeat(
            values
                .last()
                .map(|value| Tendril::from_slice(value))
                .unwrap(),
        );

        // if any of values ends \n it's linewise paste
        let linewise = values.iter().any(|value| value.ends_with('\n'));

        let mut values = values.into_iter().map(Tendril::from).chain(repeat);

        let text = doc.text();

        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
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
        return Some(transaction);
    }
    None
}

pub fn replace_with_yanked(cx: &mut Context) {
    let reg = cx.register.name();

    if let Some(values) = register::get(reg) {
        let (view, doc) = cx.current();

        if let Some(yank) = values.first() {
            let transaction =
                Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                    let max_to = doc.text().len_chars().saturating_sub(1);
                    let to = std::cmp::min(max_to, range.to());
                    (range.from(), to, Some(yank.as_str().into()))
                });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view.id);
        }
    }
}

// alt-p => paste every yanked selection after selected text
// alt-P => paste every yanked selection before selected text
// R => replace selected text with yanked text
// alt-R => replace selected text with every yanked text
//
// append => insert at next line
// insert => insert at start of line
// replace => replace
// default insert

pub fn paste_after(cx: &mut Context) {
    let reg = cx.register.name();
    let (view, doc) = cx.current();

    if let Some(transaction) = _paste(reg, doc, view, Paste::After) {
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view.id);
    }
}

pub fn paste_before(cx: &mut Context) {
    let reg = cx.register.name();
    let (view, doc) = cx.current();

    if let Some(transaction) = _paste(reg, doc, view, Paste::Before) {
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

pub fn indent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
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

pub fn unindent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();
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

pub fn format_selections(cx: &mut Context) {
    let (view, doc) = cx.current();

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

pub fn join_selections(cx: &mut Context) {
    use movement::skip_while;
    let (view, doc) = cx.current();
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
            let mut start = text.line_to_char(line + 1).saturating_sub(1);
            let mut end = start + 1;
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

pub fn keep_selections(cx: &mut Context) {
    // keep selections matching regex
    let prompt = ui::regex_prompt(cx, "keep:".to_string(), move |view, doc, regex| {
        let text = doc.text().slice(..);

        if let Some(selection) = selection::keep_matches(text, doc.selection(view.id), &regex) {
            doc.set_selection(view.id, selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

pub fn keep_primary_selection(cx: &mut Context) {
    let (view, doc) = cx.current();

    let range = doc.selection(view.id).primary();
    let selection = Selection::single(range.anchor, range.head);
    doc.set_selection(view.id, selection);
}

//

pub fn save(cx: &mut Context) {
    // Spawns an async task to actually do the saving. This way we prevent blocking.

    // TODO: handle save errors somehow?
    tokio::spawn(cx.doc().save());
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

    let (view, doc) = cx.current();

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();

    let pos = pos_to_lsp_pos(doc.text(), doc.selection(view.id).cursor(), offset_encoding);

    // TODO: handle fails
    let future = language_server.completion(doc.identifier(), pos);

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
    //  TODO: Server error: content modified

    //    // TODO!: when iterating over items, show the docs in popup

    //    // language server client needs to be accessible via a registry of some sort
    //}
}

pub fn hover(cx: &mut Context) {
    let (view, doc) = cx.current();

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
    let future = language_server.text_document_hover(doc.identifier(), pos);

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

                let contents = ui::Markdown::new(contents);
                let mut popup = Popup::new(contents);
                compositor.push(Box::new(popup));
            }
        },
    );
}

// comments
pub fn toggle_comments(cx: &mut Context) {
    let (view, doc) = cx.current();
    let transaction = comment::toggle_line_comments(doc.text(), doc.selection(view.id));

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view.id);
}

// tree sitter node selection

pub fn expand_selection(cx: &mut Context) {
    let (view, doc) = cx.current();

    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let selection = object::expand_selection(syntax, text, doc.selection(view.id));
        doc.set_selection(view.id, selection);
    }
}

pub fn match_brackets(cx: &mut Context) {
    let (view, doc) = cx.current();

    if let Some(syntax) = doc.syntax() {
        let pos = doc.selection(view.id).cursor();
        if let Some(pos) = match_brackets::find(syntax, doc.text(), pos) {
            let selection = Selection::point(pos);
            doc.set_selection(view.id, selection);
        };
    }
}

//

pub fn jump_forward(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();

    if let Some((id, selection)) = view.jumps.forward(count) {
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = cx.current(); // refetch doc
        doc.set_selection(view.id, selection);

        align_view(doc, view, Align::Center);
    };
}

pub fn jump_backward(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = cx.current();

    if let Some((id, selection)) = view.jumps.backward(view.id, doc, count) {
        // manually set the alternate_file as we cannot use the Editor::switch function here.
        if view.doc != *id {
            view.last_accessed_doc = Some(view.doc)
        }
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = cx.current(); // refetch doc
        doc.set_selection(view.id, selection);

        align_view(doc, view, Align::Center);
    };
}

pub fn window_mode(cx: &mut Context) {
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

pub fn rotate_view(cx: &mut Context) {
    cx.editor.focus_next()
}

// split helper, clear it later
use helix_view::editor::Action;
fn split(cx: &mut Context, action: Action) {
    use helix_view::editor::Action;
    let (view, doc) = cx.current();
    let id = doc.id();
    let selection = doc.selection(view.id).clone();
    let first_line = view.first_line;

    cx.editor.switch(id, action);

    // match the selection in the previous view
    let (view, doc) = cx.current();
    view.first_line = first_line;
    doc.set_selection(view.id, selection);
}

pub fn hsplit(cx: &mut Context) {
    split(cx, Action::HorizontalSplit);
}

pub fn vsplit(cx: &mut Context) {
    split(cx, Action::VerticalSplit);
}

pub fn wclose(cx: &mut Context) {
    let view_id = cx.view().id;
    // close current split
    cx.editor.close(view_id, /* close_buffer */ false);
}

pub fn select_register(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            cx.editor.register.select(ch);
        }
    })
}

pub fn space_mode(cx: &mut Context) {
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
                // ' ' => toggle_alternate_buffer(cx),
                // TODO: temporary since space mode took its old key
                ' ' => keep_primary_selection(cx),
                _ => (),
            }
        }
    })
}

pub fn view_mode(cx: &mut Context) {
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
                    let (view, doc) = cx.current();

                    align_view(doc, view, match ch {
                        'z' | 'c' => Align::Center,
                        't' => Align::Top,
                        'b' => Align::Bottom,
                        _ => unreachable!()
                    });
                }
                'm' => {
                    let (view, doc) = cx.current();
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

pub fn left_bracket_mode(cx: &mut Context) {
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

pub fn right_bracket_mode(cx: &mut Context) {
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
