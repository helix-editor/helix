//! This module contains the functionality toggle comments on lines over the selection
//! using the comment character defined in the user's `languages.toml`

use smallvec::SmallVec;

use crate::{
    syntax::config::BlockCommentToken, Change, Range, Rope, RopeSlice, Selection, Tendril,
    Transaction,
};
use helix_stdx::rope::RopeSliceExt;
use std::borrow::Cow;

pub const DEFAULT_COMMENT_TOKEN: &str = "#";

/// Returns the longest matching comment token of the given line (if it exists).
pub fn get_comment_token<'a, S: AsRef<str>>(
    text: RopeSlice,
    tokens: &'a [S],
    line_num: usize,
) -> Option<&'a str> {
    let line = text.line(line_num);
    let start = line.first_non_whitespace_char()?;

    tokens
        .iter()
        .map(AsRef::as_ref)
        .filter(|token| line.slice(start..).starts_with(token))
        .max_by_key(|token| token.len())
}

/// Given text, a comment token, and a set of line indices, returns the following:
/// - Whether the given lines should be considered commented
///     - If any of the lines are uncommented, all lines are considered as such.
/// - The lines to change for toggling comments
///     - This is all provided lines excluding blanks lines.
/// - The column of the comment tokens
///     - Column of existing tokens, if the lines are commented; column to place tokens at otherwise.
/// - The margin to the right of the comment tokens
///     - Defaults to `1`. If any existing comment token is not followed by a space, changes to `0`.
fn find_line_comment(
    token: &str,
    text: RopeSlice,
    lines: impl IntoIterator<Item = usize>,
) -> (bool, Vec<usize>, usize, usize) {
    let mut commented = true;
    let mut to_change = Vec::new();
    let mut min = usize::MAX; // minimum col for first_non_whitespace_char
    let mut margin = 1;
    let token_len = token.chars().count();

    for line in lines {
        let line_slice = text.line(line);
        if let Some(pos) = line_slice.first_non_whitespace_char() {
            let len = line_slice.len_chars();

            min = std::cmp::min(min, pos);

            // line can be shorter than pos + token len
            let fragment = Cow::from(line_slice.slice(pos..std::cmp::min(pos + token.len(), len)));

            // as soon as one of the non-blank lines doesn't have a comment, the whole block is
            // considered uncommented.
            if fragment != token {
                commented = false;
            }

            // determine margin of 0 or 1 for uncommenting; if any comment token is not followed by a space,
            // a margin of 0 is used for all lines.
            if !matches!(line_slice.get_char(pos + token_len), Some(c) if c == ' ') {
                margin = 0;
            }

            // blank lines don't get pushed.
            to_change.push(line);
        }
    }

    (commented, to_change, min, margin)
}

#[must_use]
pub fn toggle_line_comments(doc: &Rope, selection: &Selection, token: Option<&str>) -> Transaction {
    let text = doc.slice(..);

    let token = token.unwrap_or(DEFAULT_COMMENT_TOKEN);
    let comment = Tendril::from(format!("{} ", token));

    let mut lines: Vec<usize> = Vec::with_capacity(selection.len());

    let mut min_next_line = 0;
    for selection in selection {
        let (start, end) = selection.line_range(text);
        let start = start.clamp(min_next_line, text.len_lines());
        let end = (end + 1).min(text.len_lines());

        lines.extend(start..end);
        min_next_line = end;
    }

    let (commented, to_change, min, margin) = find_line_comment(token, text, lines);

    let mut changes: Vec<Change> = Vec::with_capacity(to_change.len());

    for line in to_change {
        let pos = text.line_to_char(line) + min;

        if !commented {
            // comment line
            changes.push((pos, pos, Some(comment.clone())));
        } else {
            // uncomment line
            changes.push((pos, pos + token.len() + margin, None));
        }
    }

    Transaction::change(doc, changes.into_iter())
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommentChange {
    Commented {
        range: Range,
        start_pos: usize,
        end_pos: usize,
        start_margin: bool,
        end_margin: bool,
        start_token: String,
        end_token: String,
    },
    Uncommented {
        range: Range,
        start_pos: usize,
        end_pos: usize,
        start_token: String,
        end_token: String,
    },
    Whitespace {
        range: Range,
    },
}

pub fn find_block_comments(
    tokens: &[BlockCommentToken],
    text: RopeSlice,
    selection: &Selection,
) -> (bool, Vec<CommentChange>) {
    let mut commented = true;
    let mut only_whitespace = true;
    let mut comment_changes = Vec::with_capacity(selection.len());
    let default_tokens = tokens.first().cloned().unwrap_or_default();
    let mut start_token = default_tokens.start.clone();
    let mut end_token = default_tokens.end.clone();

    let mut tokens = tokens.to_vec();
    // sort the tokens by length, so longer tokens will match first
    tokens.sort_by(|a, b| {
        if a.start.len() == b.start.len() {
            b.end.len().cmp(&a.end.len())
        } else {
            b.start.len().cmp(&a.start.len())
        }
    });
    for range in selection {
        let selection_slice = range.slice(text);
        if let (Some(start_pos), Some(end_pos)) = (
            selection_slice.first_non_whitespace_char(),
            selection_slice.last_non_whitespace_char(),
        ) {
            let mut line_commented = false;
            let mut after_start = 0;
            let mut before_end = 0;
            let len = (end_pos + 1) - start_pos;

            for BlockCommentToken { start, end } in &tokens {
                let start_len = start.chars().count();
                let end_len = end.chars().count();
                after_start = start_pos + start_len;
                before_end = end_pos.saturating_sub(end_len);

                if len >= start_len + end_len {
                    let start_fragment = selection_slice.slice(start_pos..after_start);
                    let end_fragment = selection_slice.slice(before_end + 1..end_pos + 1);

                    // block commented with these tokens
                    if start_fragment == start.as_str() && end_fragment == end.as_str() {
                        start_token = start.to_string();
                        end_token = end.to_string();
                        line_commented = true;
                        break;
                    }
                }
            }

            if !line_commented {
                comment_changes.push(CommentChange::Uncommented {
                    range: *range,
                    start_pos,
                    end_pos,
                    start_token: default_tokens.start.clone(),
                    end_token: default_tokens.end.clone(),
                });
                commented = false;
            } else {
                comment_changes.push(CommentChange::Commented {
                    range: *range,
                    start_pos,
                    end_pos,
                    start_margin: selection_slice.get_char(after_start) == Some(' '),
                    end_margin: after_start != before_end
                        && (selection_slice.get_char(before_end) == Some(' ')),
                    start_token: start_token.to_string(),
                    end_token: end_token.to_string(),
                });
            }
            only_whitespace = false;
        } else {
            comment_changes.push(CommentChange::Whitespace { range: *range });
        }
    }
    if only_whitespace {
        commented = false;
    }
    (commented, comment_changes)
}

#[must_use]
pub fn create_block_comment_transaction(
    doc: &Rope,
    selection: &Selection,
    commented: bool,
    comment_changes: Vec<CommentChange>,
) -> (Transaction, SmallVec<[Range; 1]>) {
    let mut changes: Vec<Change> = Vec::with_capacity(selection.len() * 2);
    let mut ranges: SmallVec<[Range; 1]> = SmallVec::with_capacity(selection.len());
    let mut offs = 0;
    for change in comment_changes {
        if commented {
            if let CommentChange::Commented {
                range,
                start_pos,
                end_pos,
                start_token,
                end_token,
                start_margin,
                end_margin,
            } = change
            {
                let from = range.from();
                changes.push((
                    from + start_pos,
                    from + start_pos + start_token.len() + start_margin as usize,
                    None,
                ));
                changes.push((
                    from + end_pos - end_token.len() - end_margin as usize + 1,
                    from + end_pos + 1,
                    None,
                ));
            }
        } else {
            // uncommented so manually map ranges through changes
            match change {
                CommentChange::Uncommented {
                    range,
                    start_pos,
                    end_pos,
                    start_token,
                    end_token,
                } => {
                    let from = range.from();
                    changes.push((
                        from + start_pos,
                        from + start_pos,
                        Some(Tendril::from(format!("{} ", start_token))),
                    ));
                    changes.push((
                        from + end_pos + 1,
                        from + end_pos + 1,
                        Some(Tendril::from(format!(" {}", end_token))),
                    ));

                    let offset = start_token.chars().count() + end_token.chars().count() + 2;
                    ranges.push(
                        Range::new(from + offs, from + offs + end_pos + 1 + offset)
                            .with_direction(range.direction()),
                    );
                    offs += offset;
                }
                CommentChange::Commented { range, .. } | CommentChange::Whitespace { range } => {
                    ranges.push(Range::new(range.from() + offs, range.to() + offs));
                }
            }
        }
    }
    (Transaction::change(doc, changes.into_iter()), ranges)
}

#[must_use]
pub fn toggle_block_comments(
    doc: &Rope,
    selection: &Selection,
    tokens: &[BlockCommentToken],
) -> Transaction {
    let text = doc.slice(..);
    let (commented, comment_changes) = find_block_comments(tokens, text, selection);
    let (mut transaction, ranges) =
        create_block_comment_transaction(doc, selection, commented, comment_changes);
    if !commented {
        transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));
    }
    transaction
}

pub fn split_lines_of_selection(text: RopeSlice, selection: &Selection) -> Selection {
    let mut ranges = SmallVec::new();
    for range in selection.ranges() {
        let (line_start, line_end) = range.line_range(text.slice(..));
        let mut pos = text.line_to_char(line_start);
        for line in text.slice(pos..text.line_to_char(line_end + 1)).lines() {
            let start = pos;
            pos += line.len_chars();
            ranges.push(Range::new(start, pos));
        }
    }
    Selection::new(ranges, 0)
}

#[cfg(test)]
mod test {
    use super::*;

    mod find_line_comment {
        use super::*;

        #[test]
        fn not_commented() {
            // four lines, two space indented, except for line 1 which is blank.
            let doc = Rope::from("  1\n\n  2\n  3");

            let text = doc.slice(..);

            let res = find_line_comment("//", text, 0..3);
            // (commented = false, to_change = [line 0, line 2], min = col 2, margin = 0)
            assert_eq!(res, (false, vec![0, 2], 2, 0));
        }

        #[test]
        fn is_commented() {
            // three lines where the second line is empty.
            let doc = Rope::from("// hello\n\n// there");

            let res = find_line_comment("//", doc.slice(..), 0..3);

            // (commented = true, to_change = [line 0, line 2], min = col 0, margin = 1)
            assert_eq!(res, (true, vec![0, 2], 0, 1));
        }
    }

    // TODO: account for uncommenting with uneven comment indentation
    mod toggle_line_comment {
        use super::*;

        #[test]
        fn comment() {
            // four lines, two space indented, except for line 1 which is blank.
            let mut doc = Rope::from("  1\n\n  2\n  3");
            // select whole document
            let selection = Selection::single(0, doc.len_chars() - 1);

            let transaction = toggle_line_comments(&doc, &selection, None);
            transaction.apply(&mut doc);

            assert_eq!(doc, "  # 1\n\n  # 2\n  # 3");
        }

        #[test]
        fn uncomment() {
            let mut doc = Rope::from("  # 1\n\n  # 2\n  # 3");
            let mut selection = Selection::single(0, doc.len_chars() - 1);

            let transaction = toggle_line_comments(&doc, &selection, None);
            transaction.apply(&mut doc);
            selection = selection.map(transaction.changes());

            assert_eq!(doc, "  1\n\n  2\n  3");
            assert!(selection.len() == 1); // to ignore the selection unused warning
        }

        #[test]
        fn uncomment_0_margin_comments() {
            let mut doc = Rope::from("  #1\n\n  #2\n  #3");
            let mut selection = Selection::single(0, doc.len_chars() - 1);

            let transaction = toggle_line_comments(&doc, &selection, None);
            transaction.apply(&mut doc);
            selection = selection.map(transaction.changes());

            assert_eq!(doc, "  1\n\n  2\n  3");
            assert!(selection.len() == 1); // to ignore the selection unused warning
        }

        #[test]
        fn uncomment_0_margin_comments_with_no_space() {
            let mut doc = Rope::from("#");
            let mut selection = Selection::single(0, doc.len_chars() - 1);

            let transaction = toggle_line_comments(&doc, &selection, None);
            transaction.apply(&mut doc);
            selection = selection.map(transaction.changes());
            assert_eq!(doc, "");
            assert!(selection.len() == 1); // to ignore the selection unused warning
        }
    }

    #[test]
    fn test_find_block_comments() {
        // three lines 5 characters.
        let mut doc = Rope::from("1\n2\n3");
        // select whole document
        let selection = Selection::single(0, doc.len_chars());

        let text = doc.slice(..);

        let res = find_block_comments(&[BlockCommentToken::default()], text, &selection);

        assert_eq!(
            res,
            (
                false,
                vec![CommentChange::Uncommented {
                    range: Range::new(0, 5),
                    start_pos: 0,
                    end_pos: 4,
                    start_token: "/*".to_string(),
                    end_token: "*/".to_string(),
                }]
            )
        );

        // comment
        let transaction = toggle_block_comments(&doc, &selection, &[BlockCommentToken::default()]);
        transaction.apply(&mut doc);

        assert_eq!(doc, "/* 1\n2\n3 */");

        // uncomment
        let selection = Selection::single(0, doc.len_chars());
        let transaction = toggle_block_comments(&doc, &selection, &[BlockCommentToken::default()]);
        transaction.apply(&mut doc);
        assert_eq!(doc, "1\n2\n3");

        // don't panic when there is just a space in comment
        doc = Rope::from("/* */");
        let selection = Selection::single(0, doc.len_chars());
        let transaction = toggle_block_comments(&doc, &selection, &[BlockCommentToken::default()]);
        transaction.apply(&mut doc);
        assert_eq!(doc, "");
    }

    /// Test, if `get_comment_tokens` works, even if the content of the file includes chars, whose
    /// byte size unequal the amount of chars
    #[test]
    fn test_get_comment_with_char_boundaries() {
        let rope = Rope::from("··");
        let tokens = ["//", "///"];

        assert_eq!(
            super::get_comment_token(rope.slice(..), tokens.as_slice(), 0),
            None
        );
    }

    /// Test for `get_comment_token`.
    ///
    /// Assuming the comment tokens are stored as `["///", "//"]`, `get_comment_token` should still
    /// return `///` instead of `//` if the user is in a doc-comment section.
    #[test]
    fn test_use_longest_comment() {
        let text = Rope::from("    /// amogus");
        let tokens = ["///", "//"];

        assert_eq!(
            super::get_comment_token(text.slice(..), tokens.as_slice(), 0),
            Some("///")
        );
    }
}
