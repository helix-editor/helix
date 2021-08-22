use std::{
    borrow,
    convert::{TryFrom, TryInto},
    iter::FromIterator,
};

use crate::{
    text_size::{TextRange, TextRange1, TextSize},
    Tendril, Tendril1,
};
use ropey::Rope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Insert { at: TextSize, contents: Tendril1 },
    Delete(TextRange1),
    Replace(ReplaceKind),
    // we love our monoids
    Empty,
}

impl Change {
    fn new(range: TextRange, contents: Tendril) -> Change {
        use Change::*;
        match (TextRange1::try_from(range), Tendril1::new(contents)) {
            (Ok(range), None) => Delete(range),
            (Ok(range), Some(contents)) => Replace(ReplaceKind::Normal { range, contents }),
            (Err(()), None) => Empty,
            (Err(()), Some(contents)) => Insert {
                at: range.try_into().unwrap(),
                contents,
            },
        }
    }
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
    #[inline]
    fn new() -> ChangeSetBuilder {
        Self::default()
    }

    // fn finish() -> ChangeSet {

    // }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    pub(crate) changes: Vec<Change>,
    len: TextSize,
    len_after: TextSize,
}

impl ChangeSet {
    fn union(&mut self, other: ChangeSet) {
        self.changes.extend(other.changes)
    }
}

fn assert_disjoint(indels: &mut [impl borrow::Borrow<Change>]) {
    assert!(check_disjoint(indels));
}
fn check_disjoint(indels: &mut [impl borrow::Borrow<Change>]) -> bool {
    true
    // indels.sort_by_key(|indel| (indel.borrow().delete.start(), indel.borrow().delete.end()));
    // indels
    //     .iter()
    //     .zip(indels.iter().skip(1))
    //     .all(|(l, r)| l.borrow().delete.end() <= r.borrow().delete.start())
}
