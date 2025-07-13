use std::iter;

use crate::tree_sitter::Node;
use ropey::RopeSlice;

use crate::movement::Direction::{self, Backward, Forward};
use crate::Syntax;

const MAX_PLAINTEXT_SCAN: usize = 10000;
const MATCH_LIMIT: usize = 16;

pub const BRACKETS: [(char, char); 9] = [
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('<', '>'),
    ('‘', '’'),
    ('“', '”'),
    ('«', '»'),
    ('「', '」'),
    ('（', '）'),
];

// The difference between BRACKETS and PAIRS is that we can find matching
// BRACKETS in a plain text file, but we can't do the same for PAIRs.
// PAIRS also contains all BRACKETS.
pub const PAIRS: [(char, char); BRACKETS.len() + 3] = {
    let mut pairs = [(' ', ' '); BRACKETS.len() + 3];
    let mut idx = 0;
    while idx < BRACKETS.len() {
        pairs[idx] = BRACKETS[idx];
        idx += 1;
    }
    pairs[idx] = ('"', '"');
    pairs[idx + 1] = ('\'', '\'');
    pairs[idx + 2] = ('`', '`');
    pairs
};

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
    if pos >= doc.len_chars() || !is_valid_pair(doc.char(pos)) {
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
    let pos = doc.char_to_byte(pos_) as u32;

    let root = syntax.tree_for_byte_range(pos, pos).root_node();
    let mut node = root.descendant_for_byte_range(pos, pos)?;

    loop {
        if node.is_named() && node.child_count() >= 2 {
            let open = node.child(0).unwrap();
            let close = node.child(node.child_count() - 1).unwrap();

            if let (Some((start_pos, open)), Some((end_pos, close))) =
                (as_char(doc, &open), as_char(doc, &close))
            {
                if PAIRS.contains(&(open, close)) {
                    if end_pos == pos_ {
                        return Some(start_pos);
                    }

                    // We return the end char if the cursor is either on the start char
                    // or at some arbitrary position between start and end char.
                    if traverse_parents || start_pos == pos_ {
                        return Some(end_pos);
                    }
                }
            }
        }
        // this node itselt wasn't a pair but maybe its siblings are

        if let Some((start_char, end_char)) = as_close_pair(doc, &node) {
            if let Some(pair_start) =
                find_pair_end(doc, node.prev_sibling(), start_char, end_char, Backward)
            {
                return Some(pair_start);
            }
        }
        if let Some((start_char, end_char)) = as_open_pair(doc, &node) {
            if let Some(pair_end) =
                find_pair_end(doc, node.next_sibling(), start_char, end_char, Forward)
            {
                return Some(pair_end);
            }
        }

        if traverse_parents {
            for sibling in
                iter::successors(node.next_sibling(), |node| node.next_sibling()).take(MATCH_LIMIT)
            {
                let Some((start_char, end_char)) = as_close_pair(doc, &sibling) else {
                    continue;
                };
                if find_pair_end(doc, sibling.prev_sibling(), start_char, end_char, Backward)
                    .is_some()
                {
                    return doc.try_byte_to_char(sibling.start_byte() as usize).ok();
                }
            }
        } else if node.is_named() {
            break;
        }

        let Some(parent) = node.parent() else {
            break;
        };
        node = parent;
    }
    let node = root.named_descendant_for_byte_range(pos, pos + 1)?;
    if node.child_count() != 0 {
        return None;
    }
    let node_start = doc.byte_to_char(node.start_byte() as usize);
    let node_text = doc.byte_slice(node.start_byte() as usize..node.end_byte() as usize);
    find_matching_bracket_plaintext(node_text, pos_ - node_start).map(|pos| pos + node_start)
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
    let bracket = doc.get_char(cursor_pos)?;
    let matching_bracket = {
        let pair = get_pair(bracket);
        if pair.0 == bracket {
            pair.1
        } else {
            pair.0
        }
    };
    // Don't do anything when the cursor is not on top of a bracket.
    if !is_valid_bracket(bracket) {
        return None;
    }

    // Determine the direction of the matching.
    let is_fwd = is_open_bracket(bracket);
    let chars_iter = if is_fwd {
        doc.chars_at(cursor_pos + 1)
    } else {
        doc.chars_at(cursor_pos).reversed()
    };

    let mut open_cnt = 1;

    for (i, candidate) in chars_iter.take(MAX_PLAINTEXT_SCAN).enumerate() {
        if candidate == bracket {
            open_cnt += 1;
        } else if candidate == matching_bracket {
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

/// Returns the open and closing chars pair. If not found in
/// [`BRACKETS`] returns (ch, ch).
///
/// ```
/// use helix_core::match_brackets::get_pair;
///
/// assert_eq!(get_pair('['), ('[', ']'));
/// assert_eq!(get_pair('}'), ('{', '}'));
/// assert_eq!(get_pair('"'), ('"', '"'));
/// ```
pub fn get_pair(ch: char) -> (char, char) {
    PAIRS
        .iter()
        .find(|(open, close)| *open == ch || *close == ch)
        .copied()
        .unwrap_or((ch, ch))
}

pub fn is_open_bracket(ch: char) -> bool {
    BRACKETS.iter().any(|(l, _)| *l == ch)
}

pub fn is_close_bracket(ch: char) -> bool {
    BRACKETS.iter().any(|(_, r)| *r == ch)
}

pub fn is_valid_bracket(ch: char) -> bool {
    BRACKETS.iter().any(|(l, r)| *l == ch || *r == ch)
}

pub fn is_open_pair(ch: char) -> bool {
    PAIRS.iter().any(|(l, _)| *l == ch)
}

pub fn is_close_pair(ch: char) -> bool {
    PAIRS.iter().any(|(_, r)| *r == ch)
}

pub fn is_valid_pair(ch: char) -> bool {
    PAIRS.iter().any(|(l, r)| *l == ch || *r == ch)
}

/// Tests if this node is a pair close char and returns the expected open char
/// and close char contained in this node
fn as_close_pair(doc: RopeSlice, node: &Node) -> Option<(char, char)> {
    let close = as_char(doc, node)?.1;
    PAIRS
        .iter()
        .find_map(|&(open, close_)| (close_ == close).then_some((close, open)))
}

/// Checks if `node` or its siblings (at most MATCH_LIMIT nodes) is the specified closing char
///
/// # Returns
///
/// The position of the found node or `None` otherwise
fn find_pair_end(
    doc: RopeSlice,
    node: Option<Node>,
    start_char: char,
    end_char: char,
    direction: Direction,
) -> Option<usize> {
    let advance = match direction {
        Forward => Node::next_sibling,
        Backward => Node::prev_sibling,
    };
    let mut depth = 0;
    iter::successors(node, advance)
        .take(MATCH_LIMIT)
        .find_map(|node| {
            let (pos, c) = as_char(doc, &node)?;
            if c == end_char {
                if depth == 0 {
                    return Some(pos);
                }
                depth -= 1;
            } else if c == start_char {
                depth += 1;
            }
            None
        })
}

/// Tests if this node is a pair open char and returns the expected close char
/// and open char contained in this node
fn as_open_pair(doc: RopeSlice, node: &Node) -> Option<(char, char)> {
    let open = as_char(doc, node)?.1;
    PAIRS
        .iter()
        .find_map(|&(open_, close)| (open_ == open).then_some((open, close)))
}

/// If node is a single char return it (and its char position)
fn as_char(doc: RopeSlice, node: &Node) -> Option<(usize, char)> {
    // TODO: multi char/non ASCII pairs
    if node.byte_range().len() != 1 {
        return None;
    }
    let pos = doc.try_byte_to_char(node.start_byte() as usize).ok()?;
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
