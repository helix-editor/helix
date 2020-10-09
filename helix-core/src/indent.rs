use crate::{
    syntax::Syntax,
    tree_sitter::{Node, Tree},
    Rope, RopeSlice, State,
};

const TAB_WIDTH: usize = 4;

fn indent_level_for_line(line: RopeSlice) -> usize {
    let mut len = 0;
    for ch in line.chars() {
        match ch {
            '\t' => len += TAB_WIDTH,
            ' ' => len += 1,
            _ => break,
        }
    }

    len / TAB_WIDTH
}

/// Find the highest syntax node at position.
/// This is to identify the column where this node (e.g., an HTML closing tag) ends.
fn get_highest_syntax_node_at_bytepos(syntax: &Syntax, pos: usize) -> Option<Node> {
    let tree = syntax.root_layer.tree.as_ref().unwrap();

    let mut node = match tree.root_node().named_descendant_for_byte_range(pos, pos) {
        Some(node) => node,
        None => return None,
    };

    while let Some(parent) = node.parent() {
        if parent.start_byte() == node.start_byte() {
            node = parent
        } else {
            break;
        }
    }

    Some(node)
}

fn walk(node: Option<Node>) -> usize {
    let node = match node {
        Some(node) => node,
        None => return 0,
    };

    let parent = match node.parent() {
        Some(node) => node,
        None => return 0,
    };

    let mut increment = 0;

    let not_first_or_last_sibling = node.next_sibling().is_some() && node.prev_sibling().is_some();
    let is_scope = true;

    if not_first_or_last_sibling && is_scope {
        increment += 1;
    }

    walk(Some(parent)) + increment
}

// for_line_at_col
fn suggested_indent_for_line(state: &State, line_num: usize) -> usize {
    let line = state.doc.line(line_num);
    let current = indent_level_for_line(line);

    let mut byte_start = state.doc.line_to_byte(line_num);

    // find first non-whitespace char
    for ch in line.chars() {
        // TODO: could use memchr with chunks?
        if ch != ' ' && ch != '\t' {
            break;
        }
        byte_start += 1;
    }

    if let Some(syntax) = &state.syntax {
        let node = get_highest_syntax_node_at_bytepos(state.syntax.as_ref().unwrap(), byte_start);

        // let indentation = walk()
        // special case for comments

        // if preserve_leading_whitespace

        unimplemented!()
    } else {
        // TODO: case for non-tree sitter grammars
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn indent_level() {
        let line = Rope::from("        fn new"); // 8 spaces
        assert_eq!(indent_level_for_line(line.slice(..)), 2);
        let line = Rope::from("\t\t\tfn new"); // 3 tabs
        assert_eq!(indent_level_for_line(line.slice(..)), 3);
        // mixed indentation
        let line = Rope::from("\t    \tfn new"); // 1 tab, 4 spaces, tab
        assert_eq!(indent_level_for_line(line.slice(..)), 3);
    }
}
