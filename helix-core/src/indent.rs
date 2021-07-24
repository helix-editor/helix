use crate::{
    find_first_non_whitespace_char,
    syntax::{IndentQuery, LanguageConfiguration, Syntax},
    tree_sitter::Node,
    RopeSlice,
};

/// To determine indentation of a newly inserted line, figure out the indentation at the last col
/// of the previous line.
#[allow(dead_code)]
fn indent_level_for_line(line: RopeSlice, tab_width: usize) -> usize {
    let mut len = 0;
    for ch in line.chars() {
        match ch {
            '\t' => len += tab_width,
            ' ' => len += 1,
            _ => break,
        }
    }

    len / tab_width
}

/// Find the highest syntax node at position.
/// This is to identify the column where this node (e.g., an HTML closing tag) ends.
fn get_highest_syntax_node_at_bytepos(syntax: &Syntax, pos: usize) -> Option<Node> {
    let tree = syntax.tree();

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

fn calculate_indentation(query: &IndentQuery, node: Option<Node>, newline: bool) -> usize {
    // NOTE: can't use contains() on query because of comparing Vec<String> and &str
    // https://doc.rust-lang.org/std/vec/struct.Vec.html#method.contains

    let mut increment: isize = 0;

    let mut node = match node {
        Some(node) => node,
        None => return 0,
    };

    let mut prev_start = node.start_position().row;

    // if we're calculating indentation for a brand new line then the current node will become the
    // parent node. We need to take it's indentation level into account too.
    let node_kind = node.kind();
    if newline && query.indent.contains(node_kind) {
        increment += 1;
    }

    while let Some(parent) = node.parent() {
        let parent_kind = parent.kind();
        let start = parent.start_position().row;

        // detect deeply nested indents in the same line
        // .map(|a| {       <-- ({ is two scopes
        //     let len = 1; <-- indents one level
        // })               <-- }) is two scopes
        let starts_same_line = start == prev_start;

        if query.outdent.contains(node.kind()) && !starts_same_line {
            // we outdent by skipping the rules for the current level and jumping up
            // node = parent;
            increment -= 1;
            // continue;
        }

        if query.indent.contains(parent_kind) // && not_first_or_last_sibling
            && !starts_same_line
        {
            // println!("is_scope {}", parent_kind);
            prev_start = start;
            increment += 1
        }

        // if last_scope && increment > 0 && ...{ ignore }

        node = parent;
    }

    increment.max(0) as usize
}

#[allow(dead_code)]
fn suggested_indent_for_line(
    language_config: &LanguageConfiguration,
    syntax: Option<&Syntax>,
    text: RopeSlice,
    line_num: usize,
    _tab_width: usize,
) -> usize {
    if let Some(start) = find_first_non_whitespace_char(text.line(line_num)) {
        return suggested_indent_for_pos(
            Some(language_config),
            syntax,
            text,
            start + text.line_to_char(line_num),
            false,
        );
    };

    // if the line is blank, indent should be zero
    0
}

// TODO: two usecases: if we are triggering this for a new, blank line:
// - it should return 0 when mass indenting stuff
// - it should look up the wrapper node and count it too when we press o/O
pub fn suggested_indent_for_pos(
    language_config: Option<&LanguageConfiguration>,
    syntax: Option<&Syntax>,
    text: RopeSlice,
    pos: usize,
    new_line: bool,
) -> usize {
    if let (Some(query), Some(syntax)) = (
        language_config.and_then(|config| config.indent_query()),
        syntax,
    ) {
        let byte_start = text.char_to_byte(pos);
        let node = get_highest_syntax_node_at_bytepos(syntax, byte_start);

        // let config = load indentation query config from Syntax(should contain language_config)

        // TODO: special case for comments
        // TODO: if preserve_leading_whitespace
        calculate_indentation(query, node, new_line)
    } else {
        // TODO: heuristics for non-tree sitter grammars
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Rope;

    #[test]
    fn test_indent_level() {
        let tab_width = 4;
        let line = Rope::from("        fn new"); // 8 spaces
        assert_eq!(indent_level_for_line(line.slice(..), tab_width), 2);
        let line = Rope::from("\t\t\tfn new"); // 3 tabs
        assert_eq!(indent_level_for_line(line.slice(..), tab_width), 3);
        // mixed indentation
        let line = Rope::from("\t    \tfn new"); // 1 tab, 4 spaces, tab
        assert_eq!(indent_level_for_line(line.slice(..), tab_width), 3);
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

pub fn change<I>(document: &Document, changes: I) -> Self
where
    I: IntoIterator<Item = Change> + ExactSizeIterator,
{
    [
        1,
        2,
        3,
    ];
    (
        1,
        2
    );
    true
}
",
        );

        let doc = Rope::from(doc);
        use crate::syntax::{
            Configuration, IndentationConfiguration, LanguageConfiguration, Loader,
        };
        use once_cell::sync::OnceCell;
        let loader = Loader::new(Configuration {
            language: vec![LanguageConfiguration {
                scope: "source.rust".to_string(),
                file_types: vec!["rs".to_string()],
                language_id: "Rust".to_string(),
                highlight_config: OnceCell::new(),
                config: None,
                //
                roots: vec![],
                comment_token: None,
                auto_format: false,
                language_server: None,
                indent: Some(IndentationConfiguration {
                    tab_width: 4,
                    unit: String::from("    "),
                }),
                indent_query: OnceCell::new(),
            }],
        });

        // set runtime path so we can find the queries
        let mut runtime = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        runtime.push("../runtime");
        std::env::set_var("HELIX_RUNTIME", runtime.to_str().unwrap());

        let language_config = loader.language_config_for_scope("source.rust").unwrap();
        let highlight_config = language_config.highlight_config(&[]).unwrap();
        let syntax = Syntax::new(&doc, highlight_config.clone());
        let text = doc.slice(..);
        let tab_width = 4;

        for i in 0..doc.len_lines() {
            let line = text.line(i);
            let indent = indent_level_for_line(line, tab_width);
            assert_eq!(
                suggested_indent_for_line(&language_config, Some(&syntax), text, i, tab_width),
                indent,
                "line {}: {}",
                i,
                line
            );
        }
    }
}
