use std::iter;

use ropey::RopeSlice;
use tree_sitter::Node;

use crate::movement::Direction::{self, Backward, Forward};
use crate::Syntax;

const MAX_PLAINTEXT_SCAN: usize = 10000;
const MATCH_LIMIT: usize = 16;

// Limit matching pairs to only ( ) { } [ ] < > ' ' " "
const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('<', '>'),
    ('\'', '\''),
    ('\"', '\"'),
];

/// Returns the position of the matching bracket under cursor.
///
/// If the cursor is on the opening bracket, the position of
/// the closing bracket is returned. If the cursor on the closing
/// bracket, the position of the opening bracket is returned.
///
/// If the cursor is not on a bracket, `None` is returned.
///
/// If no matching bracket is found, `None` is returned.
#[must_use]
pub fn find_matching_bracket(syntax: &Syntax, doc: RopeSlice, pos: usize) -> Option<usize> {
    if pos >= doc.len_chars() || !is_valid_bracket(doc.char(pos)) {
        return None;
    }
    find_pair(syntax, doc, pos, false)
}

// Returns the position of the bracket that is closing the current scope.
//
// If the cursor is on an opening or closing bracket, the function
// behaves equivalent to [`find_matching_bracket`].
//
// If the cursor position is within a scope, the function searches
// for the surrounding scope that is surrounded by brackets and
// returns the position of the closing bracket for that scope.
//
// If no surrounding scope is found, the function returns `None`.
#[must_use]
pub fn find_matching_bracket_fuzzy(syntax: &Syntax, doc: RopeSlice, pos: usize) -> Option<usize> {
    find_pair(syntax, doc, pos, true)
}

fn find_pair(
    syntax: &Syntax,
    doc: RopeSlice,
    pos_: usize,
    traverse_parents: bool,
) -> Option<usize> {
    let tree = syntax.tree();
    let pos = doc.char_to_byte(pos_);

    let mut node = tree.root_node().descendant_for_byte_range(pos, pos)?;

    loop {
        if node.is_named() {
            let (start_byte, end_byte) = surrounding_bytes(doc, &node)?;
            let (start_char, end_char) = (doc.byte_to_char(start_byte), doc.byte_to_char(end_byte));

            if is_valid_pair(doc, start_char, end_char) {
                if end_byte == pos {
                    return Some(start_char);
                }

                // We return the end char if the cursor is either on the start char
                // or at some arbitrary position between start and end char.
                if traverse_parents || start_byte == pos {
                    return Some(end_char);
                }
            }
        }
        // this node itselt wasn't a pair but maybe its siblings are

        // check if we are *on* the pair (special cased so we don't look
        // at the current node twice and to jump to the start on that case)
        if let Some(open) = as_close_pair(doc, &node) {
            if let Some(pair_start) = find_pair_end(doc, node.prev_sibling(), open, Backward) {
                return Some(pair_start);
            }
        }

        if !traverse_parents {
            // check if we are *on* the opening pair (special cased here as
            // an opptimization since we only care about bracket on the cursor
            // here)
            if let Some(close) = as_open_pair(doc, &node) {
                if let Some(pair_end) = find_pair_end(doc, node.next_sibling(), close, Forward) {
                    return Some(pair_end);
                }
            }
            if node.is_named() {
                break;
            }
        }

        for close in
            iter::successors(node.next_sibling(), |node| node.next_sibling()).take(MATCH_LIMIT)
        {
            let Some(open) = as_close_pair(doc, &close) else {
                continue;
            };
            if find_pair_end(doc, Some(node), open, Backward).is_some() {
                return doc.try_byte_to_char(close.start_byte()).ok();
            }
        }
        let Some(parent) = node.parent() else {
            break;
        };
        node = parent;
    }
    let node = tree.root_node().named_descendant_for_byte_range(pos, pos)?;
    if node.child_count() != 0 {
        return None;
    }
    let node_start = doc.byte_to_char(node.start_byte());
    find_matching_bracket_plaintext(doc.byte_slice(node.byte_range()), pos_ - node_start)
        .map(|pos| pos + node_start)
}

/// Returns the position of the matching bracket under cursor.
/// This function works on plain text and ignores tree-sitter grammar.
/// The search is limited to `MAX_PLAINTEXT_SCAN` characters
///
/// If the cursor is on the opening bracket, the position of
/// the closing bracket is returned. If the cursor on the closing
/// bracket, the position of the opening bracket is returned.
///
/// If the cursor is not on a bracket, `None` is returned.
///
/// If no matching bracket is found, `None` is returned.
#[must_use]
pub fn find_matching_bracket_plaintext(doc: RopeSlice, cursor_pos: usize) -> Option<usize> {
    // Don't do anything when the cursor is not on top of a bracket.
    let bracket = doc.get_char(cursor_pos)?;
    if !is_valid_bracket(bracket) {
        return None;
    }

    // Determine the direction of the matching.
    let is_fwd = is_forward_bracket(bracket);
    let chars_iter = if is_fwd {
        doc.chars_at(cursor_pos + 1)
    } else {
        doc.chars_at(cursor_pos).reversed()
    };

    let mut open_cnt = 1;

    for (i, candidate) in chars_iter.take(MAX_PLAINTEXT_SCAN).enumerate() {
        if candidate == bracket {
            open_cnt += 1;
        } else if is_valid_pair(
            doc,
            if is_fwd {
                cursor_pos
            } else {
                cursor_pos - i - 1
            },
            if is_fwd {
                cursor_pos + i + 1
            } else {
                cursor_pos
            },
        ) {
            // Return when all pending brackets have been closed.
            if open_cnt == 1 {
                return Some(if is_fwd {
                    cursor_pos + i + 1
                } else {
                    cursor_pos - i - 1
                });
            }
            open_cnt -= 1;
        }
    }

    None
}

fn is_valid_bracket(c: char) -> bool {
    PAIRS.iter().any(|(l, r)| *l == c || *r == c)
}

fn is_forward_bracket(c: char) -> bool {
    PAIRS.iter().any(|(l, _)| *l == c)
}

fn is_valid_pair(doc: RopeSlice, start_char: usize, end_char: usize) -> bool {
    PAIRS.contains(&(doc.char(start_char), doc.char(end_char)))
}

fn surrounding_bytes(doc: RopeSlice, node: &Node) -> Option<(usize, usize)> {
    let len = doc.len_bytes();

    let start_byte = node.start_byte();
    let end_byte = node.end_byte().saturating_sub(1);

    if start_byte >= len || end_byte >= len {
        return None;
    }

    Some((start_byte, end_byte))
}

/// Tests if this node is a pair close char and returns the expected open char
fn as_close_pair(doc: RopeSlice, node: &Node) -> Option<char> {
    let close = as_char(doc, node)?.1;
    PAIRS
        .iter()
        .find_map(|&(open, close_)| (close_ == close).then_some(open))
}

/// Checks if `node` or its siblings (at most MATCH_LIMIT nodes) is the specified closing char
///
/// # Returns
///
/// The position of the found node or `None` otherwise
fn find_pair_end(
    doc: RopeSlice,
    node: Option<Node>,
    end_char: char,
    direction: Direction,
) -> Option<usize> {
    let advance = match direction {
        Forward => Node::next_sibling,
        Backward => Node::prev_sibling,
    };
    iter::successors(node, advance)
        .take(MATCH_LIMIT)
        .find_map(|node| {
            let (pos, c) = as_char(doc, &node)?;
            (end_char == c).then_some(pos)
        })
}

/// Tests if this node is a pair close char and returns the expected open char
fn as_open_pair(doc: RopeSlice, node: &Node) -> Option<char> {
    let open = as_char(doc, node)?.1;
    PAIRS
        .iter()
        .find_map(|&(open_, close)| (open_ == open).then_some(close))
}

/// If node is a single char return it (and its char position)
fn as_char(doc: RopeSlice, node: &Node) -> Option<(usize, char)> {
    // TODO: multi char/non ASCII pairs
    if node.byte_range().len() != 1 {
        return None;
    }
    let pos = doc.try_byte_to_char(node.start_byte()).ok()?;
    Some((pos, doc.char(pos)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_matching_bracket_empty_file() {
        let actual = find_matching_bracket_plaintext("".into(), 0);
        assert_eq!(actual, None);
    }

    #[test]
    fn test_find_matching_bracket_current_line_plaintext() {
        let assert = |input: &str, pos, expected| {
            let input = RopeSlice::from(input);
            let actual = find_matching_bracket_plaintext(input, pos);
            assert_eq!(expected, actual.unwrap());

            let actual = find_matching_bracket_plaintext(input, expected);
            assert_eq!(pos, actual.unwrap(), "expected symmetrical behaviour");
        };

        assert("(hello)", 0, 6);
        assert("((hello))", 0, 8);
        assert("((hello))", 1, 7);
        assert("(((hello)))", 2, 8);

        assert("key: ${value}", 6, 12);
        assert("key: ${value} # (some comment)", 16, 29);

        assert("(paren (paren {bracket}))", 0, 24);
        assert("(paren (paren {bracket}))", 7, 23);
        assert("(paren (paren {bracket}))", 14, 22);

        assert("(prev line\n ) (middle) ( \n next line)", 0, 12);
        assert("(prev line\n ) (middle) ( \n next line)", 14, 21);
        assert("(prev line\n ) (middle) ( \n next line)", 23, 36);
    }
}
