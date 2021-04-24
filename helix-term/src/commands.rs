use helix_core::{
    comment, coords_at_pos, graphemes, indent, match_brackets,
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
    util::{lsp_pos_to_pos, pos_to_lsp_pos, range_to_lsp_range},
    OffsetEncoding,
};

use crate::{
    compositor::{Callback, Component, Compositor},
    ui::{self, Completion, Picker, Popup, Prompt, PromptEvent},
};

use crate::application::{LspCallbackWrapper, LspCallbacks};
use futures_util::FutureExt;
use std::future::Future;

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use once_cell::sync::Lazy;

pub struct Context<'a> {
    pub count: usize,
    pub editor: &'a mut Editor,
    pub view_id: ViewId,

    pub callback: Option<crate::compositor::Callback>,
    pub on_next_key_callback: Option<Box<dyn FnOnce(&mut Context, KeyEvent)>>,
    pub callbacks: &'a mut LspCallbacks,
    pub status_msg: Option<String>,
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
        self.callback = Some(Box::new(
            |compositor: &mut Compositor, editor: &mut Editor| compositor.push(component),
        ));
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

    // TODO: allow &'static str?
    pub fn set_status(&mut self, msg: String) {
        self.status_msg = Some(msg);
    }
}

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(cx: &mut Context);

pub fn move_char_left(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Backward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(view.id, selection);
}

pub fn move_char_right(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Forward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(view.id, selection);
}

pub fn move_line_up(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Backward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(view.id, selection);
}

pub fn move_line_down(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Forward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(view.id, selection);
}

pub fn move_line_end(cx: &mut Context) {
    let (view, doc) = cx.current();
    let lines = selection_lines(doc.text(), doc.selection(view.id));

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.

            // Line end is pos at the start of next line - 1
            // subtract another 1 because the line ends with \n
            doc.text().line_to_char(index + 1).saturating_sub(2)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    doc.set_selection(view.id, selection);
}

pub fn move_line_start(cx: &mut Context) {
    let (view, doc) = cx.current();
    let lines = selection_lines(doc.text(), doc.selection(view.id));

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the start of the line.
            doc.text().line_to_char(index)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    doc.set_selection(view.id, selection);
}

// TODO: move vs extend could take an extra type Extend/Move that would
// Range::new(if Move { pos } if Extend { range.anchor }, pos)
// since these all really do the same thing

pub fn move_next_word_start(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|range| {
        movement::move_next_word_start(text, range.head, count).unwrap_or(range)
    });

    doc.set_selection(view.id, selection);
}

pub fn move_prev_word_start(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|range| {
        movement::move_prev_word_start(text, range.head, count).unwrap_or(range)
    });

    doc.set_selection(view.id, selection);
}

pub fn move_next_word_end(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .transform(|range| movement::move_next_word_end(text, range.head, count).unwrap_or(range));

    doc.set_selection(view.id, selection);
}

pub fn move_file_start(cx: &mut Context) {
    push_jump(cx);
    let (view, doc) = cx.current();
    doc.set_selection(view.id, Selection::point(0));
}

pub fn move_file_end(cx: &mut Context) {
    push_jump(cx);
    let (view, doc) = cx.current();
    let text = doc.text();
    let last_line = text.line_to_char(text.len_lines().saturating_sub(2));
    doc.set_selection(view.id, Selection::point(last_line));
}

pub fn extend_next_word_start(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_next_word_start(text, range.head, count).unwrap_or(range);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });

    doc.set_selection(view.id, selection);
}

pub fn extend_prev_word_start(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_prev_word_start(text, range.head, count).unwrap_or(range);
        let pos = word.head;
        Range::new(range.anchor, pos)
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_next_word_end(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).transform(|mut range| {
        let word = movement::move_next_word_end(text, range.head, count).unwrap_or(range);
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
    F: Fn(RopeSlice, char, usize, usize, bool) -> Option<usize>,
{
    // TODO: count is reset to 1 before next key so we move it into the closure here.
    // Would be nice to carry over.
    let count = cx.count;

    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            let (view, doc) = cx.current();
            let text = doc.text().slice(..);

            let selection = doc.selection(view.id).transform(|mut range| {
                search::find_nth_next(text, ch, range.head, count, inclusive).map_or(range, |pos| {
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
        }
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

pub fn replace(cx: &mut Context) {
    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            let text = Tendril::from_char(ch);

            let (view, doc) = cx.current();

            let transaction =
                Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                    (range.from(), range.to() + 1, Some(text.clone()))
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
    let line = cursor.row.clamp(
        view.first_line + scrolloff,
        last_line.saturating_sub(scrolloff),
    );

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
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Backward,
            count,
            true, /* extend */
        )
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_char_right(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Forward,
            count,
            true, /* extend */
        )
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_line_up(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Backward,
            count,
            true, /* extend */
        )
    });
    doc.set_selection(view.id, selection);
}

pub fn extend_line_down(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Forward,
            count,
            true, /* extend */
        )
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
    let start = selection.cursor();

    // use find_at to find the next match after the cursor, loop around the end
    let mat = regex
        .find_at(contents, start)
        .or_else(|| regex.find(contents));
    if let Some(mat) = mat {
        let start = text.byte_to_char(mat.start());
        let end = text.byte_to_char(mat.end());

        let head = end - 1;

        let selection = if extend {
            selection.clone().push(Range::new(start, head))
        } else {
            Selection::single(start, head)
        };

        // TODO: (first_match, regex) stuff in register?
        doc.set_selection(view.id, selection);
        // TODO: extract this centering into a function to share with _goto?
        let line = doc.text().char_to_line(head);
        view.first_line = line.saturating_sub(view.area.height as usize / 2);
    };
}

// TODO: use one function for search vs extend
pub fn search(cx: &mut Context) {
    let doc = cx.doc();

    // TODO: could probably share with select_on_matches?

    // HAXX: sadly we can't avoid allocating a single string for the whole buffer since we can't
    // feed chunks into the regex yet
    let contents = doc.text().slice(..).to_string();

    let view_id = cx.view_id;
    let prompt = ui::regex_prompt(cx, "search:".to_string(), move |view, doc, regex| {
        let text = doc.text();
        let start = doc.selection(view.id).cursor();
        _search(doc, view, &contents, &regex, false);

        // TODO: only store on enter (accept), not update
        register::set('\\', vec![regex.as_str().to_string()]);
    });

    cx.push_layer(Box::new(prompt));
}

pub fn _search_next(cx: &mut Context, extend: bool) {
    if let Some(query) = register::get('\\') {
        let query = query.first().unwrap();
        let (view, doc) = cx.current();
        let contents = doc.text().slice(..).to_string();
        let regex = Regex::new(&query).unwrap();
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
    let count = cx.count;
    let (view, doc) = cx.current();

    let pos = doc.selection(view.id).primary();
    let text = doc.text();

    let line = text.char_to_line(pos.head);
    let start = text.line_to_char(line);
    let end = text.line_to_char(line + count).saturating_sub(1);

    doc.set_selection(view.id, Selection::single(start, end));
}
pub fn extend_line(cx: &mut Context) {
    let count = cx.count;
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

fn _delete_selection(doc: &mut Document, view_id: ViewId) {
    // first yank the selection
    let values: Vec<String> = doc
        .selection(view_id)
        .fragments(doc.text().slice(..))
        .map(Cow::into_owned)
        .collect();

    // TODO: allow specifying reg
    let reg = '"';
    register::set(reg, values);

    // then delete
    let transaction =
        Transaction::change_by_selection(doc.text(), doc.selection(view_id), |range| {
            (range.from(), range.to() + 1, None)
        });
    doc.apply(&transaction, view_id);
}

pub fn delete_selection(cx: &mut Context) {
    let (view, doc) = cx.current();
    _delete_selection(doc, view.id);

    doc.append_changes_to_history(view.id);
}

pub fn change_selection(cx: &mut Context) {
    let (view, doc) = cx.current();
    _delete_selection(doc, view.id);
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
    doc.set_selection(view.id, selection);
}

const COMMAND_LIST: &[&str] = &["write", "open", "quit"];

pub fn command_mode(cx: &mut Context) {
    let prompt = Prompt::new(
        ":".to_owned(),
        |input: &str| {
            // we use .this over split_ascii_whitespace() because we care about empty segments
            let parts = input.split(' ').collect::<Vec<&str>>();

            // simple heuristic: if there's no space, complete command.
            // if there's a space, file completion kicks in. We should specialize by command later.
            if parts.len() <= 1 {
                use std::{borrow::Cow, ops::Range};
                let end = 0..;
                COMMAND_LIST
                    .iter()
                    .filter(|command| command.contains(input))
                    .map(|command| (end.clone(), Cow::Borrowed(*command)))
                    .collect()
            } else {
                let part = parts.last().unwrap();
                ui::completers::filename(part)
                    .into_iter()
                    .map(|(range, file)| {
                        // offset ranges to input
                        let offset = input.len() - part.len();
                        let range = (range.start + offset)..;
                        (range, file)
                    })
                    .collect()

                // TODO
                // additionally, completion items could have a info section that would get
                // displayed in a popup above the prompt when items are tabbed over
            }
        }, // completion
        move |editor: &mut Editor, input: &str, event: PromptEvent| {
            if event != PromptEvent::Validate {
                return;
            }

            let parts = input.split_ascii_whitespace().collect::<Vec<&str>>();

            match *parts.as_slice() {
                ["q"] | ["quit"] => {
                    editor.close(editor.view().id);
                    // editor.should_close = true,
                }
                ["o", path] | ["open", path] => {
                    use helix_view::editor::Action;
                    editor.open(path.into(), Action::Replace);
                }
                ["w"] | ["write"] => {
                    // TODO: non-blocking via save() command
                    let id = editor.view().doc;
                    let doc = &mut editor.documents[id];
                    smol::block_on(doc.save());
                }

                _ => (),
            }
        },
    );
    cx.push_layer(Box::new(prompt));
}

fn find_root(root: Option<&str>) -> Option<PathBuf> {
    let current_dir = std::env::current_dir().expect("unable to determine current directory");

    let root = match root {
        Some(root) => {
            let root = Path::new(root);
            if root.is_absolute() {
                root.to_path_buf()
            } else {
                current_dir.join(root)
            }
        }
        None => current_dir,
    };

    for ancestor in root.ancestors() {
        // TODO: also use defined roots if git isn't found
        if ancestor.join(".git").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
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
                None => "[NEW]".into(),
            }
        },
        |editor: &mut Editor, (_, path): &(DocumentId, Option<PathBuf>), _action| match path {
            Some(path) => {
                use helix_view::editor::Action;
                editor
                    .open(path.into(), Action::Replace)
                    .expect("editor.open failed");
            }
            None => (),
        },
    );
    cx.push_layer(Box::new(picker));
}

// calculate line numbers for each selection range
fn selection_lines(doc: &Rope, selection: &Selection) -> Vec<usize> {
    let mut lines = selection
        .iter()
        .map(|range| doc.char_to_line(range.head))
        .collect::<Vec<_>>();

    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();

    lines
}

// I inserts at the start of each line with a selection
pub fn prepend_to_line(cx: &mut Context) {
    move_line_start(cx);
    let doc = cx.doc();
    enter_insert_mode(doc);
}

// A inserts at the end of each line with a selection
pub fn append_to_line(cx: &mut Context) {
    move_line_end(cx);

    let (view, doc) = cx.current();
    enter_insert_mode(doc);

    // offset by another 1 char since move_line_end will position on the last char, we want to
    // append past that
    let selection = doc.selection(view.id).transform(|range| {
        let pos = range.head + 1;
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
}

enum Open {
    Below,
    Above,
}

fn open(cx: &mut Context, open: Open) {
    let count = cx.count;
    let (view, doc) = cx.current();
    enter_insert_mode(doc);

    let text = doc.text().slice(..);
    let lines = selection_lines(doc.text(), doc.selection(view.id));

    let mut ranges = SmallVec::with_capacity(lines.len());

    let changes: Vec<Change> = lines
        .into_iter()
        .map(|line| {
            let line = match open {
                // adjust position to the end of the line (next line - 1)
                Open::Below => line + 1,
                // adjust position to the end of the previous line (current line - 1)
                Open::Above => line,
            };

            let index = doc.text().line_to_char(line).saturating_sub(1);

            // TODO: share logic with insert_newline for indentation
            let indent_level = indent::suggested_indent_for_pos(doc.syntax(), text, index, true);
            let indent = doc.indent_unit().repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push('\n');
            text.push_str(&indent);
            let text = text.repeat(count);

            // calculate new selection range
            let pos = index + text.len();
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
fn push_jump(cx: &mut Context) {
    let (view, doc) = cx.current();
    let jump = { (doc.id(), doc.selection(view.id).clone()) };
    view.jumps.push(jump);
}

pub fn goto_mode(cx: &mut Context) {
    let count = cx.count;

    if count > 1 {
        push_jump(cx);

        // TODO: can't go to line 1 since we can't distinguish between g and 1g, g gets converted
        // to 1g
        let (view, doc) = cx.current();
        let pos = doc.text().line_to_char(count - 1);
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
            match ch {
                'g' => move_file_start(cx),
                'e' => move_file_end(cx),
                'd' => goto_definition(cx),
                't' => goto_type_definition(cx),
                'r' => goto_reference(cx),
                'i' => goto_implementation(cx),
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

fn _goto(cx: &mut Context, locations: Vec<lsp::Location>, offset_encoding: OffsetEncoding) {
    use helix_view::editor::Action;

    push_jump(cx);

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
        let new_pos = lsp_pos_to_pos(doc.text(), definition_pos, offset_encoding);
        doc.set_selection(view.id, Selection::point(new_pos));
        let line = doc.text().char_to_line(new_pos);
        view.first_line = line.saturating_sub(view.area.height as usize / 2);
    }

    match locations.as_slice() {
        [location] => {
            jump_to(cx.editor, location, offset_encoding, Action::Replace);
        }
        [] => (), // maybe show user message that no definition was found?
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
            cx.push_layer(Box::new(picker));
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
    let res =
        smol::block_on(language_server.goto_definition(doc.identifier(), pos)).unwrap_or_default();
    _goto(cx, res, offset_encoding);
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
    let res = smol::block_on(language_server.goto_type_definition(doc.identifier(), pos))
        .unwrap_or_default();
    _goto(cx, res, offset_encoding);
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
    let res = smol::block_on(language_server.goto_implementation(doc.identifier(), pos))
        .unwrap_or_default();
    _goto(cx, res, offset_encoding);
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
    let res =
        smol::block_on(language_server.goto_reference(doc.identifier(), pos)).unwrap_or_default();
    _goto(cx, res, offset_encoding);
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

    let res = smol::block_on(language_server.text_document_signature_help(doc.identifier(), pos))
        .unwrap_or_default();

    if let Some(signature_help) = res {
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
}

// NOTE: Transactions in this module get appended to history when we switch back to normal mode.
pub mod insert {
    use super::*;
    pub type Hook = fn(&Rope, &Selection, char) -> Option<Transaction>;
    pub type PostHook = fn(&mut Context, char);

    use helix_core::auto_pairs;
    const HOOKS: &[Hook] = &[auto_pairs::hook];

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

    // TODO: the pre-hook handles ( so post hook never gets called
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
    }

    const POST_HOOKS: &[PostHook] = &[completion, signature_help];

    // TODO: insert means add text just before cursor, on exit we should be on the last letter.
    pub fn insert_char(cx: &mut Context, c: char) {
        let (view, doc) = cx.current();

        // run through insert hooks, stopping on the first one that returns Some(t)
        for hook in HOOKS {
            if let Some(transaction) = hook(doc.text(), doc.selection(view.id), c) {
                doc.apply(&transaction, view.id);
                return;
            }
        }

        let t = Tendril::from_char(c);
        let transaction = Transaction::insert(doc.text(), doc.selection(view.id), t);

        doc.apply(&transaction, view.id);

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
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.head;
                // TODO: offset range.head by 1? when calculating?
                let indent_level = indent::suggested_indent_for_pos(doc.syntax(), text, pos, true);
                let indent = doc.indent_unit().repeat(indent_level);
                let mut text = String::with_capacity(1 + indent.len());
                text.push('\n');
                text.push_str(&indent);
                (pos, pos, Some(text.into()))
            });
        doc.apply(&transaction, view.id);
    }

    // TODO: handle indent-aware delete
    pub fn delete_char_backward(cx: &mut Context) {
        let count = cx.count;
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
        let count = cx.count;
        let doc = cx.doc();
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
}

// Undo / Redo

// TODO: each command could simply return a Option<transaction>, then the higher level handles
// storing it?

pub fn undo(cx: &mut Context) {
    let view_id = cx.view_id;
    cx.doc().undo(view_id);
}

pub fn redo(cx: &mut Context) {
    let view_id = cx.view_id;
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

    // TODO: allow specifying reg
    let reg = '"';
    let msg = format!("yanked {} selection(s) to register {}", values.len(), reg);

    register::set(reg, values);

    cx.set_status(msg)
}

#[derive(Copy, Clone)]
enum Paste {
    Before,
    After,
}

fn _paste(doc: &mut Document, view: &View, action: Paste) -> Option<Transaction> {
    // TODO: allow specifying reg
    let reg = '"';
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
    let (view, doc) = cx.current();

    if let Some(transaction) = _paste(doc, view, Paste::After) {
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view.id);
    }
}

pub fn paste_before(cx: &mut Context) {
    let (view, doc) = cx.current();

    if let Some(transaction) = _paste(doc, view, Paste::Before) {
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
    let (view, doc) = cx.current();
    let lines = get_lines(doc, view.id);

    // Indent by one level
    let indent = Tendril::from(doc.indent_unit());

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
    let (view, doc) = cx.current();
    let lines = get_lines(doc, view.id);
    let mut changes = Vec::with_capacity(lines.len());
    let tab_width = doc.tab_width();

    for line_idx in lines {
        let line = doc.text().line(line_idx);
        let mut width = 0;

        for ch in line.chars() {
            match ch {
                ' ' => width += 1,
                '\t' => width = (width / tab_width + 1) * tab_width,
                _ => break,
            }

            if width >= tab_width {
                break;
            }
        }

        if width > 0 {
            let start = doc.text().line_to_char(line_idx);
            changes.push((start, start + width, None))
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
        let edits = smol::block_on(language_server.text_document_range_formatting(
            doc.identifier(),
            range,
            lsp::FormattingOptions::default(),
        ))
        .unwrap_or_default();

        let transaction = helix_lsp::util::generate_transaction_from_edits(
            doc.text(),
            edits,
            language_server.offset_encoding(),
        );

        doc.apply(&transaction, view.id);
    }

    doc.append_changes_to_history(view.id);
}

pub fn join_selections(cx: &mut Context) {
    use movement::skip_over_next;
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
            skip_over_next(slice, &mut end, |ch| matches!(ch, ' ' | '\t'));

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
    cx.editor.executor.spawn(cx.doc().save()).detach();
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

    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id).cursor(),
        language_server.offset_encoding(),
    );

    // TODO: handle fails
    let res = smol::block_on(language_server.completion(doc.identifier(), pos)).unwrap();

    let trigger_offset = doc.selection(view.id).cursor();

    cx.callback(
        res,
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
            if !items.is_empty() {
                use crate::compositor::AnyComponent;
                let size = compositor.size();
                let ui = compositor.find("hx::ui::editor::EditorView").unwrap();
                if let Some(ui) = ui.as_any_mut().downcast_mut::<ui::EditorView>() {
                    ui.set_completion(items, offset_encoding, trigger_offset, size);
                };
            }
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

    // TODO: blocking here is not ideal, make commands async fn?
    // not like we can process additional input meanwhile though
    let pos = pos_to_lsp_pos(
        doc.text(),
        doc.selection(view.id).cursor(),
        language_server.offset_encoding(),
    );

    // TODO: handle fails
    let res = smol::block_on(language_server.text_document_hover(doc.identifier(), pos))
        .unwrap_or_default();

    if let Some(hover) = res {
        // hover.contents / .range <- used for visualizing
        let contents = match hover.contents {
            lsp::HoverContents::Scalar(contents) => {
                // markedstring(string/languagestring to be highlighted)
                // TODO
                unimplemented!("{:?}", contents)
            }
            lsp::HoverContents::Array(contents) => {
                unimplemented!("{:?}", contents)
            }
            // TODO: render markdown
            lsp::HoverContents::Markup(contents) => contents.value,
        };

        // skip if contents empty

        let contents = ui::Markdown::new(contents);
        let mut popup = Popup::new(contents);
        cx.push_layer(Box::new(popup));
    }
}

// view movements
pub fn next_view(cx: &mut Context) {
    cx.editor.focus_next()
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
    let count = cx.count;
    let (view, doc) = cx.current();

    if let Some((id, selection)) = view.jumps.forward(count) {
        view.doc = *id;
        let selection = selection.clone();
        let cursor = selection.cursor();
        doc.set_selection(view.id, selection);
        // TODO: extract this centering into a function to share with _goto?
        let line = doc.text().char_to_line(cursor);
        view.first_line = line.saturating_sub(view.area.height as usize / 2);
    };
}

pub fn jump_backward(cx: &mut Context) {
    let count = cx.count;
    let (view, doc) = cx.current();

    if let Some((id, selection)) = view.jumps.backward(count) {
        view.doc = *id;
        let selection = selection.clone();
        let cursor = selection.cursor();
        doc.set_selection(view.id, selection);
        // TODO: extract this centering into a function to share with _goto?
        let line = doc.text().char_to_line(cursor);
        view.first_line = line.saturating_sub(view.area.height as usize / 2);
    };
}

//

pub fn vsplit(cx: &mut Context) {
    // TODO: use doc.id directly, this can only split saved files
    let path = cx.doc().path().cloned();

    if let Some(path) = path {
        // open the same file again. this will vsplit
        cx.editor
            .open(path, helix_view::editor::Action::VerticalSplit);
    }
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
                'v' => vsplit(cx),
                'w' => {
                    // save current buffer
                    let doc = cx.doc();
                    smol::block_on(doc.save());
                }
                'c' => {
                    // close current split
                    cx.editor.close(cx.view_id);
                }
                // ' ' => toggle_alternate_buffer(cx),
                // TODO: temporary since space mode took it's old key
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
                    let pos = doc.selection(view.id).cursor();
                    // TODO: extract this centering into a function to share with _goto?
                    let line = doc.text().char_to_line(pos);

                    let relative = match ch {
                        'z' | 'c' => view.area.height as usize / 2,
                        't' => 0,
                        'b' => view.area.height as usize,
                        _ => unreachable!()
                    };
                    view.first_line = line.saturating_sub(relative);
                }
                'm' => {
                    let (view, doc) = cx.current();
                    let pos = doc.selection(view.id).cursor();
                    let pos = coords_at_pos(doc.text().slice(..), pos);

                    const OFFSET: usize = 7; // gutters
                    view.first_col = pos.col.saturating_sub((view.area.width as usize - OFFSET) / 2);
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
