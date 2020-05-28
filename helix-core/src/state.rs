use crate::{Buffer, Selection};

/// A state represents the current editor state of a single buffer.
pub struct State {
    // TODO: maybe doc: ?
    buffer: Buffer,
    selection: Selection,
}

impl State {
    #[must_use]
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            selection: Selection::single(0, 0),
        }
    }

    // TODO: buf/selection accessors

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
