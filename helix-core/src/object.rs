use crate::{movement::Direction, syntax::TreeCursor, Range, RopeSlice, Selection, Syntax};

pub fn expand_selection(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    let cursor = &mut syntax.walk();

    selection.transform(|range| {
        let from = text.char_to_byte(range.from()) as u32;
        let to = text.char_to_byte(range.to()) as u32;

        let byte_range = from..to;
        cursor.reset_to_byte_range(from, to);

        while cursor.node().byte_range() == byte_range {
            if !cursor.goto_parent() {
                break;
            }
        }

        let node = cursor.node();
        let from = text.byte_to_char(node.start_byte() as usize);
        let to = text.byte_to_char(node.end_byte() as usize);

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
        |cursor| move_to_sibling(cursor, Direction::Forward),
        Some(Direction::Forward),
    )
}

pub fn select_all_siblings(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    let mut cursor = syntax.walk();
    selection.transform_iter(move |range| {
        let (from, to) = range.into_byte_range(text);
        cursor.reset_to_byte_range(from as u32, to as u32);

        if !cursor.goto_parent_with(|parent| parent.child_count() > 1) {
            return vec![range].into_iter();
        }

        select_children(&mut cursor, text, range).into_iter()
    })
}

pub fn select_all_children(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    let mut cursor = syntax.walk();
    selection.transform_iter(move |range| {
        let (from, to) = range.into_byte_range(text);
        cursor.reset_to_byte_range(from as u32, to as u32);
        select_children(&mut cursor, text, range).into_iter()
    })
}

fn select_children(cursor: &mut TreeCursor, text: RopeSlice, range: Range) -> Vec<Range> {
    let children = cursor
        .children()
        .filter(|child| child.is_named())
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
        |cursor| move_to_sibling(cursor, Direction::Backward),
        Some(Direction::Backward),
    )
}


/// Move the cursor to its sibling along the requested direction, wrapping at
/// the ends.
///
/// If the node has no siblings, the cursor is left unchanged.
fn move_to_sibling(cursor: &mut TreeCursor, direction: Direction) {
    // A node can either be a root, an only child or have siblings.
    let Some(parent) = cursor.node().parent() else {
        return; // Root node case
    };
    if parent.child_count() <= 1 {
        return; // Only child case
    }

    let moved = match direction {
        Direction::Forward => cursor.goto_next_sibling(),
        Direction::Backward => cursor.goto_previous_sibling(),
    };
    if moved {
        return; // Had a sibling in the requested direction
    }

    // Wrap
    cursor.goto_parent();
    // Workaround. Currently used version of `tree-house` does not expose
    // `goto_last_child` binding from `tree-house-bindings`. The following is
    // equivalent to
    // ```rust
    // match direction {
    //      Direction::Forward => cursor.goto_first_child(),
    //      Direction::Backward => cursor.goto_last_child(),
    // }
    // ```
    cursor.goto_first_child();
    if direction == Direction::Backward {
        while cursor.goto_next_sibling() {}
    }
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
        let from = text.char_to_byte(range.from()) as u32;
        let to = text.char_to_byte(range.to()) as u32;

        cursor.reset_to_byte_range(from, to);

        motion(cursor);

        let node = cursor.node();
        let from = text.byte_to_char(node.start_byte() as usize);
        let to = text.byte_to_char(node.end_byte() as usize);

        Range::new(from, to).with_direction(direction.unwrap_or_else(|| range.direction()))
    })
}


#[cfg(test)]
mod tests {
    use std::{assert_eq, sync::LazyLock};

    use super::*;
    use crate::{config::default_lang_loader, Rope};

    static LOADER: LazyLock<crate::syntax::Loader> = LazyLock::new(default_lang_loader);

    #[test]
    fn sibling_motions_are_mutual_inverses_and_wrap() {
        let language = LOADER.language_for_name("scheme").unwrap();
        let source = Rope::from_str("(a)\n(b)");
        let syntax = Syntax::new(source.slice(..), language, &LOADER).unwrap();
        let list = syntax.tree().root_node(); // (a)\n(b)

        let a = list.named_child(0).unwrap(); // (a)
        let b = list.named_child(1).unwrap(); // (b)

        let mut cursor = syntax.walk();
        cursor.goto_first_child(); // (a)
        move_to_sibling(&mut cursor, Direction::Backward);
        assert_eq!(cursor.node().id(), b.id()); // Wraps
        move_to_sibling(&mut cursor, Direction::Forward);
        assert_eq!(cursor.node().id(), a.id()); // Is inverse
    }
}
