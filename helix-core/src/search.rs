use crate::RopeSlice;

pub fn find_nth_next(text: RopeSlice, ch: char, mut pos: usize, n: usize) -> Option<usize> {
    if pos >= text.len_chars() || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos);

    for _ in 0..n {
        loop {
            let c = chars.next()?;

            pos += 1;

            if c == ch {
                break;
            }
        }
    }

    Some(pos - 1)
}

pub fn find_nth_prev(text: RopeSlice, ch: char, mut pos: usize, n: usize) -> Option<usize> {
    if pos == 0 || n == 0 {
        return None;
    }

    let mut chars = text.chars_at(pos);

    for _ in 0..n {
        loop {
            let c = chars.prev()?;

            pos -= 1;

            if c == ch {
                break;
            }
        }
    }

    Some(pos)
}

use crate::movement::Direction;
use regex_automata::{dense, DenseDFA, Error as RegexError, DFA};
use std::ops::Range;

pub struct Searcher {
    /// Locate end of match searching right.
    right_fdfa: DenseDFA<Vec<usize>, usize>,
    /// Locate start of match searching right.
    right_rdfa: DenseDFA<Vec<usize>, usize>,

    /// Locate start of match searching left.
    left_fdfa: DenseDFA<Vec<usize>, usize>,
    /// Locate end of match searching left.
    left_rdfa: DenseDFA<Vec<usize>, usize>,
}

impl Searcher {
    pub fn new(pattern: &str) -> Result<Searcher, RegexError> {
        // Check case info for smart case
        let has_uppercase = pattern.chars().any(|c| c.is_uppercase());

        // Create Regex DFAs for all search directions.
        let mut builder = dense::Builder::new();
        let builder = builder.case_insensitive(!has_uppercase);

        let left_fdfa = builder.clone().reverse(true).build(pattern)?;
        let left_rdfa = builder
            .clone()
            .anchored(true)
            .longest_match(true)
            .build(pattern)?;

        let right_fdfa = builder.clone().build(pattern)?;
        let right_rdfa = builder
            .anchored(true)
            .longest_match(true)
            .reverse(true)
            .build(pattern)?;

        Ok(Searcher {
            right_fdfa,
            right_rdfa,
            left_fdfa,
            left_rdfa,
        })
    }
    pub fn search_prev(&self, text: RopeSlice, offset: usize) -> Option<Range<usize>> {
        let text = text.slice(..offset);
        let start = self.rfind(text, &self.left_fdfa)?;
        let end = self.find(text.slice(start..), &self.left_rdfa)?;

        Some(start..start + end)
    }

    pub fn search_next(&self, text: RopeSlice, offset: usize) -> Option<Range<usize>> {
        let text = text.slice(offset..);
        let end = self.find(text, &self.right_fdfa)?;
        let start = self.rfind(text.slice(..end), &self.right_rdfa)?;

        Some(offset + start..offset + end)
    }

    /// Returns the end offset of the longest match. If no match exists, then None is returned.
    /// NOTE: based on DFA::find_at
    fn find(&self, text: RopeSlice, dfa: &impl DFA) -> Option<usize> {
        // TODO: check this inside main search
        // if dfa.is_anchored() && start > 0 {
        //     return None;
        // }

        let mut state = dfa.start_state();
        let mut last_match = if dfa.is_dead_state(state) {
            return None;
        } else if dfa.is_match_state(state) {
            Some(0)
        } else {
            None
        };

        let mut chunk_byte_offset = 0;

        for chunk in text.chunks() {
            for (i, &b) in chunk.as_bytes().iter().enumerate() {
                state = unsafe { dfa.next_state_unchecked(state, b) };
                if dfa.is_match_or_dead_state(state) {
                    if dfa.is_dead_state(state) {
                        return last_match;
                    }
                    last_match = Some(chunk_byte_offset + i + 1);
                }
            }
            chunk_byte_offset += chunk.len();
        }

        last_match
    }

    /// Returns the start offset of the longest match in reverse, by searching from the end of the
    /// input towards the start of the input. If no match exists, then None is returned. In other
    /// words, this has the same match semantics as find, but in reverse.
    ///
    /// NOTE: based on DFA::rfind_at
    fn rfind(&self, text: RopeSlice, dfa: &impl DFA) -> Option<usize> {
        // if dfa.is_anchored() && start < bytes.len() {
        //     return None;
        // }

        let mut state = dfa.start_state();
        let mut last_match = if dfa.is_dead_state(state) {
            return None;
        } else if dfa.is_match_state(state) {
            Some(text.len_bytes())
        } else {
            None
        };

        // This is basically chunks().rev()
        let (mut chunks, mut chunk_byte_offset, _, _) = text.chunks_at_byte(text.len_bytes());

        while let Some(chunk) = chunks.prev() {
            for (i, &b) in chunk.as_bytes().iter().rev().enumerate() {
                state = unsafe { dfa.next_state_unchecked(state, b) };
                if dfa.is_match_or_dead_state(state) {
                    if dfa.is_dead_state(state) {
                        return last_match;
                    }
                    last_match = Some(chunk_byte_offset - i - 1);
                }
            }
            chunk_byte_offset -= chunk.len();
        }
        last_match
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_search_next() {
        use crate::Rope;
        let text = Rope::from("hello world!");

        let searcher = Searcher::new(r"\w+").unwrap();

        let result = searcher.search_next(text.slice(..), 0).unwrap();
        let fragment = text.slice(result.start..result.end);
        assert_eq!("hello", fragment);

        let result = searcher.search_next(text.slice(..), result.end).unwrap();
        let fragment = text.slice(result.start..result.end);
        assert_eq!("world", fragment);

        let result = searcher.search_next(text.slice(..), result.end);
        assert!(result.is_none());
    }

    #[test]
    fn test_search_prev() {
        use crate::Rope;
        let text = Rope::from("hello world!");

        let searcher = Searcher::new(r"\w+").unwrap();

        let result = searcher
            .search_prev(text.slice(..), text.len_bytes())
            .unwrap();
        let fragment = text.slice(result.start..result.end);
        assert_eq!("world", fragment);

        let result = searcher.search_prev(text.slice(..), result.start).unwrap();
        let fragment = text.slice(result.start..result.end);
        assert_eq!("hello", fragment);

        let result = searcher.search_prev(text.slice(..), result.start);
        assert!(result.is_none());
    }
}
