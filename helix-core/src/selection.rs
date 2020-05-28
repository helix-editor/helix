//! Selections are the primary editing construct. Even a single cursor is defined as an empty
//! single selection range.
//!
//! All positioning is done via `char` offsets into the buffer.
use crate::{Assoc, ChangeSet};
use smallvec::{smallvec, SmallVec};

#[inline]
fn abs_difference(x: usize, y: usize) -> usize {
    if x < y {
        y - x
    } else {
        x - y
    }
}

/// A single selection range. Anchor-inclusive, head-exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    // TODO: optimize into u32
    /// The anchor of the range: the side that doesn't move when extending.
    pub anchor: usize,
    /// The head of the range, moved when extending.
    pub head: usize,
}

impl Range {
    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// Start of the range.
    #[inline]
    #[must_use]
    pub fn from(&self) -> usize {
        std::cmp::min(self.anchor, self.head)
    }

    /// End of the range.
    #[inline]
    #[must_use]
    pub fn to(&self) -> usize {
        std::cmp::max(self.anchor, self.head)
    }

    /// `true` when head and anchor are at the same position.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Check two ranges for overlap.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        // cursor overlap is checked differently
        if self.is_empty() {
            self.from() <= other.to()
        } else {
            self.from() < other.to()
        }
    }

    /// Map a range through a set of changes. Returns a new range representing the same position
    /// after the changes are applied.
    pub fn map(self, changes: &ChangeSet) -> Self {
        let anchor = changes.map_pos(self.anchor, Assoc::Before);
        let head = changes.map_pos(self.head, Assoc::Before);

        // TODO: possibly unnecessary
        if self.anchor == anchor && self.head == head {
            return self;
        }
        Self { anchor, head }
    }

    /// Extend the range to cover at least `from` `to`.
    #[must_use]
    pub fn extend(&self, from: usize, to: usize) -> Self {
        if from <= self.anchor && to >= self.anchor {
            return Range {
                anchor: from,
                head: to,
            };
        }

        Range {
            anchor: self.anchor,
            head: if abs_difference(from, self.anchor) > abs_difference(to, self.anchor) {
                from
            } else {
                to
            },
        }
    }

    // groupAt
}

/// A selection consists of one or more selection ranges.
pub struct Selection {
    // TODO: decide how many ranges to inline SmallVec<[Range; 1]>
    ranges: SmallVec<[Range; 1]>,
    primary_index: usize,
}

impl Selection {
    // map
    // eq

    #[must_use]
    pub fn primary(&self) -> Range {
        self.ranges[self.primary_index]
    }

    /// Ensure selection containing only the primary selection.
    pub fn into_single(self) -> Self {
        if self.ranges.len() == 1 {
            self
        } else {
            Self {
                ranges: smallvec![self.ranges[self.primary_index]],
                primary_index: 0,
            }
        }
    }

    // add_range // push
    // replace_range

    /// Map selections over a set of changes. Useful for adjusting the selection position after
    /// applying changes to a document.
    pub fn map(self, changes: &ChangeSet) -> Self {
        if changes.is_empty() {
            return self;
        }

        Self::new(
            self.ranges
                .into_iter()
                .map(|range| range.map(changes))
                .collect(),
            self.primary_index,
        )
    }

    #[must_use]
    /// Constructs a selection holding a single range.
    pub fn single(anchor: usize, head: usize) -> Self {
        Self {
            ranges: smallvec![Range { anchor, head }],
            primary_index: 0,
        }
    }

    #[must_use]
    pub fn new(ranges: SmallVec<[Range; 1]>, primary_index: usize) -> Self {
        fn normalize(mut ranges: SmallVec<[Range; 1]>, mut primary_index: usize) -> Selection {
            let primary = ranges[primary_index];
            ranges.sort_unstable_by_key(Range::from);
            primary_index = ranges.iter().position(|&range| range == primary).unwrap();

            let mut result: SmallVec<[Range; 1]> = SmallVec::new();

            // TODO: we could do with one vec by removing elements as we mutate

            for (i, range) in ranges.into_iter().enumerate() {
                // if previous value exists
                if let Some(prev) = result.last_mut() {
                    // and we overlap it
                    if range.overlaps(prev) {
                        let from = prev.from();
                        let to = std::cmp::max(range.to(), prev.to());

                        if i <= primary_index {
                            primary_index -= 1
                        }

                        // merge into previous
                        if range.anchor > range.head {
                            prev.anchor = to;
                            prev.head = from;
                        } else {
                            prev.anchor = from;
                            prev.head = to;
                        }
                        continue;
                    }
                }

                result.push(range)
            }

            Selection {
                ranges: result,
                primary_index,
            }
        }

        // TODO: only normalize if needed (any ranges out of order)
        normalize(ranges, primary_index)
    }
}

// TODO: checkSelection -> check if valid for doc length

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_create_normalizes_and_merges() {
        let sel = Selection::new(
            smallvec![
                Range::new(10, 12),
                Range::new(6, 7),
                Range::new(4, 5),
                Range::new(3, 4),
                Range::new(0, 6),
                Range::new(7, 8),
                Range::new(9, 13),
                Range::new(13, 14),
            ],
            0,
        );

        let res = sel
            .ranges
            .into_iter()
            .map(|range| format!("{}/{}", range.anchor, range.head))
            .collect::<Vec<String>>()
            .join(",");

        assert_eq!(res, "0/6,6/7,7/8,9/13,13/14");
    }

    #[test]
    fn test_create_merges_adjacent_points() {
        let sel = Selection::new(
            smallvec![
                Range::new(10, 12),
                Range::new(12, 12),
                Range::new(12, 12),
                Range::new(10, 10),
                Range::new(8, 10),
            ],
            0,
        );

        let res = sel
            .ranges
            .into_iter()
            .map(|range| format!("{}/{}", range.anchor, range.head))
            .collect::<Vec<String>>()
            .join(",");

        assert_eq!(res, "8/10,10/12");
    }
}
