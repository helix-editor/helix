use crate::graphemes;
use crate::selection::{Range, Selection};
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

// TODO: I, A, o and O can share a lot of the primitives.

fn selection_lines(state: &State) -> Vec<usize> {
    // calculate line numbers for each selection range
    let mut lines = state
        .selection
        .ranges()
        .iter()
        .map(|range| state.doc.char_to_line(range.head))
        .collect::<Vec<_>>();

    lines.sort();
    lines.dedup();

    lines
}

// I inserts at the start of each line with a selection
pub fn prepend_to_line(state: &mut State, _count: usize) {
    state.mode = Mode::Insert;

    let lines = selection_lines(state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the start of the line.
            state.doc.line_to_char(index)
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    let transaction = Transaction::new(state).with_selection(selection);

    transaction.apply(state);
    // TODO: need to store into history if successful
}

// A inserts at the end of each line with a selection
pub fn append_to_line(state: &mut State, _count: usize) {
    state.mode = Mode::Insert;

    let lines = selection_lines(state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.
            let line = state.doc.line(index);
            let line_start = state.doc.line_to_char(index);
            line_start + line.len_chars() - 1
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    let transaction = Transaction::new(state).with_selection(selection);

    transaction.apply(state);
    // TODO: need to store into history if successful
}

// o inserts a new line after each line with a selection
pub fn open_below(state: &mut State, _count: usize) {
    state.mode = Mode::Insert;

    let lines = selection_lines(state);

    let positions: Vec<_> = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.
            let line = state.doc.line(index);
            let line_start = state.doc.line_to_char(index);
            line_start + line.len_chars()
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

    let transaction = Transaction::change(state, changes).with_selection(selection);

    transaction.apply(state);
    // TODO: need to store into history if successful
}

// O inserts a new line before each line with a selection

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
