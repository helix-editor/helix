//! This modules encapsulates a tiny bit of unsafe code that
//! makes diffing significantly faster and more ergonomic to implement.
//! This code is necessary because diffing requires quick random
//! access to the lines of the text that is being diffed.
//!
//! Therefore it is best to collect the `Rope::lines` iterator into a vec
//! first because access to the vec is `O(1)` where `Rope::line` is `O(log N)`.
//! However this process can allocate a (potentially quite large) vector.
//!
//! To avoid reallocation for every diff, the vector is reused.
//! However the RopeSlice references the original rope and therefore forms a self-referential data structure.
//! A transmute is used to change the lifetime of the slice to static to circumvent that project.
use std::mem::transmute;

use helix_core::{Rope, RopeSlice};
use imara_diff::{InternedInput, Interner};

use super::{MAX_DIFF_BYTES, MAX_DIFF_LINES};

/// A cache that stores the `lines` of a rope as a vector.
/// It allows safely reusing the allocation of the vec when updating the rope
pub(crate) struct InternedRopeLines {
    diff_base: Box<Rope>,
    doc: Box<Rope>,
    num_tokens_diff_base: u32,
    interned: InternedInput<RopeSlice<'static>>,
}

impl InternedRopeLines {
    pub fn new(diff_base: Rope, doc: Rope) -> InternedRopeLines {
        let mut res = InternedRopeLines {
            interned: InternedInput {
                before: Vec::with_capacity(diff_base.len_lines()),
                after: Vec::with_capacity(doc.len_lines()),
                interner: Interner::new(diff_base.len_lines() + doc.len_lines()),
            },
            diff_base: Box::new(diff_base),
            doc: Box::new(doc),
            // will be populated by update_diff_base_impl
            num_tokens_diff_base: 0,
        };
        res.update_diff_base_impl();
        res
    }

    pub fn doc(&self) -> Rope {
        Rope::clone(&*self.doc)
    }

    pub fn diff_base(&self) -> Rope {
        Rope::clone(&*self.diff_base)
    }

    /// Updates the `diff_base` and optionally the document if `doc` is not None
    pub fn update_diff_base(&mut self, diff_base: Rope, doc: Option<Rope>) {
        self.interned.clear();
        *self.diff_base = diff_base;
        if let Some(doc) = doc {
            *self.doc = doc
        }
        if !self.is_too_large() {
            self.update_diff_base_impl();
        }
    }

    /// Updates the `doc` without reinterning the `diff_base`, this function
    /// is therefore significantly faster than `update_diff_base` when only the document changes.
    pub fn update_doc(&mut self, doc: Rope) {
        // Safety: we clear any tokens that were added after
        // the interning of `self.diff_base` finished so
        // all lines that refer to `self.doc` have been purged.

        self.interned
            .interner
            .erase_tokens_after(self.num_tokens_diff_base.into());

        *self.doc = doc;
        if self.is_too_large() {
            self.interned.after.clear();
        } else {
            self.update_doc_impl();
        }
    }

    fn update_diff_base_impl(&mut self) {
        // Safety: This transmute is safe because it only transmutes a lifetime, which has no effect.
        // The backing storage for the RopeSlices referred to by the lifetime is stored in `self.diff_base`.
        // Therefore as long as `self.diff_base` is not dropped/replaced this memory remains valid.
        // `self.diff_base` is only changed in `self.update_diff_base`, which clears the interner.
        // When the interned lines are exposed to consumer in `self.diff_input`, the lifetime is bounded to a reference to self.
        // That means that on calls to update there exist no references to `self.interned`.
        let before = self
            .diff_base
            .lines()
            .map(|line: RopeSlice| -> RopeSlice<'static> { unsafe { transmute(line) } });
        self.interned.update_before(before);
        self.num_tokens_diff_base = self.interned.interner.num_tokens();
        // the has to be interned again because the interner was fully cleared
        self.update_doc_impl()
    }

    fn update_doc_impl(&mut self) {
        // Safety: This transmute is save because it only transmutes a lifetime, which has no effect.
        // The backing storage for the RopeSlices referred to by the lifetime is stored in `self.doc`.
        // Therefore as long as `self.doc` is not dropped/replaced this memory remains valid.
        // `self.doc` is only changed in `self.update_doc`, which clears the interner.
        // When the interned lines are exposed to consumer in `self.diff_input`, the lifetime is bounded to a reference to self.
        // That means that on calls to update there exist no references to `self.interned`.
        let after = self
            .doc
            .lines()
            .map(|line: RopeSlice| -> RopeSlice<'static> { unsafe { transmute(line) } });
        self.interned.update_after(after);
    }

    fn is_too_large(&self) -> bool {
        // bound both lines and bytes to avoid huge files with few (but huge) lines
        // or huge file with tiny lines. While this makes no difference to
        // diff itself (the diff performance only depends on the number of tokens)
        // the interning runtime depends mostly on filesize and is actually dominant
        // for large files
        self.doc.len_lines() > MAX_DIFF_LINES
            || self.diff_base.len_lines() > MAX_DIFF_LINES
            || self.doc.len_bytes() > MAX_DIFF_BYTES
            || self.diff_base.len_bytes() > MAX_DIFF_BYTES
    }

    /// Returns the `InternedInput` for performing the diff.
    /// If `diff_base` or `doc` is so large that performing a diff could slow the editor
    /// this function returns `None`.
    pub fn interned_lines(&self) -> Option<&InternedInput<RopeSlice<'_>>> {
        if self.is_too_large() {
            None
        } else {
            Some(&self.interned)
        }
    }
}
