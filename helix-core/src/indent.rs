use crate::{
    find_first_non_whitespace_char,
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
    let mut increment: i32 = 0;

    // Hardcoded for rust for now
    let indent_scopes = &[
        // indent except first or block?
        "while_expression",
        "for_expression",
        "loop_expression",
        "if_expression",
        "if_let_expression",
        // "match_expression",
        // "match_arm",
    ];

    // this is for multiline things, such as:
    // self.method()
    //  .chain()
    //  .chain()
    //  where the first line isn't indented
    let indent_except_first_scopes = &[
        "block",
        "match_block",
        "arguments",
        "declaration_list",
        "field_declaration_list",
        "enum_variant_list",
        // "function_item",
        // "closure_expression",
        "binary_expression",
        "field_expression",
        //
        "where_clause",
        //
        "use_list",
    ];

    let outdent = &["where", "}", "]", ")"];

    let mut node = match node {
        Some(node) => node,
        None => return 0,
    };

    let mut prev_start = node.start_position().row;

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
        let start = parent.start_position().row;

        // println!(
        //     "name: {}\tparent: {}\trange:\t{} {}\tfirst={:?}\tlast={:?} start={} prev={}",
        //     node.kind(),
        //     parent.kind(),
        //     node.range().start_point,
        //     node.range().end_point,
        //     node.prev_sibling().is_none(),
        //     node.next_sibling().is_none(),
        //     node.start_position(),
        //     prev_start,
        // );

        // detect deeply nested indents in the same line
        let starts_same_line = start == prev_start;

        if outdent.contains(&node.kind()) {
            // we outdent by skipping the rules for the current level and jumping up
            // println!("skipping..");
            // node = parent;
            increment -= 1;
            // continue;
        }

        // TODO: problem seems to be, ({ is two scopes that merge into one.
        // so when seeing } we're supposed to jump all the way out of both scopes, but we only do
        // so for one.
        // .map(|a| {
        //     let len = 1;
        // })

        if (indent_scopes.contains(&parent_kind) // && not_first_or_last_sibling
            || indent_except_first_scopes.contains(&parent_kind))
            && !starts_same_line
        {
            // println!("is_scope {}", parent_kind);
            prev_start = start;
            increment += 1
        }

        // TODO: detect deeply nested indents in same line:
        // std::panic::set_hook(Box::new(move |info| {
        //     hook(info); <-- indent here is 1
        // }));

        // if last_scope && increment > 0 && ...{ ignore }

        node = parent;
    }

    assert!(increment >= 0);

    increment as usize
}

fn suggested_indent_for_line(syntax: Option<&Syntax>, state: &State, line_num: usize) -> usize {
    let line = state.doc.line(line_num);
    let current = indent_level_for_line(line);

    if let Some(start) = find_first_non_whitespace_char(state.doc.slice(..), line_num) {
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
use std::{
    io::{self, stdout, Stdout, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
}
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
match test {
    Some(a) => 1,
    None => {
        unimplemented!()
    }
}
std::panic::set_hook(Box::new(move |info| {
    hook(info);
}));

{ { {
    1
}}}

pub fn change<I>(state: &State, changes: I) -> Self
where
    I: IntoIterator<Item = Change> + ExactSizeIterator,
{
    true
}
",
        );

        let state = State::new(doc);
        // TODO: set_language
        let language_config = crate::syntax::LOADER
            .language_config_for_scope("source.rust")
            .unwrap();
        let highlight_config = language_config.highlight_config(&[]).unwrap();
        let syntax = Syntax::new(&state.doc, highlight_config.clone());
        let text = state.doc.slice(..);

        for i in 0..state.doc.len_lines() {
            let line = text.line(i);
            let indent = indent_level_for_line(line);
            assert_eq!(
                suggested_indent_for_line(Some(&syntax), &state, i),
                indent,
                "line {}: {}",
                i,
                line
            );
        }
    }
}
