use crate::{
    syntax::Syntax,
    tree_sitter::{Node, Tree},
    Rope, RopeSlice, State,
};

/// To determine indentation of a newly inserted line, figure out the indentation at the last col
/// of the previous line.

pub const TAB_WIDTH: usize = 4;

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

    // Hardcoded for rust for now
    let indent_scopes = &[
        "block",
        "function_item",
        "closure_expression",
        "while_expression",
        "for_expression",
        "loop_expression",
        "if_expression",
        "if_let_expression",
        "binary_expression",
        "match_expression",
        "match_arm",
        //
        "struct_item",
        "enum_item",
        "impl_item",
        //
        "mod_item",
    ];

    // let indent_except_first_scopes = &[];

    let not_first_sibling = node.next_sibling().is_some();
    let not_last_sibling = node.prev_sibling().is_some();
    let not_first_or_last_sibling = not_first_sibling && not_last_sibling;

    let parent_kind = parent.kind();
    let is_scope = indent_scopes.iter().any(|scope| scope == &parent_kind);

    // && not_first_or_last_sibling
    if is_scope {
        increment += 1
    }

    // if last_scope && increment > 0 && ...{ ignore }

    walk(Some(parent)) + increment
}

fn find_first_non_whitespace_char(state: &State, line_num: usize) -> usize {
    let line = state.doc.line(line_num);
    let mut start = state.doc.line_to_char(line_num);

    // find first non-whitespace char
    for ch in line.chars() {
        // TODO: could use memchr with chunks?
        if ch != ' ' && ch != '\t' {
            break;
        }
        start += 1;
    }
    start
}

fn suggested_indent_for_line(state: &State, line_num: usize) -> usize {
    let line = state.doc.line(line_num);
    let current = indent_level_for_line(line);

    let start = find_first_non_whitespace_char(state, line_num);

    suggested_indent_for_pos(state, start)
}

pub fn suggested_indent_for_pos(state: &State, pos: usize) -> usize {
    if let Some(syntax) = &state.syntax {
        let byte_start = state.doc.char_to_byte(pos);
        let node = get_highest_syntax_node_at_bytepos(syntax, byte_start);

        let indentation = walk(node);
        // special case for comments

        // if preserve_leading_whitespace

        indentation
    } else {
        // TODO: heuristics for non-tree sitter grammars
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_indent_level() {
        let line = Rope::from("        fn new"); // 8 spaces
        assert_eq!(indent_level_for_line(line.slice(..)), 2);
        let line = Rope::from("\t\t\tfn new"); // 3 tabs
        assert_eq!(indent_level_for_line(line.slice(..)), 3);
        // mixed indentation
        let line = Rope::from("\t    \tfn new"); // 1 tab, 4 spaces, tab
        assert_eq!(indent_level_for_line(line.slice(..)), 3);
    }

    #[test]
    fn test_suggested_indent_for_line() {
        let doc = Rope::from(
            "mod test {
    fn hello_world() {
        1 + 1
    }
}
",
        );

        let mut state = State::new(doc);
        state.set_language("source.rust", &[]);

        assert_eq!(suggested_indent_for_line(&state, 0), 0); // mod
        assert_eq!(suggested_indent_for_line(&state, 1), 1); // fn
        assert_eq!(suggested_indent_for_line(&state, 2), 2); // 1 + 1
        assert_eq!(suggested_indent_for_line(&state, 4), 1); // }
        assert_eq!(suggested_indent_for_line(&state, 5), 0); // }
    }
}
