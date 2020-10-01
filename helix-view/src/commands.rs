use helix_core::{
    graphemes,
    regex::Regex,
    selection,
    state::{Direction, Granularity, Mode, State},
    Range, Selection, Tendril, Transaction,
};
use once_cell::sync::Lazy;

use crate::view::View;

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(view: &mut View, count: usize);

pub fn move_char_left(view: &mut View, count: usize) {
    // TODO: use a transaction
    let selection = view
        .state
        .move_selection(Direction::Backward, Granularity::Character, count);
    view.state.selection = selection;
}

pub fn move_char_right(view: &mut View, count: usize) {
    // TODO: use a transaction
    view.state.selection =
        view.state
            .move_selection(Direction::Forward, Granularity::Character, count);
}

pub fn move_line_up(view: &mut View, count: usize) {
    // TODO: use a transaction
    view.state.selection = view
        .state
        .move_selection(Direction::Backward, Granularity::Line, count);
}

pub fn move_line_down(view: &mut View, count: usize) {
    // TODO: use a transaction
    view.state.selection = view
        .state
        .move_selection(Direction::Forward, Granularity::Line, count);
}

pub fn move_line_end(view: &mut View, _count: usize) {
    // TODO: use a transaction
    let lines = selection_lines(&view.state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.

            // Line end is pos at the start of next line - 1
            // subtract another 1 because the line ends with \n
            view.state.doc.line_to_char(index + 1).saturating_sub(2)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    let transaction = Transaction::new(&mut view.state).with_selection(selection);

    transaction.apply(&mut view.state);
}

pub fn move_line_start(view: &mut View, _count: usize) {
    let lines = selection_lines(&view.state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the start of the line.
            view.state.doc.line_to_char(index)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    let transaction = Transaction::new(&mut view.state).with_selection(selection);

    transaction.apply(&mut view.state);
}

pub fn move_next_word_start(view: &mut View, count: usize) {
    let pos = view.state.move_pos(
        view.state.selection.cursor(),
        Direction::Forward,
        Granularity::Word,
        count,
    );

    // TODO: use a transaction
    view.state.selection = Selection::single(pos, pos);
}

pub fn move_prev_word_start(view: &mut View, count: usize) {
    let pos = view.state.move_pos(
        view.state.selection.cursor(),
        Direction::Backward,
        Granularity::Word,
        count,
    );

    // TODO: use a transaction
    view.state.selection = Selection::single(pos, pos);
}

pub fn move_next_word_end(view: &mut View, count: usize) {
    let pos = State::move_next_word_end(
        &view.state.doc().slice(..),
        view.state.selection.cursor(),
        count,
    );

    // TODO: use a transaction
    view.state.selection = Selection::single(pos, pos);
}

// avoid select by default by having a visual mode switch that makes movements into selects

pub fn extend_char_left(view: &mut View, count: usize) {
    // TODO: use a transaction
    let selection = view
        .state
        .extend_selection(Direction::Backward, Granularity::Character, count);
    view.state.selection = selection;
}

pub fn extend_char_right(view: &mut View, count: usize) {
    // TODO: use a transaction
    view.state.selection =
        view.state
            .extend_selection(Direction::Forward, Granularity::Character, count);
}

pub fn extend_line_up(view: &mut View, count: usize) {
    // TODO: use a transaction
    view.state.selection =
        view.state
            .extend_selection(Direction::Backward, Granularity::Line, count);
}

pub fn extend_line_down(view: &mut View, count: usize) {
    // TODO: use a transaction
    view.state.selection =
        view.state
            .extend_selection(Direction::Forward, Granularity::Line, count);
}

pub fn split_selection_on_newline(view: &mut View, _count: usize) {
    let text = &view.state.doc.slice(..);
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n").unwrap());
    // TODO: use a transaction
    view.state.selection = selection::split_on_matches(text, view.state.selection(), &REGEX)
}

pub fn delete_selection(view: &mut View, _count: usize) {
    let transaction =
        Transaction::change_by_selection(&view.state, |range| (range.from(), range.to(), None));
    transaction.apply(&mut view.state);
}

pub fn change_selection(view: &mut View, count: usize) {
    delete_selection(view, count);
    insert_mode(view, count);
}

pub fn collapse_selection(view: &mut View, _count: usize) {
    view.state.selection = view
        .state
        .selection
        .transform(|range| Range::new(range.head, range.head))
}

// insert mode:
// first we calculate the correct cursors/selections
// then we just append at each cursor
// lastly, if it was append mode we shift cursor by 1?

// inserts at the start of each selection
pub fn insert_mode(view: &mut View, _count: usize) {
    view.state.mode = Mode::Insert;

    view.state.selection = view
        .state
        .selection
        .transform(|range| Range::new(range.to(), range.from()))
}

// inserts at the end of each selection
pub fn append_mode(view: &mut View, _count: usize) {
    view.state.mode = Mode::Insert;
    view.state.restore_cursor = true;

    // TODO: as transaction
    let text = &view.state.doc.slice(..);
    view.state.selection = view.state.selection.transform(|range| {
        // TODO: to() + next char
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(text, range.to()),
        )
    })
}

// TODO: I, A, o and O can share a lot of the primitives.

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
pub fn prepend_to_line(view: &mut View, count: usize) {
    view.state.mode = Mode::Insert;

    move_line_start(view, count);
}

// A inserts at the end of each line with a selection
pub fn append_to_line(view: &mut View, count: usize) {
    view.state.mode = Mode::Insert;

    move_line_end(view, count);
}

// o inserts a new line after each line with a selection
pub fn open_below(view: &mut View, _count: usize) {
    view.state.mode = Mode::Insert;

    let lines = selection_lines(&view.state);

    let positions: Vec<_> = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line/start of the next one.
            view.state.doc.line_to_char(index + 1)
        })
        .collect();

    let changes = positions.iter().copied().map(|index|
        // generate changes
        (index, index, Some(Tendril::from_char('\n'))));

    // TODO: count actually inserts "n" new lines and starts editing on all of them.
    // TODO: append "count" newlines and modify cursors to those lines

    let selection = Selection::new(
        positions
            .iter()
            .copied()
            .map(|pos| Range::new(pos, pos))
            .collect(),
        0,
    );

    let transaction = Transaction::change(&view.state, changes).with_selection(selection);

    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
}

// O inserts a new line before each line with a selection

pub fn normal_mode(view: &mut View, _count: usize) {
    view.state.mode = Mode::Normal;

    // if leaving append mode, move cursor back by 1
    if view.state.restore_cursor {
        let text = &view.state.doc.slice(..);
        view.state.selection = view.state.selection.transform(|range| {
            Range::new(
                range.from(),
                graphemes::prev_grapheme_boundary(text, range.to()),
            )
        });

        view.state.restore_cursor = false;
    }
}

// TODO: insert means add text just before cursor, on exit we should be on the last letter.
pub fn insert_char(view: &mut View, c: char) {
    let c = Tendril::from_char(c);
    let transaction = Transaction::insert(&view.state, c);

    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
}

pub fn insert_newline(view: &mut View, _count: usize) {
    insert_char(view, '\n');
}

// TODO: handle indent-aware delete
pub fn delete_char_backward(view: &mut View, count: usize) {
    let text = &view.state.doc.slice(..);
    let transaction = Transaction::change_by_selection(&view.state, |range| {
        (
            graphemes::nth_prev_grapheme_boundary(text, range.head, count),
            range.head,
            None,
        )
    });
    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
}

pub fn delete_char_forward(view: &mut View, count: usize) {
    let text = &view.state.doc.slice(..);
    let transaction = Transaction::change_by_selection(&view.state, |range| {
        (
            range.head,
            graphemes::nth_next_grapheme_boundary(text, range.head, count),
            None,
        )
    });
    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
}
