use helix_core::{
    comment, coords_at_pos, graphemes, match_brackets,
    movement::{self, Direction},
    object, pos_at_coords,
    regex::{self, Regex},
    register, search, selection, Change, ChangeSet, Position, Range, Rope, RopeSlice, Selection,
    Tendril, Transaction,
};

use once_cell::sync::Lazy;

use crate::{
    compositor::{Callback, Compositor},
    ui::{self, Picker, Popup, Prompt, PromptEvent},
};

use std::path::PathBuf;

use helix_view::{
    document::Mode,
    view::{View, PADDING},
    Document, DocumentId, Editor,
};

use crossterm::event::{KeyCode, KeyEvent};

use helix_lsp::lsp;

pub struct Context<'a> {
    pub count: usize,
    pub editor: &'a mut Editor,

    pub callback: Option<crate::compositor::Callback>,
    pub on_next_key_callback: Option<Box<dyn FnOnce(&mut Context, KeyEvent)>>,
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

    /// Push a new component onto the compositor.
    pub fn push_layer(&mut self, mut component: Box<dyn crate::compositor::Component>) {
        self.callback = Some(Box::new(
            |compositor: &mut Compositor, editor: &mut Editor| {
                let size = compositor.size();
                // trigger required_size on init
                component.required_size((size.width, size.height));
                compositor.push(component);
            },
        ));
    }

    #[inline]
    pub fn on_next_key(
        &mut self,
        on_next_key_callback: impl FnOnce(&mut Context, KeyEvent) + 'static,
    ) {
        self.on_next_key_callback = Some(Box::new(on_next_key_callback));
    }
}

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(cx: &mut Context);

pub fn move_char_left(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Backward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn move_char_right(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Forward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn move_line_up(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Backward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn move_line_down(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Forward,
            count,
            false, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn move_line_end(cx: &mut Context) {
    let doc = cx.doc();
    let lines = selection_lines(doc.text(), doc.selection());

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

    doc.set_selection(selection);
}

pub fn move_line_start(cx: &mut Context) {
    let doc = cx.doc();
    let lines = selection_lines(doc.text(), doc.selection());

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the start of the line.
            doc.text().line_to_char(index)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    doc.set_selection(selection);
}

// TODO: move vs extend could take an extra type Extend/Move that would
// Range::new(if Move { pos } if Extend { range.anchor }, pos)
// since these all really do the same thing

pub fn move_next_word_start(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);

    let selection = doc.selection().transform(|range| {
        let pos = movement::move_next_word_start(text, range.head, count);
        Range::new(pos, pos)
    });

    doc.set_selection(selection);
}

pub fn move_prev_word_start(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);

    let selection = doc.selection().transform(|range| {
        let pos = movement::move_prev_word_start(text, range.head, count);
        Range::new(pos, pos)
    });

    doc.set_selection(selection);
}

pub fn move_next_word_end(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);

    let selection = doc.selection().transform(|range| {
        let pos = movement::move_next_word_end(text, range.head, count);
        Range::new(pos, pos)
    });

    doc.set_selection(selection);
}

pub fn move_file_start(cx: &mut Context) {
    let doc = cx.doc();
    doc.set_selection(Selection::point(0));

    doc.mode = Mode::Normal;
}

pub fn move_file_end(cx: &mut Context) {
    let doc = cx.doc();
    let text = doc.text();
    let last_line = text.line_to_char(text.len_lines().saturating_sub(2));
    doc.set_selection(Selection::point(last_line));

    doc.mode = Mode::Normal;
}

pub fn extend_next_word_start(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);

    let selection = doc.selection().transform(|mut range| {
        let pos = movement::move_next_word_start(text, range.head, count);
        Range::new(range.anchor, pos)
    });

    doc.set_selection(selection);
}

pub fn extend_prev_word_start(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);

    let selection = doc.selection().transform(|mut range| {
        let pos = movement::move_prev_word_start(text, range.head, count);
        Range::new(range.anchor, pos)
    });
    doc.set_selection(selection);
}

pub fn extend_next_word_end(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);

    let selection = doc.selection().transform(|mut range| {
        let pos = movement::move_next_word_end(text, range.head, count);
        Range::new(range.anchor, pos)
    });

    doc.set_selection(selection);
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
            let doc = cx.doc();
            let text = doc.text().slice(..);

            let selection = doc.selection().transform(|mut range| {
                if let Some(pos) = search::find_nth_next(text, ch, range.head, count, inclusive) {
                    if extend {
                        Range::new(range.anchor, pos)
                    } else {
                        // select
                        Range::new(range.head, pos)
                    }
                    // or (pos, pos) to move to found val
                } else {
                    range
                }
            });

            doc.set_selection(selection);
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

            let doc = cx.doc();

            let transaction =
                Transaction::change_by_selection(doc.text(), doc.selection(), |range| {
                    (range.from(), range.to() + 1, Some(text.clone()))
                });

            doc.apply(&transaction);
            doc.append_changes_to_history();
        }
    })
}

fn scroll(cx: &mut Context, offset: usize, direction: Direction) {
    use Direction::*;
    let view = cx.editor.view();
    let doc = cx.editor.document(view.doc).unwrap();
    let cursor = coords_at_pos(doc.text().slice(..), doc.selection().cursor());
    let doc_last_line = doc.text().len_lines() - 1;

    let last_line = view.last_line(doc);

    if direction == Backward && view.first_line == 0
        || direction == Forward && last_line == doc_last_line
    {
        return;
    }

    let scrolloff = PADDING; // min(user pref, half win width/height)

    // cursor visual offset
    let cursor_off = cursor.row - view.first_line;

    // upgrade to mut reference
    let view = cx.editor.view_mut();

    view.first_line = match direction {
        Forward => view.first_line + offset,
        Backward => view.first_line.saturating_sub(offset),
    }
    .min(doc_last_line);

    // clamp into viewport
    let line = (view.first_line + cursor_off).clamp(
        view.first_line + scrolloff,
        last_line.saturating_sub(scrolloff),
    );

    // view drops here

    // upgrade to mut reference
    let doc = cx.doc();

    let text = doc.text().slice(..);
    let pos = pos_at_coords(text, Position::new(line, cursor.col)); // this func will properly truncate to line end

    doc.set_selection(Selection::point(pos));
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
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Backward,
            count,
            true, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn extend_char_right(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_horizontally(
            text,
            range,
            Direction::Forward,
            count,
            true, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn extend_line_up(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Backward,
            count,
            true, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn extend_line_down(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        movement::move_vertically(
            text,
            range,
            Direction::Forward,
            count,
            true, /* extend */
        )
    });
    doc.set_selection(selection);
}

pub fn select_all(cx: &mut Context) {
    let doc = cx.doc();

    let end = doc.text().len_chars().saturating_sub(1);
    doc.set_selection(Selection::single(0, end))
}

pub fn select_regex(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "select:".to_string(), |doc, regex| {
        let text = doc.text().slice(..);
        if let Some(selection) = selection::select_on_matches(text, doc.selection(), &regex) {
            doc.set_selection(selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

pub fn split_selection(cx: &mut Context) {
    let prompt = ui::regex_prompt(cx, "split:".to_string(), |doc, regex| {
        let text = doc.text().slice(..);
        let selection = selection::split_on_matches(text, doc.selection(), &regex);
        doc.set_selection(selection);
    });

    cx.push_layer(Box::new(prompt));
}

pub fn split_selection_on_newline(cx: &mut Context) {
    let doc = cx.doc();
    let text = doc.text().slice(..);
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
    let selection = selection::split_on_matches(text, doc.selection(), &REGEX);
    doc.set_selection(selection);
}

// search: searches for the first occurence in file, provides a prompt
// search_next: reuses the last search regex and searches for the next match. The next match becomes the main selection.
// -> we always search from after the cursor.head
// TODO: be able to use selection as search query (*/alt *)
// I'd probably collect all the matches right now and store the current index. The cache needs
// wiping if input happens.

fn _search(doc: &mut Document, contents: &str, regex: &Regex) {
    let text = doc.text();
    let start = doc.selection().cursor();

    // TODO: use find_at to find the next match after the cursor, loop around the end
    if let Some(mat) = regex.find_at(&contents, start) {
        let start = text.byte_to_char(mat.start());
        let end = text.byte_to_char(mat.end());
        let selection = Selection::single(start, end - 1);
        // TODO: (first_match, regex) stuff in register?
        doc.set_selection(selection);
    };
}

// TODO: use one function for search vs extend
pub fn search(cx: &mut Context) {
    let doc = cx.doc();

    // TODO: could probably share with select_on_matches?

    // HAXX: sadly we can't avoid allocating a single string for the whole buffer since we can't
    // feed chunks into the regex yet
    let contents = doc.text().slice(..).to_string();

    let prompt = ui::regex_prompt(cx, "search:".to_string(), move |doc, regex| {
        let text = doc.text();
        let start = doc.selection().cursor();
        _search(doc, &contents, &regex);

        // TODO: only store on enter (accept), not update
        register::set('\\', vec![regex.as_str().to_string()]);
    });

    cx.push_layer(Box::new(prompt));
}

pub fn search_next(cx: &mut Context) {
    if let Some(query) = register::get('\\') {
        let query = query.first().unwrap();
        let doc = cx.doc();
        let contents = doc.text().slice(..).to_string();
        let regex = Regex::new(&query).unwrap();
        _search(doc, &contents, &regex);
    }
}

pub fn search_selection(cx: &mut Context) {
    let doc = cx.doc();
    let contents = doc.text().slice(..);
    let query = doc.selection().primary().fragment(contents);
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
    let doc = cx.doc();

    let pos = doc.selection().primary();
    let text = doc.text();

    let line = text.char_to_line(pos.head);
    let start = text.line_to_char(line);
    let end = text.line_to_char(line + count).saturating_sub(1);

    doc.set_selection(Selection::single(start, end));
}
pub fn extend_line(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();

    let pos = doc.selection().primary();
    let text = doc.text();

    let line_start = text.char_to_line(pos.anchor);
    let mut line = text.char_to_line(pos.head);
    let line_end = text.line_to_char(line + 1).saturating_sub(1);
    if line_start <= pos.anchor && pos.head == line_end && line != text.len_lines() {
        line += 1;
    }

    let start = text.line_to_char(line_start);
    let end = text.line_to_char(line + 1).saturating_sub(1);

    doc.set_selection(Selection::single(start, end));
}

// heuristic: append changes to history after each command, unless we're in insert mode

fn _delete_selection(doc: &mut Document) {
    let transaction = Transaction::change_by_selection(doc.text(), doc.selection(), |range| {
        (range.from(), range.to() + 1, None)
    });
    doc.apply(&transaction);
}

pub fn delete_selection(cx: &mut Context) {
    let doc = cx.doc();
    _delete_selection(doc);

    doc.append_changes_to_history();
}

pub fn change_selection(cx: &mut Context) {
    let doc = cx.doc();
    _delete_selection(doc);
    insert_mode(cx);
}

pub fn collapse_selection(cx: &mut Context) {
    let doc = cx.doc();
    let selection = doc
        .selection()
        .transform(|range| Range::new(range.head, range.head));

    doc.set_selection(selection);
}

pub fn flip_selections(cx: &mut Context) {
    let doc = cx.doc();
    let selection = doc
        .selection()
        .transform(|range| Range::new(range.head, range.anchor));

    doc.set_selection(selection);
}

fn enter_insert_mode(doc: &mut Document) {
    doc.mode = Mode::Insert;
}

// inserts at the start of each selection
pub fn insert_mode(cx: &mut Context) {
    let doc = cx.doc();
    enter_insert_mode(doc);

    let selection = doc
        .selection()
        .transform(|range| Range::new(range.to(), range.from()));
    doc.set_selection(selection);
}

// inserts at the end of each selection
pub fn append_mode(cx: &mut Context) {
    let doc = cx.doc();
    enter_insert_mode(doc);
    doc.restore_cursor = true;

    let text = doc.text().slice(..);
    let selection = doc.selection().transform(|range| {
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(text, range.to()), // to() + next char
        )
    });
    doc.set_selection(selection);
}

const COMMAND_LIST: &[&str] = &["write", "open", "quit"];

// TODO: I, A, o and O can share a lot of the primitives.
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
                    let doc = &mut editor.document(id).unwrap();
                    smol::block_on(doc.save());
                }

                _ => (),
            }
        },
    );
    cx.push_layer(Box::new(prompt));
}
pub fn file_picker(cx: &mut Context) {
    let picker = ui::file_picker("./");
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
            match path {
                Some(path) => {
                    if *id == current {
                        format!("{} (*)", path.to_str().unwrap()).into()
                    } else {
                        path.to_str().unwrap().into()
                    }
                }
                None => "[NEW]".into(),
            }
        },
        |editor: &mut Editor, (_, path): &(DocumentId, Option<PathBuf>)| match path {
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
    let doc = cx.doc();
    enter_insert_mode(doc);

    move_line_start(cx);
}

// A inserts at the end of each line with a selection
pub fn append_to_line(cx: &mut Context) {
    let doc = cx.doc();
    enter_insert_mode(doc);

    move_line_end(cx);
}

// o inserts a new line after each line with a selection
pub fn open_below(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    enter_insert_mode(doc);

    let lines = selection_lines(doc.text(), doc.selection());

    let positions = lines.into_iter().map(|index| {
        // adjust all positions to the end of the line (next line minus one)
        doc.text().line_to_char(index + 1).saturating_sub(1)
    });

    let text = doc.text().slice(..);

    let changes: Vec<Change> = positions
        .map(|index| {
            // TODO: share logic with insert_newline for indentation
            let indent_level =
                helix_core::indent::suggested_indent_for_pos(doc.syntax(), text, index, true);
            let indent = doc.indent_unit().repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push('\n');
            text.push_str(&indent);
            let text = text.repeat(count);

            (index, index, Some(text.into()))
        })
        .collect();

    // TODO: count actually inserts "n" new lines and starts editing on all of them.
    // TODO: append "count" newlines and modify cursors to those lines

    let selection = Selection::new(
        changes
            .iter()
            .map(|(start, _end, text): &Change| {
                let len = text.as_ref().map(|text| text.len()).unwrap(); // minus newline
                let pos = start + len;
                Range::new(pos, pos)
            })
            .collect(),
        0,
    );

    let transaction =
        Transaction::change(doc.text(), changes.into_iter()).with_selection(selection);

    doc.apply(&transaction);
}

// O inserts a new line before each line with a selection
pub fn open_above(cx: &mut Context) {
    let count = cx.count;
    let doc = cx.doc();
    enter_insert_mode(doc);

    let lines = selection_lines(doc.text(), doc.selection());

    let positions = lines.into_iter().map(|index| {
        // adjust all positions to the end of the previous line
        doc.text().line_to_char(index).saturating_sub(1)
    });

    let text = doc.text().slice(..);

    let changes: Vec<Change> = positions
        .map(|index| {
            // TODO: share logic with insert_newline for indentation
            let indent_level =
                helix_core::indent::suggested_indent_for_pos(doc.syntax(), text, index, true);
            let indent = doc.indent_unit().repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push('\n');
            text.push_str(&indent);
            let text = text.repeat(count);

            // generate changes
            (index, index, Some(text.into()))
        })
        .collect();

    // TODO: count actually inserts "n" new lines and starts editing on all of them.
    // TODO: append "count" newlines and modify cursors to those lines

    let selection = Selection::new(
        changes
            .iter()
            .map(|(start, _end, text): &Change| {
                let len = text.as_ref().map(|text| text.len()).unwrap(); // minus newline
                let pos = start + len;
                Range::new(pos, pos)
            })
            .collect(),
        0,
    );

    let transaction =
        Transaction::change(doc.text(), changes.into_iter()).with_selection(selection);

    doc.apply(&transaction);
}

pub fn normal_mode(cx: &mut Context) {
    let doc = cx.doc();

    doc.mode = Mode::Normal;

    doc.append_changes_to_history();

    // if leaving append mode, move cursor back by 1
    if doc.restore_cursor {
        let text = doc.text().slice(..);
        let selection = doc.selection().transform(|range| {
            Range::new(
                range.from(),
                graphemes::prev_grapheme_boundary(text, range.to()),
            )
        });
        doc.set_selection(selection);

        doc.restore_cursor = false;
    }
}

pub fn goto_mode(cx: &mut Context) {
    cx.doc().mode = Mode::Goto;
}

pub fn select_mode(cx: &mut Context) {
    cx.doc().mode = Mode::Select;
}

pub fn exit_select_mode(cx: &mut Context) {
    cx.doc().mode = Mode::Normal;
}

fn goto(cx: &mut Context, locations: Vec<lsp::Location>) {
    use helix_view::editor::Action;
    cx.doc().mode = Mode::Normal;

    match locations.as_slice() {
        [location] => {
            cx.editor
                .open(PathBuf::from(location.uri.path()), Action::Replace);
            let doc = cx.doc();
            let definition_pos = location.range.start;
            let new_pos = helix_lsp::util::lsp_pos_to_pos(doc.text(), definition_pos);
            doc.set_selection(Selection::point(new_pos));
        }
        [] => (), // maybe show user message that no definition was found?
        _locations => {
            let mut picker = ui::Picker::new(
                locations,
                |item| {
                    let file = item.uri.as_str();
                    let line = item.range.start.line;
                    format!("{}:{}", file, line).into()
                },
                move |editor: &mut Editor, item| {
                    editor.open(PathBuf::from(item.uri.path()), Action::Replace);
                    // TODO: issues with doc already being broo
                    let id = editor.view().doc;
                    let doc = &mut editor.documents[id];
                    let definition_pos = item.range.start;
                    let new_pos = helix_lsp::util::lsp_pos_to_pos(doc.text(), definition_pos);
                    doc.set_selection(Selection::point(new_pos));
                },
            );
            cx.push_layer(Box::new(picker));
        }
    }
}

pub fn goto_definition(cx: &mut Context) {
    let doc = cx.doc();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: blocking here is not ideal
    let pos = helix_lsp::util::pos_to_lsp_pos(doc.text(), doc.selection().cursor());

    // TODO: handle fails
    let res =
        smol::block_on(language_server.goto_definition(doc.identifier(), pos)).unwrap_or_default();
    goto(cx, res);
}

pub fn goto_type_definition(cx: &mut Context) {
    let doc = cx.doc();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: blocking here is not ideal
    let pos = helix_lsp::util::pos_to_lsp_pos(doc.text(), doc.selection().cursor());

    // TODO: handle fails
    let res = smol::block_on(language_server.goto_type_definition(doc.identifier(), pos))
        .unwrap_or_default();
    goto(cx, res);
}

pub fn goto_implementation(cx: &mut Context) {
    let doc = cx.doc();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: blocking here is not ideal
    let pos = helix_lsp::util::pos_to_lsp_pos(doc.text(), doc.selection().cursor());

    // TODO: handle fails
    let res = smol::block_on(language_server.goto_implementation(doc.identifier(), pos))
        .unwrap_or_default();
    goto(cx, res);
}

pub fn goto_reference(cx: &mut Context) {
    let doc = cx.doc();
    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: blocking here is not ideal
    let pos = helix_lsp::util::pos_to_lsp_pos(doc.text(), doc.selection().cursor());

    // TODO: handle fails
    let res =
        smol::block_on(language_server.goto_reference(doc.identifier(), pos)).unwrap_or_default();
    goto(cx, res);
}

pub fn signature_help(cx: &mut Context) {
    let doc = cx.doc();

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: blocking here is not ideal
    let pos = helix_lsp::util::pos_to_lsp_pos(doc.text(), doc.selection().cursor());

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
        let doc = cx.doc();

        // run through insert hooks, stopping on the first one that returns Some(t)
        for hook in HOOKS {
            if let Some(transaction) = hook(doc.text(), doc.selection(), c) {
                doc.apply(&transaction);
                return;
            }
        }

        let t = Tendril::from_char(c);
        let transaction = Transaction::insert(doc.text(), doc.selection(), t);

        doc.apply(&transaction);

        // TODO: need a post insert hook too for certain triggers (autocomplete, signature help, etc)
        // this could also generically look at Transaction, but it's a bit annoying to look at
        // Operation instead of Change.
        for hook in POST_HOOKS {
            hook(cx, c);
        }
    }

    pub fn insert_tab(cx: &mut Context) {
        let doc = cx.doc();
        // TODO: round out to nearest indentation level (for example a line with 3 spaces should
        // indent by one to reach 4 spaces).

        let indent = Tendril::from(doc.indent_unit());
        let transaction = Transaction::insert(doc.text(), doc.selection(), indent);
        doc.apply(&transaction);
    }

    pub fn insert_newline(cx: &mut Context) {
        let doc = cx.doc();
        let text = doc.text().slice(..);
        let transaction = Transaction::change_by_selection(doc.text(), doc.selection(), |range| {
            // TODO: offset range.head by 1? when calculating?
            let indent_level =
                helix_core::indent::suggested_indent_for_pos(doc.syntax(), text, range.head, true);
            let indent = doc.indent_unit().repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push('\n');
            text.push_str(&indent);
            (range.head, range.head, Some(text.into()))
        });
        doc.apply(&transaction);
    }

    // TODO: handle indent-aware delete
    pub fn delete_char_backward(cx: &mut Context) {
        let count = cx.count;
        let doc = cx.doc();
        let text = doc.text().slice(..);
        let transaction = Transaction::change_by_selection(doc.text(), doc.selection(), |range| {
            (
                graphemes::nth_prev_grapheme_boundary(text, range.head, count),
                range.head,
                None,
            )
        });
        doc.apply(&transaction);
    }

    pub fn delete_char_forward(cx: &mut Context) {
        let count = cx.count;
        let doc = cx.doc();
        let text = doc.text().slice(..);
        let transaction = Transaction::change_by_selection(doc.text(), doc.selection(), |range| {
            (
                range.head,
                graphemes::nth_next_grapheme_boundary(text, range.head, count),
                None,
            )
        });
        doc.apply(&transaction);
    }
}

// Undo / Redo

// TODO: each command could simply return a Option<transaction>, then the higher level handles
// storing it?

pub fn undo(cx: &mut Context) {
    cx.doc().undo();
}

pub fn redo(cx: &mut Context) {
    cx.doc().redo();
}

// Yank / Paste

pub fn yank(cx: &mut Context) {
    // TODO: should selections be made end inclusive?
    let doc = cx.doc();
    let values = doc
        .selection()
        .fragments(doc.text().slice(..))
        .map(|cow| cow.into_owned())
        .collect();

    // TODO: allow specifying reg
    let reg = '"';
    register::set(reg, values);
}

pub fn paste(cx: &mut Context) {
    // TODO: allow specifying reg
    let reg = '"';
    if let Some(values) = register::get(reg) {
        let repeat = std::iter::repeat(
            values
                .last()
                .map(|value| Tendril::from_slice(value))
                .unwrap(),
        );

        // TODO: if any of values ends \n it's linewise paste
        //
        // p => paste after
        // P => paste before
        // alt-p => paste every yanked selection after selected text
        // alt-P => paste every yanked selection before selected text
        // R => replace selected text with yanked text
        // alt-R => replace selected text with every yanked text
        //
        // append => insert at next line
        // insert => insert at start of line
        // replace => replace
        // default insert

        let linewise = values.iter().any(|value| value.ends_with('\n'));

        let mut values = values.into_iter().map(Tendril::from).chain(repeat);

        let doc = cx.doc();

        let transaction = if linewise {
            // paste on the next line
            // TODO: can simply take a range + modifier and compute the right pos without ifs
            let text = doc.text();
            Transaction::change_by_selection(doc.text(), doc.selection(), |range| {
                let line_end = text.line_to_char(text.char_to_line(range.head) + 1);
                (line_end, line_end, Some(values.next().unwrap()))
            })
        } else {
            Transaction::change_by_selection(doc.text(), doc.selection(), |range| {
                (range.head + 1, range.head + 1, Some(values.next().unwrap()))
            })
        };

        doc.apply(&transaction);
        doc.append_changes_to_history();
    }
}

fn get_lines(doc: &Document) -> Vec<usize> {
    let mut lines = Vec::new();

    // Get all line numbers
    for range in doc.selection() {
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
    let doc = cx.doc();
    let lines = get_lines(doc);

    // Indent by one level
    let indent = Tendril::from(doc.indent_unit());

    let transaction = Transaction::change(
        doc.text(),
        lines.into_iter().map(|line| {
            let pos = doc.text().line_to_char(line);
            (pos, pos, Some(indent.clone()))
        }),
    );
    doc.apply(&transaction);
    doc.append_changes_to_history();
}

pub fn unindent(cx: &mut Context) {
    let doc = cx.doc();
    let lines = get_lines(doc);
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

    doc.apply(&transaction);
    doc.append_changes_to_history();
}

pub fn format_selections(cx: &mut Context) {
    use helix_lsp::lsp;
    let doc = cx.doc();

    // via lsp if available
    // else via tree-sitter indentation calculations

    // TODO: blocking here is not ideal

    let ranges: Vec<lsp::Range> = doc
        .selection()
        .iter()
        .map(|range| helix_lsp::util::range_to_lsp_range(doc.text(), *range))
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

        let transaction = helix_lsp::util::generate_transaction_from_edits(doc.text(), edits);

        doc.apply(&transaction);
    }

    doc.append_changes_to_history();
}

pub fn join_selections(cx: &mut Context) {
    use movement::skip_over_next;
    let doc = cx.doc();
    let text = doc.text();
    let slice = doc.text().slice(..);

    let mut changes = Vec::new();
    let fragment = Tendril::from(" ");

    for selection in doc.selection() {
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

    doc.apply(&transaction);
    doc.append_changes_to_history();
}

pub fn keep_selections(cx: &mut Context) {
    // keep selections matching regex
    let prompt = ui::regex_prompt(cx, "keep:".to_string(), |doc, regex| {
        let text = doc.text().slice(..);

        if let Some(selection) = selection::keep_matches(text, doc.selection(), &regex) {
            doc.set_selection(selection);
        }
    });

    cx.push_layer(Box::new(prompt));
}

pub fn keep_primary_selection(cx: &mut Context) {
    let doc = cx.doc();

    let range = doc.selection().primary();
    let selection = Selection::single(range.anchor, range.head);
    doc.set_selection(selection);
}

//

pub fn save(cx: &mut Context) {
    // Spawns an async task to actually do the saving. This way we prevent blocking.

    // TODO: handle save errors somehow?
    cx.editor.executor.spawn(cx.doc().save()).detach();
}

pub fn completion(cx: &mut Context) {
    let doc = cx.doc();

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: blocking here is not ideal
    let pos = helix_lsp::util::pos_to_lsp_pos(doc.text(), doc.selection().cursor());

    // TODO: handle fails

    let res = smol::block_on(language_server.completion(doc.identifier(), pos)).unwrap_or_default();

    // TODO: if no completion, show some message or something
    if !res.is_empty() {
        // let snapshot = doc.state.clone();
        let mut menu = ui::Menu::new(
            res,
            |item| {
                // format_fn
                item.label.as_str().into()

                // TODO: use item.filter_text for filtering
            },
            move |editor: &mut Editor, item, event| {
                match event {
                    PromptEvent::Abort => {
                        // revert state
                        // let id = editor.view().doc;
                        // let doc = &mut editor.documents[id];
                        // doc.state = snapshot.clone();
                    }
                    PromptEvent::Validate => {
                        let id = editor.view().doc;
                        let doc = &mut editor.documents[id];

                        // revert state to what it was before the last update
                        // doc.state = snapshot.clone();

                        // extract as fn(doc, item):

                        // TODO: need to apply without composing state...
                        // TODO: need to update lsp on accept/cancel by diffing the snapshot with
                        // the final state?
                        // -> on update simply update the snapshot, then on accept redo the call,
                        // finally updating doc.changes + notifying lsp.
                        //
                        // or we could simply use doc.undo + apply when changing between options

                        // always present here
                        let item = item.unwrap();

                        use helix_lsp::{lsp, util};
                        // determine what to insert: text_edit | insert_text | label
                        let edit = if let Some(edit) = &item.text_edit {
                            match edit {
                                lsp::CompletionTextEdit::Edit(edit) => edit.clone(),
                                lsp::CompletionTextEdit::InsertAndReplace(item) => {
                                    unimplemented!("completion: insert_and_replace {:?}", item)
                                }
                            }
                        } else {
                            item.insert_text.as_ref().unwrap_or(&item.label);
                            unimplemented!();
                            // lsp::TextEdit::new(); TODO: calculate a TextEdit from insert_text
                            // and we insert at position.
                        };

                        // TODO: merge edit with additional_text_edits
                        if let Some(additional_edits) = &item.additional_text_edits {
                            if !additional_edits.is_empty() {
                                unimplemented!(
                                    "completion: additional_text_edits: {:?}",
                                    additional_edits
                                );
                            }
                        }

                        // TODO: <-- if state has changed by further input, transaction will panic on len
                        let transaction =
                            util::generate_transaction_from_edits(doc.text(), vec![edit]);
                        doc.apply(&transaction);
                        // TODO: doc.append_changes_to_history(); if not in insert mode?
                    }
                    _ => (),
                };
            },
        );

        let popup = Popup::new(Box::new(menu));
        cx.push_layer(Box::new(popup));

        // TODO!: when iterating over items, show the docs in popup

        // language server client needs to be accessible via a registry of some sort
    }
}

pub fn hover(cx: &mut Context) {
    use helix_lsp::lsp;

    let doc = cx.doc();

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    // TODO: factor out a doc.position_identifier() that returns lsp::TextDocumentPositionIdentifier

    // TODO: blocking here is not ideal, make commands async fn?
    // not like we can process additional input meanwhile though
    let pos = helix_lsp::util::pos_to_lsp_pos(doc.text(), doc.selection().cursor());

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
        let mut popup = Popup::new(Box::new(contents));
        cx.push_layer(Box::new(popup));
    }
}

// view movements
pub fn next_view(cx: &mut Context) {
    cx.editor.focus_next()
}

// comments
pub fn toggle_comments(cx: &mut Context) {
    let doc = cx.doc();
    let transaction = comment::toggle_line_comments(doc.text(), doc.selection());

    doc.apply(&transaction);
    doc.append_changes_to_history();
}

// tree sitter node selection

pub fn expand_selection(cx: &mut Context) {
    let doc = cx.doc();

    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let selection = object::expand_selection(syntax, text, doc.selection());
        doc.set_selection(selection);
    }
}

pub fn match_brackets(cx: &mut Context) {
    let doc = cx.doc();

    if let Some(syntax) = doc.syntax() {
        let pos = doc.selection().cursor();
        if let Some(pos) = match_brackets::find(syntax, doc.text(), pos) {
            let selection = Selection::point(pos);
            doc.set_selection(selection);
        };
    }
}

//

pub fn jump_forward(cx: &mut Context) {
    let count = cx.count;
    let view = cx.view();

    if let Some((id, selection)) = view.jumps.forward(count) {
        view.first_line = 0;
        view.doc = *id;
    };
}

pub fn jump_backward(cx: &mut Context) {
    let count = cx.count;
    let view = cx.view();

    if let Some((id, selection)) = view.jumps.backward(count) {
        view.first_line = 0;
        view.doc = *id;
    };
}
