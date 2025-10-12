//! This module provides text folding primitives.
//!
//! A fold consists of the following components:
//!
//! 1. **`Object`** - a type of fold, which indicates how a user has folded the text.
//!     For example, the object is **Selection** when a user has folded arbitrarily selected text.
//!     And the object is **TextObject** when a user has folded a text object, such as a function, class, and so on.
//!
//! 2. **`Header`** - a fragment that describes what is folded.
//!     For example, the header of a folded function is its signature.
//!     Additionally, headers are used to unfold text.
//!
//! 3. **`Target`** - a fragment that defines the block that will be folded.
//!     For example, for a function, the target is a span of the **function.inside** capture.
//!
//! 4. **`Block`** - a folded (non-visible) text. It is a range of lines.
//!
//! Look at the following code:
//! ```
//! fn f(a: u32) -> u32 {
//!     a + a
//! }
//! ```
//! Let's assume that a user has folded the `f` function,
//! thus the new fold has been created with the following components:
//!
//! - **`Object`** is **TextObject** with the value **function**.
//!
//! - **`Header`** is the fragment **"fn f(a: u32) -> u32"**.
//!
//! - **`Target`** is the fragment that spans the **function.inside** capture.
//!
//! - **`Block`** is the range of lines from 2 through 3.
//!
//! The block spans only two lines.
//! This is because the start line of the target has supplementary non-whitespace text.
//! The block must contain only the text described by the header.
//! In this case, that text is the function definition (textobject function.inside).
//!
//! Consider the additional example:
//! ```
//! fn f<T>(a: T) -> T
//! where
//!     T: std::ops::Add<Output = T> + Copy
//! /* interfering comment*/ {
//!     a + a
//! } /* interfering comment */
//!
//! fn g<U>(b: U) -> U
//! where
//!     U: std::ops::Sub<Output = U> + Copy
//! {
//!     b - b
//! }
//! ```
//! The `f` and `g` functions are also folded. Let the folds be called `F` and `G`, respectively.
//! These folds differ in their blocks. Their blocks span one line and three lines, respectively.
//! This is because the `F` fold target has supplementary comments at its boundary lines, but the `G` fold target does not.
//! However, if a user removes the interfering comments the `F` fold block will be extended to three lines.
//!
//! The process of calculating a block is called **normalization**.
//!
//! Folds can be nested within others.
//! ```
//! trait T {
//!     fn f() {
//!         println!("hello world");
//!     }
//! }
//! ```
//! If trait `T` and function `f` are folded.
//! Then, the fold of function `f` is nested in the fold of trait `T`.
//! Folds that span other folds are called **super** folds.
//! Folds that are not nested are called **superest** folds.

use std::cmp::{max, min, Ordering};
use std::fmt;
use std::iter::once;
use std::ops;

use helix_stdx::rope::RopeSliceExt;

use crate::graphemes::prev_grapheme_boundary;
use crate::line_ending::line_end_byte_index;
use crate::line_ending::line_end_char_index;
use crate::line_ending::rope_is_line_ending;
use crate::Range;
use crate::RopeSlice;
use crate::Selection;

#[cfg(test)]
pub(crate) mod test_utils;

#[cfg(test)]
mod test;

/// A kind of fold.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FoldObject {
    /// Indicates an arbitrary folded text
    Selection,
    /// Indicates a folded text of text object (class, function, etc.)
    TextObject(&'static str),
}

impl fmt::Display for FoldObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Selection => write!(f, "something"),
            Self::TextObject(textobject) => write!(f, "{textobject}"),
        }
    }
}

/// A start of fold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartFoldPoint {
    pub object: FoldObject,

    /// The first char of header.
    pub header: usize,

    /// The first char of target.
    pub target: usize,

    /// The first byte of block.
    pub byte: usize,
    /// The first char of block.
    pub char: usize,
    /// The first line of block.
    pub line: usize,

    /// An index of `EndFoldPoint` relating to the same fold.
    link: usize,
    /// An index of `StartFoldPoint` relating to the super fold.
    super_link: Option<usize>,
}

impl StartFoldPoint {
    /// Returns the fold.
    pub fn fold<'a>(&'a self, container: &'a FoldContainer) -> Fold {
        Fold::new(self, &container.end_points[self.link])
    }

    pub fn is_superest(&self) -> bool {
        self.super_link.is_none()
    }

    fn from(text: RopeSlice, object: FoldObject, header: usize, target: usize) -> Self {
        let mut result = Self {
            object,
            header,
            target,
            byte: 0,
            char: 0,
            line: 0,
            link: 0,
            super_link: None,
        };
        result.set_block(text, result.block_line(text));
        result
    }

    /// Returns the first line of the block.
    fn block_line(&self, text: RopeSlice) -> usize {
        let truncate = text
            .graphemes_at(text.char_to_byte(self.target))
            .reversed()
            .take_while(|&g| !rope_is_line_ending(g))
            .flat_map(|g| g.chars())
            .any(|c| !c.is_whitespace());

        text.char_to_line(self.target) + truncate as usize
    }

    /// Sets `byte`, `char`, `line` fields.
    fn set_block(&mut self, text: RopeSlice, line: usize) {
        self.byte = text.line_to_byte(line);
        self.char = text.line_to_char(line);
        self.line = line;
    }
}

/// An end of fold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndFoldPoint {
    /// The last char of target.
    pub target: usize,

    /// The last grapheme aligned byte of block
    pub byte: usize,
    /// The last grapheme aligned char of blcok.
    pub char: usize,
    /// The last line of block.
    pub line: usize,

    /// An index of `StartFoldPoint` of the same fold.
    link: usize,
}

impl EndFoldPoint {
    /// Returns the fold.
    pub fn fold<'a>(&'a self, container: &'a FoldContainer) -> Fold {
        Fold::new(&container.start_points[self.link], self)
    }

    fn from(text: RopeSlice, target: usize) -> Self {
        let mut result = Self {
            target,
            byte: 0,
            char: 0,
            line: 0,
            link: 0,
        };
        result.set_block(text, result.block_line(text));
        result
    }

    /// Returns the last line of the block.
    fn block_line(&self, text: RopeSlice) -> usize {
        let truncate = text
            .graphemes_at(text.char_to_byte(self.target))
            .skip({
                let end_char = line_end_char_index(&text, text.char_to_line(self.target));
                (self.target != end_char) as usize
            })
            .take_while(|&g| !rope_is_line_ending(g))
            .flat_map(|g| g.chars())
            .any(|c| !c.is_whitespace());

        text.char_to_line(self.target) - truncate as usize
    }

    /// Sets `byte`, `char`, `line` fields.
    fn set_block(&mut self, text: RopeSlice, line: usize) {
        self.byte = line_end_byte_index(&text, line);
        self.char = line_end_char_index(&text, line);
        self.line = line;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fold<'a> {
    pub start: &'a StartFoldPoint,
    pub end: &'a EndFoldPoint,
}

impl<'a> Fold<'a> {
    /// Creates a pair of fold points.
    pub fn new_points(
        text: RopeSlice,
        object: FoldObject,
        header: usize,
        target: &ops::RangeInclusive<usize>,
    ) -> (StartFoldPoint, EndFoldPoint) {
        (
            StartFoldPoint::from(text, object, header, *target.start()),
            EndFoldPoint::from(text, *target.end()),
        )
    }

    pub fn new(start: &'a StartFoldPoint, end: &'a EndFoldPoint) -> Self {
        Self { start, end }
    }

    pub fn object(self) -> &'a FoldObject {
        &self.start.object
    }

    pub fn header(self) -> usize {
        self.start.header
    }

    pub fn is_superest(self) -> bool {
        self.start.super_link.is_none()
    }

    pub fn super_fold(self, container: &'a FoldContainer) -> Option<Self> {
        self.start
            .super_link
            .map(|idx| container.start_points[idx].fold(container))
    }

    pub fn superest_fold(self, container: &'a FoldContainer) -> Option<Self> {
        self.super_fold(container)
            .map(|super_fold| super_fold.superest_fold(container).unwrap_or(super_fold))
    }

    /// Returns the index of the start fold point.
    pub fn start_idx(self) -> usize {
        self.end.link
    }

    /// Returns the index of the end fold point.
    pub fn end_idx(self) -> usize {
        self.start.link
    }
}

/// A fold manager.
/// All folds of `View` are contained in it.
#[derive(Debug, Default, Clone)]
pub struct FoldContainer {
    start_points: Vec<StartFoldPoint>,
    end_points: Vec<EndFoldPoint>,
}

impl FoldContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.start_points.is_empty()
    }

    pub fn len(&self) -> usize {
        self.start_points.len()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.start_points.reserve(additional);
        self.end_points.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.start_points.clear();
        self.end_points.clear();
    }

    pub fn from(text: RopeSlice, points: Vec<(StartFoldPoint, EndFoldPoint)>) -> Self {
        let mut ret = Self::new();
        ret.add(text, points);
        ret
    }

    /// Adds new folds to the container.
    pub fn add(&mut self, text: RopeSlice, points: Vec<(StartFoldPoint, EndFoldPoint)>) {
        self.reserve(points.len());

        for (mut sfp, mut efp) in points.into_iter() {
            sfp.link = self.len();
            efp.link = self.len();
            self.start_points.push(sfp);
            self.end_points.push(efp);
        }

        self.sort_start_points();

        let deletables = self.normalize(text);
        self.delete(deletables);

        self.sort_end_points();
        self.set_super_links();
    }

    /// Adds new folds to the container and removes existing ones that overlap them.
    pub fn replace(&mut self, text: RopeSlice, mut points: Vec<(StartFoldPoint, EndFoldPoint)>) {
        // `true` if `f1` overlaps with `f2`
        let overlap = |f1: Fold, f2: Fold| {
            let range = |fold: Fold| {
                let start = text.char_to_line(fold.start.header);
                let end = text.char_to_line(fold.end.target);
                start..=end
            };

            let r1 = range(f1);
            let r2 = range(f2);

            let start = max(*r1.start(), *r2.start());
            let end = min(*r1.end(), *r2.end());

            !(start..=end).is_empty()
        };

        for (sfp, efp) in points.iter_mut() {
            let replacement = Fold::new(sfp, efp);

            // collect folds that overlap with replacement
            let overlappables: Vec<_> = self
                .start_points
                .iter()
                .map(|sfp| sfp.fold(self))
                .filter_map(|fold| overlap(fold, replacement).then_some(fold.start_idx()))
                .collect();

            // extend replacement
            if let Some(fold) = overlappables
                .last()
                .map(|&i| self.start_points[i].fold(self))
            {
                efp.target = max(efp.target, fold.end.target);
            }

            self.remove(text, overlappables);
        }

        self.add(text, points);
    }

    /// Removes folds from the container for the passed `start_indices`.
    /// # Invariant
    /// Start indices must be sorted and unique.
    pub fn remove(&mut self, text: RopeSlice, start_indices: Vec<usize>) {
        self.delete(start_indices);

        let removables = self.normalize(text);
        self.delete(removables);

        self.sort_end_points();
        self.set_super_links();
    }

    /// Removes folds that contain the anchor or the head of a selection.
    pub fn remove_by_selection(&mut self, text: RopeSlice, selection: &Selection) {
        let mut removables: Vec<_> = selection
            .iter()
            .flat_map(|range| {
                let (start, end) = range.line_range(text);
                once(start).chain((start != end).then_some(end))
            })
            .filter_map(|line| {
                // the range of folds that potentially contain `line`
                let range = {
                    let start = {
                        let start_fold = self
                            .end_points
                            .get(self.end_points.partition_point(|efp| efp.line < line))
                            .map(|efp| efp.fold(self))?;
                        start_fold
                            .superest_fold(self)
                            .unwrap_or(start_fold)
                            .start_idx()
                    };
                    let end =
                        start + self.start_points[start..].partition_point(|sfp| sfp.line <= line);

                    start..end
                };

                let self_ref = &*self;
                Some(self_ref.start_points[range].iter().filter_map(move |sfp| {
                    let fold = sfp.fold(self_ref);
                    let block = fold.start.line..=fold.end.line;
                    block.contains(&line).then_some(fold.start_idx())
                }))
            })
            .flatten()
            .collect();

        removables.sort();
        removables.dedup();

        self.remove(text, removables);
    }

    /// Moves the left side of `range` to the start of the header if it is contained in the fold.
    /// Moves the right side of `range` to the end of the header if it is contained in the fold.
    pub fn throw_range_out_of_folds(&self, text: RopeSlice, range: Range) -> Range {
        let block = |fold: Fold| fold.start.char..=fold.end.char;

        let from = self
            .superest_fold_containing(range.from(), block)
            .map(|fold| fold.start.header);

        let to = self
            .superest_fold_containing(
                if range.is_empty() {
                    range.to()
                } else {
                    prev_grapheme_boundary(text, range.to())
                },
                block,
            )
            .map(|fold| match fold.start.char.cmp(&fold.start.target) {
                Ordering::Greater => fold.start.target,
                _ => fold.start.char,
            });

        Range::new(from.unwrap_or(range.from()), to.unwrap_or(range.to()))
            .with_direction(range.direction())
    }

    /// Finds fold.
    pub fn find(
        &self,
        object: &FoldObject,
        range: &ops::RangeInclusive<usize>,
        mut get_range: impl FnMut(Fold) -> ops::RangeInclusive<usize>,
    ) -> Option<Fold> {
        self.start_points
            .binary_search_by(|sfp| {
                let fold_range = get_range(sfp.fold(self));
                fold_range
                    .start()
                    .cmp(range.start())
                    .then(fold_range.end().cmp(range.end()).reverse())
                    .then(sfp.object.cmp(object))
            })
            .map(|idx| self.start_points[idx].fold(self))
            .ok()
    }

    pub fn start_points_in_range(
        &self,
        range: &ops::RangeInclusive<usize>,
        mut get_idx: impl FnMut(&StartFoldPoint) -> usize,
    ) -> &[StartFoldPoint] {
        let start = self
            .start_points
            .partition_point(|sfp| get_idx(sfp) < *range.start());

        let Some(start_points) = self.start_points.get(start..) else {
            return &[];
        };

        let end = start + start_points.partition_point(|sfp| get_idx(sfp) <= *range.end());

        &self.start_points[start..end]
    }

    pub fn fold_containing(
        &self,
        idx: usize,
        mut get_range: impl FnMut(Fold) -> ops::RangeInclusive<usize>,
    ) -> Option<Fold> {
        let end_idx = self.end_points.partition_point(|efp| {
            let range = get_range(efp.fold(self));
            *range.end() < idx
        });

        let mut fold = self.end_points.get(end_idx)?.fold(self);
        while !get_range(fold).contains(&idx) {
            fold = match fold.super_fold(self) {
                Some(fold) => fold,
                None => return None,
            }
        }

        Some(fold)
    }

    pub fn superest_fold_containing(
        &self,
        idx: usize,
        get_range: impl FnMut(Fold) -> ops::RangeInclusive<usize>,
    ) -> Option<Fold> {
        self.fold_containing(idx, get_range)
            .map(|fold| fold.superest_fold(self).unwrap_or(fold))
    }

    pub fn start_points(&self) -> &[StartFoldPoint] {
        &self.start_points
    }
}

impl FoldContainer {
    fn sort_start_points(&mut self) {
        self.start_points.sort_by(|sfp1, sfp2| {
            let efp1 = &self.end_points[sfp1.link];
            let efp2 = &self.end_points[sfp2.link];
            sfp1.target
                .cmp(&sfp2.target)
                .then(efp1.target.cmp(&efp2.target).reverse())
                .then(sfp1.object.cmp(&sfp2.object))
                .then_with(|| unreachable!("Unexpected doubles."))
        });

        for (i, sfp) in self.start_points.iter().enumerate() {
            self.end_points[sfp.link].link = i;
        }
    }

    fn sort_end_points(&mut self) {
        self.end_points.sort_by(|efp1, efp2| {
            efp1.target
                .cmp(&efp2.target)
                .then(efp1.link.cmp(&efp2.link).reverse())
                .then_with(|| unreachable!("Unexpected doubles."))
        });

        for (i, efp) in self.end_points.iter().enumerate() {
            self.start_points[efp.link].link = i;
        }
    }

    /// Normalizes folds and returns start indices of folds to remove.
    ///
    /// # Invariant
    /// Returned folds must be removed.
    fn normalize(&mut self, text: RopeSlice) -> Vec<usize> {
        let range = |fold: Fold| {
            let start = fold.header();
            let end = fold.end.target;
            start..=end
        };

        // Returns `true` if `r1` and `r2` overlap
        let overlap = |r1: &ops::RangeInclusive<_>, r2: &ops::RangeInclusive<_>| {
            let start = max(*r1.start(), *r2.start());
            let end = min(*r1.end(), *r2.end());

            !(start..=end).is_empty()
        };

        // Returns `true` if `r1` spans `r2`
        let span = |r1: &ops::RangeInclusive<_>, r2: &ops::RangeInclusive<_>| {
            let start = max(*r1.start(), *r2.start());
            let end = min(*r1.end(), *r2.end());

            (start..=end) == *r2
        };

        let mut removables = Vec::new();
        for i in 0..self.len() {
            let fold = self.start_points[i].fold(self);

            // get the start line of the block
            let block_start = {
                let init = fold.start.block_line(text);
                self.start_points
                    .iter()
                    .take(i)
                    .rev()
                    .map(|sfp| sfp.fold(self))
                    .take_while(|&prev_fold| prev_fold.end.line == init - 1)
                    .find_map(|prev_fold| {
                        (!removables.contains(&prev_fold.start_idx())).then_some(init + 1)
                    })
                    .unwrap_or(init)
            };

            // if the subsequent fold that overlaps with the current fold is found,
            // then add the current fold to removables
            if self
                .start_points
                .iter()
                .skip(i + 1)
                .map(|next_sfp| next_sfp.fold(self))
                .take_while(|&next_fold| overlap(&range(fold), &range(next_fold)))
                .find_map(|next_fold| {
                    let fold_range = &range(fold);
                    let next_fold_range = &range(fold);

                    (!span(fold_range, next_fold_range) && !span(next_fold_range, fold_range))
                        .then(|| text.char_to_line(next_fold.header()) - 1)
                })
                .is_some()
            {
                removables.push(i);
                continue;
            }

            let block_end = fold.end.block_line(text);

            if block_start > block_end {
                removables.push(i);
                continue;
            }

            let sfp = &mut self.start_points[i];
            let efp = &mut self.end_points[sfp.link];

            sfp.set_block(text, block_start);
            efp.set_block(text, block_end);
        }

        removables
    }

    // Sets hierarchy of folds.
    fn set_super_links(&mut self) {
        if self.is_empty() {
            return;
        }
        let full_range = 0..=self.len() - 1;
        self.set_super_links_impl(&full_range, None, 0);
    }

    fn set_super_links_impl(
        &mut self,
        range: &ops::RangeInclusive<usize>,
        super_link: Option<usize>,
        nesting: usize,
    ) {
        let mut idx = *range.start();
        while idx <= *range.end() {
            self.start_points[idx].super_link = super_link;
            if idx == *range.end() {
                return;
            }

            let nested_range = {
                let fold = self.start_points[idx].fold(self);
                let start = fold.start_idx() + 1;
                let end = min(*range.end(), fold.end_idx() + nesting);
                start..=end
            };

            if nested_range.is_empty() {
                idx += 1;
            } else {
                self.set_super_links_impl(&nested_range, Some(idx), nesting + 1);
                idx = *nested_range.end() + 1;
            }
        }
    }

    /// Just deletes folds at `start_indices`
    /// # Attention
    /// It is service method.
    /// It is probably not the method you want to use; see `remove` method.
    fn delete(&mut self, start_indices: Vec<usize>) {
        for start_idx in start_indices.into_iter().rev() {
            let end_idx = self.start_points[start_idx].link;

            // remove start point
            self.start_points.remove(start_idx);
            for sfp in self.start_points.iter().skip(start_idx) {
                self.end_points[sfp.link].link -= 1;
            }

            // remove end point
            self.end_points.remove(end_idx);
            for efp in self.end_points.iter().skip(end_idx) {
                self.start_points[efp.link].link -= 1;
            }
        }
    }
}
