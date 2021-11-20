use tree_sitter::Node;

use crate::{Rope, Syntax};

const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('<', '>'),
    ('\'', '\''),
    ('\"', '\"'),
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
pub fn find_matching_bracket_fuzzy(syntax: &Syntax, doc: &Rope, pos: usize) -> Option<usize> {
    find_pair(syntax, doc, pos, true)
}

fn find_pair(syntax: &Syntax, doc: &Rope, pos: usize, traverse_parents: bool) -> Option<usize> {
    let tree = syntax.tree();
    let pos = doc.char_to_byte(pos);

    let mut node = tree.root_node().named_descendant_for_byte_range(pos, pos)?;

    loop {
        let (start_byte, end_byte) = surrounding_bytes(doc, &node)?;
        let (start_char, end_char) = (doc.byte_to_char(start_byte), doc.byte_to_char(end_byte));

        if is_valid_pair(doc, start_char, end_char) {
            if end_byte == pos {
                return Some(start_char);
            }
            // We return the end char if the cursor is either on the start char
            // or at some arbitrary position between start and end char.
            return Some(end_char);
        }

        if traverse_parents {
            node = node.parent()?;
        } else {
            return None;
        }
    }
}

fn is_valid_bracket(c: char) -> bool {
    PAIRS.iter().any(|(l, r)| *l == c || *r == c)
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
