use crate::{Rope, Selections};

/// A state represents the current editor state of a single buffer.
#[derive(Debug, Clone)]
pub struct State {
    pub doc: Rope,
    pub selection: Selections,
}

impl State {
    #[must_use]
    pub fn new(doc: Rope) -> Self {
        Self {
            doc,
            selection: Selections::point(0),
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
