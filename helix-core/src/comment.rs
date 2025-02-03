//! This module contains the functionality toggle comments on lines over the selection
//! using the comment character defined in the user's `languages.toml`

use slotmap::DefaultKey as LayerId;
use smallvec::SmallVec;

use crate::{syntax::BlockCommentToken, Change, Range, Rope, RopeSlice, Syntax, Tendril};
use helix_stdx::rope::RopeSliceExt;
use std::borrow::Cow;

pub const DEFAULT_COMMENT_TOKEN: &str = "#";

/// Returns the longest matching comment token of the given line (if it exists).
pub fn get_comment_token(
    syntax: Option<&Syntax>,
    text: RopeSlice,
    doc_default_tokens: Option<&Vec<String>>,
    line_num: usize,
) -> Option<String> {
    let line = text.line(line_num);
    let start = line.first_non_whitespace_char()?;
    let start_char = text.line_to_char(line_num) + start;

    let injected_tokens = get_injected_tokens(syntax, start_char, start_char)
        // we only care about line comment tokens
        .0
        .and_then(|tokens| {
            tokens
                .into_iter()
                .filter(|token| line.slice(start..).starts_with(token))
                .max_by_key(|token| token.len())
        });

    injected_tokens.or(
        // no comment tokens found for injection, use doc comments if exists
        doc_default_tokens.and_then(|tokens| {
            tokens
                .iter()
                .filter(|token| line.slice(start..).starts_with(token))
                .max_by_key(|token| token.len())
                .cloned()
        }),
    )
}

pub fn get_injected_tokens(
    syntax: Option<&Syntax>,
    start: usize,
    end: usize,
) -> (Option<Vec<String>>, Option<Vec<BlockCommentToken>>) {
    // Find the injection with the most tightly encompassing range.
    syntax
        .and_then(|syntax| {
            injection_for_range(syntax, start, end).map(|language_id| {
                let config = syntax.layer_config(language_id);
                (
                    config.comment_tokens.clone(),
                    config.block_comment_tokens.clone(),
                )
            })
        })
        .unwrap_or_default()
}

/// For a given range in the document, get the most tightly encompassing
/// injection layer corresponding to that range.
pub fn injection_for_range(syntax: &Syntax, from: usize, to: usize) -> Option<LayerId> {
    let mut best_fit = None;
    let mut min_gap = usize::MAX;

    for (layer_id, layer) in &syntax.layers {
        for ts_range in &layer.ranges {
            let is_encompassing = ts_range.start_byte <= from && ts_range.end_byte >= to;
            if is_encompassing {
                let gap = ts_range.end_byte - ts_range.start_byte;
                let config = syntax.layer_config(layer_id);
                // ignore the language family for which it won't make
                // sense to consider their comment.
                //
                // This includes, for instance, `comment`, `jsdoc`, `regex`
                let has_comment_tokens =
                    config.comment_tokens.is_some() || config.block_comment_tokens.is_some();

                if gap < min_gap && has_comment_tokens {
                    best_fit = Some(layer_id);
                    min_gap = gap;
                }
            }
        }
    }

    best_fit
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
pub fn toggle_line_comments(doc: &Rope, range: &Range, token: Option<&str>) -> Vec<Change> {
    let text = doc.slice(..);

    let token = token.unwrap_or(DEFAULT_COMMENT_TOKEN);
    let comment = Tendril::from(format!("{} ", token));

    let start = text.char_to_line(range.from());
    let end = text.char_to_line(range.to().saturating_sub(1));
    let line_count = text.len_lines();
    let start = start.clamp(0, line_count);
    let end = (end + 1).min(line_count);

    let mut lines = vec![];
    lines.extend(start..end);

    let (was_commented, to_change, min, margin) = find_line_comment(token, text, lines);

    let mut changes: Vec<Change> = Vec::with_capacity(to_change.len());

    for line in to_change {
        let pos = text.line_to_char(line) + min;

        if !was_commented {
            // comment line
            changes.push((pos, pos, Some(comment.clone())));
        } else {
            // uncomment line
            changes.push((pos, pos + token.len() + margin, None));
        }
    }

    changes
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
    ranges: &Vec<Range>,
) -> (bool, Vec<CommentChange>) {
    let mut was_commented = true;
    let mut only_whitespace = true;
    let mut comment_changes = Vec::with_capacity(ranges.len());
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
    for range in ranges {
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
                was_commented = false;
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
        was_commented = false;
    }
    (was_commented, comment_changes)
}

#[must_use]
pub fn create_block_comment_transaction(
    ranges: &[Range],
    was_commented: bool,
    comment_changes: Vec<CommentChange>,
) -> (Vec<Change>, SmallVec<[Range; 1]>) {
    let mut changes: Vec<Change> = Vec::with_capacity(ranges.len() * 2);
    let mut ranges: SmallVec<[Range; 1]> = SmallVec::with_capacity(ranges.len());
    let mut offs = 0;
    for change in comment_changes {
        if was_commented {
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
                let keep_from = from + start_pos + start_token.len() + start_margin as usize;
                changes.push((from + start_pos, keep_from, None));
                let keep_until = from + end_pos - end_token.len() - end_margin as usize + 1;
                changes.push((keep_until, from + end_pos + 1, None));
                // The range of characters keep_from..keep_until remain in the document
                ranges.push(Range::new(keep_from, keep_until).with_direction(range.direction()));
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
    (changes, ranges)
}

#[must_use]
pub fn toggle_block_comments(
    doc: &Rope,
    ranges: &Vec<Range>,
    tokens: &[BlockCommentToken],
    selections: &mut SmallVec<[Range; 1]>,
    added_chars: &mut usize,
    removed_chars: &mut usize,
) -> Vec<Change> {
    let text = doc.slice(..);
    let (was_commented, comment_changes) = find_block_comments(tokens, text, ranges);
    let (changes, new_ranges) =
        create_block_comment_transaction(ranges, was_commented, comment_changes);

    if was_commented {
        for (range, changes) in new_ranges.iter().zip(changes.chunks_exact(2)) {
            // every 2 elements (from, to) in `changes` corresponds
            // the `from` - `to` represents the range of text that will be deleted.
            // to 1 element in `new_ranges`
            //
            // Left token:
            //
            // "<!-- "
            //  ^ left_from
            //       ^ left_to
            //
            // Right token:
            //
            // " -->"
            //  ^ right_from
            //      ^ right_to
            let [(left_from, left_to, _), (right_from, right_to, _)] = changes else {
                unreachable!()
            };

            *removed_chars += left_to - left_from;

            // We slide the range to the left by the amount of characters
            // we've deleted so far + the amount of chars deleted for
            // the left comment token of the current iteration
            selections.push(Range::new(
                range.anchor + *added_chars - *removed_chars,
                range.head + *added_chars - *removed_chars,
            ));

            *removed_chars += right_to - right_from;
        }

        changes
    } else {
        // we're never removing or
        // creating ranges. Only shifting / increasing size
        // of existing ranges to accomodate the newly added
        // comment tokens.
        //
        // when we add comment tokens, we want to extend our selection to
        // also include the added tokens.
        for (range, old_range) in new_ranges.iter().zip(ranges) {
            // Will not underflow because the new range must always be
            // at least the same size as the old range, since we're
            // adding comment token characters, never removing.
            let range = Range::new(
                range.anchor + *added_chars - *removed_chars,
                range.head + *added_chars - *removed_chars,
            );
            selections.push(range);
            *added_chars += range.len() - old_range.len();
        }

        changes
    }
}

pub fn split_lines_of_range(text: RopeSlice, range: &Range) -> Vec<Range> {
    let mut ranges = vec![];
    let (line_start, line_end) = range.line_range(text.slice(..));
    let mut pos = text.line_to_char(line_start);
    for line in text.slice(pos..text.line_to_char(line_end + 1)).lines() {
        let start = pos;
        pos += line.len_chars();
        ranges.push(Range::new(start, pos));
    }
    ranges
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
        use crate::Transaction;

        use super::*;

        #[test]
        fn comment() {
            // four lines, two space indented, except for line 1 which is blank.
            let mut doc = Rope::from("  1\n\n  2\n  3");
            // select whole document
            let range = Range::new(0, doc.len_chars() - 1);

            let changes = toggle_line_comments(&doc, &range, None);
            let transaction = Transaction::change(&doc, changes.into_iter());

            transaction.apply(&mut doc);

            assert_eq!(doc, "  # 1\n\n  # 2\n  # 3");
        }

        #[test]
        fn uncomment() {
            let mut doc = Rope::from("  # 1\n\n  # 2\n  # 3");
            let mut range = Range::new(0, doc.len_chars() - 1);

            let changes = toggle_line_comments(&doc, &range, None);
            let transaction = Transaction::change(&doc, changes.into_iter());
            transaction.apply(&mut doc);
            range = range.map(transaction.changes());

            assert_eq!(doc, "  1\n\n  2\n  3");
            assert_eq!(range, range); // to ignore the selection unused warning
        }

        #[test]
        fn uncomment_0_margin_comments() {
            let mut doc = Rope::from("  #1\n\n  #2\n  #3");
            let mut range = Range::new(0, doc.len_chars() - 1);

            let changes = toggle_line_comments(&doc, &range, None);
            let transaction = Transaction::change(&doc, changes.into_iter());
            transaction.apply(&mut doc);
            range = range.map(transaction.changes());

            assert_eq!(doc, "  1\n\n  2\n  3");
            assert_eq!(range, range); // to ignore the selection unused warning
        }

        #[test]
        fn uncomment_0_margin_comments_with_no_space() {
            let mut doc = Rope::from("#");
            let mut range = Range::new(0, doc.len_chars() - 1);

            let changes = toggle_line_comments(&doc, &range, None);
            let transaction = Transaction::change(&doc, changes.into_iter());
            transaction.apply(&mut doc);
            range = range.map(transaction.changes());
            assert_eq!(doc, "");
            assert_eq!(range, range); // to ignore the selection unused warning
        }

        #[test]
        fn test_find_block_comments() {
            // three lines 5 characters.
            let mut doc = Rope::from("1\n2\n3");
            // select whole document
            let range = Range::new(0, doc.len_chars());

            let text = doc.slice(..);

            let res = find_block_comments(&[BlockCommentToken::default()], text, &vec![range]);

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
            let changes = toggle_block_comments(
                &doc,
                &vec![range],
                &[BlockCommentToken::default()],
                &mut SmallVec::new(),
                &mut 0,
                &mut 0,
            );
            let transaction = Transaction::change(&doc, changes.into_iter());
            transaction.apply(&mut doc);

            assert_eq!(doc, "/* 1\n2\n3 */");

            // uncomment
            let range = Range::new(0, doc.len_chars());
            let changes = toggle_block_comments(
                &doc,
                &vec![range],
                &[BlockCommentToken::default()],
                &mut SmallVec::new(),
                &mut 0,
                &mut 0,
            );
            let transaction = Transaction::change(&doc, changes.into_iter());
            transaction.apply(&mut doc);
            assert_eq!(doc, "1\n2\n3");

            // don't panic when there is just a space in comment
            doc = Rope::from("/* */");
            let range = Range::new(0, doc.len_chars());
            let changes = toggle_block_comments(
                &doc,
                &vec![range],
                &[BlockCommentToken::default()],
                &mut SmallVec::new(),
                &mut 0,
                &mut 0,
            );
            let transaction = Transaction::change(&doc, changes.into_iter());
            transaction.apply(&mut doc);
            assert_eq!(doc, "");
        }

        /// Test, if `get_comment_tokens` works, even if the content of the file includes chars, whose
        /// byte size unequal the amount of chars
        #[test]
        fn test_get_comment_with_char_boundaries() {
            let rope = Rope::from("··");
            let tokens = vec!["//".to_owned(), "///".to_owned()];

            assert_eq!(
                super::get_comment_token(None, rope.slice(..), Some(&tokens), 0),
                None
            );
        }

        /// Test for `get_comment_token`.
        ///
        /// Assuming the comment tokens are stored as `["///", "//"]`, `get_comment_token` should still
        /// return `///` instead of `//` if the user is in a doc-comment section.
        #[test]
        fn test_use_longest_comment() {
            let text = Rope::from("    /// amogus ඞ");
            let tokens = vec!["///".to_owned(), "//".to_owned()];

            assert_eq!(
                super::get_comment_token(None, text.slice(..), Some(&tokens), 0),
                Some("///".to_owned())
            );
        }
    }
}
