use crate::{movement::Direction, syntax::TreeCursor, Range, RopeSlice, Selection, Syntax};

pub fn expand_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        text,
        selection,
        |cursor, byte_range| {
            while cursor.node().byte_range() == byte_range {
                if !cursor.goto_parent() {
                    break;
                }
            }
        },
        None,
    )
}

pub fn shrink_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        text,
        selection,
        |cursor, byte_range| {
            cursor.goto_first_child();
            while cursor.node().start_byte() < byte_range.start
                || cursor.node().end_byte() > byte_range.end
            {
                if !cursor.goto_next_sibling() {
                    // If a child within the range couldn't be found, default to the first child.
                    cursor.goto_parent();
                    cursor.goto_first_child();
                    break;
                }
            }
        },
        None,
    )
}

pub fn select_next_sibling(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        text,
        selection,
        |cursor, _byte_range| {
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    break;
                }
            }
        },
        Some(Direction::Forward),
    )
}

pub fn select_all_siblings(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    selection.transform_iter(|range| {
        let mut cursor = syntax.walk();
        let (from, to) = range.into_byte_range(text);
        cursor.reset_to_byte_range(from, to);

        if !cursor.goto_parent_with(|parent| parent.child_count() > 1) {
            return vec![range].into_iter();
        }

        select_children(&mut cursor, text, range).into_iter()
    })
}

pub fn select_all_children(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    selection.transform_iter(|range| {
        let mut cursor = syntax.walk();
        let (from, to) = range.into_byte_range(text);
        cursor.reset_to_byte_range(from, to);
        select_children(&mut cursor, text, range).into_iter()
    })
}

fn select_children<'n>(
    cursor: &'n mut TreeCursor<'n>,
    text: RopeSlice,
    range: Range,
) -> Vec<Range> {
    let children = cursor
        .named_children()
        .map(|child| Range::from_node(child, text, range.direction()))
        .collect::<Vec<_>>();

    if !children.is_empty() {
        children
    } else {
        vec![range]
    }
}

pub fn select_prev_sibling(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        text,
        selection,
        |cursor, _byte_range| {
            while !cursor.goto_prev_sibling() {
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
    text: RopeSlice,
    selection: Selection,
    motion: F,
    direction: Option<Direction>,
) -> Selection
where
    // Fn(tree cursor, original selection's byte range)
    F: Fn(&mut TreeCursor, std::ops::Range<usize>),
{
    let cursor = &mut syntax.walk();

    selection.transform(|range| {
        let from = text.char_to_byte(range.from());
        let to = text.char_to_byte(range.to());

        cursor.reset_to_byte_range(from, to);

        motion(cursor, from..to);

        let node = cursor.node();
        let from = text.byte_to_char(node.start_byte());
        let to = text.byte_to_char(node.end_byte());

        Range::new(from, to).with_direction(direction.unwrap_or_else(|| range.direction()))
    })
}
