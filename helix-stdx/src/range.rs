use std::ops::{self, RangeBounds};

/// A range of `char`s within the text.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct Range<T = usize> {
    pub start: T,
    pub end: T,
}

impl<T: PartialOrd> Range<T> {
    pub fn contains(&self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

impl<T> RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> ops::Bound<&T> {
        ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> ops::Bound<&T> {
        ops::Bound::Excluded(&self.end)
    }
}

/// Returns true if all ranges yielded by `sub_set` are contained by
/// `super_set`. This is essentially an optimized implementation of
/// `sub_set.all(|rb| super_set.any(|ra| ra.contains(rb)))` that runs in O(m+n)
/// instead of O(mn) (and in many cases faster).
///
/// Both iterators must uphold a the following invariants:
/// * ranges must not overlap (but they can be adjacent)
/// * ranges must be sorted
pub fn is_subset<const ALLOW_EMPTY: bool>(
    mut super_set: impl Iterator<Item = Range>,
    mut sub_set: impl Iterator<Item = Range>,
) -> bool {
    let (mut super_range, mut sub_range) = (super_set.next(), sub_set.next());
    loop {
        match (super_range, sub_range) {
            // skip over irrelevant ranges
            (Some(ra), Some(rb))
                if ra.end <= rb.start && (ra.start != rb.start || !ALLOW_EMPTY) =>
            {
                super_range = super_set.next();
            }
            (Some(ra), Some(rb)) => {
                if ra.contains(rb) {
                    sub_range = sub_set.next();
                } else {
                    return false;
                }
            }
            (None, Some(_)) => {
                // exhausted `super_set`, we can't match the reminder of `sub_set`
                return false;
            }
            (_, None) => {
                // no elements from `sub_sut` left to match, `super_set` contains `sub_set`
                return true;
            }
        }
    }
}

pub fn is_exact_subset(
    mut super_set: impl Iterator<Item = Range>,
    mut sub_set: impl Iterator<Item = Range>,
) -> bool {
    let (mut super_range, mut sub_range) = (super_set.next(), sub_set.next());
    let mut super_range_matched = true;
    loop {
        match (super_range, sub_range) {
            // skip over irrelevant ranges
            (Some(ra), Some(rb)) if ra.end <= rb.start && ra.start < rb.start => {
                if !super_range_matched {
                    return false;
                }
                super_range_matched = false;
                super_range = super_set.next();
            }
            (Some(ra), Some(rb)) => {
                if ra.contains(rb) {
                    super_range_matched = true;
                    sub_range = sub_set.next();
                } else {
                    return false;
                }
            }
            (None, Some(_)) => {
                // exhausted `super_set`, we can't match the reminder of `sub_set`
                return false;
            }
            (_, None) => {
                // no elements from `sub_sut` left to match, `super_set` contains `sub_set`
                return super_set.next().is_none();
            }
        }
    }
}
