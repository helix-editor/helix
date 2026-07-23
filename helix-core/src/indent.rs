use hashbrown::{HashMap, HashSet};
use std::borrow::Cow;

use helix_stdx::rope::RopeSliceExt;
use tree_house::TREE_SITTER_MATCH_LIMIT;

use crate::{
    chars::{char_is_line_ending, char_is_whitespace},
    graphemes::{grapheme_width, tab_width_at},
    syntax::{self, config::IndentationHeuristic},
    tree_sitter::{
        self,
        query::{InvalidPredicateError, UserPredicate},
        Capture, Grammar, InactiveQueryCursor, Node, Pattern, Query, QueryMatch, RopeInput,
    },
    Position, Rope, RopeSlice, Syntax, Tendril,
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
pub const MAX_INDENT: u8 = 16;

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
        // Gives less weight to single indent, as single spaces are
        // often used in certain languages' comment systems and rarely
        // used as the actual document indentation.
        if histogram[1] > 1 {
            histogram[1] /= 2;
        }

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

    // Return the auto-detected result if we're confident enough in its
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
    for grapheme in text.graphemes() {
        if grapheme == "\t" {
            s.push('\t');
        } else {
            s.extend(std::iter::repeat_n(
                ' ',
                grapheme_width(&Cow::from(grapheme)),
            ));
        }
    }
    s
}

/// normalizes indentation to tabs/spaces based on user configuration
/// This function does not change the actual indentation width, just the character
/// composition.
pub fn normalize_indentation(
    prefix: RopeSlice<'_>,
    line: RopeSlice<'_>,
    dst: &mut Tendril,
    indent_style: IndentStyle,
    tab_width: usize,
) -> usize {
    #[allow(deprecated)]
    let off = crate::visual_coords_at_pos(prefix, prefix.len_chars(), tab_width).col;
    let mut len = 0;
    let mut original_len = 0;
    for ch in line.chars() {
        match ch {
            '\t' => len += tab_width_at(len + off, tab_width as u16),
            ' ' => len += 1,
            _ => break,
        }
        original_len += 1;
    }
    if indent_style == IndentStyle::Tabs {
        dst.extend(std::iter::repeat_n('\t', len / tab_width));
        len %= tab_width;
    }
    dst.extend(std::iter::repeat_n(' ', len));
    original_len
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
        let base_indent_rope = RopeSlice::from(base_indent.as_str());
        #[allow(deprecated)]
        let base_indent_width =
            crate::visual_coords_at_pos(base_indent_rope, base_indent_rope.len_chars(), tab_width)
                .col;
        let target_indent_width = base_indent_width
            .saturating_sub((-added_indent_level) as usize * indent_style.indent_width(tab_width));
        #[allow(deprecated)]
        let char_end_idx = crate::pos_at_visual_coords(
            base_indent_rope,
            Position {
                row: 0,
                col: target_indent_width,
            },
            tab_width,
        );
        let byte_end_idx = base_indent_rope.char_to_byte(char_end_idx);
        base_indent.truncate(byte_end_idx);
        base_indent
    }
}

#[derive(Debug, Default)]
pub struct IndentQueryPredicates {
    not_kind_eq: Vec<(Capture, Box<str>)>,
    same_line: Option<(Capture, Capture, bool)>,
    one_line: Option<(Capture, bool)>,
}

impl IndentQueryPredicates {
    fn are_satisfied(
        &self,
        match_: &QueryMatch,
        text: RopeSlice,
        new_line_byte_pos: Option<u32>,
    ) -> bool {
        for (capture, not_expected_kind) in self.not_kind_eq.iter() {
            let node = match_.nodes_for_capture(*capture).next();
            if node.is_some_and(|n| n.kind() == not_expected_kind.as_ref()) {
                return false;
            }
        }

        if let Some((capture1, capture2, negated)) = self.same_line {
            let n1 = match_.nodes_for_capture(capture1).next();
            let n2 = match_.nodes_for_capture(capture2).next();
            let satisfied = n1.zip(n2).is_some_and(|(n1, n2)| {
                let n1_line = get_node_start_line(text, n1, new_line_byte_pos);
                let n2_line = get_node_start_line(text, n2, new_line_byte_pos);
                let same_line = n1_line == n2_line;
                same_line != negated
            });

            if !satisfied {
                return false;
            }
        }

        if let Some((capture, negated)) = self.one_line {
            let node = match_.nodes_for_capture(capture).next();
            let satisfied = node.is_some_and(|node| {
                let start_line = get_node_start_line(text, node, new_line_byte_pos);
                let end_line = get_node_end_line(text, node, new_line_byte_pos);
                let one_line = end_line == start_line;
                one_line != negated
            });

            if !satisfied {
                return false;
            }
        }

        true
    }
}

#[derive(Debug)]
pub struct IndentQuery {
    query: Query,
    /// Patterns carrying `(#set! "scope" "header")` — the only indent scope the
    /// containment engine reads.
    header_patterns: HashSet<Pattern>,
    predicates: HashMap<Pattern, IndentQueryPredicates>,
    indent_capture: Option<Capture>,
    indent_always_capture: Option<Capture>,
    outdent_capture: Option<Capture>,
    outdent_always_capture: Option<Capture>,
    align_capture: Option<Capture>,
    anchor_capture: Option<Capture>,
    extend_capture: Option<Capture>,
    extend_prevent_once_capture: Option<Capture>,
    opaque_capture: Option<Capture>,
}

impl IndentQuery {
    pub fn new(grammar: Grammar, source: &str) -> Result<Self, tree_sitter::query::ParseError> {
        let mut header_patterns = HashSet::new();
        let mut predicates: HashMap<Pattern, IndentQueryPredicates> = HashMap::new();
        let query = Query::new(grammar, source, |pattern, predicate| match predicate {
            UserPredicate::SetProperty { key: "scope", val } => {
                match val {
                    Some("header") => {
                        header_patterns.insert(pattern);
                    }
                    Some(other) => {
                        return Err(format!("unknown scope (#set! scope \"{other}\")").into())
                    }
                    None => return Err("missing scope value (#set! scope ...)".into()),
                };

                Ok(())
            }
            UserPredicate::Other(predicate) => {
                let name = predicate.name();
                match name {
                    "not-kind-eq?" => {
                        predicate.check_arg_count(2)?;
                        let capture = predicate.capture_arg(0)?;
                        let not_expected_kind = predicate.str_arg(1)?;

                        predicates
                            .entry(pattern)
                            .or_default()
                            .not_kind_eq
                            .push((capture, not_expected_kind.into()));
                        Ok(())
                    }
                    "same-line?" | "not-same-line?" => {
                        predicate.check_arg_count(2)?;
                        let capture1 = predicate.capture_arg(0)?;
                        let capture2 = predicate.capture_arg(1)?;
                        let negated = name == "not-same-line?";

                        predicates.entry(pattern).or_default().same_line =
                            Some((capture1, capture2, negated));
                        Ok(())
                    }
                    "one-line?" | "not-one-line?" => {
                        predicate.check_arg_count(1)?;
                        let capture = predicate.capture_arg(0)?;
                        let negated = name == "not-one-line?";

                        predicates.entry(pattern).or_default().one_line = Some((capture, negated));
                        Ok(())
                    }
                    _ => Err(InvalidPredicateError::unknown(UserPredicate::Other(
                        predicate,
                    ))),
                }
            }
            _ => Err(InvalidPredicateError::unknown(predicate)),
        })?;

        Ok(Self {
            header_patterns,
            predicates,
            indent_capture: query.get_capture("indent"),
            indent_always_capture: query.get_capture("indent.always"),
            outdent_capture: query.get_capture("outdent"),
            outdent_always_capture: query.get_capture("outdent.always"),
            align_capture: query.get_capture("align"),
            anchor_capture: query.get_capture("anchor"),
            extend_capture: query.get_capture("extend"),
            extend_prevent_once_capture: query.get_capture("extend.prevent-once"),
            opaque_capture: query.get_capture("opaque"),
            query,
        })
    }
}

/// The total indent for some line of code.
/// This is usually constructed in one of 2 ways:
/// - Successively add indent captures to get the (added) indent from a single line
/// - Successively add the indent results for each line
///   The string that this indentation defines starts with the string contained in the align field (unless it is None), followed by:
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
    /// `(#set! "scope" "header")`: open this `@indent`'s scope at the captured
    /// node's *header* (its parent's start line) instead of the node's own first
    /// line, so a brace-less body (`if (c)\n stmt;`) whose own first line needs
    /// the indent is contained.
    header: bool,
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
    /// Byte ranges of nodes captured by `@opaque`, collected in the same pass so the
    /// opaque-interior check doesn't need a second query.
    opaque_ranges: Vec<std::ops::Range<u32>>,
}

fn get_node_start_line(text: RopeSlice, node: &Node, new_line_byte_pos: Option<u32>) -> usize {
    let mut node_line = text.byte_to_line(node.start_byte() as usize);
    // Adjust for the new line that will be inserted
    if new_line_byte_pos.is_some_and(|pos| node.start_byte() >= pos) {
        node_line += 1;
    }
    node_line
}
fn get_node_end_line(text: RopeSlice, node: &Node, new_line_byte_pos: Option<u32>) -> usize {
    let mut node_line = text.byte_to_line(node.end_byte() as usize);
    // Adjust for the new line that will be inserted (with a strict inequality since end_byte is exclusive)
    if new_line_byte_pos.is_some_and(|pos| node.end_byte() > pos) {
        node_line += 1;
    }
    node_line
}

fn query_indents<'a>(
    query: &IndentQuery,
    root: &Node,
    text: RopeSlice<'a>,
    range: std::ops::Range<u32>,
    new_line_byte_pos: Option<u32>,
) -> IndentQueryResult<'a> {
    let mut indent_captures: HashMap<usize, Vec<IndentCapture>> = HashMap::new();
    let mut extend_captures: HashMap<usize, Vec<ExtendCapture>> = HashMap::new();
    let mut opaque_ranges: Vec<std::ops::Range<u32>> = Vec::new();

    let mut cursor = InactiveQueryCursor::new(range, TREE_SITTER_MATCH_LIMIT).execute_query(
        &query.query,
        root,
        RopeInput::new(text),
    );

    // Iterate over all captures from the query
    while let Some(m) = cursor.next_match() {
        // Skip matches where not all custom predicates are fulfilled
        if query
            .predicates
            .get(&m.pattern())
            .is_some_and(|preds| !preds.are_satisfied(&m, text, new_line_byte_pos))
        {
            continue;
        }
        // A list of pairs (node_id, indent_capture) that are added by this match.
        // They cannot be added to indent_captures immediately since they may depend on other captures (such as an @anchor).
        let mut added_indent_captures: Vec<(usize, IndentCapture)> = Vec::new();
        // The row/column position of the optional anchor in this query
        let mut anchor: Option<&Node> = None;
        for matched_node in m.matched_nodes() {
            let node_id = matched_node.node.id();
            let capture = Some(matched_node.capture);
            let capture_type = if capture == query.indent_capture {
                IndentCaptureType::Indent
            } else if capture == query.indent_always_capture {
                IndentCaptureType::IndentAlways
            } else if capture == query.outdent_capture {
                IndentCaptureType::Outdent
            } else if capture == query.outdent_always_capture {
                IndentCaptureType::OutdentAlways
            } else if capture == query.align_capture {
                IndentCaptureType::Align(RopeSlice::from(""))
            } else if capture == query.anchor_capture {
                if anchor.is_some() {
                    log::error!("Invalid indent query: Encountered more than one @anchor in the same match.")
                } else {
                    anchor = Some(&matched_node.node);
                }
                continue;
            } else if capture == query.extend_capture {
                extend_captures
                    .entry(node_id)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(ExtendCapture::Extend);
                continue;
            } else if capture == query.extend_prevent_once_capture {
                extend_captures
                    .entry(node_id)
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(ExtendCapture::PreventOnce);
                continue;
            } else if capture == query.opaque_capture {
                // Collected here so `treesitter_indent_for_pos` can test for an
                // opaque interior without a second query pass.
                opaque_ranges.push(matched_node.node.start_byte()..matched_node.node.end_byte());
                continue;
            } else {
                // Ignore any unknown captures (these may be needed for predicates such as #match?)
                continue;
            };

            // Apply additional settings for this capture
            let indent_capture = IndentCapture {
                capture_type,
                header: query.header_patterns.contains(&m.pattern()),
            };
            added_indent_captures.push((node_id, indent_capture))
        }
        for (node_id, mut capture) in added_indent_captures {
            // Set the anchor for all align queries.
            if let IndentCaptureType::Align(_) = capture.capture_type {
                let Some(anchor) = anchor else {
                    log::error!("Invalid indent query: @align requires an accompanying @anchor.");
                    continue;
                };
                let line = text.byte_to_line(anchor.start_byte() as usize);
                let line_start = text.line_to_byte(line);
                capture.capture_type = IndentCaptureType::Align(
                    text.byte_slice(line_start..anchor.start_byte() as usize),
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
        opaque_ranges,
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
                        if text.byte_to_line(deepest_preceding.end_byte() as usize) == line {
                            extend_node = true;
                        } else {
                            let cursor_indent =
                                indent_level_for_line(text.line(line), tab_width, indent_width);
                            let node_indent = indent_level_for_line(
                                text.line(
                                    text.byte_to_line(deepest_preceding.start_byte() as usize),
                                ),
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
            *node = deepest_preceding.clone();
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

/// When a newline is typed after a header (`while (c)`, `if x:`), the body it
/// opens begins on the *next* line — a following sibling the upward indent walk
/// never reaches, which is why brace-less bodies need either a wrapper `@indent`
/// rule or descent into the body before the containment walk starts.
///
/// Walk up from the deepest preceding node and return the immediate next sibling
/// of the first ancestor whose end lies before the cursor. The caller then
/// decides whether to actually descend — the signal is the query's
/// `(#set! "scope" "header")` annotation on that sibling, which is exactly how
/// brace-less body rules already mark their body capture (c/java/nix and parts
/// of ecma). This keeps tree-sitter field names out of the engine: the query
/// alone says what is a body. A statement-list child is not header-scoped in any
/// query, so `foo();` then `bar();` correctly finds no descent target.
fn candidate_body_for_new_line<'a>(
    deepest_preceding: &Node<'a>,
    byte_pos: u32,
) -> Option<Node<'a>> {
    let mut node = deepest_preceding.clone();
    while let Some(parent) = node.parent() {
        // Only consider ancestors that lie entirely before the cursor: those are
        // part of the header. Once an ancestor extends past the cursor it
        // *contains* the new line (an already-open body like `if (c) {`), which
        // the normal containment walk handles — stop.
        if node.end_byte() > byte_pos {
            break;
        }
        // The body the new line opens is the sibling immediately after this
        // header node (`while (c)` -> body, `if x:` -> consequence). Requiring
        // the *immediate* next sibling avoids jumping to a later body such as
        // an `else` arm while the cursor is still in the consequence.
        let mut cursor = parent.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().id() == node.id() {
                    if cursor.goto_next_sibling() {
                        let child = cursor.node();
                        if child.start_byte() >= byte_pos {
                            return Some(child);
                        }
                    }
                    break;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        node = parent;
    }
    None
}

/// Prepare an indent query by computing:
/// - The node from which to start the query (this is non-trivial due to `@extend` captures)
/// - The indent captures for all relevant nodes.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn init_indent_query<'a, 'b>(
    query: &IndentQuery,
    root: &Node<'a>,
    text: RopeSlice<'b>,
    tab_width: usize,
    indent_width: usize,
    line: usize,
    byte_pos: u32,
    new_line_byte_pos: Option<u32>,
) -> Option<(
    Node<'a>,
    HashMap<usize, Vec<IndentCapture<'b>>>,
    Vec<std::ops::Range<u32>>,
)> {
    // The innermost tree-sitter node which is considered for the indent
    // computation. It may change if some preceding node is extended
    let mut node = root.descendant_for_byte_range(byte_pos, byte_pos)?;

    let (query_result, deepest_preceding, candidate_body) = {
        // The query range should intersect with all nodes directly preceding
        // the position of the indent query in case one of them is extended.
        let mut deepest_preceding = None; // The deepest node preceding the indent query position
        for child in node.children() {
            if child.byte_range().end <= byte_pos {
                deepest_preceding = Some(child.clone());
            }
        }
        deepest_preceding = deepest_preceding.map(|mut prec| {
            // Get the deepest directly preceding node
            while prec.child_count() > 0 {
                prec = prec.child(prec.child_count() - 1).unwrap().clone();
            }
            prec
        });
        // When typing a newline, the body the new line opens is a sibling past
        // the cursor — extend the query range to cover it so its capture (and
        // in particular its `(#set! "scope" "header")` annotation) is present
        // in the result, the signal we use to drive descent.
        let candidate_body = new_line_byte_pos
            .and(deepest_preceding.as_ref())
            .and_then(|dp| candidate_body_for_new_line(dp, byte_pos));
        let upper = candidate_body
            .as_ref()
            .map(|b| b.end_byte())
            .unwrap_or(byte_pos + 1);
        let query_range = deepest_preceding
            .as_ref()
            .map(|prec| prec.byte_range().end - 1..upper)
            .unwrap_or(byte_pos..upper);

        let query_result = query_indents(query, root, text, query_range, new_line_byte_pos);
        (query_result, deepest_preceding, candidate_body)
    };
    let extend_captures = query_result.extend_captures;
    let opaque_ranges = query_result.opaque_ranges;

    // When typing a newline, descend into the body the new line opens so its own
    // scope governs the indent (structural replacement for wrapper/`@extend`
    // rules on delimited and brace-less bodies). The query — via its
    // `(#set! "scope" "header")` annotation on the body capture — is the single
    // source of truth for whether the candidate sibling is actually a body to
    // descend into. Otherwise fall back to `@extend`.
    let descended = candidate_body
        .as_ref()
        .filter(|body| {
            query_result
                .indent_captures
                .get(&body.id())
                .is_some_and(|defs| defs.iter().any(|c| c.header))
        })
        .map(|body| node = body.clone())
        .is_some();

    // Check for extend captures, potentially changing the node that the indent calculation starts with
    if let Some(deepest_preceding) = deepest_preceding {
        if !descended {
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
    }
    Some((node, query_result.indent_captures, opaque_ranges))
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
///
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
/// Whether `byte_pos` falls in the interior of an `@opaque` node spanning
/// `[start, end)` that began on an *earlier* line — i.e. a string / heredoc /
/// block-comment body whose leading whitespace is literal content. Shared by the
/// hot path (which collects `@opaque` ranges in the containment pass) and the
/// standalone [`is_opaque_interior`].
fn opaque_hit(start: u32, end: u32, byte_pos: u32, text: RopeSlice) -> bool {
    start <= byte_pos
        && byte_pos < end
        && text.byte_to_line(start as usize) < text.byte_to_line(byte_pos as usize)
}

/// Whether `byte_pos` lies in the *interior* of a node captured `@opaque` — a
/// string / heredoc / block-comment body that began on an earlier line. The
/// leading whitespace of such lines is literal content, so indentation must
/// leave it untouched rather than reformat it as code.
///
/// `treesitter_indent_for_pos` does not call this — it derives the same answer
/// from the captures of its single containment pass — so this exists for external
/// callers (e.g. the indent-check xtask) that only need the opaque test.
pub fn is_opaque_interior(
    query: &IndentQuery,
    syntax: &Syntax,
    text: RopeSlice,
    byte_pos: u32,
) -> bool {
    let Some(opaque) = query.opaque_capture else {
        return false;
    };
    let mut cursor = InactiveQueryCursor::new(byte_pos..byte_pos + 1, TREE_SITTER_MATCH_LIMIT)
        .execute_query(
            &query.query,
            &syntax.tree_for_byte_range(byte_pos, byte_pos).root_node(),
            RopeInput::new(text),
        );
    while let Some(m) = cursor.next_match() {
        for matched in m.matched_nodes() {
            if matched.capture == opaque
                && opaque_hit(
                    matched.node.start_byte(),
                    matched.node.end_byte(),
                    byte_pos,
                    text,
                )
            {
                return true;
            }
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub fn treesitter_indent_for_pos<'a>(
    query: &IndentQuery,
    syntax: &'a Syntax,
    loader: &syntax::Loader,
    tab_width: usize,
    indent_width: usize,
    text: RopeSlice<'a>,
    line: usize,
    pos: usize,
    new_line: bool,
) -> Option<Indentation<'a>> {
    let byte_pos = text.char_to_byte(pos) as u32;
    // Inside an injection layer indent with that language's query and tree,
    // then shift the whole result by the injection's base indent.
    let layer = syntax.layer_for_byte_range(byte_pos, byte_pos);
    // Only a *different* language injection needs special handling; a
    // same-language injection (e.g. rust macro token-trees) is already indented
    // correctly by the root walk, and offsetting it would double-count.
    let injected = (layer != syntax.root_layer()
        && syntax.layer(layer).language != syntax.root_language())
    .then(|| loader.indent_query(syntax.layer(layer).language))
    .flatten();
    let query = injected.unwrap_or(query);
    let tree = if injected.is_some() {
        syntax.tree_for_byte_range(byte_pos, byte_pos)
    } else {
        syntax.tree()
    };
    let root = tree.root_node();

    // Compute the indent by *scope containment*: the level of a line is the
    // number of `@indent` scopes that contain it (see `containment_accounting`).
    let (mut result, opaque) = containment_accounting(
        query,
        &root,
        text,
        tab_width,
        indent_width,
        line,
        byte_pos,
        new_line,
    )?;
    // Lines inside an `@opaque` node are literal content, not code: preserve their
    // existing leading whitespace instead of reformatting it.
    if opaque {
        let line_slice = text.line(line);
        let first = line_slice
            .first_non_whitespace_char()
            .unwrap_or_else(|| line_slice.len_chars());
        return Some(Indentation {
            align: Some(line_slice.slice(..first)),
            ..Default::default()
        });
    }
    // The injected walk above is relative to the injection's own tree root (the
    // embedded code starts at level 0). Shift it by the indent of the injection's
    // first content line so it sits where the host language placed the block.
    if injected.is_some() {
        result.indent += injection_base_level(syntax, text, byte_pos, tab_width, indent_width);
    }
    Some(result)
}

/// Compute indentation by *scope containment*.
///
/// Each node carrying an `@indent`/`@indent.always` capture defines a scope
/// spanning the lines after its start up to its end. The indent level of a line
/// is the number of such scopes containing it (collapsing scopes that open on the
/// same physical line to one level), minus any `@outdent` whose token begins the line.
/// `@align`/`@extend`/`@opaque` are kept as overlays.
#[allow(clippy::too_many_arguments)]
fn containment_accounting<'a>(
    query: &IndentQuery,
    root: &Node<'a>,
    text: RopeSlice<'a>,
    tab_width: usize,
    indent_width: usize,
    line: usize,
    byte_pos: u32,
    new_line: bool,
) -> Option<(Indentation<'a>, bool)> {
    let new_line_byte_pos = new_line.then_some(byte_pos);
    // Reuse the standard setup: this applies @extend repositioning / body-descent
    // and returns the per-node capture map (all ancestors of the cursor intersect
    // the query range, so their captures are present).
    let (start_node, captures, opaque_ranges) = init_indent_query(
        query,
        root,
        text,
        tab_width,
        indent_width,
        line,
        byte_pos,
        new_line_byte_pos,
    )?;

    // Opaque interior: determined from the captures of the pass above, so no
    // second query is needed. The caller preserves the line's existing leading
    // whitespace in this case.
    if opaque_ranges
        .iter()
        .any(|r| opaque_hit(r.start, r.end, byte_pos, text))
    {
        return Some((Indentation::default(), true));
    }

    // The line whose indent we are computing (post-insertion coordinates).
    let target_line = line + new_line as usize;

    let mut indent_levels: usize = 0;
    let mut indent_always: usize = 0;
    let mut outdent: usize = 0;
    let mut outdent_always: usize = 0;
    let mut align: Option<RopeSlice<'a>> = None;
    // Same-line collapse: a scope contributes at most one level per physical line
    // it opens on.
    let mut counted_start_lines: Vec<usize> = Vec::new();

    let mut node = start_node;
    loop {
        if let Some(defs) = captures.get(&node.id()) {
            // Aggregate this node's captures.
            let mut has_indent = false;
            let mut has_indent_always = false;
            let mut has_outdent = false;
            let mut has_outdent_always = false;
            let mut header_scoped = false;
            let mut node_align: Option<RopeSlice<'a>> = None;
            for def in defs {
                match def.capture_type {
                    IndentCaptureType::Indent => {
                        has_indent = true;
                        header_scoped |= def.header;
                    }
                    IndentCaptureType::IndentAlways => {
                        has_indent_always = true;
                        header_scoped |= def.header;
                    }
                    IndentCaptureType::Outdent => has_outdent = true,
                    IndentCaptureType::OutdentAlways => has_outdent_always = true,
                    IndentCaptureType::Align(a) => node_align = Some(a),
                }
            }

            let node_start = get_node_start_line(text, &node, new_line_byte_pos);
            let end = get_node_end_line(text, &node, new_line_byte_pos);
            // `scope "header"` (set by the query on a brace-less body rule such as
            // `(if_statement consequence: (_) @indent (#set! scope "header"))`)
            // opens the scope at the *header* — the captured node's parent — so the
            // body's own first line is contained. The query selects the body node,
            // so its parent is the header by construction.
            let start = if header_scoped {
                node.parent()
                    .map(|p| get_node_start_line(text, &p, new_line_byte_pos))
                    .unwrap_or(node_start)
            } else {
                node_start
            };
            // A scope contains the target line if it opens before it and closes on
            // or after it.
            let contains = start < target_line && target_line <= end;
            // A token-style outdent (`}`, `else`, `case`) sits on the line it
            // dedents.
            let opens_target = node_start == target_line;

            if (has_indent || has_indent_always) && (has_outdent || has_outdent_always) {
                // A node carrying both indent and outdent (e.g. swift's nested
                // else-if `(if_statement (if_statement) @outdent)`) cancels its own
                // level: it must not create a scope.
            } else {
                if has_indent && contains && !counted_start_lines.contains(&start) {
                    counted_start_lines.push(start);
                    indent_levels += 1;
                }
                if has_indent_always && contains {
                    indent_always += 1;
                }
                if has_outdent && opens_target {
                    outdent += 1;
                }
                if has_outdent_always && opens_target {
                    outdent_always += 1;
                }
            }
            // Innermost containing alignment wins (first seen on the upward walk).
            if let Some(a) = node_align {
                if contains && align.is_none() {
                    align = Some(a);
                }
            }
        }
        match node.parent() {
            Some(parent) => node = parent,
            None => break,
        }
    }

    // `@align` is an absolute alignment (to the anchor column); the containment
    // levels it spans are already encoded in that column, so don't stack them on
    // top.
    if align.is_some() {
        indent_levels = 0;
        indent_always = 0;
    }
    Some((
        Indentation {
            indent: indent_levels,
            indent_always,
            outdent,
            outdent_always,
            align,
        },
        false,
    ))
}

/// The indent level (in the host document) of the first content line of the
/// injection layer that `byte_pos` belongs to (i.e. where the embedded code
/// begins). Used to offset an injection's own (0-based) indentation.
fn injection_base_level(
    syntax: &Syntax,
    text: RopeSlice,
    byte_pos: u32,
    tab_width: usize,
    indent_width: usize,
) -> usize {
    let layer = syntax.layer_for_byte_range(byte_pos, byte_pos);
    // A content line of the injection is one whose first non-whitespace char
    // belongs to this layer (the opening `<script>` line does not — its first
    // token is host markup).
    let in_layer = |line: usize| match text.line(line).first_non_whitespace_char() {
        Some(fnw) => {
            let b = text.char_to_byte(text.line_to_char(line) + fnw) as u32;
            syntax.layer_for_byte_range(b, b) == layer
        }
        None => false,
    };
    let bl = text.byte_to_line(byte_pos as usize);
    let first = if in_layer(bl) {
        // Inside the injection: walk back to its first content line.
        let mut f = bl;
        while f > 0 && in_layer(f - 1) {
            f -= 1;
        }
        f
    } else {
        // Typing the first content line from the host boundary (e.g. just after
        // `<script>`): the injection begins on a following line.
        let total = text.len_lines();
        let mut f = bl + 1;
        while f + 1 < total && !in_layer(f) {
            f += 1;
        }
        f
    };
    indent_level_for_line(text.line(first), tab_width, indent_width)
}

/// Whether the token at `byte_pos` (expected to be the first token on its line)
/// is captured `@outdent`/`@outdent.always` by the indent query — i.e. the
/// editor dedents that line as the token is entered (a closing bracket, a
/// `case`/`else`/`except` keyword, …). Used to tell a *recoverable* typing
/// over-indent (the leading token will pull the line back) from a real one (a
/// plain statement that the new-line indent placed too deep).
pub fn is_outdent_token_at(
    query: &IndentQuery,
    syntax: &Syntax,
    text: RopeSlice,
    byte_pos: u32,
) -> bool {
    let root = syntax.tree_for_byte_range(byte_pos, byte_pos).root_node();
    let Some(mut node) = root.descendant_for_byte_range(byte_pos, byte_pos) else {
        return false;
    };
    let result = query_indents(query, &root, text, byte_pos..byte_pos + 1, None);
    // Check the leading token and any ancestor that starts at the same byte (the
    // @outdent may sit on the token itself or on a node it opens, e.g.
    // `(access_specifier) @outdent`).
    loop {
        if result.indent_captures.get(&node.id()).is_some_and(|caps| {
            caps.iter().any(|c| {
                matches!(
                    c.capture_type,
                    IndentCaptureType::Outdent | IndentCaptureType::OutdentAlways
                )
            })
        }) {
            return true;
        }
        match node.parent() {
            Some(parent) if parent.start_byte() == node.start_byte() => node = parent,
            _ => return false,
        }
    }
}

/// Returns the indentation for a new line.
/// This is done either using treesitter, or if that's not available by copying the indentation from the current line
#[allow(clippy::too_many_arguments)]
pub fn indent_for_newline(
    loader: &syntax::Loader,
    syntax: Option<&Syntax>,
    indent_heuristic: &IndentationHeuristic,
    indent_style: &IndentStyle,
    tab_width: usize,
    text: RopeSlice,
    line_before: usize,
    line_before_end_pos: usize,
    current_line: usize,
) -> String {
    let indent_width = indent_style.indent_width(tab_width);
    if let (
        IndentationHeuristic::TreeSitter | IndentationHeuristic::Hybrid,
        Some(query),
        Some(syntax),
    ) = (
        indent_heuristic,
        syntax.and_then(|syntax| loader.indent_query(syntax.root_language())),
        syntax,
    ) {
        if let Some(indent) = treesitter_indent_for_pos(
            query,
            syntax,
            loader,
            tab_width,
            indent_width,
            text,
            line_before,
            line_before_end_pos,
            true,
        ) {
            if *indent_heuristic == IndentationHeuristic::Hybrid {
                // We want to compute the indentation not only based on the
                // syntax tree but also on the actual indentation of a previous
                // line. This makes indentation computation more resilient to
                // incomplete queries, incomplete source code & differing indentation
                // styles for the same language.
                // However, using the indent of a previous line as a baseline may not
                // make sense, e.g. if it has a different alignment than the new line.
                // In order to prevent edge cases with long running times, we only try
                // a constant number of (non-empty) lines.
                const MAX_ATTEMPTS: usize = 4;
                let mut num_attempts = 0;
                for line_idx in (0..=line_before).rev() {
                    let line = text.line(line_idx);
                    let first_non_whitespace_char = match line.first_non_whitespace_char() {
                        Some(i) => i,
                        None => {
                            continue;
                        }
                    };
                    if let Some(indent) = (|| {
                        let computed_indent = treesitter_indent_for_pos(
                            query,
                            syntax,
                            loader,
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
            }
            return indent.to_string(indent_style, tab_width);
        };
    }
    // Fallback in case we either don't have indent queries or they failed for some reason
    let indent_level = indent_level_for_line(text.line(current_line), tab_width, indent_width);
    indent_style.as_str().repeat(indent_level)
}

pub fn get_scopes<'a>(syntax: Option<&'a Syntax>, text: RopeSlice, pos: usize) -> Vec<&'a str> {
    let mut scopes = Vec::new();
    if let Some(syntax) = syntax {
        let pos = text.char_to_byte(pos) as u32;
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
