use crate::{Range, RopeSlice, Selection, Syntax};

// TODO: to contract_selection we'd need to store the previous ranges before expand.
// Maybe just contract to the first child node?
pub fn expand_selection(syntax: &Syntax, text: RopeSlice, selection: &Selection) -> Selection {
    let tree = syntax.tree();

    selection.clone().transform(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        // find parent of a descendant that matches the range
        let parent = match tree
            .root_node()
            .descendant_for_byte_range(from, to)
            .and_then(|node| {
                if node.child_count() == 0 || (node.start_byte() == from && node.end_byte() == to) {
                    node.parent()
                } else {
                    Some(node)
                }
            }) {
            Some(parent) => parent,
            None => return range,
        };

        let from = text.byte_to_char(parent.start_byte());
        let to = text.byte_to_char(parent.end_byte());

        if range.head < range.anchor {
            Range::new(to, from)
        } else {
            Range::new(from, to)
        }
    })
}
