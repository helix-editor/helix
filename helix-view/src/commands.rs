use helix_core::{
    graphemes,
    state::{Direction, Granularity, Mode, State},
    Range, Selection, Tendril, Transaction,
};

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

    lines.sort();
    lines.dedup();

    lines
}

// I inserts at the start of each line with a selection
pub fn prepend_to_line(view: &mut View, _count: usize) {
    view.state.mode = Mode::Insert;

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
    // TODO: need to store into history if successful
}

// A inserts at the end of each line with a selection
pub fn append_to_line(view: &mut View, _count: usize) {
    view.state.mode = Mode::Insert;

    let lines = selection_lines(&view.state);

    let positions = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.
            let line = view.state.doc.line(index);
            let line_start = view.state.doc.line_to_char(index);
            line_start + line.len_chars() - 1
        })
        .map(|pos| Range::new(pos, pos));

    let selection = Selection::new(positions.collect(), 0);

    let transaction = Transaction::new(&mut view.state).with_selection(selection);

    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
}

// o inserts a new line after each line with a selection
pub fn open_below(view: &mut View, _count: usize) {
    view.state.mode = Mode::Insert;

    let lines = selection_lines(&view.state);

    let positions: Vec<_> = lines
        .into_iter()
        .map(|index| {
            // adjust all positions to the end of the line.
            let line = view.state.doc.line(index);
            let line_start = view.state.doc.line_to_char(index);
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

    let transaction = Transaction::change(&view.state, changes).with_selection(selection);

    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
}

// O inserts a new line before each line with a selection

pub fn normal_mode(view: &mut View, _count: usize) {
    // TODO: if leaving append mode, move cursor back by 1
    view.state.mode = Mode::Normal;
}

// TODO: insert means add text just before cursor, on exit we should be on the last letter.
pub fn insert_char(view: &mut View, c: char) {
    let c = Tendril::from_char(c);
    let transaction = Transaction::insert(&view.state, c);

    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
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
            graphemes::nth_next_grapheme_boundary(text, range.head, count),
            range.head,
            None,
        )
    });
    transaction.apply(&mut view.state);
    // TODO: need to store into history if successful
}
