//! Processing of folds when a transaction is applied.
//!
//! During a transaction, the fold container removes disturbed folds.
//!
//! Disturbed folds are folds that:
//! 1. header has been mixed with the outer text
//! 2. header has been completely removed
//! 3. header and target have been mixed
//! 4. start char of target has been removed
//! 5. end char of target has been removed
//!
//! After that, the fold container normalizes its folds.

use std::cmp::{max, min};
use std::iter::once;
use std::ops;

use ropey::RopeSlice;

use crate::ChangeSet;
use crate::{graphemes::prev_grapheme_boundary, transaction::UpdatePosition, Transaction};

use super::FoldContainer;

impl FoldContainer {
    pub fn update_by_transaction(
        &mut self,
        new_text: RopeSlice,
        old_text: RopeSlice,
        transaction: &Transaction,
    ) {
        let disturbed = self.disturbed_folds(old_text, transaction);
        let mut sort = !disturbed.is_empty();

        self.delete(disturbed);

        self.update(new_text, transaction.changes());

        let removables = self.normalize(new_text);
        sort |= !removables.is_empty();

        self.delete(removables);

        if sort {
            self.sort_end_points();
            self.set_super_links();
        }
    }

    /// Returns the start indices of folds that have been disturbed when the transaction is applied.
    fn disturbed_folds(&self, text: RopeSlice, transaction: &Transaction) -> Vec<usize> {
        let mut disturbed: Vec<_> = transaction
            .changes_iter()
            .filter_map(|(from, to, fragment)| {
                // an insertion disturbs no folds
                if from == to {
                    return None;
                }

                let change_range = from..=to - 1;

                // the range of potentially disturbed folds
                let range = {
                    let start = {
                        let start_fold = self
                            .end_points
                            .get(self.end_points.partition_point(|efp| {
                                max(efp.target, efp.char) < *change_range.start()
                            }))
                            .map(|efp| efp.fold(self))?;
                        start_fold
                            .superest_fold(self)
                            .unwrap_or(start_fold)
                            .start_idx()
                    };
                    let end = start
                        + self.start_points[start..]
                            .partition_point(|sfp| sfp.header <= *change_range.end());
                    start..end
                };

                Some(
                    self.start_points[range]
                        .iter()
                        .map(|sfp| sfp.fold(self))
                        .filter_map(move |fold| {
                            // returns the overlapping range of the passed `range` and `change_range`
                            let overlap = |range: &ops::RangeInclusive<_>| {
                                let start = max(*range.start(), *change_range.start());
                                let end = min(*range.end(), *change_range.end());
                                start..=end
                            };

                            let header = {
                                let start = fold.header();
                                let end = prev_grapheme_boundary(text, fold.start.target);
                                start..=end
                            };
                            let target = {
                                let start = fold.start.target;
                                let end = fold.end.target;
                                start..=end
                            };

                            let header_overlap = overlap(&header);
                            let target_overlap = overlap(&target);

                            // 1. header has been mixed with the outer text
                            if !header_overlap.is_empty()
                                && change_range.start() < header.start()
                                && fragment.is_some()
                            {
                                return Some(fold.start_idx());
                            }

                            // 2. header has been completely removed
                            if header_overlap == header && fragment.is_none() {
                                return Some(fold.start_idx());
                            }

                            // 3. header and target have been mixed
                            if !header_overlap.is_empty() && !target_overlap.is_empty() {
                                return Some(fold.start_idx());
                            }

                            // 4. start char of target has been removed
                            if target_overlap.contains(target.start()) {
                                return Some(fold.start_idx());
                            }

                            // 5. end char of target has been removed
                            if target_overlap.contains(target.end()) {
                                return Some(fold.start_idx());
                            }

                            None
                        }),
                )
            })
            .flatten()
            .collect();

        disturbed.sort();
        disturbed.dedup();

        disturbed
    }

    /// Updates headers and targets.
    fn update(&mut self, new_text: RopeSlice, changes: &ChangeSet) {
        use Component::*;

        let mut start_points = self.start_points.iter_mut().peekable();
        let mut end_points = self.end_points.iter_mut().peekable();

        // create a partially sorted positions iterator for a fast update
        let sorted_positions =
            std::iter::from_fn(move || match (start_points.peek(), end_points.peek()) {
                (None, None) => None,

                (Some(sfp), efp) if efp.map_or(true, |efp| sfp.header < efp.target) => {
                    start_points.next().map(|sfp| {
                        once(Header.update(&mut sfp.header))
                            .chain(Some(StartTarget.update(&mut sfp.target)))
                    })
                }

                (_, Some(_)) => end_points
                    .next()
                    .map(|efp| once(EndTarget.update(&mut efp.target)).chain(None)),

                _ => unreachable!("Patterns must be exhausted."),
            })
            .flatten();

        changes.update_positions_with_helper(new_text, sorted_positions);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Header,
    StartTarget,
    EndTarget,
}

impl Component {
    fn update(self, position: &mut usize) -> ComponentUpdater<'_> {
        ComponentUpdater {
            component: self,
            position,
        }
    }
}

struct ComponentUpdater<'a> {
    component: Component,
    position: &'a mut usize,
}

impl<'a> UpdatePosition<RopeSlice<'a>> for ComponentUpdater<'a> {
    fn get_pos(&self) -> usize {
        *self.position
    }

    fn set_pos(&mut self, new_pos: usize) {
        *self.position = new_pos;
    }

    fn insert(&mut self, new_pos: usize, fragment: &str, text: &mut RopeSlice<'a>) {
        use Component::*;

        // abc -> aXYbc
        // before insertion -> a
        // start of insertion -> X
        // end of insertion -> Y
        // after insertion -> b
        #[rustfmt::skip]
        match self.component {
            // before insertion
            EndTarget
                => self.set_pos(prev_grapheme_boundary(*text, new_pos)),

            // start of insertion
            // _ => self.set_pos(new_pos),

            // end of insertion
            // _ => self.set_pos(prev_grapheme_boundary(*text, new_pos + fragment.chars().count())),

            // after insertion
            Header
            | StartTarget
                => self.set_pos(new_pos + fragment.chars().count()),
        };
    }

    fn delete(&mut self, _: usize, _: usize, new_pos: usize, text: &mut RopeSlice) {
        use Component::*;

        // abc -> ac
        // before deletion -> a
        // after deletion -> c
        #[rustfmt::skip]
        match self.component {
            // before deletion
            StartTarget
                =>  self.set_pos(prev_grapheme_boundary(*text, new_pos)),

            // after deletion
            Header
            | EndTarget
                => self.set_pos(new_pos),
        };
    }

    fn replace(
        &mut self,
        _: usize,
        _: usize,
        new_pos: usize,
        fragment: &str,
        text: &mut RopeSlice,
    ) {
        use Component::*;

        // abc -> aXYc
        // before replacement -> a
        // start of replacement -> X
        // end of replacement -> Y
        // after replacement -> c
        #[rustfmt::skip]
        match self.component {
            // before replacement
            // _ => self.set_pos(prev_grapheme_boundary(*text, new_pos)),

            // start of replacement
            Header
            | StartTarget
                => self.set_pos(new_pos),

            // end of replacement
            EndTarget
                => self.set_pos(prev_grapheme_boundary(*text, new_pos + fragment.chars().count())),

            // after replacement
            // _ => self.set_pos(new_pos + fragment.chars().count()),
        };
    }
}
