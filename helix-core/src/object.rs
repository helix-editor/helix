use crate::{movement::Direction, syntax::TreeCursor, Range, RopeSlice, Selection, Syntax};
use tree_sitter::{Node, Tree};

pub fn expand_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    let cursor = &mut syntax.walk();

    selection.transform(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        let byte_range = from..to;
        cursor.reset_to_byte_range(from, to);

        while cursor.node().byte_range() == byte_range {
            if !cursor.goto_parent() {
                break;
            }
        }

        let node = cursor.node();
        let from = text.byte_to_char(node.start_byte());
        let to = text.byte_to_char(node.end_byte());

        Range::new(to, from).with_direction(range.direction())
    })
}

pub fn shrink_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(syntax, text, selection, |cursor| {
        cursor.goto_first_child();
    })
}

pub fn select_next_sibling(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(syntax, text, selection, |cursor| {
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                break;
            }
        }
    })
}

fn find_parent_with_more_children(mut node: Node) -> Option<Node> {
    while let Some(parent) = node.parent() {
        if parent.child_count() > 1 {
            return Some(parent);
        }

        node = parent;
    }

    None
}

pub fn select_all_siblings(tree: &Tree, text: RopeSlice, selection: Selection) -> Selection {
    let root_node = &tree.root_node();

    selection.transform_iter(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        root_node
            .descendant_for_byte_range(from, to)
            .and_then(find_parent_with_more_children)
            .and_then(|parent| select_children(parent, text, range.direction()))
            .unwrap_or_else(|| vec![range].into_iter())
    })
}

pub fn select_all_children(tree: &Tree, text: RopeSlice, selection: Selection) -> Selection {
    let root_node = &tree.root_node();

    selection.transform_iter(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        root_node
            .descendant_for_byte_range(from, to)
            .and_then(|parent| select_children(parent, text, range.direction()))
            .unwrap_or_else(|| vec![range].into_iter())
    })
}

fn select_children(
    node: Node,
    text: RopeSlice,
    direction: Direction,
) -> Option<<Vec<Range> as std::iter::IntoIterator>::IntoIter> {
    let mut cursor = node.walk();

    let children = node
        .named_children(&mut cursor)
        .map(|child| {
            let from = text.byte_to_char(child.start_byte());
            let to = text.byte_to_char(child.end_byte());

            if direction == Direction::Backward {
                Range::new(to, from)
            } else {
                Range::new(from, to)
            }
        })
        .collect::<Vec<_>>();

    if !children.is_empty() {
        Some(children.into_iter())
    } else {
        None
    }
}

fn find_sibling_recursive<F>(node: Node, sibling_fn: F) -> Option<Node>
where
    F: Fn(Node) -> Option<Node>,
{
    sibling_fn(node).or_else(|| {
        node.parent()
            .and_then(|node| find_sibling_recursive(node, sibling_fn))
    })
}

pub fn select_prev_sibling(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(syntax, text, selection, |cursor| {
        while !cursor.goto_prev_sibling() {
            if !cursor.goto_parent() {
                break;
            }
        }
    })
}

fn select_node_impl<F>(
    syntax: &Syntax,
    text: RopeSlice,
    selection: Selection,
    motion: F,
) -> Selection
where
    F: Fn(&mut TreeCursor),
{
    let cursor = &mut syntax.walk();

    selection.transform(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        cursor.reset_to_byte_range(from, to);

        motion(cursor);

        let node = cursor.node();
        let from = text.byte_to_char(node.start_byte());
        let to = text.byte_to_char(node.end_byte());

        Range::new(from, to).with_direction(range.direction())
    })
}
