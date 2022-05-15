//! This module contains the some comment-related features
//! using the comment character defined in the user's `languages.toml`:
//! * toggle comments on lines over the selection.
//! * continue comment when opening a new line.

use tree_sitter::QueryCursor;

use crate::{
    find_first_non_whitespace_char,
    syntax::{CapturedNode, LanguageConfiguration},
    Change, Range, Rope, RopeSlice, Selection, Syntax, Tendril, Transaction,
};
use std::borrow::Cow;

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
    let mut min = usize::MAX; // minimum col for find_first_non_whitespace_char
    let mut margin = 1;
    let token_len = token.chars().count();
    for line in lines {
        let line_slice = text.line(line);
        if let Some(pos) = find_first_non_whitespace_char(line_slice) {
            let len = line_slice.len_chars();

            if pos < min {
                min = pos;
            }

            // line can be shorter than pos + token len
            let fragment = Cow::from(line_slice.slice(pos..std::cmp::min(pos + token.len(), len)));

            if fragment != token {
                // as soon as one of the non-blank lines doesn't have a comment, the whole block is
                // considered uncommented.
                commented = false;
            }

            // determine margin of 0 or 1 for uncommenting; if any comment token is not followed by a space,
            // a margin of 0 is used for all lines.
            if matches!(line_slice.get_char(pos + token_len), Some(c) if c != ' ') {
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

    let token = token.unwrap_or("//");
    let comment = Tendril::from(format!("{} ", token));

    let mut lines: Vec<usize> = Vec::with_capacity(selection.len());

    let mut min_next_line = 0;
    for selection in selection {
        let (start, end) = selection.line_range(text);
        let start = start.max(min_next_line).min(text.len_lines());
        let end = (end + 1).min(text.len_lines());

        lines.extend(start..end);
        min_next_line = end + 1;
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

/// Return token if the current line is commented.
/// Otherwise, return None.
pub fn continue_comment<'a>(doc: &Rope, line: usize, tokens: &'a [String]) -> Option<&'a str> {
    // TODO: don't continue shebangs.
    if tokens.is_empty() {
        return None;
    }

    let mut result = None;
    let line_slice = doc.line(line);
    if let Some(pos) = find_first_non_whitespace_char(line_slice) {
        let len = line_slice.len_chars();
        for token in tokens {
            // line can be shorter than pos + token len
            let fragment = Cow::from(line_slice.slice(pos..std::cmp::min(pos + token.len(), len)));
            if fragment == *token {
                // Purposefully not break here to overwrite the result when a longer comment token
                // matches.
                result = Some(token.as_str());
            }
        }
    }

    result
}

pub fn continue_block_comment<'a>(
    doc: &Rope,
    syntax: Option<&Syntax>,
    lang_config: &'a LanguageConfiguration,
    range: &Range,
    open_below: bool,
) -> Option<&'a str> {
    if let Some((doc_syntax, block_comment_tokens)) =
        syntax.zip(lang_config.block_comment_tokens.as_ref())
    {
        let slice_tree = doc_syntax.tree().root_node();
        let slice = doc.slice(..);
        let line_pos = slice.char_to_line(range.cursor(slice));
        let mut cursor = QueryCursor::new();
        let mut found_block_comments = false;

        let should_insert_comment_middle = |node: CapturedNode| {
            let node_start = doc.byte_to_line(node.start_byte());
            let node_end = doc.byte_to_line(node.end_byte());
            // NOTE: we use line comparison to allow opening a comment on the newline after the
            // end of the block comment.
            // We also do not want to continue the comment if opening below when the cursor is
            // on the last line of the block comment.
            line_pos >= node_start && (line_pos < node_end || (line_pos == node_end && !open_below))
        };

        {
            let nodes = lang_config.textobject_query().and_then(|query| {
                query.capture_nodes_any(&["comment.block.around"], slice_tree, slice, &mut cursor)
            });
            if let Some(nodes) = nodes {
                for node in nodes {
                    found_block_comments = true;
                    if should_insert_comment_middle(node) {
                        return Some(&block_comment_tokens.middle);
                    }
                }
            }
        }

        // Many tree-sitter grammars don't contain a block comment token, so search the comment
        // token and check that it starts and ends with the correct block comment tokens.
        // FIXME: this doesn't take into account that a line comment followed by a block comment
        // counts as only one comment object.
        // TODO: Maybe that's caused by the + in the query?
        if !found_block_comments {
            let nodes = lang_config.textobject_query().and_then(|query| {
                query.capture_nodes_any(&["comment.around"], slice_tree, slice, &mut cursor)
            });
            if let Some(nodes) = nodes {
                for node in nodes {
                    let node_start = doc.byte_to_char(node.start_byte());
                    let node_end = doc.byte_to_char(node.end_byte());
                    let start_token_len = block_comment_tokens.start.len();
                    let end_token_len = block_comment_tokens.end.len();
                    if doc.len_chars() > node_start + start_token_len && doc.len_chars() > node_end
                    {
                        let comment_start = doc.slice(node_start..node_start + start_token_len);
                        let comment_end = doc.slice(node_end - end_token_len..node_end);
                        if comment_start == block_comment_tokens.start
                            && comment_end == block_comment_tokens.end
                        {
                            // We're in a block comment.
                            if should_insert_comment_middle(node) {
                                return Some(&block_comment_tokens.middle);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_line_comment() {
        use crate::State;

        // four lines, two space indented, except for line 1 which is blank.
        let doc = Rope::from("  1\n\n  2\n  3");

        let mut state = State::new(doc);
        // select whole document
        state.selection = Selection::single(0, state.doc.len_chars() - 1);

        let text = state.doc.slice(..);

        let res = find_line_comment("//", text, 0..3);
        // (commented = true, to_change = [line 0, line 2], min = col 2, margin = 1)
        assert_eq!(res, (false, vec![0, 2], 2, 1));

        // comment
        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.map(transaction.changes());

        assert_eq!(state.doc, "  // 1\n\n  // 2\n  // 3");

        // uncomment
        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.map(transaction.changes());
        assert_eq!(state.doc, "  1\n\n  2\n  3");

        // 0 margin comments
        state.doc = Rope::from("  //1\n\n  //2\n  //3");
        // reset the selection.
        state.selection = Selection::single(0, state.doc.len_chars() - 1);

        let transaction = toggle_line_comments(&state.doc, &state.selection, None);
        transaction.apply(&mut state.doc);
        state.selection = state.selection.map(transaction.changes());
        assert_eq!(state.doc, "  1\n\n  2\n  3");

        // TODO: account for uncommenting with uneven comment indentation
    }
}
