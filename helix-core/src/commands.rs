use crate::state::{Direction, Granularity, State};

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
type Command = fn(state: &mut State, count: usize);

fn move_char_left(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(
        state.selection,
        Direction::Backward,
        Granularity::Character,
        count,
    );
}

fn move_char_right(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(
        state.selection,
        Direction::Forward,
        Granularity::Character,
        count,
    );
}

fn move_line_up(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(
        state.selection,
        Direction::Backward,
        Granularity::Line,
        count,
    );
}

fn move_line_down(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(
        state.selection,
        Direction::Forward,
        Granularity::Line,
        count,
    );
}
