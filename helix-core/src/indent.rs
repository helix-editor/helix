use std::{borrow::Cow, collections::HashMap};

use tree_sitter::{Query, QueryCursor, QueryPredicateArg};

use crate::{
    chars::{char_is_line_ending, char_is_whitespace},
    find_first_non_whitespace_char,
    graphemes::{grapheme_width, tab_width_at},
    syntax::{LanguageConfiguration, RopeProvider, Syntax},
    tree_sitter::Node,
    Rope, RopeGraphemes, RopeSlice,
};

/// Enum representing indentation style.
///
/// Only values 1-8 are valid for the `Spaces` variant.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IndentStyle {
    Tabs,
    Spaces(u8),
}

// 16 spaces
const INDENTS: &str = "                ";
const MAX_INDENT: u8 = 16;

impl IndentStyle {
    /// Creates an `IndentStyle` from an indentation string.
    ///
    /// For example, passing `"    "` (four spaces) will create `IndentStyle::Spaces(4)`.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn from_str(indent: &str) -> Self {
        // XXX: do we care about validating the input more than this?  Probably not...?
        debug_assert!(!indent.is_empty() && indent.len() <= MAX_INDENT as usize);

        if indent.starts_with(' ') {
            IndentStyle::Spaces(indent.len().clamp(1, MAX_INDENT as usize) as u8)
        } else {
            IndentStyle::Tabs
        }
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        match *self {
            IndentStyle::Tabs => "\t",
            IndentStyle::Spaces(n) => {
                // Unsupported indentation style.  This should never happen,
                debug_assert!(n > 0 && n <= MAX_INDENT);

                // Either way, clamp to the nearest supported value
                let closest_n = n.clamp(1, MAX_INDENT) as usize;
                &INDENTS[0..closest_n]
            }
        }
    }

    #[inline]
    pub fn indent_width(&self, tab_width: usize) -> usize {
        match *self {
            IndentStyle::Tabs => tab_width,
            IndentStyle::Spaces(width) => width as usize,
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
    // Index 0 is for tabs, the rest are 1-MAX_INDENT spaces.
    let histogram: [usize; MAX_INDENT as usize + 1] = {
        let mut histogram = [0; MAX_INDENT as usize + 1];
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
                    if amount <= MAX_INDENT as usize {
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
pub fn indent_level_for_line(line: RopeSlice, tab_width: usize, indent_width: usize) -> usize {
    let mut len = 0;
    for ch in line.chars() {
        match ch {
            '\t' => len += tab_width_at(len, tab_width as u16),
            ' ' => len += 1,
            _ => break,
        }
    }

    len / indent_width
}

/// Create a string of tabs & spaces that has the same visual width as the given RopeSlice (independent of the tab width).
fn whitespace_with_same_width(text: RopeSlice) -> String {
    let mut s = String::new();
    for grapheme in RopeGraphemes::new(text) {
        if grapheme == "\t" {
            s.push('\t');
        } else {
            s.extend(std::iter::repeat(' ').take(grapheme_width(&Cow::from(grapheme))));
        }
    }
    s
}

fn add_indent_level(
    mut base_indent: String,
    added_indent_level: isize,
    indent_style: &IndentStyle,
    tab_width: usize,
) -> String {
    if added_indent_level >= 0 {
        // Adding a non-negative indent is easy, we can simply append the indent string
        base_indent.push_str(&indent_style.as_str().repeat(added_indent_level as usize));
        base_indent
    } else {
        // In this case, we want to return a prefix of `base_indent`.
        // Since the width of a tab depends on its offset, we cannot simply iterate over
        // the chars of `base_indent` in reverse until we have the desired indent reduction,
        // instead we iterate over them twice in forward direction.
        let mut base_indent_width: usize = 0;
        for c in base_indent.chars() {
            base_indent_width += match c {
                '\t' => tab_width_at(base_indent_width, tab_width as u16),
                // This is only true since `base_indent` consists only of tabs and spaces
                _ => 1,
            };
        }
        let target_indent_width = base_indent_width
            .saturating_sub((-added_indent_level) as usize * indent_style.indent_width(tab_width));
        let mut reduced_indent_width = 0;
        for (i, c) in base_indent.char_indices() {
            if reduced_indent_width >= target_indent_width {
                base_indent.truncate(i);
                return base_indent;
            }
            reduced_indent_width += match c {
                '\t' => tab_width_at(base_indent_width, tab_width as u16),
                _ => 1,
            };
        }
        base_indent
    }
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
/// The string that this indentation defines starts with the string contained in the align field (unless it is None), followed by:
/// - max(0, indent - outdent) tabs, if tabs are used for indentation
/// - max(0, indent - outdent)*indent_width spaces, if spaces are used for indentation
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Indentation<'a> {
    indent: usize,
    indent_always: usize,
    outdent: usize,
    outdent_always: usize,
    /// The alignment, as a string containing only tabs & spaces. Storing this as a string instead of e.g.
    /// the (visual) width ensures that the alignment is preserved even if the tab width changes.
    align: Option<RopeSlice<'a>>,
}

impl<'a> Indentation<'a> {
    /// Add some other [Indentation] to this.
    /// The added indent should be the total added indent from one line.
    /// Indent should always be added starting from the bottom (or equivalently, the innermost tree-sitter node).
    fn add_line(&mut self, added: Indentation<'a>) {
        // Align overrides the indent from outer scopes.
        if self.align.is_some() {
            return;
        }
        if added.align.is_some() {
            self.align = added.align;
            return;
        }
        self.indent += added.indent;
        self.indent_always += added.indent_always;
        self.outdent += added.outdent;
        self.outdent_always += added.outdent_always;
    }

    /// Add an indent capture to this indent.
    /// Only captures that apply to the same line should be added together in this way (otherwise use `add_line`)
    /// and the captures should be added starting from the innermost tree-sitter node (currently this only matters
    /// if multiple `@align` patterns occur on the same line).
    fn add_capture(&mut self, added: IndentCaptureType<'a>) {
        match added {
            IndentCaptureType::Indent => {
                if self.indent_always == 0 {
                    self.indent = 1;
                }
            }
            IndentCaptureType::IndentAlways => {
                // any time we encounter an `indent.always` on the same line, we
                // want to cancel out all regular indents
                self.indent_always += 1;
                self.indent = 0;
            }
            IndentCaptureType::Outdent => {
                if self.outdent_always == 0 {
                    self.outdent = 1;
                }
            }
            IndentCaptureType::OutdentAlways => {
                self.outdent_always += 1;
                self.outdent = 0;
            }
            IndentCaptureType::Align(align) => {
                if self.align.is_none() {
                    self.align = Some(align);
                }
            }
        }
    }
    fn net_indent(&self) -> isize {
        (self.indent + self.indent_always) as isize
            - ((self.outdent + self.outdent_always) as isize)
    }
    /// Convert `self` into a string, taking into account the computed and actual indentation of some other line.
    fn relative_indent(
        &self,
        other_computed_indent: &Self,
        other_leading_whitespace: RopeSlice,
        indent_style: &IndentStyle,
        tab_width: usize,
    ) -> Option<String> {
        if self.align == other_computed_indent.align {
            // If self and baseline are either not aligned to anything or both aligned the same way,
            // we can simply take `other_leading_whitespace` and add some indent / outdent to it (in the second
            // case, the alignment should already be accounted for in `other_leading_whitespace`).
            let indent_diff = self.net_indent() - other_computed_indent.net_indent();
            Some(add_indent_level(
                String::from(other_leading_whitespace),
                indent_diff,
                indent_style,
                tab_width,
            ))
        } else {
            // If the alignment of both lines is different, we cannot compare their indentation in any meaningful way
            None
        }
    }
    pub fn to_string(&self, indent_style: &IndentStyle, tab_width: usize) -> String {
        add_indent_level(
            self.align
                .map_or_else(String::new, whitespace_with_same_width),
            self.net_indent(),
            indent_style,
            tab_width,
        )
    }
}

/// An indent definition which corresponds to a capture from the indent query
#[derive(Debug)]
struct IndentCapture<'a> {
    capture_type: IndentCaptureType<'a>,
    scope: IndentScope,
}
#[derive(Debug, Clone, PartialEq)]
enum IndentCaptureType<'a> {
    Indent,
    IndentAlways,
    Outdent,
    OutdentAlways,
    /// Alignment given as a string of whitespace
    Align(RopeSlice<'a>),
}

impl<'a> IndentCaptureType<'a> {
    fn default_scope(&self) -> IndentScope {
        match self {
            IndentCaptureType::Indent | IndentCaptureType::IndentAlways => IndentScope::Tail,
            IndentCaptureType::Outdent | IndentCaptureType::OutdentAlways => IndentScope::All,
            IndentCaptureType::Align(_) => IndentScope::All,
        }
    }
}
/// This defines which part of a node an [IndentCapture] applies to.
/// Each [IndentCaptureType] has a default scope, but the scope can be changed
/// with `#set!` property declarations.
#[derive(Debug, Clone, Copy)]
enum IndentScope {
    /// The indent applies to the whole node
    All,
    /// The indent applies to everything except for the first line of the node
    Tail,
}

/// A capture from the indent query which does not define an indent but extends
/// the range of a node. This is used before the indent is calculated.
#[derive(Debug)]
enum ExtendCapture {
    Extend,
    PreventOnce,
}

/// The result of running a tree-sitter indent query. This stores for
/// each node (identified by its ID) the relevant captures (already filtered
/// by predicates).
#[derive(Debug)]
struct IndentQueryResult<'a> {
    indent_captures: HashMap<usize, Vec<IndentCapture<'a>>>,
    extend_captures: HashMap<usize, Vec<ExtendCapture>>,
}

fn get_node_start_line(node: Node, new_line_byte_pos: Option<usize>) -> usize {
    let mut node_line = node.start_position().row;
    // Adjust for the new line that will be inserted
    if new_line_byte_pos.map_or(false, |pos| node.start_byte() >= pos) {
        node_line += 1;
    }
    node_line
}
fn get_node_end_line(node: Node, new_line_byte_pos: Option<usize>) -> usize {
    let mut node_line = node.end_position().row;
    // Adjust for the new line that will be inserted (with a strict inequality since end_byte is exclusive)
    if new_line_byte_pos.map_or(false, |pos| node.end_byte() > pos) {
        node_line += 1;
    }
    node_line
}

fn query_indents<'a>(
    query: &Query,
    syntax: &Syntax,
    cursor: &mut QueryCursor,
    text: RopeSlice<'a>,
    range: std::ops::Range<usize>,
    new_line_byte_pos: Option<usize>,
) -> IndentQueryResult<'a> {
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
                            let n1 = m.nodes_for_capture_index(*capt1).next();
                            let n2 = m.nodes_for_capture_index(*capt2).next();
                            match (n1, n2) {
                                (Some(n1), Some(n2)) => {
                                    let n1_line = get_node_start_line(n1, new_line_byte_pos);
                                    let n2_line = get_node_start_line(n2, new_line_byte_pos);
                                    let same_line = n1_line == n2_line;
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
                "one-line?" | "not-one-line?" => match pred.args.get(0) {
                    Some(QueryPredicateArg::Capture(capture_idx)) => {
                        let node = m.nodes_for_capture_index(*capture_idx).next();

                        match node {
                            Some(node) => {
                                let (start_line, end_line) = (get_node_start_line(node,new_line_byte_pos), get_node_end_line(node, new_line_byte_pos));
                                let one_line = end_line == start_line;
                                one_line != (pred.operator.as_ref() == "not-one-line?")
                            },
                            _ => true,
                        }
                    }
                    _ => {
                        panic!("Invalid indent query: Arguments to \"not-kind-eq?\" must be a capture and a string");
                    }
                },
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
        // A list of pairs (node_id, indent_capture) that are added by this match.
        // They cannot be added to indent_captures immediately since they may depend on other captures (such as an @anchor).
        let mut added_indent_captures: Vec<(usize, IndentCapture)> = Vec::new();
        // The row/column position of the optional anchor in this query
        let mut anchor: Option<tree_sitter::Node> = None;
        for capture in m.captures {
            let capture_name = query.capture_names()[capture.index as usize].as_str();
            let capture_type = match capture_name {
                "indent" => IndentCaptureType::Indent,
                "indent.always" => IndentCaptureType::IndentAlways,
                "outdent" => IndentCaptureType::Outdent,
                "outdent.always" => IndentCaptureType::OutdentAlways,
                // The alignment will be updated to the correct value at the end, when the anchor is known.
                "align" => IndentCaptureType::Align(RopeSlice::from("")),
                "anchor" => {
                    if anchor.is_some() {
                        log::error!("Invalid indent query: Encountered more than one @anchor in the same match.")
                    } else {
                        anchor = Some(capture.node);
                    }
                    continue;
                }
                "extend" => {
                    extend_captures
                        .entry(capture.node.id())
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(ExtendCapture::Extend);
                    continue;
                }
                "extend.prevent-once" => {
                    extend_captures
                        .entry(capture.node.id())
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(ExtendCapture::PreventOnce);
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
            added_indent_captures.push((capture.node.id(), indent_capture))
        }
        for (node_id, mut capture) in added_indent_captures {
            // Set the anchor for all align queries.
            if let IndentCaptureType::Align(_) = capture.capture_type {
                let anchor = match anchor {
                    None => {
                        log::error!(
                            "Invalid indent query: @align requires an accompanying @anchor."
                        );
                        continue;
                    }
                    Some(anchor) => anchor,
                };
                capture.capture_type = IndentCaptureType::Align(
                    text.line(anchor.start_position().row)
                        .byte_slice(0..anchor.start_position().column),
                );
            }
            indent_captures
                .entry(node_id)
                .or_insert_with(|| Vec::with_capacity(1))
                .push(capture);
        }
    }

    let result = IndentQueryResult {
        indent_captures,
        extend_captures,
    };

    log::trace!("indent result = {:?}", result);

    result
}

/// Handle extend queries. deepest_preceding is the deepest descendant of node that directly precedes the cursor position.
/// Any ancestor of deepest_preceding which is also a descendant of node may be "extended". In that case, node will be updated,
/// so that the indent computation starts with the correct syntax node.
fn extend_nodes<'a>(
    node: &mut Node<'a>,
    mut deepest_preceding: Node<'a>,
    extend_captures: &HashMap<usize, Vec<ExtendCapture>>,
    text: RopeSlice,
    line: usize,
    tab_width: usize,
    indent_width: usize,
) {
    let mut stop_extend = false;

    while deepest_preceding != *node {
        let mut extend_node = false;
        // This will be set to true if this node is captured, regardless of whether
        // it actually will be extended (e.g. because the cursor isn't indented
        // more than the node).
        let mut node_captured = false;
        if let Some(captures) = extend_captures.get(&deepest_preceding.id()) {
            for capture in captures {
                match capture {
                    ExtendCapture::PreventOnce => {
                        stop_extend = true;
                    }
                    ExtendCapture::Extend => {
                        node_captured = true;
                        // We extend the node if
                        // - the cursor is on the same line as the end of the node OR
                        // - the line that the cursor is on is more indented than the
                        //   first line of the node
                        if deepest_preceding.end_position().row == line {
                            extend_node = true;
                        } else {
                            let cursor_indent =
                                indent_level_for_line(text.line(line), tab_width, indent_width);
                            let node_indent = indent_level_for_line(
                                text.line(deepest_preceding.start_position().row),
                                tab_width,
                                indent_width,
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
        if node_captured && stop_extend {
            stop_extend = false;
        } else if extend_node && !stop_extend {
            *node = deepest_preceding;
            break;
        }
        // If the tree contains a syntax error, `deepest_preceding` may not
        // have a parent despite being a descendant of `node`.
        deepest_preceding = match deepest_preceding.parent() {
            Some(parent) => parent,
            None => return,
        }
    }
}

/// Prepare an indent query by computing:
/// - The node from which to start the query (this is non-trivial due to `@extend` captures)
/// - The indent captures for all relevant nodes.
#[allow(clippy::too_many_arguments)]
fn init_indent_query<'a, 'b>(
    query: &Query,
    syntax: &'a Syntax,
    text: RopeSlice<'b>,
    tab_width: usize,
    indent_width: usize,
    line: usize,
    byte_pos: usize,
    new_line_byte_pos: Option<usize>,
) -> Option<(Node<'a>, HashMap<usize, Vec<IndentCapture<'b>>>)> {
    // The innermost tree-sitter node which is considered for the indent
    // computation. It may change if some predeceding node is extended
    let mut node = syntax
        .tree()
        .root_node()
        .descendant_for_byte_range(byte_pos, byte_pos)?;

    let (query_result, deepest_preceding) = {
        // The query range should intersect with all nodes directly preceding
        // the position of the indent query in case one of them is extended.
        let mut deepest_preceding = None; // The deepest node preceding the indent query position
        let mut tree_cursor = node.walk();
        for child in node.children(&mut tree_cursor) {
            if child.byte_range().end <= byte_pos {
                deepest_preceding = Some(child);
            }
        }
        deepest_preceding = deepest_preceding.map(|mut prec| {
            // Get the deepest directly preceding node
            while prec.child_count() > 0 {
                prec = prec.child(prec.child_count() - 1).unwrap();
            }
            prec
        });
        let query_range = deepest_preceding
            .map(|prec| prec.byte_range().end - 1..byte_pos + 1)
            .unwrap_or(byte_pos..byte_pos + 1);

        crate::syntax::PARSER.with(|ts_parser| {
            let mut ts_parser = ts_parser.borrow_mut();
            let mut cursor = ts_parser.cursors.pop().unwrap_or_else(QueryCursor::new);
            let query_result = query_indents(
                query,
                syntax,
                &mut cursor,
                text,
                query_range,
                new_line_byte_pos,
            );
            ts_parser.cursors.push(cursor);
            (query_result, deepest_preceding)
        })
    };
    let extend_captures = query_result.extend_captures;

    // Check for extend captures, potentially changing the node that the indent calculation starts with
    if let Some(deepest_preceding) = deepest_preceding {
        extend_nodes(
            &mut node,
            deepest_preceding,
            &extend_captures,
            text,
            line,
            tab_width,
            indent_width,
        );
    }
    Some((node, query_result.indent_captures))
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
pub fn treesitter_indent_for_pos<'a>(
    query: &Query,
    syntax: &Syntax,
    tab_width: usize,
    indent_width: usize,
    text: RopeSlice<'a>,
    line: usize,
    pos: usize,
    new_line: bool,
) -> Option<Indentation<'a>> {
    let byte_pos = text.char_to_byte(pos);
    let new_line_byte_pos = new_line.then_some(byte_pos);
    let (mut node, mut indent_captures) = init_indent_query(
        query,
        syntax,
        text,
        tab_width,
        indent_width,
        line,
        byte_pos,
        new_line_byte_pos,
    )?;
    let mut first_in_line = get_first_in_line(node, new_line.then_some(byte_pos));

    let mut result = Indentation::default();
    // We always keep track of all the indent changes on one line, in order to only indent once
    // even if there are multiple "indent" nodes on the same line
    let mut indent_for_line = Indentation::default();
    let mut indent_for_line_below = Indentation::default();

    loop {
        // This can safely be unwrapped because `first_in_line` contains
        // one entry for each ancestor of the node (which is what we iterate over)
        let is_first = *first_in_line.last().unwrap();

        // Apply all indent definitions for this node.
        // Since we only iterate over each node once, we can remove the
        // corresponding captures from the HashMap to avoid cloning them.
        if let Some(definitions) = indent_captures.remove(&node.id()) {
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
            let node_line = get_node_start_line(node, new_line_byte_pos);
            let parent_line = get_node_start_line(parent, new_line_byte_pos);

            if node_line != parent_line {
                // Don't add indent for the line below the line of the query
                if node_line < line + (new_line as usize) {
                    result.add_line(indent_for_line_below);
                }

                if node_line == parent_line + 1 {
                    indent_for_line_below = indent_for_line;
                } else {
                    result.add_line(indent_for_line);
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
                result.add_line(indent_for_line_below);
            }
            result.add_line(indent_for_line);
            break;
        }
    }
    Some(result)
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
    let indent_width = indent_style.indent_width(tab_width);
    if let (Some(query), Some(syntax)) = (
        language_config.and_then(|config| config.indent_query()),
        syntax,
    ) {
        if let Some(indent) = treesitter_indent_for_pos(
            query,
            syntax,
            tab_width,
            indent_width,
            text,
            line_before,
            line_before_end_pos,
            true,
        ) {
            // We want to compute the indentation not only based on the
            // syntax tree but also on the actual indentation of a previous
            // line. This makes indentation computation more resilient to
            // incomplete queries, incomplete source code & differing indentation
            // styles for the same language.
            // However, using the indent of a previous line as a baseline may not
            // make sense, e.g. if it has a different alignment than the new line.
            // In order to prevent edge cases with long running times, we only try
            // a constant number of (non-empty) lines.
            const MAX_ATTEMPTS: usize = 2;
            let mut num_attempts = 0;
            for line_idx in (0..=line_before).rev() {
                let line = text.line(line_idx);
                let first_non_whitespace_char = match find_first_non_whitespace_char(line) {
                    Some(i) => i,
                    None => {
                        continue;
                    }
                };
                if let Some(indent) = (|| {
                    let computed_indent = treesitter_indent_for_pos(
                        query,
                        syntax,
                        tab_width,
                        indent_width,
                        text,
                        line_idx,
                        text.line_to_char(line_idx) + first_non_whitespace_char,
                        false,
                    )?;
                    let leading_whitespace = line.slice(0..first_non_whitespace_char);
                    indent.relative_indent(
                        &computed_indent,
                        leading_whitespace,
                        indent_style,
                        tab_width,
                    )
                })() {
                    return indent;
                }
                num_attempts += 1;
                if num_attempts == MAX_ATTEMPTS {
                    break;
                }
            }
            return indent.to_string(indent_style, tab_width);
        };
    }
    // Fallback in case we either don't have indent queries or they failed for some reason
    let indent_level = indent_level_for_line(text.line(current_line), tab_width, indent_width);
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
        let indent_width = 4;
        let line = Rope::from("        fn new"); // 8 spaces
        assert_eq!(
            indent_level_for_line(line.slice(..), tab_width, indent_width),
            2
        );
        let line = Rope::from("\t\t\tfn new"); // 3 tabs
        assert_eq!(
            indent_level_for_line(line.slice(..), tab_width, indent_width),
            3
        );
        // mixed indentation
        let line = Rope::from("\t    \tfn new"); // 1 tab, 4 spaces, tab
        assert_eq!(
            indent_level_for_line(line.slice(..), tab_width, indent_width),
            3
        );
    }

    #[test]
    fn test_large_indent_level() {
        let tab_width = 16;
        let indent_width = 16;
        let line = Rope::from("                fn new"); // 16 spaces
        assert_eq!(
            indent_level_for_line(line.slice(..), tab_width, indent_width),
            1
        );
        let line = Rope::from("                                fn new"); // 32 spaces
        assert_eq!(
            indent_level_for_line(line.slice(..), tab_width, indent_width),
            2
        );
    }

    #[test]
    fn add_capture() {
        let indent = || Indentation {
            indent: 1,
            ..Default::default()
        };
        let indent_always = || Indentation {
            indent_always: 1,
            ..Default::default()
        };
        let outdent = || Indentation {
            outdent: 1,
            ..Default::default()
        };
        let outdent_always = || Indentation {
            outdent_always: 1,
            ..Default::default()
        };

        fn add_capture<'a>(
            mut indent: Indentation<'a>,
            capture: IndentCaptureType<'a>,
        ) -> Indentation<'a> {
            indent.add_capture(capture);
            indent
        }

        // adding an indent to no indent makes an indent
        assert_eq!(
            indent(),
            add_capture(Indentation::default(), IndentCaptureType::Indent)
        );
        assert_eq!(
            indent_always(),
            add_capture(Indentation::default(), IndentCaptureType::IndentAlways)
        );
        assert_eq!(
            outdent(),
            add_capture(Indentation::default(), IndentCaptureType::Outdent)
        );
        assert_eq!(
            outdent_always(),
            add_capture(Indentation::default(), IndentCaptureType::OutdentAlways)
        );

        // adding an indent to an already indented has no effect
        assert_eq!(indent(), add_capture(indent(), IndentCaptureType::Indent));
        assert_eq!(
            outdent(),
            add_capture(outdent(), IndentCaptureType::Outdent)
        );

        // adding an always to a regular makes it always
        assert_eq!(
            indent_always(),
            add_capture(indent(), IndentCaptureType::IndentAlways)
        );
        assert_eq!(
            outdent_always(),
            add_capture(outdent(), IndentCaptureType::OutdentAlways)
        );

        // adding an always to an always is additive
        assert_eq!(
            Indentation {
                indent_always: 2,
                ..Default::default()
            },
            add_capture(indent_always(), IndentCaptureType::IndentAlways)
        );
        assert_eq!(
            Indentation {
                outdent_always: 2,
                ..Default::default()
            },
            add_capture(outdent_always(), IndentCaptureType::OutdentAlways)
        );

        // adding regular to always should be associative
        assert_eq!(
            Indentation {
                indent_always: 1,
                ..Default::default()
            },
            add_capture(
                add_capture(indent(), IndentCaptureType::Indent),
                IndentCaptureType::IndentAlways
            )
        );
        assert_eq!(
            Indentation {
                indent_always: 1,
                ..Default::default()
            },
            add_capture(
                add_capture(indent(), IndentCaptureType::IndentAlways),
                IndentCaptureType::Indent
            )
        );
        assert_eq!(
            Indentation {
                outdent_always: 1,
                ..Default::default()
            },
            add_capture(
                add_capture(outdent(), IndentCaptureType::Outdent),
                IndentCaptureType::OutdentAlways
            )
        );
        assert_eq!(
            Indentation {
                outdent_always: 1,
                ..Default::default()
            },
            add_capture(
                add_capture(outdent(), IndentCaptureType::OutdentAlways),
                IndentCaptureType::Outdent
            )
        );
    }

    #[test]
    fn test_relative_indent() {
        let indent_style = IndentStyle::Spaces(4);
        let tab_width: usize = 4;
        let no_align = [
            Indentation::default(),
            Indentation {
                indent: 1,
                ..Default::default()
            },
            Indentation {
                indent: 5,
                outdent: 1,
                ..Default::default()
            },
        ];
        let align = no_align.clone().map(|indent| Indentation {
            align: Some(RopeSlice::from("12345")),
            ..indent
        });
        let different_align = Indentation {
            align: Some(RopeSlice::from("123456")),
            ..Default::default()
        };

        // Check that relative and absolute indentation computation are the same when the line we compare to is
        // indented as we expect.
        let check_consistency = |indent: &Indentation, other: &Indentation| {
            assert_eq!(
                indent.relative_indent(
                    other,
                    RopeSlice::from(other.to_string(&indent_style, tab_width).as_str()),
                    &indent_style,
                    tab_width
                ),
                Some(indent.to_string(&indent_style, tab_width))
            );
        };
        for a in &no_align {
            for b in &no_align {
                check_consistency(a, b);
            }
        }
        for a in &align {
            for b in &align {
                check_consistency(a, b);
            }
        }

        // Relative indent computation makes no sense if the alignment differs
        assert_eq!(
            align[0].relative_indent(
                &no_align[0],
                RopeSlice::from("      "),
                &indent_style,
                tab_width
            ),
            None
        );
        assert_eq!(
            align[0].relative_indent(
                &different_align,
                RopeSlice::from("      "),
                &indent_style,
                tab_width
            ),
            None
        );
    }
}
