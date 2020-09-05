use crate::state::{Direction, Granularity, Mode, State};

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

pub fn insert_mode(state: &mut State, _count: usize) {
    state.mode = Mode::Insert;
}

pub fn normal_mode(state: &mut State, _count: usize) {
    state.mode = Mode::Normal;
}

// TODO: insert means add text just before cursor, on exit we should be on the last letter.
pub fn insert(state: &mut State, c: char) {
    // TODO: needs to work with multiple cursors
    use crate::transaction::ChangeSet;

    let pos = state.selection.primary().head;
    let changes = ChangeSet::insert(&state.doc, pos, c);
    // TODO: need to store history
    changes.apply(state.contents_mut());
    state.selection = state.selection.clone().map(&changes);
}
