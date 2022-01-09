use crate::{Range, RopeSlice, Selection, Syntax};

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
                if node.start_byte() == from && node.end_byte() == to {
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

pub fn shrink_selection(syntax: &Syntax, text: RopeSlice, selection: &Selection) -> Selection {
    let tree = syntax.tree();

    selection.clone().transform(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        let descendant = match tree.root_node().descendant_for_byte_range(from, to) {
            // find first child, if not possible, fallback to the node that contains selection
            Some(descendant) => match descendant.child(0) {
                Some(child) => child,
                None => descendant,
            },
            None => return range,
        };

        let from = text.byte_to_char(descendant.start_byte());
        let to = text.byte_to_char(descendant.end_byte());

        if range.head < range.anchor {
            Range::new(to, from)
        } else {
            Range::new(from, to)
        }
    })
}
