use tree_sitter::TreeCursor;

use crate::{
    chars::{char_is_line_ending, char_is_whitespace},
    syntax::{IndentQuery, IndentQueryNode, IndentQueryScopes, LanguageConfiguration, Syntax},
    tree_sitter::Node,
    Rope, RopeSlice,
};

/// Enum representing indentation style.
///
/// Only values 1-8 are valid for the `Spaces` variant.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IndentStyle {
    Tabs,
    Spaces(u8),
}

impl IndentStyle {
    /// Creates an `IndentStyle` from an indentation string.
    ///
    /// For example, passing `"    "` (four spaces) will create `IndentStyle::Spaces(4)`.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn from_str(indent: &str) -> Self {
        // XXX: do we care about validating the input more than this?  Probably not...?
        debug_assert!(!indent.is_empty() && indent.len() <= 8);

        if indent.starts_with(' ') {
            IndentStyle::Spaces(indent.len() as u8)
        } else {
            IndentStyle::Tabs
        }
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        match *self {
            IndentStyle::Tabs => "\t",
            IndentStyle::Spaces(1) => " ",
            IndentStyle::Spaces(2) => "  ",
            IndentStyle::Spaces(3) => "   ",
            IndentStyle::Spaces(4) => "    ",
            IndentStyle::Spaces(5) => "     ",
            IndentStyle::Spaces(6) => "      ",
            IndentStyle::Spaces(7) => "       ",
            IndentStyle::Spaces(8) => "        ",

            // Unsupported indentation style.  This should never happen,
            // but just in case fall back to two spaces.
            IndentStyle::Spaces(n) => {
                debug_assert!(n > 0 && n <= 8); // Always triggers. `debug_panic!()` wanted.
                "  "
            }
        }
    }
}

/// Attempts to detect the indentation style used in a document.
///
/// Returns the indentation style if the auto-detect confidence is
/// reasonably high, otherwise returns `None`.
pub fn auto_detect_indent_style(document_text: &Rope) -> Option<IndentStyle> {
    // Build a histogram of the indentation *increases* between
    // subsequent lines, ignoring lines that are all whitespace.
    //
    // Index 0 is for tabs, the rest are 1-8 spaces.
    let histogram: [usize; 9] = {
        let mut histogram = [0; 9];
        let mut prev_line_is_tabs = false;
        let mut prev_line_leading_count = 0usize;

        // Loop through the lines, checking for and recording indentation
        // increases as we go.
        'outer: for line in document_text.lines().take(1000) {
            let mut c_iter = line.chars();

            // Is first character a tab or space?
            let is_tabs = match c_iter.next() {
                Some('\t') => true,
                Some(' ') => false,

                // Ignore blank lines.
                Some(c) if char_is_line_ending(c) => continue,

                _ => {
                    prev_line_is_tabs = false;
                    prev_line_leading_count = 0;
                    continue;
                }
            };

            // Count the line's total leading tab/space characters.
            let mut leading_count = 1;
            let mut count_is_done = false;
            for c in c_iter {
                match c {
                    '\t' if is_tabs && !count_is_done => leading_count += 1,
                    ' ' if !is_tabs && !count_is_done => leading_count += 1,

                    // We stop counting if we hit whitespace that doesn't
                    // qualify as indent or doesn't match the leading
                    // whitespace, but we don't exit the loop yet because
                    // we still want to determine if the line is blank.
                    c if char_is_whitespace(c) => count_is_done = true,

                    // Ignore blank lines.
                    c if char_is_line_ending(c) => continue 'outer,

                    _ => break,
                }

                // Bound the worst-case execution time for weird text files.
                if leading_count > 256 {
                    continue 'outer;
                }
            }

            // If there was an increase in indentation over the previous
            // line, update the histogram with that increase.
            if (prev_line_is_tabs == is_tabs || prev_line_leading_count == 0)
                && prev_line_leading_count < leading_count
            {
                if is_tabs {
                    histogram[0] += 1;
                } else {
                    let amount = leading_count - prev_line_leading_count;
                    if amount <= 8 {
                        histogram[amount] += 1;
                    }
                }
            }

            // Store this line's leading whitespace info for use with
            // the next line.
            prev_line_is_tabs = is_tabs;
            prev_line_leading_count = leading_count;
        }

        // Give more weight to tabs, because their presence is a very
        // strong indicator.
        histogram[0] *= 2;

        histogram
    };

    // Find the most frequent indent, its frequency, and the frequency of
    // the next-most frequent indent.
    let indent = histogram
        .iter()
        .enumerate()
        .max_by_key(|kv| kv.1)
        .unwrap()
        .0;
    let indent_freq = histogram[indent];
    let indent_freq_2 = *histogram
        .iter()
        .enumerate()
        .filter(|kv| kv.0 != indent)
        .map(|kv| kv.1)
        .max()
        .unwrap();

    // Return the the auto-detected result if we're confident enough in its
    // accuracy, based on some heuristics.
    if indent_freq >= 1 && (indent_freq_2 as f64 / indent_freq as f64) < 0.66 {
        Some(match indent {
            0 => IndentStyle::Tabs,
            _ => IndentStyle::Spaces(indent as u8),
        })
    } else {
        None
    }
}

/// To determine indentation of a newly inserted line, figure out the indentation at the last col
/// of the previous line.
pub fn indent_level_for_line(line: RopeSlice, tab_width: usize) -> usize {
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

/// The indent that is added for a single tree-sitter node/a single line.
/// This is different from the total indent ([IndentResult]) because multiple indents/outdents on the same line don't stack.
struct AddedIndent {
    indent: bool,
    outdent: bool,
}
impl AddedIndent {
    fn new() -> Self {
        AddedIndent {
            indent: false,
            outdent: false,
        }
    }
    /// Combine this [AddedIndent] with other.
    /// This is intended for indents that apply to the same line.
    fn combine_with(&mut self, other: &AddedIndent) {
        self.indent |= other.indent;
        self.outdent |= other.outdent;
    }
}

/// The total indent for some line of code.
/// This is usually constructed by successively adding instances of [AddedIndent]
struct IndentResult {
    /// The total indent (the number of indent levels).
    /// The string that this results in depends on the indent style (spaces or tabs, etc.)
    indent: i32,
}
impl IndentResult {
    fn new() -> Self {
        IndentResult { indent: 0 }
    }
    /// Add the given [AddedIndent] to the [IndentResult].
    /// The [AddedIndent] should be the combination of all the added indents for one line.
    fn add(&mut self, added: &AddedIndent) {
        if added.indent && !added.outdent {
            self.indent += 1;
        } else if added.outdent && !added.indent {
            self.indent -= 1;
        }
    }
    fn as_string(&self, indent_style: &IndentStyle) -> String {
        indent_style.as_str().repeat(0.max(self.indent) as usize)
    }
}

// Get the node where to start the indent query (this is usually just the lowest node containing byte_pos)
fn get_lowest_node<'a>(root: Node<'a>, _query: &IndentQuery, byte_pos: usize) -> Option<Node<'a>> {
    root.descendant_for_byte_range(byte_pos, byte_pos)
    // TODO Special handling for languages like python
}

// Computes for node and all ancestors whether they are the first node on their line
// The first entry in the return value represents the root node, the last one the node itself
fn get_first_in_line(mut node: Node, byte_pos: usize, new_line: bool) -> Vec<bool> {
    let mut first_in_line = Vec::new();
    loop {
        if let Some(prev) = node.prev_sibling() {
            // If we insert a new line, the first node at/after the cursor is considered to be the first in its line
            let first = prev.end_position().row != node.start_position().row
                || (new_line && node.start_byte() >= byte_pos && prev.start_byte() < byte_pos);
            first_in_line.push(Some(first));
        } else {
            // Nodes that have no previous siblings are first in their line if and only if their parent is
            // (which we don't know yet)
            first_in_line.push(None);
        }
        if let Some(parent) = node.parent() {
            node = parent;
        } else {
            break;
        }
    }

    let mut result = Vec::with_capacity(first_in_line.len());
    let mut parent_is_first = true; // The root node is by definition the first node in its line
    for first in first_in_line.into_iter().rev() {
        if let Some(first) = first {
            result.push(first);
            parent_is_first = first;
        } else {
            result.push(parent_is_first);
        }
    }
    result
}

// This assumes that the kind matches and checks for all the other conditions
fn matches<'a>(query_node: &IndentQueryNode, node: Node<'a>, cursor: &mut TreeCursor<'a>) -> bool {
    match query_node {
        IndentQueryNode::SimpleNode(_) => true,
        IndentQueryNode::ComplexNode {
            kind: _,
            kind_not_in,
            parent_kind_in,
            field_name_in,
        } => {
            if !kind_not_in.is_empty() {
                let kind = node.kind();
                if kind_not_in.iter().any(|k| k == kind) {
                    return false;
                }
            }
            if !parent_kind_in.is_empty() {
                let parent_matches = node.parent().map_or(false, |p| {
                    let parent_kind = p.kind();
                    parent_kind_in
                        .iter()
                        .any(|kind| kind.as_str() == parent_kind)
                });
                if !parent_matches {
                    return false;
                }
            }
            if !field_name_in.is_empty() {
                let parent = match node.parent() {
                    None => {
                        return false;
                    }
                    Some(p) => p,
                };
                cursor.reset(parent);
                debug_assert!(cursor.goto_first_child());
                loop {
                    if cursor.node() == node {
                        if let Some(cursor_name) = cursor.field_name() {
                            if !field_name_in.iter().any(|n| n == cursor_name) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                        break;
                    }
                    debug_assert!(cursor.goto_next_sibling());
                }
            }
            true
        }
    }
}

fn contains_match<'a>(
    scope: &[IndentQueryNode],
    node: Node<'a>,
    cursor: &mut TreeCursor<'a>,
) -> bool {
    let current_kind = node.kind();
    let first = scope.partition_point(|n| n.kind() < Some(current_kind));
    if scope[first..]
        .iter()
        .take_while(|n| n.kind() == Some(current_kind))
        .any(|named_node| matches(named_node, node, cursor))
    {
        return true;
    }
    scope
        .iter()
        .take_while(|n| n.kind().is_none())
        .any(|unnamed_node| matches(unnamed_node, node, cursor))
}

/// Returns whether the given scopes contain a match for this line and/or for the next
fn scopes_contain_match<'a>(
    scopes: &IndentQueryScopes,
    node: Node<'a>,
    cursor: &mut TreeCursor<'a>,
) -> (bool, bool) {
    let match_for_line = contains_match(&scopes.all, node, cursor);
    let match_for_next = contains_match(&scopes.tail, node, cursor);
    (match_for_line, match_for_next)
}

/// The added indent for the line of the node and the next line
fn added_indent<'a>(
    query: &IndentQuery,
    node: Node<'a>,
    cursor: &mut TreeCursor<'a>,
) -> (AddedIndent, AddedIndent) {
    let (indent, next_indent) = scopes_contain_match(&query.indent, node, cursor);
    let (outdent, next_outdent) = scopes_contain_match(&query.outdent, node, cursor);
    let line = AddedIndent { indent, outdent };
    let next = AddedIndent {
        indent: next_indent,
        outdent: next_outdent,
    };
    (line, next)
}

/// Use the syntax tree to determine the indentation for a given position.
/// This can be used in 2 ways:
///
/// - To get the correct indentation for an existing line (new_line=false), not necessarily equal to the current indentation.
///   - In this case, pos should be inside the first tree-sitter node on that line.
///     In most cases, this can just be the first non-whitespace on that line.
///   - To get the indentation for a new line (new_line=true). This behaves like the first usecase if the part of the current line
///     after pos were moved to a new line.
///
/// The indentation is determined by traversing all the tree-sitter nodes containing the position.
/// Each of these nodes produces some [AddedIndent] for:
///
/// - The line of the (beginning of the) node. This is defined by the scope `all` if this is the first node on its line.
/// - The line after the node. This is defined by:
///   - The scope `tail`.
///   - The scope `all` if this node is not the first node on its line.
/// Intuitively, `all` applies to everything contained in this node while `tail` applies to everything except for the first line of the node.
/// The indents from different nodes for the same line are then combined.
/// The [IndentResult] is simply the sum of the [AddedIndent] for all lines.
///
/// Specifying which line exactly an [AddedIndent] applies to is important because indents on the same line combine differently than indents on different lines:
/// ```ignore
/// some_function(|| {
///     // Both the function parameters as well as the contained block should be indented.
///     // Because they are on the same line, this only yields one indent level
/// });
/// ```
///
/// ```ignore
/// some_function(
///     parm1,
///     || {
///         // Here we get 2 indent levels because the 'parameters' and the 'block' node begin on different lines
///     },
/// );
/// ```
fn treesitter_indent_for_pos(
    query: &IndentQuery,
    syntax: &Syntax,
    indent_style: &IndentStyle,
    text: RopeSlice,
    line: usize,
    pos: usize,
    new_line: bool,
) -> Option<String> {
    let mut cursor = syntax.tree().walk();
    let byte_pos = text.char_to_byte(pos);
    let mut node = get_lowest_node(syntax.tree().root_node(), query, byte_pos)?;
    let mut first_in_line = get_first_in_line(node, byte_pos, new_line);

    let mut result = IndentResult::new();
    // We always keep track of all the indent changes on one line, in order to only indent once
    // even if there are multiple "indent" nodes on the same line
    let mut indent_for_line = AddedIndent::new();
    let mut indent_for_line_below = AddedIndent::new();
    loop {
        let node_indents = added_indent(query, node, &mut cursor);
        if *first_in_line.last().unwrap() {
            indent_for_line.combine_with(&node_indents.0);
        } else {
            indent_for_line_below.combine_with(&node_indents.0);
        }
        indent_for_line_below.combine_with(&node_indents.1);

        if let Some(parent) = node.parent() {
            let mut node_line = node.start_position().row;
            let mut parent_line = parent.start_position().row;
            if node.start_position().row == line && new_line {
                // Also consider the line that will be inserted
                if node.start_byte() >= byte_pos {
                    node_line += 1;
                }
                if parent.start_byte() >= byte_pos {
                    parent_line += 1;
                }
            };
            if node_line != parent_line {
                if node_line < line + (new_line as usize) {
                    // Don't add indent for the line below the line of the query
                    result.add(&indent_for_line_below);
                }
                if node_line == parent_line + 1 {
                    indent_for_line_below = indent_for_line;
                } else {
                    result.add(&indent_for_line);
                    indent_for_line_below = AddedIndent::new();
                }
                indent_for_line = AddedIndent::new();
            }

            node = parent;
            first_in_line.pop();
        } else {
            result.add(&indent_for_line_below);
            result.add(&indent_for_line);
            break;
        }
    }
    Some(result.as_string(indent_style))
}

/// Returns the indentation for a new line.
/// This is done either using treesitter, or if that's not available by copying the indentation from the current line
#[allow(clippy::too_many_arguments)]
pub fn indent_for_newline(
    language_config: Option<&LanguageConfiguration>,
    syntax: Option<&Syntax>,
    indent_style: &IndentStyle,
    tab_width: usize,
    text: RopeSlice,
    line_before: usize,
    line_before_end_pos: usize,
    current_line: usize,
) -> String {
    if let (Some(query), Some(syntax)) = (
        language_config.and_then(|config| config.indent_query()),
        syntax,
    ) {
        if let Some(indent) = treesitter_indent_for_pos(
            query,
            syntax,
            indent_style,
            text,
            line_before,
            line_before_end_pos,
            true,
        ) {
            return indent;
        };
    }

    let indent_level = indent_level_for_line(text.line(current_line), tab_width);
    indent_style.as_str().repeat(indent_level)
}

pub fn get_scopes(syntax: Option<&Syntax>, text: RopeSlice, pos: usize) -> Vec<&'static str> {
    let mut scopes = Vec::new();
    if let Some(syntax) = syntax {
        let pos = text.char_to_byte(pos);
        let mut node = match syntax
            .tree()
            .root_node()
            .descendant_for_byte_range(pos, pos)
        {
            Some(node) => node,
            None => return scopes,
        };

        scopes.push(node.kind());

        while let Some(parent) = node.parent() {
            scopes.push(parent.kind());
            node = parent;
        }
    }

    scopes.reverse();
    scopes
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

        let mut really_long_variable_name_using_up_the_line =
            really_long_fn_that_should_definitely_go_on_the_next_line();
        really_long_variable_name_using_up_the_line =
            really_long_fn_that_should_definitely_go_on_the_next_line();
        really_long_variable_name_using_up_the_line |=
            really_long_fn_that_should_definitely_go_on_the_next_line();

        let (
            a_long_variable_name_in_this_tuple,
            b_long_variable_name_in_this_tuple,
            c_long_variable_name_in_this_tuple,
            d_long_variable_name_in_this_tuple,
            e_long_variable_name_in_this_tuple,
        ): (usize, usize, usize, usize, usize) =
            if really_long_fn_that_should_definitely_go_on_the_next_line() {
                (
                    03294239434,
                    1213412342314,
                    21231234134,
                    834534234549898789,
                    9879234234543853457,
                )
            } else {
                (0, 1, 2, 3, 4)
            };

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

        let doc = doc;
        use crate::diagnostic::Severity;
        use crate::syntax::{
            Configuration, IndentationConfiguration, LanguageConfiguration, Loader,
        };
        use once_cell::sync::OnceCell;
        let loader = Loader::new(Configuration {
            language: vec![LanguageConfiguration {
                scope: "source.rust".to_string(),
                file_types: vec!["rs".to_string()],
                shebangs: vec![],
                language_id: "Rust".to_string(),
                highlight_config: OnceCell::new(),
                config: None,
                //
                injection_regex: None,
                roots: vec![],
                comment_token: None,
                auto_format: false,
                diagnostic_severity: Severity::Warning,
                tree_sitter_library: None,
                language_server: None,
                indent: Some(IndentationConfiguration {
                    tab_width: 4,
                    unit: String::from("    "),
                }),
                indent_query: OnceCell::new(),
                textobject_query: OnceCell::new(),
            }],
        });

        // set runtime path so we can find the queries
        let mut runtime = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        runtime.push("../runtime");
        std::env::set_var("HELIX_RUNTIME", runtime.to_str().unwrap());

        let language_config = loader.language_config_for_scope("source.rust").unwrap();
        let highlight_config = language_config.highlight_config(&[]).unwrap();
        let syntax = Syntax::new(&doc, highlight_config, std::sync::Arc::new(loader));
        let text = doc.slice(..);

        for i in 0..doc.len_lines() {
            let line = text.line(i);
            if let Some(pos) = crate::find_first_non_whitespace_char(line) {
                let suggested_indent = treesitter_indent_for_pos(
                    language_config.indent_query().unwrap(),
                    &syntax,
                    &IndentStyle::Spaces(4),
                    text,
                    i,
                    text.line_to_char(i) + pos,
                    false,
                )
                .unwrap();
                assert!(
                    line.get_slice(..suggested_indent.chars().count())
                        .map_or(false, |s| s == suggested_indent),
                    "Wrong indentation on line {}:\n\"{}\" (original line)\n\"{}\" (suggested indentation)\n",
                    i,
                    line.slice(..line.len_chars()-1),
                    suggested_indent,
                );
            }
        }
    }
}
