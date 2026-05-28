use std::ops::Range;
use std::time::Instant;

use imara_diff::{Algorithm, Diff, Hunk, IndentHeuristic, IndentLevel, InternedInput};
use ropey::RopeSlice;

use crate::{ChangeSet, Rope, Tendril, Transaction};

struct ChangeSetBuilder<'a> {
    res: ChangeSet,
    after: RopeSlice<'a>,
    file: &'a InternedInput<RopeSlice<'a>>,
    current_hunk: InternedInput<char>,
    char_diff: Diff,
    pos: u32,
}

impl ChangeSetBuilder<'_> {
    fn process_hunk(&mut self, before: Range<u32>, after: Range<u32>) {
        let len = self.file.before[self.pos as usize..before.start as usize]
            .iter()
            .map(|&it| self.file.interner[it].len())
            .sum();
        self.res.retain(len);
        self.pos = before.end;

        // do not perform diffs on large hunks
        let len_before = before.end - before.start;
        let len_after = after.end - after.start;

        // Pure insertions/removals do not require a character diff.
        // Very large changes are ignored because their character diff is expensive to compute
        // TODO adjust heuristic to detect large changes?
        if len_before == 0
            || len_after == 0
            || len_after > 5 * len_before
            || 5 * len_after < len_before && len_before > 10
            || len_before + len_after > 200
        {
            let remove = self.file.before[before.start as usize..before.end as usize]
                .iter()
                .map(|&it| self.file.interner[it].len())
                .sum();
            self.res.delete(remove);
            let mut fragment = Tendril::new();
            if len_after > 500 {
                // copying a rope line by line is slower then copying the entire
                // rope. Use to_string for very large changes instead..
                if self.file.after.len() == after.end as usize {
                    if after.start == 0 {
                        fragment = self.after.to_string().into();
                    } else {
                        let start = self.after.line_to_byte_idx(after.start as usize, crate::LINE_TYPE);
                        fragment = self.after.slice(start..).to_string().into();
                    }
                } else if after.start == 0 {
                    let end = self.after.line_to_byte_idx(after.end as usize, crate::LINE_TYPE);
                    fragment = self.after.slice(..end).to_string().into();
                } else {
                    let start = self.after.line_to_byte_idx(after.start as usize, crate::LINE_TYPE);
                    let end = self.after.line_to_byte_idx(after.end as usize, crate::LINE_TYPE);
                    fragment = self.after.slice(start..end).to_string().into();
                }
            } else {
                for &line in &self.file.after[after.start as usize..after.end as usize] {
                    for chunk in self.file.interner[line].chunks() {
                        fragment.push_str(chunk)
                    }
                }
            };
            self.res.insert(fragment);
        } else {
            // for reasonably small hunks, generating a ChangeSet from char diff can save memory
            // TODO use a tokenizer (word diff?) for improved performance
            let hunk_before = self.file.before[before.start as usize..before.end as usize]
                .iter()
                .flat_map(|&it| self.file.interner[it].chars());
            let hunk_after = self.file.after[after.start as usize..after.end as usize]
                .iter()
                .flat_map(|&it| self.file.interner[it].chars());
            self.current_hunk.update_before(hunk_before);
            self.current_hunk.update_after(hunk_after);
            // the histogram heuristic does not work as well
            // for characters because the same characters often reoccur
            // use myer diff instead
            self.char_diff.compute_with(
                Algorithm::Myers,
                &self.current_hunk.before,
                &self.current_hunk.after,
                self.current_hunk.interner.num_tokens(),
            );
            // `before.start`/`end` are *char-token* positions in
            // `self.current_hunk.before`; the `ChangeSet` operates in bytes.
            // Translate every span by summing each char's `len_utf8`.
            let token_bytes = |range: std::ops::Range<u32>| -> usize {
                self.current_hunk.before[range.start as usize..range.end as usize]
                    .iter()
                    .map(|&token| self.current_hunk.interner[token].len_utf8())
                    .sum()
            };

            let mut pos = 0;
            for Hunk { before, after } in self.char_diff.hunks() {
                self.res.retain(token_bytes(pos..before.start));
                self.res.delete(token_bytes(before.start..before.end));
                pos = before.end;

                let res = self.current_hunk.after[after.start as usize..after.end as usize]
                    .iter()
                    .map(|&token| self.current_hunk.interner[token])
                    .collect();

                self.res.insert(res);
            }
            self.res
                .retain(token_bytes(pos..self.current_hunk.before.len() as u32));
            // reuse allocations
            self.current_hunk.clear();
        }
    }

    fn finish(mut self) -> ChangeSet {
        let len = self.file.before[self.pos as usize..]
            .iter()
            .map(|&it| self.file.interner[it].len())
            .sum();

        self.res.retain(len);
        self.res
    }
}

struct RopeLines<'a>(RopeSlice<'a>);

impl<'a> imara_diff::TokenSource for RopeLines<'a> {
    type Token = RopeSlice<'a>;
    type Tokenizer = ropey::iter::Lines<'a>;

    fn tokenize(&self) -> Self::Tokenizer {
        self.0.lines(crate::LINE_TYPE)
    }

    fn estimate_tokens(&self) -> u32 {
        // we can provide a perfect estimate which is very nice for performance
        self.0.len_lines(crate::LINE_TYPE) as u32
    }
}

/// Compares `old` and `new` to generate a [`Transaction`] describing
/// the steps required to get from `old` to `new`.
pub fn compare_ropes(before: &Rope, after: &Rope) -> Transaction {
    let start = Instant::now();
    let res = ChangeSet::with_capacity(32);
    let after = after.slice(..);
    let file = InternedInput::new(RopeLines(before.slice(..)), RopeLines(after));
    let mut builder = ChangeSetBuilder {
        res,
        file: &file,
        after,
        pos: 0,
        current_hunk: InternedInput::default(),
        char_diff: Diff::default(),
    };
    let mut diff = Diff::compute(Algorithm::Histogram, &file);
    diff.postprocess_with_heuristic(
        &file,
        IndentHeuristic::new(|token| IndentLevel::for_ascii_line(file.interner[token].bytes(), 4)),
    );
    for hunk in diff.hunks() {
        builder.process_hunk(hunk.before, hunk.after)
    }
    let res = builder.finish().into();

    log::debug!(
        "rope diff took {}s",
        Instant::now().duration_since(start).as_secs_f64()
    );
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity(a: &str, b: &str) {
        let mut old = Rope::from(a);
        let new = Rope::from(b);
        compare_ropes(&old, &new).apply(&mut old);
        assert_eq!(old, new);
    }

    quickcheck::quickcheck! {
        fn test_compare_ropes(a: String, b: String) -> bool {
            let mut old = Rope::from(a);
            let new = Rope::from(b);
            compare_ropes(&old, &new).apply(&mut old);
            old == new
        }
    }

    #[test]
    fn equal_files() {
        test_identity("foo", "foo");
    }

    #[test]
    fn trailing_newline() {
        test_identity("foo\n", "foo");
        test_identity("foo", "foo\n");
    }

    #[test]
    fn new_file() {
        test_identity("", "foo");
    }

    #[test]
    fn deleted_file() {
        test_identity("foo", "");
    }

    /// Regression: `process_hunk`'s inner char-diff branch builds a `ChangeSet`
    /// across CHAR-token positions but the ChangeSet operates in bytes. Tiny
    /// edits inside lines containing multi-byte chars used to silently produce
    /// a malformed transaction that fails to apply (or panics on apply).
    #[test]
    fn small_hunk_multibyte() {
        // Replace one char inside a line with multi-byte content.
        test_identity("héllo\nworld\n", "héxlo\nworld\n");
        // Insert a multi-byte char.
        test_identity("hello\n", "héllo\n");
        // Delete a multi-byte char.
        test_identity("héllo\n", "hllo\n");
        // CJK content with a single-char edit.
        test_identity("今日はいい\n", "今日はいい!\n");
        test_identity("今日はいい\n", "今日いい\n");
        // Edit straddling multi-byte chars.
        test_identity("aëbëc\n", "axëbëc\n");
    }
}
