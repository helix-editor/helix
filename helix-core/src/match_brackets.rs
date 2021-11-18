use tree_sitter::Node;

use crate::{Rope, Syntax};

const O_PAREN: char = '(';
const C_PAREN: char = ')';
const O_CURLY: char = '{';
const C_CURLY: char = '}';
const O_BRCKT: char = '[';
const C_BRCKT: char = ']';
const O_ANGLE: char = '<';
const C_ANGLE: char = '>';

const SINGLE_QUOTE: char = '\'';
const DOUBLE_QUOTE: char = '\"';

const PAIRS: &[(char, char)] = &[
    (O_PAREN, C_PAREN),
    (O_CURLY, C_CURLY),
    (O_BRCKT, C_BRCKT),
    (O_ANGLE, C_ANGLE),
    (SINGLE_QUOTE, SINGLE_QUOTE),
    (DOUBLE_QUOTE, DOUBLE_QUOTE),
];

// limit matching pairs to only ( ) { } [ ] < >

// Returns the position of the matching bracket under cursor.
//
// If the cursor is one the opening bracket, the position of
// the closing bracket is returned. If the cursor in the closing
// bracket, the position of the opening bracket is returned.
//
// If the cursor is not on a bracket, `None` is returned.
#[must_use]
pub fn find_matching_bracket(syntax: &Syntax, doc: &Rope, pos: usize) -> Option<usize> {
    let tree = syntax.tree();
    let byte_pos = doc.char_to_byte(pos);

    let node = match tree
        .root_node()
        .named_descendant_for_byte_range(byte_pos, byte_pos)
    {
        Some(node) if !node.is_error() => node,
        _ => return None,
    };

    let (start_byte, end_byte) = surrounding_bytes(&doc, &node)?;
    let (start_char, end_char) = (doc.byte_to_char(start_byte), doc.byte_to_char(end_byte));

    if is_valid_pair(&doc, start_char, end_char) {
        if start_byte == byte_pos {
            return Some(end_char);
        }
        if end_byte == byte_pos {
            return Some(start_char);
        }
    }

    None
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
pub fn find_matching_bracket_fuzzy(syntax: &Syntax, doc: &Rope, pos: usize) -> Option<usize> {
    let tree = syntax.tree();
    let byte_pos = doc.char_to_byte(pos);

    let mut cursor = tree.walk();
    let mut node = None;

    // Walk the tree until we find the node for the given byte position.
    while cursor.goto_first_child_for_byte(byte_pos).is_some() {
        if cursor.node().is_named() && !cursor.node().is_error() {
            node = Some(cursor.node());
        }
    }

    let mut node = node?;

    // Travere the tree upwards to find first node with surrounding brackets.
    loop {
        let (start_byte, end_byte) = surrounding_bytes(&doc, &node)?;
        let (start_char, end_char) = (doc.byte_to_char(start_byte), doc.byte_to_char(end_byte));

        if is_valid_pair(&doc, start_char, end_char) {
            if start_byte == byte_pos {
                return Some(end_char);
            }
            if end_byte == byte_pos {
                return Some(start_char);
            }
            // We found a surrounding node, but the cursor
            // is within the scope of that node. Hence, we
            // return the closing bracket.
            return Some(end_char);
        }

        if cursor.goto_parent() {
            node = cursor.node();
        } else {
            return None;
        }
    }
}

fn is_valid_pair(doc: &Rope, start_char: usize, end_char: usize) -> bool {
    PAIRS.contains(&(doc.char(start_char), doc.char(end_char)))
}

fn surrounding_bytes(doc: &Rope, node: &Node) -> Option<(usize, usize)> {
    let len = doc.len_bytes();

    let start_byte = node.start_byte();
    let end_byte = node.end_byte().saturating_sub(1);

    if start_byte >= len || end_byte >= len {
        return None;
    }

    Some((start_byte, end_byte))
}
