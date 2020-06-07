use crate::state::{Direction, Granularity, State};

/// A command is a function that takes the current state and a count, and does a side-effect on the
/// state (usually by creating and applying a transaction).
pub type Command = fn(state: &mut State, count: usize);

pub fn move_char_left(state: &mut State, count: usize) {
    // TODO: use a transaction
    let selection = state.move_selection(
        // TODO: remove the clone here
        state.selection.clone(),
        Direction::Backward,
        Granularity::Character,
        count,
    );
    state.selection = selection;
}

pub fn move_char_right(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(
        // TODO: remove the clone here
        state.selection.clone(),
        Direction::Forward,
        Granularity::Character,
        count,
    );
}

pub fn move_line_up(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(
        // TODO: remove the clone here
        state.selection.clone(),
        Direction::Backward,
        Granularity::Line,
        count,
    );
}

pub fn move_line_down(state: &mut State, count: usize) {
    // TODO: use a transaction
    state.selection = state.move_selection(
        // TODO: remove the clone here
        state.selection.clone(),
        Direction::Forward,
        Granularity::Line,
        count,
    );
}
