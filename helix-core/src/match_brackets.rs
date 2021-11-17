use crate::{Rope, Syntax};

const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('<', '>'),
    ('\'', '\''),
    ('"', '"'),
];
// limit matching pairs to only ( ) { } [ ] < >

#[must_use]
pub fn find(syntax: &Syntax, doc: &Rope, pos: usize) -> Option<usize> {
    let tree = syntax.tree();

    let byte_pos = doc.char_to_byte(pos);

    // a little less naive implementation: find the innermost syntax node at
    // the given byte position. Traverse up the syntax tree until we reach the
    // first node with surrounding pairs.

    let mut cursor = tree.walk();
    let mut node = None;

    while let Some(_) = cursor.goto_first_child_for_byte(byte_pos) {
        if cursor.node().is_named() {
            node = Some(cursor.node());
        }
    }

    let mut node = node?;

    if node.is_error() {
        return None;
    }

    let len = doc.len_bytes();

    // Travere the tree upwards to find first node with enclosing brackets.
    loop {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte().saturating_sub(1); // it's end exclusive

        if start_byte >= len || end_byte >= len {
            return None;
        }

        let start_char = doc.byte_to_char(start_byte);
        let end_char = doc.byte_to_char(end_byte);

        if PAIRS.contains(&(doc.char(start_char), doc.char(end_char))) {
            if start_byte == byte_pos {
                return Some(end_char);
            }

            if end_byte == byte_pos {
                return Some(start_char);
            }

            return Some(end_char);
        }

        if cursor.goto_parent() {
            node = cursor.node();
        } else {
            return None;
        }
    }
}
