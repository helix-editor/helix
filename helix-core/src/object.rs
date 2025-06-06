use crate::{movement::Direction, syntax::TreeCursor, Range, Selection, Syntax};

pub fn expand_selection(syntax: &Syntax, selection: Selection) -> Selection {
    let cursor = &mut syntax.walk();

    selection.transform(|range| {
        let from = range.from() as u32;
        let to = range.to() as u32;
        let byte_range = from..to;
        cursor.reset_to_byte_range(from, to);

        while cursor.node().byte_range() == byte_range {
            if !cursor.goto_parent() {
                break;
            }
        }

        let node = cursor.node();
        Range::new(node.start_byte() as usize, node.end_byte() as usize)
            .with_direction(range.direction())
    })
}

pub fn shrink_selection(syntax: &Syntax, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        selection,
        |cursor| {
            cursor.goto_first_child();
        },
        None,
    )
}

pub fn select_next_sibling(syntax: &Syntax, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        selection,
        |cursor| {
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    break;
                }
            }
        },
        Some(Direction::Forward),
    )
}

pub fn select_all_siblings(syntax: &Syntax, selection: Selection) -> Selection {
    let mut cursor = syntax.walk();
    selection.transform_iter(move |range| {
        let from = range.from();
        let to = range.to();
        cursor.reset_to_byte_range(from as u32, to as u32);

        if !cursor.goto_parent_with(|parent| parent.child_count() > 1) {
            return vec![range].into_iter();
        }

        select_children(&mut cursor, range).into_iter()
    })
}

pub fn select_all_children(syntax: &Syntax, selection: Selection) -> Selection {
    let mut cursor = syntax.walk();
    selection.transform_iter(move |range| {
        let from = range.from();
        let to = range.to();
        cursor.reset_to_byte_range(from as u32, to as u32);
        select_children(&mut cursor, range).into_iter()
    })
}

fn select_children(cursor: &mut TreeCursor, range: Range) -> Vec<Range> {
    let children = cursor
        .children()
        .filter(|child| child.is_named())
        .map(|child| Range::from_node(child, range.direction()))
        .collect::<Vec<_>>();

    if !children.is_empty() {
        children
    } else {
        vec![range]
    }
}

pub fn select_prev_sibling(syntax: &Syntax, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        selection,
        |cursor| {
            while !cursor.goto_previous_sibling() {
                if !cursor.goto_parent() {
                    break;
                }
            }
        },
        Some(Direction::Backward),
    )
}

fn select_node_impl<F>(
    syntax: &Syntax,
    selection: Selection,
    motion: F,
    direction: Option<Direction>,
) -> Selection
where
    F: Fn(&mut TreeCursor),
{
    let cursor = &mut syntax.walk();

    selection.transform(|range| {
        cursor.reset_to_byte_range(range.from() as u32, range.to() as u32);

        motion(cursor);

        let node = cursor.node();
        Range::new(node.start_byte() as usize, node.end_byte() as usize)
            .with_direction(direction.unwrap_or_else(|| range.direction()))
    })
}
