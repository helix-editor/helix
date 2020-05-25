pub struct Change {
    from: usize,
    to: usize,
    insert: Option<String>,
}

impl Change {
    pub fn new(from: usize, to: usize, insert: Option<String>) {
        // old_extent, new_extent, insert
    }
}

pub struct Transaction {}

// ChangeSpec = Change | ChangeSet | Vec<Change>
// ChangeDesc as a ChangeSet without text: can't be applied, cheaper to store.
// ChangeSet = ChangeDesc with Text
pub struct ChangeSet {
    // basically Vec<ChangeDesc> where ChangeDesc = (current len, replacement len?)
    // (0, n>0) for insertion, (n>0, 0) for deletion, (>0, >0) for replacement
    sections: Vec<(usize, isize)>,
}
//
// trait Transaction
// trait StrictTransaction
