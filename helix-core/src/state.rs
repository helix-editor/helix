use crate::{Rope, Selection};

#[derive(Debug, Clone)]
pub struct State {
    pub doc: Rope,
    pub selection: Selection,
}

impl State {
    #[must_use]
    pub fn new(doc: Rope) -> Self {
        Self {
            doc,
            selection: Selection::point(0),
        }
    }
}
