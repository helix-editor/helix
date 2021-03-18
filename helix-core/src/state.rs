use crate::{Rope, Selection};

/// A state represents the current editor state of a single buffer.
#[derive(Clone)]
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

    // update/transact:
    // update(desc) => transaction ?  transaction.doc() for applied doc
    // transaction.apply(doc)
    // doc.transact(fn -> ... end)

    // replaceSelection (transaction that replaces selection)
    // changeByRange
    // changes
    // slice
    //
    // getters:
    // tabSize
    // indentUnit
    // languageDataAt()
    //
    // config:
    // indentation
    // tabSize
    // lineUnit
    // syntax
    // foldable
    // changeFilter/transactionFilter
}
