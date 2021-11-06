use crate::{
    chars::{char_is_line_ending, char_is_whitespace},
    syntax::{IndentQuery, LanguageConfiguration, Syntax},
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

/// Calculate the indentation at a given treesitter node.
/// If newline is false, then any "indent" nodes on the line are ignored ("outdent" still applies).
/// This is because the indentation is only increased starting at the second line of the node.
fn calculate_indentation(
    query: &IndentQuery,
    node: Option<Node>,
    line: usize,
    newline: bool,
) -> usize {
    let mut increment: isize = 0;

    let mut node = match node {
        Some(node) => node,
        None => return 0,
    };

    let mut current_line = line;
    let mut consider_indent = newline;
    let mut increment_from_line: isize = 0;

    loop {
        let node_kind = node.kind();
        let start = node.start_position().row;
        if current_line != start {
            // Indent/dedent by at most one per line:
            // .map(|a| {       <-- ({ is two scopes
            //     let len = 1; <-- indents one level
            // })               <-- }) is two scopes
            if consider_indent || increment_from_line < 0 {
                increment += increment_from_line.signum();
            }
            increment_from_line = 0;
            current_line = start;
            consider_indent = true;
        }

        if query.outdent.contains(node_kind) {
            increment_from_line -= 1;
        }
        if query.indent.contains(node_kind) {
            increment_from_line += 1;
        }

        if let Some(parent) = node.parent() {
            node = parent;
        } else {
            break;
        }
    }
    if consider_indent || increment_from_line < 0 {
        increment += increment_from_line.signum();
    }
    increment.max(0) as usize
}

// TODO: two usecases: if we are triggering this for a new, blank line:
// - it should return 0 when mass indenting stuff
// - it should look up the wrapper node and count it too when we press o/O
pub fn suggested_indent_for_pos(
    language_config: Option<&LanguageConfiguration>,
    syntax: Option<&Syntax>,
    text: RopeSlice,
    pos: usize,
    line: usize,
    new_line: bool,
) -> Option<usize> {
    if let (Some(query), Some(syntax)) = (
        language_config.and_then(|config| config.indent_query()),
        syntax,
    ) {
        let byte_start = text.char_to_byte(pos);
        let node = get_highest_syntax_node_at_bytepos(syntax, byte_start);
        // TODO: special case for comments
        // TODO: if preserve_leading_whitespace
        Some(calculate_indentation(query, node, line, new_line))
    } else {
        None
    }
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
        let syntax = Syntax::new(&doc, highlight_config.clone(), std::sync::Arc::new(loader));
        let text = doc.slice(..);
        let tab_width = 4;

        for i in 0..doc.len_lines() {
            let line = text.line(i);
            if let Some(pos) = crate::find_first_non_whitespace_char(line) {
                let indent = indent_level_for_line(line, tab_width);
                assert_eq!(
                    suggested_indent_for_pos(
                        Some(&language_config),
                        Some(&syntax),
                        text,
                        text.line_to_char(i) + pos,
                        i,
                        false
                    ),
                    Some(indent),
                    "line {}: \"{}\"",
                    i,
                    line
                );
            }
        }
    }
}
