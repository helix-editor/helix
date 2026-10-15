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

/// Move to the next or previous sibling, wrapping at either end of the sibling
/// group. If the current node has no siblings, climb through single-child
/// ancestors and use the nearest ancestor's sibling group instead.
///
/// Tree-sitter grammars may wrap a construct in a single-child node with the
/// same source range. For example, Rust's `while_expression` is wrapped in an
/// `expression_statement`. By climbing up ancestry as described above, the
/// sibling motion treats the wrapper and its child as same.
///
/// If the root is reached without finding a sibling group, leave the cursor
/// unchanged.
fn move_to_sibling(cursor: &mut TreeCursor, direction: Direction) {
    // If there are no siblings to move to, climb up ancestry to find a non-root
    // ancestor with siblings to move to.
    while cursor
        .node()
        .parent()
        .is_some_and(|parent| parent.child_count() == 1)
    {
        if !cursor.goto_parent() {
            // Root case
            return;
        }
    }

    // The root has no siblings to move between or wrap around.
    if cursor.node().parent().is_none() {
        return;
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
    // equivalent and will be available after upgrade.
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


    fn parse_syntax(lang: &str, source: &str) -> (Rope, Syntax) {
        let source = Rope::from_str(source);
        let language = LOADER.language_for_name(lang).unwrap();
        let syntax = Syntax::new(source.slice(..), language, &LOADER).unwrap();
        (source, syntax)
    }


    #[test]
    fn sibling_motions_wrap_at_group_boundaries() {
        let (_source, syntax) = parse_syntax("scheme", "(a)\n(b)");
        let root = syntax.tree().root_node();
        let first = root.child(0).unwrap(); // (a)
        let last = root.child(1).unwrap(); // (b)
        let mut cursor = syntax.walk();

        cursor.goto_first_child();
        move_to_sibling(&mut cursor, Direction::Backward);
        assert_eq!(cursor.node().id(), last.id());

        cursor.goto_parent();
        cursor.goto_first_child();
        cursor.goto_next_sibling();
        move_to_sibling(&mut cursor, Direction::Forward);
        assert_eq!(cursor.node().id(), first.id());
    }


    #[test]
    fn sibling_motions_are_mutual_inverses() {
        let (_source, syntax) = parse_syntax("scheme", "(a)\n(b)");
        let mut cursor = syntax.walk();
        cursor.goto_first_child();
        let first_id = cursor.node().id();

        move_to_sibling(&mut cursor, Direction::Forward);
        move_to_sibling(&mut cursor, Direction::Backward);
        assert_eq!(cursor.node().id(), first_id);

        cursor.goto_next_sibling();
        let second_id = cursor.node().id();
        move_to_sibling(&mut cursor, Direction::Backward);
        move_to_sibling(&mut cursor, Direction::Forward);
        assert_eq!(cursor.node().id(), second_id);
    }


    fn find_node<'tree>(
        node: crate::tree_sitter::Node<'tree>,
        kind: &str,
    ) -> Option<crate::tree_sitter::Node<'tree>> {
        if node.kind() == kind {
            return Some(node);
        }
        (0..node.child_count()).find_map(|index| find_node(node.child(index)?, kind))
    }


    #[test]
    fn sibling_motion_climbs_single_child_wrappers() {
        let (source, syntax) =
            parse_syntax("rust", "fn f() { while condition {} let node = value; }");
        let root = syntax.tree().root_node();
        let while_node = find_node(root.clone(), "while_expression").unwrap();
        let let_node = find_node(root, "let_declaration").unwrap();
        let text = source.slice(..);
        let range = Range::from_node(while_node, text, Direction::Forward);
        let selection = Selection::single(range.anchor, range.head);

        let result = select_next_sibling(&syntax, text, selection);

        assert_eq!(
            result.primary(),
            Range::from_node(let_node, text, Direction::Forward)
        );
    }
}
