//! Computation of foldable regions (code folding).
//!
//! A fold is stored as a char range of the document that is hidden when
//! rendering. The range always starts at the line ending of the line that
//! opens the region (the "header" line, which stays visible) and ends either
//! at a closing delimiter on the last line of the region (so that the folded
//! region is displayed as `fn foo() {⋯}`) or at the line ending of the last
//! line of the region.

use std::ops::Range;

use helix_stdx::rope::RopeSliceExt;

use crate::line_ending::line_end_char_index;
use crate::syntax::Syntax;
use crate::RopeSlice;

/// Chars that close a block and are kept visible after the fold marker.
const CLOSING_DELIMITERS: &[char] = &['}', ')', ']'];

/// Returns the region that can be folded with `line` as its header line.
///
/// When a syntax tree is available the region is the largest syntax node that
/// starts on `line` and spans multiple lines. Otherwise the region is
/// determined from the indentation of the following lines.
pub fn foldable_range(
    text: RopeSlice,
    syntax: Option<&Syntax>,
    line: usize,
    tab_width: usize,
) -> Option<Range<usize>> {
    if line >= text.len_lines() {
        return None;
    }
    let end_line = match syntax {
        Some(syntax) => syntax_fold_end_line(text, syntax, line)?,
        None => indent_fold_end_line(text, line, tab_width)?,
    };
    fold_range_for_lines(text, line, end_line)
}

/// Returns the innermost foldable region that contains `line`
/// (either as its header line or as a hidden line).
pub fn enclosing_foldable_range(
    text: RopeSlice,
    syntax: Option<&Syntax>,
    line: usize,
    tab_width: usize,
) -> Option<Range<usize>> {
    // limit the search to keep this cheap for huge files
    const MAX_LINES_SEARCHED: usize = 10_000;
    let line = line.min(text.len_lines().saturating_sub(1));
    let first_line = line.saturating_sub(MAX_LINES_SEARCHED);
    (first_line..=line).rev().find_map(|header| {
        let range = foldable_range(text, syntax, header, tab_width)?;
        let end_line = text.char_to_line(range.end);
        (end_line >= line).then_some(range)
    })
}

/// Returns all foldable regions of the document sorted by their start.
pub fn all_foldable_ranges(
    text: RopeSlice,
    syntax: Option<&Syntax>,
    tab_width: usize,
) -> Vec<Range<usize>> {
    (0..text.len_lines())
        .filter_map(|line| foldable_range(text, syntax, line, tab_width))
        .collect()
}

/// Builds the char range hidden by a fold that starts at `start_line`
/// (which stays visible) and ends at `end_line`.
pub fn fold_range_for_lines(
    text: RopeSlice,
    start_line: usize,
    end_line: usize,
) -> Option<Range<usize>> {
    if end_line <= start_line || end_line >= text.len_lines() {
        return None;
    }
    let start = line_end_char_index(&text, start_line);
    let end_line_start = text.line_to_char(end_line);
    let end_line_text = text.line(end_line);
    let end = match end_line_text.first_non_whitespace_char() {
        Some(first) if CLOSING_DELIMITERS.contains(&end_line_text.char(first)) => {
            end_line_start + first
        }
        _ => line_end_char_index(&text, end_line),
    };
    (start < end).then_some(start..end)
}

fn syntax_fold_end_line(text: RopeSlice, syntax: &Syntax, line: usize) -> Option<usize> {
    let line_start = text.line_to_char(line);
    let line_text = text.line(line);
    let first = line_start + line_text.first_non_whitespace_char()?;
    let last = line_start + line_text.last_non_whitespace_char()?;

    let mut best_end_line = None;
    for pos in [first, last] {
        let start_byte = text.char_to_byte(pos) as u32;
        let end_byte = text.char_to_byte(pos + 1) as u32;
        let Some(mut node) = syntax.descendant_for_byte_range(start_byte, end_byte) else {
            continue;
        };
        // walk up the tree while the nodes still start on `line`. The root node
        // is excluded since folding the entire document is never useful.
        while let Some(parent) = node.parent() {
            if text.byte_to_line(node.start_byte() as usize) != line {
                break;
            }
            // `end_byte` is exclusive, some grammars include the trailing newline
            let last_byte = (node.end_byte() as usize).saturating_sub(1);
            let end_line = text.byte_to_line(last_byte.max(node.start_byte() as usize));
            if end_line > line {
                best_end_line = best_end_line.max(Some(end_line));
            }
            node = parent;
        }
    }
    best_end_line
}

/// The indentation width of `line` or `None` for blank lines.
fn indent_width(text: RopeSlice, line: usize, tab_width: usize) -> Option<usize> {
    let mut width = 0;
    for ch in text.line(line).chars() {
        match ch {
            ' ' => width += 1,
            '\t' => width += tab_width - width % tab_width.max(1),
            '\n' | '\r' => return None,
            c if c.is_whitespace() => width += 1,
            _ => return Some(width),
        }
    }
    None
}

fn indent_fold_end_line(text: RopeSlice, line: usize, tab_width: usize) -> Option<usize> {
    let indent = indent_width(text, line, tab_width)?;
    let mut last = None;
    for next in line + 1..text.len_lines() {
        match indent_width(text, next, tab_width) {
            None => continue,
            Some(width) if width > indent => last = Some(next),
            Some(_) => {
                // include a closing delimiter at the same indentation
                // (`}` of a C-like block) so that it is shown after the marker
                let next_text = text.line(next);
                let starts_with_closer = next_text
                    .first_non_whitespace_char()
                    .is_some_and(|first| CLOSING_DELIMITERS.contains(&next_text.char(first)));
                if last.is_some() && starts_with_closer {
                    last = Some(next);
                }
                break;
            }
        }
    }
    last
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn indent_folds() {
        let text = Rope::from("def foo():\n    a\n\n    b\nc\n");
        let text = text.slice(..);
        let range = foldable_range(text, None, 0, 4).unwrap();
        // hides everything from the end of the first line to the end of `b`
        assert_eq!(text.slice(range).to_string(), "\n    a\n\n    b");
        assert_eq!(foldable_range(text, None, 1, 4), None);
        assert_eq!(foldable_range(text, None, 4, 4), None);
    }

    #[test]
    fn indent_folds_keep_closing_delimiter() {
        let text = Rope::from("foo {\n  a\n  b\n}\n");
        let text = text.slice(..);
        let range = foldable_range(text, None, 0, 4).unwrap();
        assert_eq!(text.slice(range.clone()).to_string(), "\n  a\n  b\n");
        assert_eq!(text.char(range.end), '}');
    }

    #[test]
    fn enclosing() {
        let text = Rope::from("a\n  b\n    c\n  d\ne\n");
        let text = text.slice(..);
        let range = enclosing_foldable_range(text, None, 2, 4).unwrap();
        assert_eq!(text.char_to_line(range.start), 1);
        let range = enclosing_foldable_range(text, None, 3, 4).unwrap();
        assert_eq!(text.char_to_line(range.start), 0);
        assert_eq!(enclosing_foldable_range(text, None, 4, 4), None);
    }
}
