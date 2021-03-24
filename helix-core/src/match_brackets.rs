use crate::{Range, Rope, Selection, Syntax};

// const PAIRS: &[(char, char)] = &[('(', ')'), ('{', '}'), ('[', ']')];
// limit matching pairs to only ( ) { } [ ] < >

#[must_use]
pub fn find(syntax: &Syntax, doc: &Rope, pos: usize) -> Option<usize> {
    let tree = syntax.root_layer.tree.as_ref().unwrap();

    let byte_pos = doc.char_to_byte(pos);

    // most naive implementation: find the innermost syntax node, if we're at the edge of a node,
    // return the other edge.

    let mut node = match tree
        .root_node()
        .named_descendant_for_byte_range(byte_pos, byte_pos)
    {
        Some(node) => node,
        None => return None,
    };

    let start_byte = node.start_byte();
    let end_byte = node.end_byte() - 1; // it's end exclusive

    if start_byte == byte_pos {
        return Some(doc.byte_to_char(end_byte));
    }

    if end_byte == byte_pos {
        return Some(doc.byte_to_char(start_byte));
    }

    None
}
