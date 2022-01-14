use crate::{Range, RopeSlice, Selection, Syntax};

pub fn expand_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    selection.transform(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        // find parent of a descendant that matches the range
        let parent = match node_under_range(syntax, text, &range).and_then(|node| {
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

pub fn shrink_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    selection.transform(|range| {
        let descendant = match node_under_range(syntax, text, &range) {
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

#[inline(always)]
pub fn next_sibling_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    transform_node_under_selection(syntax, text, selection, tree_sitter::Node::next_sibling)
}

#[inline(always)]
pub fn prev_sibling_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    transform_node_under_selection(syntax, text, selection, tree_sitter::Node::prev_sibling)
}

fn transform_node_under_selection<'a, F>(
    syntax: &'a Syntax,
    text: RopeSlice,
    selection: Selection,
    f: F,
) -> Selection
where
    F: Fn(&tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>>,
{
    selection.transform(|range| {
        let next_sibling = match node_under_range(syntax, text, &range) {
            // find first child, if not possible, fallback to the node that contains selection
            Some(descendant) => match f(&descendant) {
                Some(sib) => sib,
                None => return range,
            },
            None => return range,
        };

        let from = text.byte_to_char(next_sibling.start_byte());
        let to = text.byte_to_char(next_sibling.end_byte());

        if range.head < range.anchor {
            Range::new(to, from)
        } else {
            Range::new(from, to)
        }
    })
}

pub fn node_under_range<'a>(
    syntax: &'a Syntax,
    text: RopeSlice,
    range: &Range,
) -> Option<tree_sitter::Node<'a>> {
    let tree = syntax.tree();
    let from = text.char_to_byte(range.from());
    let to = text.char_to_byte(range.to());
    tree.root_node().descendant_for_byte_range(from, to)
}
