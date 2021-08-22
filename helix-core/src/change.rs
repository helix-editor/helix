use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    iter::FromIterator,
    ops,
};

use crate::{
    text_size::{TextOffset, TextRange, TextRange1, TextSize},
    Tendril, Tendril1,
};
use ropey::Rope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub delete: TextRange,
    pub insert: Tendril,
}

impl Change {
    fn new(delete: TextRange, insert: Tendril) -> Change {
        Change { delete, insert }
    }

    fn apply(&self, text: &mut Rope) {
        let bounds: ops::Range<usize> = TextRange::from(self.delete).into();
        text.remove(bounds);
        let char_idx = self.delete.start();
        text.insert(char_idx.into(), &self.insert)
    }

    fn add_offset(self, offset: TextOffset) -> Self {
        Change {
            delete: self.delete + offset,
            insert: self.insert,
        }
    }

    fn offset(&self) -> TextOffset {
        (self.insert.len() - usize::from(self.delete.len()))
            .try_into()
            .unwrap()
    }

    // fn invert(&self, original_text: &Rope) -> Self {
    //     use Change::*;
    //     match self {
    //         Delete(range) => {
    //             let text = Cow::from(original_text.slice(range.into1::<ops::Range<usize>>()));
    //             Insert {
    //                 at: range.start(),
    //                 contents: Tendril::from_slice(&text).into(),
    //             }
    //         }
    //         // Insert { at, contents} => {
    //         //     // let chars_len = contents.chars.count();
    //         //     // // Delete {
    //         //     // //     at: range,
    //         //     // // }
    //         //     // changes.delete(chars)
    //         // }
    //         _ => unimplemented!(),
    //     }
    // }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplaceKind {
    Normal {
        range: TextRange1,
        contents: Tendril1,
    },
    Entire {
        contents: Rope,
    },
}

#[derive(Default, Debug)]
pub struct ChangeSetBuilder {
    changes: Vec<Change>,
}

impl Extend<Change> for ChangeSetBuilder {
    fn extend<T: IntoIterator<Item = Change>>(&mut self, iter: T) {
        self.changes.extend(iter)
    }
}

impl FromIterator<Change> for ChangeSetBuilder {
    fn from_iter<T: IntoIterator<Item = Change>>(iter: T) -> Self {
        ChangeSetBuilder {
            changes: iter.into_iter().collect(),
        }
    }
}
impl ChangeSetBuilder {
    pub fn new() -> ChangeSetBuilder {
        Self::default()
    }

    pub fn push(&mut self, change: Change) {
        self.changes.push(change)
    }

    pub fn build(mut self) -> ChangeSet {
        assert_disjoint(&mut self.changes);
        self.build_unchecked()
    }

    pub fn build_unstable(mut self) -> ChangeSet {
        assert_disjoint_unstable(&mut self.changes);
        self.build_unchecked()
    }

    pub fn build_unchecked(self) -> ChangeSet {
        ChangeSet {
            changes: self.changes,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    changes: Vec<Change>,
}

impl ChangeSet {
    pub fn apply(self, text: &mut Rope) {
        let mut offset = 0.into();
        for change in self.changes {
            let change_offset = change.offset();
            change.add_offset(offset).apply(text);
            offset += change_offset;
        }
    }
}

impl FromIterator<Change> for ChangeSet {
    fn from_iter<T: IntoIterator<Item = Change>>(iter: T) -> Self {
        iter.into_iter().collect::<ChangeSetBuilder>().build()
    }
}

fn assert_disjoint(changes: &mut [Change]) {
    assert!(check_disjoint(changes), "Changes were not disjoint");
}

fn assert_disjoint_unstable(changes: &mut [Change]) {
    assert!(
        check_disjoint_unstable(changes),
        "Changes were not disjoint"
    )
}

fn check_disjoint_unstable(changes: &mut [Change]) -> bool {
    check_disjoint_impl(changes, true)
}

fn check_disjoint(changes: &mut [Change]) -> bool {
    check_disjoint_impl(changes, false)
}

fn check_disjoint_impl(changes: &mut [Change], unstable: bool) -> bool {
    fn key(change: &Change) -> (TextSize, TextSize) {
        (change.delete.start(), change.delete.end())
    }
    if unstable {
        changes.sort_unstable_by_key(key);
    } else {
        changes.sort_by_key(key);
    }
    changes
        .iter()
        .zip(changes.iter().skip(1))
        .all(|(l, r)| l.delete.end() <= r.delete.start())
}

#[cfg(test)]
mod tests {
    use std::array;

    use super::*;

    fn check_apply<T: Into<Rope>, U: Into<Rope>, W: Into<Tendril>, const N: usize>(
        changes: [(u32, u32, W); N],
        before: T,
        after: U,
    ) {
        let change_set: ChangeSet = array::IntoIter::new(changes)
            .map(|(start, end, contents)| Change::new((start..end).into(), contents.into()))
            .collect();
        let mut before = before.into();
        let after = after.into();
        change_set.apply(&mut before);
        assert_eq!(before, after);
    }

    #[test]
    fn test_apply() {
        check_apply(
            [(5, 6, "   "), (0, 0, "prefix "), (0, 0, "another ")],
            "hello world!",
            "prefix another hello   world!",
        );
    }

    #[should_panic]
    #[test]
    fn apply_not_disjoint() {
        check_apply(
            [(5, 6, "asdfasdf"), (5, 6, "asdfasd;fkas")],
            "asdpfoiuapdsiofuadpoif",
            "adspfoiuadf",
        );
    }

    #[should_panic]
    #[test]
    fn not_long_enough() {
        check_apply([(3, 4, "")], "", "");
    }
}
