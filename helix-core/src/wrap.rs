use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;

use crate::graphemes::{grapheme_width, tab_width_at};
use crate::line_ending::line_without_line_ending;
use crate::{Change, RopeSlice, Tendril};

#[derive(Debug, Clone, Copy)]
pub struct ReflowOptions<'a> {
    pub text_width: usize,
    pub tab_width: usize,
    pub line_ending: &'a str,
    pub comment_tokens: &'a [&'a str],
}

/// A word of a paragraph, with document char positions.
struct Word {
    start: usize,
    end: usize,
    width: usize,
}

fn display_width(s: &str, tab_width: usize) -> usize {
    s.graphemes(true).fold(0, |col, g| {
        col + if g == "\t" {
            tab_width_at(col, tab_width as u16)
        } else {
            grapheme_width(g)
        }
    })
}

/// Blank lines and lines with only a comment token separate paragraphs.
fn is_separator(line: &str, comment_tokens: &[&str]) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || comment_tokens.contains(&trimmed)
}

/// The longest comment token the line starts with.
fn comment_token<'a>(line: &str, comment_tokens: &[&'a str]) -> Option<&'a str> {
    let line = line.trim_start();
    comment_tokens
        .iter()
        .copied()
        .filter(|token| line.starts_with(token))
        .max_by_key(|token| token.len())
}

/// The words of a paragraph. Indentation and comment tokens belong to the gaps between words.
fn collect_words(
    text: RopeSlice,
    start_line: usize,
    end_line: usize,
    opts: &ReflowOptions,
) -> Vec<Word> {
    let mut words = Vec::new();
    for line in start_line..=end_line {
        let line_start = text.line_to_char(line);
        let content = Cow::from(line_without_line_ending(&text, line));
        let trimmed = content.trim_start();
        let body = content.len() - trimmed.len()
            + comment_token(&content, opts.comment_tokens).map_or(0, str::len);

        let mut word = None;
        let mut push_word = |start: (usize, usize), end: (usize, usize)| {
            let width = display_width(&content[start.1..end.1], opts.tab_width);
            words.push(Word {
                start: line_start + start.0,
                end: line_start + end.0,
                width,
            });
        };
        let mut char_idx = 0;
        for (byte, ch) in content.char_indices() {
            if byte >= body && !ch.is_whitespace() {
                word.get_or_insert((char_idx, byte));
            } else if let Some(start) = word.take() {
                push_word(start, (char_idx, byte));
            }
            char_idx += 1;
        }
        if let Some(start) = word {
            push_word(start, (char_idx, content.len()));
        }
    }
    words
}

/// Re-wraps the paragraphs in the given lines to `opts.text_width`, like vim's `gq`.
///
/// Lines are filled greedily, which makes reflowing idempotent. Only the whitespace between
/// words is changed: it either becomes a space or a line break followed by the first line's
/// indentation and comment token.
pub fn reflow(
    text: RopeSlice,
    start_line: usize,
    end_line: usize,
    opts: &ReflowOptions,
) -> Vec<Change> {
    let is_separator = |line: usize| {
        is_separator(
            &Cow::from(line_without_line_ending(&text, line)),
            opts.comment_tokens,
        )
    };

    let mut changes = Vec::new();
    let mut line = start_line;
    while line <= end_line {
        if is_separator(line) {
            line += 1;
            continue;
        }
        let paragraph_start = line;
        while line <= end_line && !is_separator(line) {
            line += 1;
        }

        let words = collect_words(text, paragraph_start, line - 1, opts);
        let Some(first_word) = words.first() else {
            continue;
        };
        let first_line = Cow::from(line_without_line_ending(&text, paragraph_start));
        let indent = &first_line[..first_line.len() - first_line.trim_start().len()];
        let prefix = match comment_token(&first_line, opts.comment_tokens) {
            Some(token) => format!("{indent}{token} "),
            None => indent.to_string(),
        };
        let prefix_width = display_width(&prefix, opts.tab_width);
        let line_break = format!("{}{prefix}", opts.line_ending);

        let mut col = prefix_width + first_word.width;
        for pair in words.windows(2) {
            let (prev, word) = (&pair[0], &pair[1]);
            let gap = if col + 1 + word.width > opts.text_width {
                col = prefix_width + word.width;
                line_break.as_str()
            } else {
                col += 1 + word.width;
                " "
            };
            if text.slice(prev.end..word.start) != gap {
                changes.push((prev.end, word.start, Some(Tendril::from(gap))));
            }
        }
    }
    changes
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Rope, Transaction};

    fn opts(text_width: usize) -> ReflowOptions<'static> {
        comment_opts(text_width, &[])
    }

    fn comment_opts(text_width: usize, tokens: &'static [&'static str]) -> ReflowOptions<'static> {
        ReflowOptions {
            text_width,
            tab_width: 4,
            line_ending: "\n",
            comment_tokens: tokens,
        }
    }

    fn reflow_all(rope: &Rope, opts: &ReflowOptions) -> Vec<Change> {
        reflow(rope.slice(..), 0, rope.len_lines() - 1, opts)
    }

    fn reflowed(text: &str, opts: &ReflowOptions) -> String {
        let mut rope = Rope::from(text);
        let changes = reflow_all(&rope, opts);
        let tx = Transaction::change(&rope, changes.into_iter());
        assert!(tx.apply(&mut rope), "transaction failed to apply");
        rope.to_string()
    }

    #[test]
    fn wraps_plain_prose_greedily() {
        let text = "the quick brown fox jumps over the lazy dog";
        assert_eq!(
            reflowed(text, &opts(20)),
            "the quick brown fox\njumps over the lazy\ndog"
        );
    }

    #[test]
    fn edits_only_the_broken_gaps() {
        let text = "the quick brown fox jumps over the lazy dog";
        let rope = Rope::from(text);
        let changes = reflow_all(&rope, &opts(20));
        assert_eq!(changes.len(), 2, "expected two break edits: {changes:?}");
        for (from, to, tendril) in &changes {
            assert_eq!(*to - *from, 1, "should replace a single space");
            assert_eq!(rope.slice(*from..*to), " ");
            assert_eq!(tendril.as_deref(), Some("\n"));
        }
    }

    /// #3734: greedy wrapping is idempotent, a second pass makes no changes.
    #[test]
    fn is_idempotent() {
        let text = "the quick brown fox jumps over the lazy dog and then keeps running";
        for width in [10, 17, 20, 33, 40] {
            let once = reflowed(text, &opts(width));
            let rope = Rope::from(once.as_str());
            let second = reflow_all(&rope, &opts(width));
            assert!(
                second.is_empty(),
                "not idempotent at width {width}: {second:?}"
            );
        }
    }

    /// #3332 / #14789: a single comment line wrapped to many keeps its marker.
    #[test]
    fn preserves_comment_marker_single_to_multi() {
        let text = "// this is a long comment that will need to be wrapped onto lines";
        assert_eq!(
            reflowed(text, &comment_opts(30, &["///", "//"])),
            "// this is a long comment that\n// will need to be wrapped\n// onto lines"
        );
    }

    /// #14789: the longest matching marker (`///`) is used, not a prefix of it.
    #[test]
    fn uses_longest_comment_marker() {
        let text = "/// doc comment that should wrap and keep the triple slash marker";
        let got = reflowed(text, &comment_opts(30, &["//", "///"]));
        for line in got.lines() {
            assert!(line.starts_with("/// "), "line lost marker: {line:?}");
        }
        let rope = Rope::from(got.as_str());
        assert!(reflow_all(&rope, &comment_opts(30, &["//", "///"])).is_empty());
    }

    /// #3622: tab indentation is preserved and counted at its display width.
    #[test]
    fn tab_indented_comment() {
        let o = comment_opts(20, &["//"]);
        let got = reflowed("\t// alpha beta gamma delta", &o);
        for line in got.lines() {
            assert!(line.starts_with("\t// "), "lost tab+marker: {line:?}");
            assert!(display_width(line, 4) <= 20, "line too wide: {line:?}");
        }
        let rope = Rope::from(got.as_str());
        assert!(reflow_all(&rope, &o).is_empty());
    }

    /// #2419: blank lines separate paragraphs, which wrap independently.
    #[test]
    fn preserves_paragraph_breaks() {
        assert_eq!(
            reflowed("first paragraph here\n\nsecond paragraph here", &opts(80)),
            "first paragraph here\n\nsecond paragraph here"
        );
        assert_eq!(
            reflowed("aaa bbb ccc\n\nddd eee fff", &opts(7)),
            "aaa bbb\nccc\n\nddd eee\nfff"
        );
    }

    /// Reflow joins short lines within a paragraph and re-fills (vim `gq`).
    #[test]
    fn joins_and_refills_paragraph() {
        let text = "short\nlines\nthat\nshould\njoin";
        assert_eq!(reflowed(text, &opts(80)), "short lines that should join");
    }

    /// #2672: the configured line ending is used throughout.
    #[test]
    fn preserves_crlf_line_ending() {
        let o = ReflowOptions {
            line_ending: "\r\n",
            ..opts(20)
        };
        let got = reflowed("the quick brown fox jumps over the lazy dog\r\n", &o);
        assert_eq!(got, "the quick brown fox\r\njumps over the lazy\r\ndog\r\n");
    }

    #[test]
    fn long_word_is_not_split() {
        assert_eq!(
            reflowed("short superlongwordthatexceedswidth short", &opts(10)),
            "short\nsuperlongwordthatexceedswidth\nshort"
        );
    }

    #[test]
    fn trailing_newline_round_trips() {
        assert!(reflowed("hello world\n", &opts(80)).ends_with('\n'));
        assert!(!reflowed("hello world", &opts(80)).ends_with('\n'));
    }

    #[test]
    fn comment_internal_blank_line() {
        let text = "// first para of comment\n//\n// second para of comment";
        let rope = Rope::from(text);
        assert!(reflow_all(&rope, &comment_opts(80, &["//"])).is_empty());
    }
}
