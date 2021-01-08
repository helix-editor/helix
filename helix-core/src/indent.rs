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

    // named_descendant
    let mut node = match tree.root_node().descendant_for_byte_range(pos, pos) {
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

fn calculate_indentation(node: Option<Node>, newline: bool) -> usize {
    let mut increment = 0;

    // Hardcoded for rust for now
    let indent_scopes = &[
        // indent except first or block?
        "while_expression",
        "for_expression",
        "loop_expression",
        "if_expression",
        "if_let_expression",
        "match_expression",
        "match_arm",
    ];

    // this is for multiline things, such as:
    // self.method()
    //  .chain()
    //  .chain()
    //  where the first line isn't indented
    let indent_except_first_scopes = &[
        "block",
        "arguments",
        "declaration_list",
        "field_declaration_list",
        "enum_variant_list",
        // "function_item",
        // "call_expression",
        // "closure_expression",
        "binary_expression",
        "field_expression",
        //
        "where_predicate", // where_clause instead?
    ];

    let outdent = &["}", "]", ")"];

    let mut node = match node {
        Some(node) => node,
        None => return 0,
    };

    // if we're calculating indentation for a brand new line then the current node will become the
    // parent node. We need to take it's indentation level into account too.
    let node_kind = node.kind();
    if newline
        && (indent_scopes.contains(&node_kind) || indent_except_first_scopes.contains(&node_kind))
    {
        increment += 1;
    }

    while let Some(parent) = node.parent() {
        let not_first_sibling = node.prev_sibling().is_some();
        let not_last_sibling = node.next_sibling().is_some();
        let not_first_or_last_sibling = not_first_sibling && not_last_sibling;

        let parent_kind = parent.kind();

        // println!(
        //     "name: {}\tparent: {}\trange:\t{} {}\tfirst={:?}\tlast={:?}",
        //     node.kind(),
        //     parent.kind(),
        //     node.range().start_point,
        //     node.range().end_point,
        //     node.prev_sibling().is_none(),
        //     node.next_sibling().is_none(),
        // );

        if outdent.contains(&node.kind()) {
            // we outdent by skipping the rules for the current level and jumping up
            node = parent;
            continue;
        }

        let is_scope = indent_scopes.contains(&parent_kind);

        // && not_first_or_last_sibling
        if is_scope && not_first_or_last_sibling {
            // println!("is_scope {}", parent_kind);
            increment += 1
        }

        let is_scope = indent_except_first_scopes.contains(&parent_kind);

        // && not_first_sibling
        if is_scope && not_first_sibling {
            // println!("is_scope_except_first {}", parent_kind);
            increment += 1
        }

        // if last_scope && increment > 0 && ...{ ignore }

        node = parent;
    }

    increment
}

fn find_first_non_whitespace_char(state: &State, line_num: usize) -> Option<usize> {
    let line = state.doc.line(line_num);
    let mut start = state.doc.line_to_char(line_num);

    // find first non-whitespace char
    for ch in line.chars() {
        // TODO: could use memchr with chunks?
        if ch != ' ' && ch != '\t' && ch != '\n' {
            return Some(start);
        }
        start += 1;
    }

    None
}

fn suggested_indent_for_line(syntax: Option<&Syntax>, state: &State, line_num: usize) -> usize {
    let line = state.doc.line(line_num);
    let current = indent_level_for_line(line);

    if let Some(start) = find_first_non_whitespace_char(state, line_num) {
        return suggested_indent_for_pos(syntax, state, start, false);
    };

    // if the line is blank, indent should be zero
    0
}

// TODO: two usecases: if we are triggering this for a new, blank line:
// - it should return 0 when mass indenting stuff
// - it should look up the wrapper node and count it too when we press o/O
pub fn suggested_indent_for_pos(
    syntax: Option<&Syntax>,
    state: &State,
    pos: usize,
    new_line: bool,
) -> usize {
    if let Some(syntax) = syntax {
        let byte_start = state.doc.char_to_byte(pos);
        let node = get_highest_syntax_node_at_bytepos(syntax, byte_start);

        // TODO: special case for comments
        // TODO: if preserve_leading_whitespace
        calculate_indentation(node, new_line)
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
            "
mod test {
    fn hello_world() {
        1 + 1;

        let does_indentation_work = 1;

        let test_function = function_with_param(this_param,
            that_param
        );

        let test_function = function_with_param(
            this_param,
            that_param
        );

        let test_function = function_with_proper_indent(param1,
            param2,
        );

        let selection = Selection::new(
            changes
                .clone()
                .map(|(start, end, text): (usize, usize, Option<Tendril>)| {
                    let len = text.map(|text| text.len()).unwrap() - 1; // minus newline
                    let pos = start + len;
                    Range::new(pos, pos)
                })
                .collect(),
            0,
        );

        return;
    }
}

impl<A, D> MyTrait<A, D> for YourType
where
    A: TraitB + TraitC,
    D: TraitE + TraitF,
{

}
#[test]
//

",
        );

        let state = State::new(doc);
        // TODO: set_language
        let language_config = crate::syntax::LOADER
            .language_config_for_scope("source.rust")
            .unwrap();
        let highlight_config = language_config.highlight_config(&[]).unwrap().unwrap();
        let syntax = Syntax::new(&state.doc, highlight_config.clone());

        // {
        //   {
        //     1 + 1
        //   }
        // }
        // assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 1), 0); // {
        // assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 2), 1); // {
        // assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 3), 2); //
        // assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 4), 1); // }
        // assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 5), 0); // }

        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 1), 0); // mod
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 2), 1); // fn
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 3), 2); // 1 + 1
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 4), 0); //
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 5), 2); // does_indentation_work
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 7), 2); // let test_function
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 8), 3); // that_param
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 9), 2); // );
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 10), 0); //
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 11), 2); // let test_function
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 12), 3); // this_param
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 13), 3); // that_param
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 14), 2); // );
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 15), 0); //
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 16), 2); // let test_function
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 17), 3); // param2
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 18), 2); // );
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 20), 2); // let selection
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 21), 3); // changes
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 22), 4); // clone()
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 23), 4); // map()
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 24), 5); // let len
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 25), 5); // let pos
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 26), 5); // Range

        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 27), 4); // })
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 28), 4); // .collect(),
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 29), 3); // 0,
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 30), 2); // })
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 31), 0); //
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 32), 2); // return;
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 33), 1); // }
        assert_eq!(suggested_indent_for_line(Some(&syntax), &state, 34), 0); // }
    }
}
