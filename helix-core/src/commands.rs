use crate::graphemes;
use crate::selection::Range;
use crate::state::{Direction, Granularity, Mode, State};
use crate::transaction::{ChangeSet, Transaction};
use crate::Tendril;

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(state: &mut State, count: usize);

pub fn move_char_left(state: &mut State, count: usize) {
    // TODO: use a transaction
    let selection = state.move_selection(Direction::Backward, Granularity::Character, count);
    state.selection = selection;
}

pub fn move_char_right(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(Direction::Forward, Granularity::Character, count);
}

pub fn move_line_up(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(Direction::Backward, Granularity::Line, count);
}

pub fn move_line_down(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(Direction::Forward, Granularity::Line, count);
}

// avoid select by default by having a visual mode switch that makes movements into selects

// insert mode:
// first we calculate the correct cursors/selections
// then we just append at each cursor
// lastly, if it was append mode we shift cursor by 1?

// inserts at the start of each selection
pub fn insert_mode(state: &mut State, _count: usize) {
    state.mode = Mode::Insert;

    state.selection = state
        .selection
        .transform(|range| Range::new(range.to(), range.from()))
}

// inserts at the end of each selection
pub fn append_mode(state: &mut State, _count: usize) {
    state.mode = Mode::Insert;

    // TODO: as transaction
    let text = &state.doc.slice(..);
    state.selection = state.selection.transform(|range| {
        // TODO: to() + next char
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(text, range.to()),
        )
    })
}

// I inserts at the start of each line with a selection
// A inserts at the end of each line with a selection
// o inserts a new line before each line with a selection
// O inserts a new line after each line with a selection

pub fn normal_mode(state: &mut State, _count: usize) {
    // TODO: if leaving append mode, move cursor back by 1
    state.mode = Mode::Normal;
}

// TODO: insert means add text just before cursor, on exit we should be on the last letter.
pub fn insert_char(state: &mut State, c: char) {
    let c = Tendril::from_char(c);
    let transaction = Transaction::insert(&state, c);

    transaction.apply(state);
    // TODO: need to store into history if successful
}

// TODO: handle indent-aware delete
pub fn delete_char_backward(state: &mut State, count: usize) {
    let text = &state.doc.slice(..);
    let transaction = Transaction::change_by_selection(state, |range| {
        (
            graphemes::nth_prev_grapheme_boundary(text, range.head, count),
            range.head,
            None,
        )
    });
    transaction.apply(state);
    // TODO: need to store into history if successful
}

pub fn delete_char_forward(state: &mut State, count: usize) {
    let text = &state.doc.slice(..);
    let transaction = Transaction::change_by_selection(state, |range| {
        (
            graphemes::nth_next_grapheme_boundary(text, range.head, count),
            range.head,
            None,
        )
    });
    transaction.apply(state);
    // TODO: need to store into history if successful
}
