use std::{
    borrow::{self, Borrow},
    convert::{TryFrom, TryInto},
    iter::FromIterator,
    ops,
};

use crate::{
    text_size::{TextOffset, TextRange, TextRange1, TextSize},
    Tendril, Tendril1,
};
use ropey::Rope;
use similar::DiffableStr;

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

    fn apply(&self, rope: &mut Rope) {
        use Change::*;
        match self {
            Insert { at, contents } => rope.insert((*at).into(), contents.as_ref()),
            Delete(range) => {
                let bounds: ops::Range<usize> = TextRange::from(*range).into();
                rope.remove(bounds);
            }
            Replace(kind) => match kind {
                ReplaceKind::Normal { range, contents } => {
                    let bounds: ops::Range<usize> = TextRange::from(*range).into();
                    rope.remove(bounds);
                    rope.insert(range.start().into(), contents.as_ref());
                }
                _ => unimplemented!(),
                // ReplaceKind::Entire { contents } => *rope = contents,
            },
            Empty => (),
        }
    }

    fn add_offset(self, offset: TextOffset) -> Self {
        use Change::*;
        match self {
            Insert { at, contents } => Insert {
                at: at + offset,
                contents,
            },
            Delete(range) => Delete(range + offset),
            Replace(kind) => Replace(match kind {
                ReplaceKind::Normal { range, contents } => ReplaceKind::Normal {
                    range: range + offset,
                    contents,
                },
                _ => unimplemented!(),
            }),
            Empty => Empty,
        }
    }

    fn offset(&self) -> TextOffset {
        use Change::*;
        match self {
            Insert { contents, .. } => contents.len().try_into().unwrap(),
            Delete(range) => range.len().try_into().unwrap(),
            Replace(kind) => match kind {
                ReplaceKind::Normal { range, contents } => (contents.len()
                    - usize::from(range.len()))
                .try_into()
                .unwrap(),
                _ => unimplemented!(),
            },
            Empty => 0.into(),
        }
    }

    fn range(&self) -> Option<TextRange> {
        use Change::*;
        match self {
            Insert { at, contents } => Some(TextRange::empty(at)),
            Delete(range) => Some(range.into()),
            Replace(kind) => match kind {
                &ReplaceKind::Normal { range, .. } => Some(range.into()),
                _ => unimplemented!(),
            },
            Empty => None,
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

    fn build(mut self) -> ChangeSet {
        assert_disjoint(&mut self.changes);
        self.build_unchecked()
    }

    fn build_unstable(mut self) -> ChangeSet {
        assert_disjoint_unstable(&mut self.changes);
        self.build_unchecked()
    }

    fn build_unchecked(self) -> ChangeSet {
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
    // fn apply(&self, text: &mut Rope) {
    //     let mut offset  = 0;
    //     for change in changes {

    //     }
    // }
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

fn check_disjoint_impl(indels: &mut [Change], unstable: bool) -> bool {
    let key = |change: &Change| {
        let change = change.range().unwrap_or(TextRange::empty(1));
        (change.start(), change.end())
    };
    if unstable {
        indels.sort_unstable_by_key(key);
    } else {
        indels.sort_by_key(key);
    }
    indels
        .iter()
        .zip(indels.iter().skip(1))
        .filter_map(|(l, r)| l.range().and_then(|l| r.range().map(|r| (l, r))))
        .all(|(l, r)| l.end() <= r.start())
}
