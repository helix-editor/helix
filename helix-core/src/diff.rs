use std::ops::Range;
use std::time::Instant;

use imara_diff::intern::InternedInput;
use imara_diff::Algorithm;
use ropey::RopeSlice;

use crate::{ChangeSet, Rope, Tendril, Transaction};

/// A `imara_diff::Sink` that builds a `ChangeSet` for a character diff of a hunk
struct CharChangeSetBuilder<'a> {
    res: &'a mut ChangeSet,
    hunk: &'a InternedInput<char>,
    pos: u32,
}

impl imara_diff::Sink for CharChangeSetBuilder<'_> {
    type Out = ();
    fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
        self.res.retain((before.start - self.pos) as usize);
        self.res.delete(before.len());
        self.pos = before.end;

        let res = self.hunk.after[after.start as usize..after.end as usize]
            .iter()
            .map(|&token| self.hunk.interner[token])
            .collect();

        self.res.insert(res);
    }

    fn finish(self) -> Self::Out {
        self.res.retain(self.hunk.before.len() - self.pos as usize);
    }
}

struct LineChangeSetBuilder<'a> {
    res: ChangeSet,
    after: RopeSlice<'a>,
    file: &'a InternedInput<RopeSlice<'a>>,
    current_hunk: InternedInput<char>,
    pos: u32,
}

impl imara_diff::Sink for LineChangeSetBuilder<'_> {
    type Out = ChangeSet;

    fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
        let len = self.file.before[self.pos as usize..before.start as usize]
            .iter()
            .map(|&it| self.file.interner[it].len_chars())
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
                .map(|&it| self.file.interner[it].len_chars())
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
                        let start = self.after.line_to_char(after.start as usize);
                        fragment = self.after.slice(start..).to_string().into();
                    }
                } else if after.start == 0 {
                    let end = self.after.line_to_char(after.end as usize);
                    fragment = self.after.slice(..end).to_string().into();
                } else {
                    let start = self.after.line_to_char(after.start as usize);
                    let end = self.after.line_to_char(after.end as usize);
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
            imara_diff::diff(
                Algorithm::Myers,
                &self.current_hunk,
                CharChangeSetBuilder {
                    res: &mut self.res,
                    hunk: &self.current_hunk,
                    pos: 0,
                },
            );

            self.current_hunk.clear();
        }
    }

    fn finish(mut self) -> Self::Out {
        let len = self.file.before[self.pos as usize..]
            .iter()
            .map(|&it| self.file.interner[it].len_chars())
            .sum();

        self.res.retain(len);
        self.res
    }
}

struct RopeLines<'a>(RopeSlice<'a>);

impl<'a> imara_diff::intern::TokenSource for RopeLines<'a> {
    type Token = RopeSlice<'a>;
    type Tokenizer = ropey::iter::Lines<'a>;

    fn tokenize(&self) -> Self::Tokenizer {
        self.0.lines()
    }

    fn estimate_tokens(&self) -> u32 {
        // we can provide a perfect estimate which is very nice for performance
        self.0.len_lines() as u32
    }
}

/// Compares `old` and `new` to generate a [`Transaction`] describing
/// the steps required to get from `old` to `new`.
pub fn compare_ropes(before: &Rope, after: &Rope) -> Transaction {
    let start = Instant::now();
    let res = ChangeSet::with_capacity(32);
    let after = after.slice(..);
    let file = InternedInput::new(RopeLines(before.slice(..)), RopeLines(after));
    let builder = LineChangeSetBuilder {
        res,
        file: &file,
        after,
        pos: 0,
        current_hunk: InternedInput::default(),
    };

    let res = imara_diff::diff(Algorithm::Histogram, &file, builder).into();

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
}
