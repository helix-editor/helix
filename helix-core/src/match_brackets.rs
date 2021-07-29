use crate::{Rope, Syntax};

const PAIRS: &[(char, char)] = &[('(', ')'), ('{', '}'), ('[', ']'), ('<', '>')];
// limit matching pairs to only ( ) { } [ ] < >

#[must_use]
pub fn find(syntax: &Syntax, doc: &Rope, pos: usize) -> Option<usize> {
    let tree = syntax.tree();

    let byte_pos = doc.char_to_byte(pos);

    // most naive implementation: find the innermost syntax node, if we're at the edge of a node,
    // return the other edge.

    let node = match tree
        .root_node()
        .named_descendant_for_byte_range(byte_pos, byte_pos)
    {
        Some(node) => node,
        None => return None,
    };

    if node.is_error() {
        return None;
    }

    let len = doc.len_bytes();
    let start_byte = node.start_byte();
    let end_byte = node.end_byte() - 1; // it's end exclusive
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
    }

    None
}
