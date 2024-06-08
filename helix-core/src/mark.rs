use std::num::NonZeroUsize;

use crate::Selection;

pub struct Mark {
    doc_id: NonZeroUsize,
    selection: Selection,
}
