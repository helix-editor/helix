use std::collections::HashMap;

use tree_sitter::{Query, QueryCursor, QueryPredicateArg};

use crate::{
    chars::{char_is_line_ending, char_is_whitespace},
    syntax::{LanguageConfiguration, RopeProvider, Syntax},
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

/// Computes for node and all ancestors whether they are the first node on their line.
/// The first entry in the return value represents the root node, the last one the node itself
fn get_first_in_line(mut node: Node, new_line_byte_pos: Option<usize>) -> Vec<bool> {
    let mut first_in_line = Vec::new();
    loop {
        if let Some(prev) = node.prev_sibling() {
            // If we insert a new line, the first node at/after the cursor is considered to be the first in its line
            let first = prev.end_position().row != node.start_position().row
                || new_line_byte_pos.map_or(false, |byte_pos| {
                    node.start_byte() >= byte_pos && prev.start_byte() < byte_pos
                });
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

/// The total indent for some line of code.
/// This is usually constructed in one of 2 ways:
/// - Successively add indent captures to get the (added) indent from a single line
/// - Successively add the indent results for each line
#[derive(Default)]
pub struct Indentation {
    /// The total indent (the number of indent levels) is defined as max(0, indent-outdent).
    /// The string that this results in depends on the indent style (spaces or tabs, etc.)
    indent: usize,
    outdent: usize,
}
impl Indentation {
    /// Add some other [Indentation] to this.
    /// The added indent should be the total added indent from one line
    fn add_line(&mut self, added: &Indentation) {
        if added.indent > 0 && added.outdent == 0 {
            self.indent += 1;
        } else if added.outdent > 0 && added.indent == 0 {
            self.outdent += 1;
        }
    }
    /// Add an indent capture to this indent.
    /// All the captures that are added in this way should be on the same line.
    fn add_capture(&mut self, added: IndentCaptureType) {
        match added {
            IndentCaptureType::Indent => {
                self.indent = 1;
            }
            IndentCaptureType::Outdent => {
                self.outdent = 1;
            }
        }
    }
    fn as_string(&self, indent_style: &IndentStyle) -> String {
        let indent_level = if self.indent >= self.outdent {
            self.indent - self.outdent
        } else {
            log::warn!("Encountered more outdent than indent nodes while calculating indentation: {} outdent, {} indent", self.outdent, self.indent);
            0
        };
        indent_style.as_str().repeat(indent_level)
    }
}

/// An indent definition which corresponds to a capture from the indent query
struct IndentCapture {
    capture_type: IndentCaptureType,
    scope: IndentScope,
}
#[derive(Clone, Copy)]
enum IndentCaptureType {
    Indent,
    Outdent,
}
impl IndentCaptureType {
    fn default_scope(&self) -> IndentScope {
        match self {
            IndentCaptureType::Indent => IndentScope::Tail,
            IndentCaptureType::Outdent => IndentScope::All,
        }
    }
}
/// This defines which part of a node an [IndentCapture] applies to.
/// Each [IndentCaptureType] has a default scope, but the scope can be changed
/// with `#set!` property declarations.
#[derive(Clone, Copy)]
enum IndentScope {
    /// The indent applies to the whole node
    All,
    /// The indent applies to everything except for the first line of the node
    Tail,
}

/// A capture from the indent query which does not define an indent but extends
/// the range of a node. This is used before the indent is calculated.
enum ExtendCapture {
    ExtendIndented,
    StopExtend,
}

/// The result of running a tree-sitter indent query. This stores for
/// each node (identified by its ID) the relevant captures (already filtered
/// by predicates).
struct IndentQueryResult {
    indent_captures: HashMap<usize, Vec<IndentCapture>>,
    extend_captures: HashMap<usize, Vec<ExtendCapture>>,
}

fn query_indents(
    query: &Query,
    syntax: &Syntax,
    cursor: &mut QueryCursor,
    text: RopeSlice,
    range: std::ops::Range<usize>,
    // Position of the (optional) newly inserted line break.
    // Given as (line, byte_pos)
    new_line_break: Option<(usize, usize)>,
) -> IndentQueryResult {
    let mut indent_captures: HashMap<usize, Vec<IndentCapture>> = HashMap::new();
    let mut extend_captures: HashMap<usize, Vec<ExtendCapture>> = HashMap::new();
    cursor.set_byte_range(range);
    // Iterate over all captures from the query
    for m in cursor.matches(query, syntax.tree().root_node(), RopeProvider(text)) {
        // Skip matches where not all custom predicates are fulfilled
        if !query.general_predicates(m.pattern_index).iter().all(|pred| {
            match pred.operator.as_ref() {
                "not-kind-eq?" => match (pred.args.get(0), pred.args.get(1)) {
                    (
                        Some(QueryPredicateArg::Capture(capture_idx)),
                        Some(QueryPredicateArg::String(kind)),
                    ) => {
                        let node = m.nodes_for_capture_index(*capture_idx).next();
                        match node {
                            Some(node) => node.kind()!=kind.as_ref(),
                            _ => true,
                        }
                    }
                    _ => {
                        panic!("Invalid indent query: Arguments to \"not-kind-eq?\" must be a capture and a string");
                    }
                },
                "same-line?" | "not-same-line?" => {
                    match (pred.args.get(0), pred.args.get(1)) {
                        (
                            Some(QueryPredicateArg::Capture(capt1)),
                            Some(QueryPredicateArg::Capture(capt2))
                        ) => {
                            let get_line_num = |node: Node| {
                                let mut node_line = node.start_position().row;
                                // Adjust for the new line that will be inserted
                                if let Some((line, byte)) = new_line_break {
                                    if node_line==line && node.start_byte()>=byte {
                                        node_line += 1;
                                    }
                                }
                                node_line
                            };
                            let n1 = m.nodes_for_capture_index(*capt1).next();
                            let n2 = m.nodes_for_capture_index(*capt2).next();
                            match (n1, n2) {
                                (Some(n1), Some(n2)) => {
                                    let same_line = get_line_num(n1)==get_line_num(n2);
                                    same_line==(pred.operator.as_ref()=="same-line?")
                                }
                                _ => true,
                            }
                        }
                        _ => {
                            panic!("Invalid indent query: Arguments to \"{}\" must be 2 captures", pred.operator);
                        }
                    }
                }
                _ => {
                    panic!(
                        "Invalid indent query: Unknown predicate (\"{}\")",
                        pred.operator
                    );
                }
            }
        }) {
            continue;
        }
        for capture in m.captures {
            let capture_name = query.capture_names()[capture.index as usize].as_str();
            let capture_type = match capture_name {
                "indent" => IndentCaptureType::Indent,
                "outdent" => IndentCaptureType::Outdent,
                "extend-indented" => {
                    extend_captures
                        .entry(capture.node.id())
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(ExtendCapture::ExtendIndented);
                    continue;
                }
                "stop-extend" => {
                    extend_captures
                        .entry(capture.node.id())
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(ExtendCapture::StopExtend);
                    continue;
                }
                _ => {
                    // Ignore any unknown captures (these may be needed for predicates such as #match?)
                    continue;
                }
            };
            let scope = capture_type.default_scope();
            let mut indent_capture = IndentCapture {
                capture_type,
                scope,
            };
            // Apply additional settings for this capture
            for property in query.property_settings(m.pattern_index) {
                match property.key.as_ref() {
                    "scope" => {
                        indent_capture.scope = match property.value.as_deref() {
                            Some("all") => IndentScope::All,
                            Some("tail") => IndentScope::Tail,
                            Some(s) => {
                                panic!("Invalid indent query: Unknown value for \"scope\" property (\"{}\")", s);
                            }
                            None => {
                                panic!(
                                    "Invalid indent query: Missing value for \"scope\" property"
                                );
                            }
                        }
                    }
                    _ => {
                        panic!(
                            "Invalid indent query: Unknown property \"{}\"",
                            property.key
                        );
                    }
                }
            }
            indent_captures
                .entry(capture.node.id())
                // Most entries only need to contain a single IndentCapture
                .or_insert_with(|| Vec::with_capacity(1))
                .push(indent_capture);
        }
    }
    IndentQueryResult {
        indent_captures,
        extend_captures,
    }
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
/// Each of these nodes produces some [Indentation] for:
///
/// - The line of the (beginning of the) node. This is defined by the scope `all` if this is the first node on its line.
/// - The line after the node. This is defined by:
///   - The scope `tail`.
///   - The scope `all` if this node is not the first node on its line.
/// Intuitively, `all` applies to everything contained in this node while `tail` applies to everything except for the first line of the node.
/// The indents from different nodes for the same line are then combined.
/// The result [Indentation] is simply the sum of the [Indentation] for all lines.
///
/// Specifying which line exactly an [Indentation] applies to is important because indents on the same line combine differently than indents on different lines:
/// ```ignore
/// some_function(|| {
///     // Both the function parameters as well as the contained block should be indented.
///     // Because they are on the same line, this only yields one indent level
/// });
/// ```
///
/// ```ignore
/// some_function(
///     param1,
///     || {
///         // Here we get 2 indent levels because the 'parameters' and the 'block' node begin on different lines
///     },
/// );
/// ```
#[allow(clippy::too_many_arguments)]
pub fn treesitter_indent_for_pos(
    query: &Query,
    syntax: &Syntax,
    indent_style: &IndentStyle,
    tab_width: usize,
    text: RopeSlice,
    line: usize,
    pos: usize,
    new_line: bool,
) -> Option<String> {
    let byte_pos = text.char_to_byte(pos);
    // The innermost tree-sitter node which is considered for the indent
    // computation. It may change if some predeceding node is extended
    let mut node = syntax
        .tree()
        .root_node()
        .descendant_for_byte_range(byte_pos, byte_pos)?;
    let (query_result, prev_child) = crate::syntax::PARSER.with(|ts_parser| {
        let mut ts_parser = ts_parser.borrow_mut();
        let mut cursor = ts_parser.cursors.pop().unwrap_or_else(QueryCursor::new);
        // The query range should intersect with all nodes directly preceding
        // the cursor in case one of them is extended.
        // prev_child is the deepest such node.
        let (query_range, prev_child) = {
            // TODO Is there some way we can reuse this cursor?
            let mut tree_cursor = node.walk();
            let mut prev_child = None;
            for child in node.children(&mut tree_cursor) {
                if child.byte_range().end <= byte_pos {
                    prev_child = Some(child);
                }
            }
            match prev_child {
                Some(mut prev_child) => {
                    // Get the deepest directly preceding node
                    while prev_child.child_count() > 0 {
                        prev_child = prev_child.child(prev_child.child_count() - 1).unwrap();
                    }
                    (
                        prev_child.byte_range().end - 1..byte_pos + 1,
                        Some(prev_child),
                    )
                }
                None => (byte_pos..byte_pos + 1, None),
            }
        };
        let query_result = query_indents(
            query,
            syntax,
            &mut cursor,
            text,
            query_range,
            new_line.then(|| (line, byte_pos)),
        );
        ts_parser.cursors.push(cursor);
        (query_result, prev_child)
    });
    let indent_captures = query_result.indent_captures;
    let extend_captures = query_result.extend_captures;

    // Check for extend captures (starting with the deepest
    // candidate node and then going up the syntax tree).
    if let Some(mut prev_child) = prev_child {
        let mut stop_extend = false;
        while prev_child != node {
            let mut extend_node = false;
            if let Some(captures) = extend_captures.get(&prev_child.id()) {
                for capture in captures {
                    match capture {
                        ExtendCapture::StopExtend => {
                            stop_extend = true;
                        }
                        ExtendCapture::ExtendIndented => {
                            // We extend the node if
                            // - the cursor is on the same line as the end of the node OR
                            // - the line that the cursor is on is more indented than the
                            //   first line of the node
                            if prev_child.end_position().row == line {
                                extend_node = true;
                            } else {
                                let cursor_indent =
                                    indent_level_for_line(text.line(line), tab_width);
                                let node_indent = indent_level_for_line(
                                    text.line(prev_child.start_position().row),
                                    tab_width,
                                );
                                if cursor_indent > node_indent {
                                    extend_node = true;
                                }
                            }
                        }
                    }
                }
            }
            // If we encountered some `StopExtend` capture before, we don't
            // extend the node even if we otherwise would
            match (extend_node, stop_extend) {
                (true, true) => {
                    stop_extend = false;
                }
                (true, false) => {
                    node = prev_child;
                    break;
                }
                _ => {}
            };
            // This parent always exists since node is an ancestor of prev_child
            prev_child = prev_child.parent().unwrap();
        }
    }

    let mut first_in_line = get_first_in_line(node, new_line.then(|| byte_pos));

    let mut result = Indentation::default();
    // We always keep track of all the indent changes on one line, in order to only indent once
    // even if there are multiple "indent" nodes on the same line
    let mut indent_for_line = Indentation::default();
    let mut indent_for_line_below = Indentation::default();
    loop {
        // This can safely be unwrapped because `first_in_line` contains
        // one entry for each ancestor of the node (which is what we iterate over)
        let is_first = *first_in_line.last().unwrap();
        // Apply all indent definitions for this node
        if let Some(definitions) = indent_captures.get(&node.id()) {
            for definition in definitions {
                match definition.scope {
                    IndentScope::All => {
                        if is_first {
                            indent_for_line.add_capture(definition.capture_type);
                        } else {
                            indent_for_line_below.add_capture(definition.capture_type);
                        }
                    }
                    IndentScope::Tail => {
                        indent_for_line_below.add_capture(definition.capture_type);
                    }
                }
            }
        }

        if let Some(parent) = node.parent() {
            let mut node_line = node.start_position().row;
            let mut parent_line = parent.start_position().row;
            if node_line == line && new_line {
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
                    result.add_line(&indent_for_line_below);
                }
                if node_line == parent_line + 1 {
                    indent_for_line_below = indent_for_line;
                } else {
                    result.add_line(&indent_for_line);
                    indent_for_line_below = Indentation::default();
                }
                indent_for_line = Indentation::default();
            }

            node = parent;
            first_in_line.pop();
        } else {
            // Only add the indentation for the line below if that line
            // is not after the line that the indentation is calculated for.
            if (node.start_position().row < line)
                || (new_line && node.start_position().row == line && node.start_byte() < byte_pos)
            {
                result.add_line(&indent_for_line_below);
            }
            result.add_line(&indent_for_line);
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
            tab_width,
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
}
