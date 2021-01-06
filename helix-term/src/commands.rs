use helix_core::{
    graphemes,
    indent::TAB_WIDTH,
    regex::Regex,
    register, selection,
    state::{Direction, Granularity, State},
    ChangeSet, Range, Selection, Tendril, Transaction,
};

use once_cell::sync::Lazy;

use crate::compositor::Compositor;
use crate::ui::{self, Prompt, PromptEvent};

use helix_view::{
    document::Mode,
    view::{View, PADDING},
    Editor,
};

pub struct Context<'a, 'b> {
    pub count: usize,
    pub view: &'a mut View,
    pub executor: &'a smol::Executor<'b>,
    pub language_servers: &'a helix_lsp::Registry,

    pub callback: Option<crate::compositor::Callback>,
}

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(cx: &mut Context);

pub fn move_char_left(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .move_selection(Direction::Backward, Granularity::Character, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn move_char_right(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .move_selection(Direction::Forward, Granularity::Character, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn move_line_up(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .move_selection(Direction::Backward, Granularity::Line, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn move_line_down(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .move_selection(Direction::Forward, Granularity::Line, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn move_line_end(cx: &mut Context) {
    let lines = selection_lines(&cx.view.doc.state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.

            // Line end is pos at the start of next line - 1
            // subtract another 1 because the line ends with \n
            cx.view.doc.text().line_to_char(index + 1).saturating_sub(2)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    cx.view.doc.set_selection(selection);
}

pub fn move_line_start(cx: &mut Context) {
    let lines = selection_lines(&cx.view.doc.state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the start of the line.
            cx.view.doc.text().line_to_char(index)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    cx.view.doc.set_selection(selection);
}

pub fn move_next_word_start(cx: &mut Context) {
    let pos = cx.view.doc.state.move_pos(
        cx.view.doc.selection().cursor(),
        Direction::Forward,
        Granularity::Word,
        cx.count,
    );

    cx.view.doc.set_selection(Selection::point(pos));
}

pub fn move_prev_word_start(cx: &mut Context) {
    let pos = cx.view.doc.state.move_pos(
        cx.view.doc.selection().cursor(),
        Direction::Backward,
        Granularity::Word,
        cx.count,
    );

    cx.view.doc.set_selection(Selection::point(pos));
}

pub fn move_next_word_end(cx: &mut Context) {
    let pos = State::move_next_word_end(
        &cx.view.doc.text().slice(..),
        cx.view.doc.selection().cursor(),
        cx.count,
    );

    cx.view.doc.set_selection(Selection::point(pos));
}

pub fn move_file_start(cx: &mut Context) {
    cx.view.doc.set_selection(Selection::point(0));

    cx.view.doc.mode = Mode::Normal;
}

pub fn move_file_end(cx: &mut Context) {
    let text = &cx.view.doc.text();
    let last_line = text.line_to_char(text.len_lines().saturating_sub(2));
    cx.view.doc.set_selection(Selection::point(last_line));

    cx.view.doc.mode = Mode::Normal;
}

pub fn check_cursor_in_view(view: &View) -> bool {
    let cursor = view.doc.selection().cursor();
    let line = view.doc.text().char_to_line(cursor);
    let document_end = view.first_line + view.size.1.saturating_sub(1) as usize;

    if (line > document_end.saturating_sub(PADDING)) | (line < view.first_line + PADDING) {
        return false;
    }
    true
}

pub fn page_up(cx: &mut Context) {
    if cx.view.first_line < PADDING {
        return;
    }

    cx.view.first_line = cx.view.first_line.saturating_sub(cx.view.size.1 as usize);

    if !check_cursor_in_view(cx.view) {
        let text = cx.view.doc.text();
        let pos = text.line_to_char(cx.view.last_line().saturating_sub(PADDING));
        cx.view.doc.set_selection(Selection::point(pos));
    }
}

pub fn page_down(cx: &mut Context) {
    cx.view.first_line += cx.view.size.1 as usize + PADDING;

    if cx.view.first_line < cx.view.doc.text().len_lines() {
        let text = cx.view.doc.text();
        let pos = text.line_to_char(cx.view.first_line as usize);
        cx.view.doc.set_selection(Selection::point(pos));
    }
}

pub fn half_page_up(cx: &mut Context) {
    if cx.view.first_line < PADDING {
        return;
    }

    cx.view.first_line = cx
        .view
        .first_line
        .saturating_sub(cx.view.size.1 as usize / 2);

    if !check_cursor_in_view(cx.view) {
        let text = &cx.view.doc.text();
        let pos = text.line_to_char(cx.view.last_line() - PADDING);
        cx.view.doc.set_selection(Selection::point(pos));
    }
}

pub fn half_page_down(cx: &mut Context) {
    let lines = cx.view.doc.text().len_lines();
    if cx.view.first_line < lines.saturating_sub(cx.view.size.1 as usize) {
        cx.view.first_line += cx.view.size.1 as usize / 2;
    }
    if !check_cursor_in_view(cx.view) {
        let text = cx.view.doc.text();
        let pos = text.line_to_char(cx.view.first_line as usize);
        cx.view.doc.set_selection(Selection::point(pos));
    }
}
// avoid select by default by having a visual mode switch that makes movements into selects

pub fn extend_char_left(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .extend_selection(Direction::Backward, Granularity::Character, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn extend_char_right(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .extend_selection(Direction::Forward, Granularity::Character, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn extend_line_up(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .extend_selection(Direction::Backward, Granularity::Line, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn extend_line_down(cx: &mut Context) {
    let selection =
        cx.view
            .doc
            .state
            .extend_selection(Direction::Forward, Granularity::Line, cx.count);
    cx.view.doc.set_selection(selection);
}

pub fn split_selection(cx: &mut Context) {
    // TODO: this needs to store initial selection state, revert on esc, confirm on enter
    // needs to also call the callback function per input change, not just final time.
    // could cheat and put it into completion_fn
    //
    // kakoune does it like this:
    // # save state to register
    // {
    //  # restore state from register
    //  # if event == abort, return early
    //  # add to history if enabled
    //  # update state
    // }

    let snapshot = cx.view.doc.state.clone();

    let prompt = Prompt::new(
        "split:".to_string(),
        |input: &str| Vec::new(), // this is fine because Vec::new() doesn't allocate
        move |editor: &mut Editor, input: &str, event: PromptEvent| {
            match event {
                PromptEvent::Abort => {
                    // revert state
                    let view = editor.view_mut().unwrap();
                    view.doc.state = snapshot.clone();
                }
                PromptEvent::Validate => {
                    //
                }
                PromptEvent::Update => {
                    match Regex::new(input) {
                        Ok(regex) => {
                            let view = editor.view_mut().unwrap();

                            // revert state to what it was before the last update
                            view.doc.state = snapshot.clone();

                            let text = &view.doc.text().slice(..);
                            let selection =
                                selection::split_on_matches(text, view.doc.selection(), &regex);
                            view.doc.set_selection(selection);
                        }
                        Err(_) => (), // TODO: mark command line as error
                    }
                }
            }
        },
    );

    cx.callback = Some(Box::new(
        move |compositor: &mut Compositor, editor: &mut Editor| {
            compositor.push(Box::new(prompt));
        },
    ));
}

pub fn split_selection_on_newline(cx: &mut Context) {
    let text = &cx.view.doc.text().slice(..);
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
    let selection = selection::split_on_matches(text, cx.view.doc.selection(), &REGEX);
    cx.view.doc.set_selection(selection);
}

pub fn select_line(cx: &mut Context) {
    // TODO: count
    let pos = cx.view.doc.selection().primary();
    let text = cx.view.doc.text();
    let line = text.char_to_line(pos.head);
    let start = text.line_to_char(line);
    let end = text.line_to_char(line + 1).saturating_sub(1);

    cx.view.doc.set_selection(Selection::single(start, end));
}

// heuristic: append changes to history after each command, unless we're in insert mode

fn _delete_selection(cx: &mut Context) {
    let transaction = Transaction::change_by_selection(&cx.view.doc.state, |range| {
        (range.from(), range.to() + 1, None)
    });
    cx.view.doc.apply(&transaction);
}

pub fn delete_selection(cx: &mut Context) {
    _delete_selection(cx);

    append_changes_to_history(cx);
}

pub fn change_selection(cx: &mut Context) {
    _delete_selection(cx);
    insert_mode(cx);
}

pub fn collapse_selection(cx: &mut Context) {
    let selection = cx
        .view
        .doc
        .selection()
        .transform(|range| Range::new(range.head, range.head));

    cx.view.doc.set_selection(selection);
}

pub fn flip_selections(cx: &mut Context) {
    let selection = cx
        .view
        .doc
        .selection()
        .transform(|range| Range::new(range.head, range.anchor));

    cx.view.doc.set_selection(selection);
}

fn enter_insert_mode(cx: &mut Context) {
    cx.view.doc.mode = Mode::Insert;

    // TODO: store selection for undo
}

// inserts at the start of each selection
pub fn insert_mode(cx: &mut Context) {
    enter_insert_mode(cx);

    let selection = cx
        .view
        .doc
        .selection()
        .transform(|range| Range::new(range.to(), range.from()));
    cx.view.doc.set_selection(selection);
}

// inserts at the end of each selection
pub fn append_mode(cx: &mut Context) {
    enter_insert_mode(cx);
    cx.view.doc.restore_cursor = true;

    // TODO: as transaction
    let text = &cx.view.doc.text().slice(..);
    let selection = cx.view.doc.selection().transform(|range| {
        // TODO: to() + next char
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(text, range.to()),
        )
    });
    cx.view.doc.set_selection(selection);
}

// TODO: I, A, o and O can share a lot of the primitives.
pub fn command_mode(cx: &mut Context) {
    cx.callback = Some(Box::new(
        |compositor: &mut Compositor, editor: &mut Editor| {
            let prompt = Prompt::new(
                ":".to_owned(),
                |_input: &str| {
                    // TODO: i need this duplicate list right now to avoid borrow checker issues
                    let command_list = vec![
                    String::from("q"),
                    String::from("aaa"),
                    String::from("bbb"),
                    String::from("ccc"),
                    String::from("ddd"),
                    String::from("eee"),
                    String::from("averylongcommandaverylongcommandaverylongcommandaverylongcommandaverylongcommand"),
                    String::from("q"),
                    String::from("aaa"),
                    String::from("bbb"),
                    String::from("ccc"),
                    String::from("ddd"),
                    String::from("eee"),
                    String::from("q"),
                    String::from("aaa"),
                    String::from("bbb"),
                    String::from("ccc"),
                    String::from("ddd"),
                    String::from("eee"),
                    String::from("q"),
                    String::from("aaa"),
                    String::from("bbb"),
                    String::from("ccc"),
                    String::from("ddd"),
                    String::from("eee"),
                    String::from("q"),
                    String::from("aaa"),
                    String::from("bbb"),
                    String::from("ccc"),
                    String::from("ddd"),
                    String::from("eee"),
                    ];
                    command_list
                        .into_iter()
                        .filter(|command| command.contains(_input))
                        .collect()
                }, // completion
                |editor: &mut Editor, input: &str, event: PromptEvent| {
                    if event != PromptEvent::Validate {
                        return;
                    }

                    let parts = input.split_ascii_whitespace().collect::<Vec<&str>>();

                    match parts.as_slice() {
                        &["q"] => editor.should_close = true,
                        &["o", path] => {
                            // TODO: make view()/view_mut() always contain a view.
                            let size = editor.view().unwrap().size;
                            editor.open(path.into(), size);
                        }
                        _ => (),
                    }
                },
            );
            compositor.push(Box::new(prompt));
        },
    ));
}
pub fn file_picker(cx: &mut Context) {
    cx.callback = Some(Box::new(
        |compositor: &mut Compositor, editor: &mut Editor| {
            let picker = ui::file_picker("./");
            compositor.push(Box::new(picker));
        },
    ));
}

pub fn buffer_picker(cx: &mut Context) {
    cx.callback = Some(Box::new(
        |compositor: &mut Compositor, editor: &mut Editor| {
            let picker = ui::buffer_picker(&editor.views, editor.focus);
            compositor.push(Box::new(picker));
        },
    ));
}

// calculate line numbers for each selection range
fn selection_lines(state: &State) -> Vec<usize> {
    let mut lines = state
        .selection
        .ranges()
        .iter()
        .map(|range| state.doc.char_to_line(range.head))
        .collect::<Vec<_>>();

    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();

    lines
}

// I inserts at the start of each line with a selection
pub fn prepend_to_line(cx: &mut Context) {
    enter_insert_mode(cx);

    move_line_start(cx);
}

// A inserts at the end of each line with a selection
pub fn append_to_line(cx: &mut Context) {
    enter_insert_mode(cx);

    move_line_end(cx);
}

// o inserts a new line after each line with a selection
pub fn open_below(cx: &mut Context) {
    enter_insert_mode(cx);

    let lines = selection_lines(&cx.view.doc.state);

    let positions: Vec<_> = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line/start of the next one.
            cx.view.doc.text().line_to_char(index + 1)
        })
        .collect();

    let changes = positions.iter().copied().map(|index| {
        // TODO: share logic with insert_newline for indentation
        let indent_level = helix_core::indent::suggested_indent_for_pos(
            cx.view.doc.syntax.as_ref(),
            &cx.view.doc.state,
            index,
        );
        let indent = " ".repeat(TAB_WIDTH).repeat(indent_level);
        let mut text = String::with_capacity(1 + indent.len());
        text.push_str(&indent);
        text.push('\n');

        // TODO: ideally we want to run a hook over the transactions to figure out and reindent all
        // \n's as a post-processing step?
        // behaviors:
        // - on insert mode enter: we add newline + indent and position cursor at the end
        // - on 3o/3O: we insert 3 newlines + indents each and position cursors at ends

        // generate changes
        (index, index, Some(text.into()))
    });

    // TODO: count actually inserts "n" new lines and starts editing on all of them.
    // TODO: append "count" newlines and modify cursors to those lines

    let selection = Selection::new(
        changes
            .clone()
            .map(|(start, end, text): (usize, usize, Option<Tendril>)| {
                let len = text.map(|text| text.len()).unwrap() - 1; // minus newline
                let pos = start + len;
                Range::new(pos, pos)
            })
            .collect(),
        0,
    );

    let transaction = Transaction::change(&cx.view.doc.state, changes).with_selection(selection);

    cx.view.doc.apply(&transaction);
}

// O inserts a new line before each line with a selection

fn append_changes_to_history(cx: &mut Context) {
    if cx.view.doc.changes.is_empty() {
        return;
    }

    // TODO: change -> change -> undo -> change -> change fails, probably old_state needs reset

    let new_changeset = ChangeSet::new(cx.view.doc.text());
    let changes = std::mem::replace(&mut cx.view.doc.changes, new_changeset);
    // Instead of doing this messy merge we could always commit, and based on transaction
    // annotations either add a new layer or compose into the previous one.
    let transaction = Transaction::from(changes).with_selection(cx.view.doc.selection().clone());

    // increment document version
    // TODO: needs to happen on undo/redo too
    cx.view.doc.version += 1;

    // TODO: trigger lsp/documentDidChange with changes

    // HAXX: we need to reconstruct the state as it was before the changes..
    let old_state = cx
        .view
        .doc
        .old_state
        .take()
        .expect("no old_state available");

    // TODO: take transaction by value?
    cx.view
        .doc
        .history
        .commit_revision(&transaction, &old_state);

    // TODO: notify LSP of changes
}

pub fn normal_mode(cx: &mut Context) {
    cx.view.doc.mode = Mode::Normal;

    append_changes_to_history(cx);

    // if leaving append mode, move cursor back by 1
    if cx.view.doc.restore_cursor {
        let text = &cx.view.doc.text().slice(..);
        let selection = cx.view.doc.selection().transform(|range| {
            Range::new(
                range.from(),
                graphemes::prev_grapheme_boundary(text, range.to()),
            )
        });
        cx.view.doc.set_selection(selection);

        cx.view.doc.restore_cursor = false;
    }
}

pub fn goto_mode(cx: &mut Context) {
    cx.view.doc.mode = Mode::Goto;
}

// NOTE: Transactions in this module get appended to history when we switch back to normal mode.
pub mod insert {
    use super::*;
    // TODO: insert means add text just before cursor, on exit we should be on the last letter.
    pub fn insert_char(cx: &mut Context, c: char) {
        let c = Tendril::from_char(c);
        let transaction = Transaction::insert(&cx.view.doc.state, c);

        cx.view.doc.apply(&transaction);
    }

    pub fn insert_tab(cx: &mut Context) {
        insert_char(cx, '\t');
    }

    pub fn insert_newline(cx: &mut Context) {
        let transaction = Transaction::change_by_selection(&cx.view.doc.state, |range| {
            let indent_level = helix_core::indent::suggested_indent_for_pos(
                cx.view.doc.syntax.as_ref(),
                &cx.view.doc.state,
                range.head,
            );
            let indent = " ".repeat(TAB_WIDTH).repeat(indent_level);
            let mut text = String::with_capacity(1 + indent.len());
            text.push('\n');
            text.push_str(&indent);
            (range.head, range.head, Some(text.into()))
        });
        cx.view.doc.apply(&transaction);
    }

    // TODO: handle indent-aware delete
    pub fn delete_char_backward(cx: &mut Context) {
        let text = &cx.view.doc.text().slice(..);
        let transaction = Transaction::change_by_selection(&cx.view.doc.state, |range| {
            (
                graphemes::nth_prev_grapheme_boundary(text, range.head, cx.count),
                range.head,
                None,
            )
        });
        cx.view.doc.apply(&transaction);
    }

    pub fn delete_char_forward(cx: &mut Context) {
        let text = &cx.view.doc.text().slice(..);
        let transaction = Transaction::change_by_selection(&cx.view.doc.state, |range| {
            (
                range.head,
                graphemes::nth_next_grapheme_boundary(text, range.head, cx.count),
                None,
            )
        });
        cx.view.doc.apply(&transaction);
    }
}

pub fn insert_char_prompt(prompt: &mut Prompt, c: char) {
    prompt.insert_char(c);
}

// Undo / Redo

// TODO: each command could simply return a Option<transaction>, then the higher level handles
// storing it?

pub fn undo(cx: &mut Context) {
    cx.view.doc.undo();
}

pub fn redo(cx: &mut Context) {
    cx.view.doc.redo();
}

// Yank / Paste

pub fn yank(cx: &mut Context) {
    // TODO: should selections be made end inclusive?
    let values = cx
        .view
        .doc
        .state
        .selection()
        .fragments(&cx.view.doc.text().slice(..))
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

        let transaction = if linewise {
            // paste on the next line
            // TODO: can simply take a range + modifier and compute the right pos without ifs
            let text = cx.view.doc.text();
            Transaction::change_by_selection(&cx.view.doc.state, |range| {
                let line_end = text.line_to_char(text.char_to_line(range.head) + 1);
                (line_end, line_end, Some(values.next().unwrap()))
            })
        } else {
            Transaction::change_by_selection(&cx.view.doc.state, |range| {
                (range.head + 1, range.head + 1, Some(values.next().unwrap()))
            })
        };

        cx.view.doc.apply(&transaction);
        append_changes_to_history(cx);
    }
}

fn get_lines(view: &View) -> Vec<usize> {
    let mut lines = Vec::new();

    // Get all line numbers
    for range in view.doc.selection().ranges() {
        let start = view.doc.text().char_to_line(range.from());
        let end = view.doc.text().char_to_line(range.to());

        for line in start..=end {
            lines.push(line)
        }
    }
    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();
    lines
}

pub fn indent(cx: &mut Context) {
    let lines = get_lines(cx.view);

    // Indent by one level
    let indent = Tendril::from(" ".repeat(TAB_WIDTH));

    let transaction = Transaction::change(
        &cx.view.doc.state,
        lines.into_iter().map(|line| {
            let pos = cx.view.doc.text().line_to_char(line);
            (pos, pos, Some(indent.clone()))
        }),
    );
    cx.view.doc.apply(&transaction);
    append_changes_to_history(cx);
}

pub fn unindent(cx: &mut Context) {
    let lines = get_lines(cx.view);
    let mut changes = Vec::with_capacity(lines.len());

    for line_idx in lines {
        let line = cx.view.doc.text().line(line_idx);
        let mut width = 0;

        for ch in line.chars() {
            match ch {
                ' ' => width += 1,
                '\t' => width = (width / TAB_WIDTH + 1) * TAB_WIDTH,
                _ => break,
            }

            if width >= TAB_WIDTH {
                break;
            }
        }

        if width > 0 {
            let start = cx.view.doc.text().line_to_char(line_idx);
            changes.push((start, start + width, None))
        }
    }

    let transaction = Transaction::change(&cx.view.doc.state, changes.into_iter());

    cx.view.doc.apply(&transaction);
    append_changes_to_history(cx);
}

pub fn indent_selection(_cx: &mut Context) {
    // loop over each line and recompute proper indentation
    unimplemented!()
}

//

pub fn save(cx: &mut Context) {
    // Spawns an async task to actually do the saving. This way we prevent blocking.

    // TODO: handle save errors somehow?
    cx.executor.spawn(cx.view.doc.save()).detach();
}

pub fn completion(cx: &mut Context) {
    let language_server = cx.language_servers.get("rust", &cx.executor).unwrap();
    use log::info;

    // TODO: blocking here is not ideal
    let pos = helix_lsp::util::pos_to_lsp_pos(
        &cx.view.doc.text().slice(..),
        cx.view.doc.selection().cursor(),
    );

    // TODO: handle fails
    let res = smol::block_on(language_server.completion(cx.view.doc.identifier(), pos))
        .unwrap_or_default();

    // TODO: if no completion, show some message or something
    if !res.is_empty() {
        let picker = ui::Picker::new(
            res,
            |item| {
                // format_fn
                item.label.as_str().into()

                // TODO: use item.filter_text for filtering
            },
            |editor: &mut Editor, item| {
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
                        unimplemented!("completion: additional_text_edits: {:?}", additional_edits);
                    }
                }

                let view = editor.view_mut().unwrap();
                let transaction =
                    util::generate_transaction_from_edits(&view.doc.state, vec![edit]);
                view.doc.apply(&transaction);
                // TODO: append_changes_to_history(cx); if not in insert mode?
            },
        );

        cx.callback = Some(Box::new(
            move |compositor: &mut Compositor, editor: &mut Editor| {
                compositor.push(Box::new(picker));
            },
        ));

        // TODO!: when iterating over items, show the docs in popup

        // language server client needs to be accessible via a registry of some sort
    }
}
