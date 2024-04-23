use crate::{movement::Direction, syntax::TreeCursor, Range, RopeSlice, Selection, Syntax};

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
    select_node_impl(
        syntax,
        text,
        selection,
        |cursor| {
            cursor.goto_first_child();
        },
        None,
    )
}

pub fn select_next_sibling(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    select_node_impl(
        syntax,
        text,
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
        |cursor| {
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

        Range::new(from, to).with_direction(direction.unwrap_or_else(|| range.direction()))
    })
}
