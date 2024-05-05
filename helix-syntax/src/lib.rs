use ::ropey::RopeSlice;
use ::tree_sitter::{Node, Parser, Point, Query, QueryCursor, Range, Tree};
use slotmap::HopSlotMap;

use std::borrow::Cow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str;
use std::sync::Arc;

use crate::injections_tree::LayerId;
use crate::parse::LayerUpdateFlags;

pub use crate::config::{read_query, HighlightConfiguration};
pub use crate::ropey::RopeProvider;
pub use merge::merge;
pub use pretty_print::pretty_print_tree;
pub use tree_cursor::TreeCursor;

mod config;
pub mod highlighter;
mod injections_tree;
mod merge;
mod parse;
mod pretty_print;
mod ropey;
mod tree_cursor;
pub mod tree_sitter;

#[derive(Debug)]
pub struct Syntax {
    layers: HopSlotMap<LayerId, LanguageLayer>,
    root: LayerId,
}

impl Syntax {
    pub fn new(
        source: RopeSlice,
        config: Arc<HighlightConfiguration>,
        injection_callback: impl Fn(&InjectionLanguageMarker) -> Option<Arc<HighlightConfiguration>>,
    ) -> Option<Self> {
        let root_layer = LanguageLayer {
            tree: None,
            config,
            depth: 0,
            flags: LayerUpdateFlags::empty(),
            ranges: vec![Range {
                start_byte: 0,
                end_byte: usize::MAX,
                start_point: Point::new(0, 0),
                end_point: Point::new(usize::MAX, usize::MAX),
            }],
            parent: None,
        };

        // track scope_descriptor: a Vec of scopes for item in tree

        let mut layers = HopSlotMap::default();
        let root = layers.insert(root_layer);

        let mut syntax = Self { root, layers };

        let res = syntax.update(source, Vec::new(), injection_callback);

        if res.is_err() {
            log::error!("TS parser failed, disabling TS for the current buffer: {res:?}");
            return None;
        }
        Some(syntax)
    }

    pub fn tree(&self) -> &Tree {
        self.layers[self.root].tree()
    }

    pub fn tree_for_byte_range(&self, start: usize, end: usize) -> &Tree {
        let mut container_id = self.root;

        for (layer_id, layer) in self.layers.iter() {
            if layer.depth > self.layers[container_id].depth
                && layer.contains_byte_range(start, end)
            {
                container_id = layer_id;
            }
        }

        self.layers[container_id].tree()
    }

    pub fn named_descendant_for_byte_range(&self, start: usize, end: usize) -> Option<Node<'_>> {
        self.tree_for_byte_range(start, end)
            .root_node()
            .named_descendant_for_byte_range(start, end)
    }

    pub fn descendant_for_byte_range(&self, start: usize, end: usize) -> Option<Node<'_>> {
        self.tree_for_byte_range(start, end)
            .root_node()
            .descendant_for_byte_range(start, end)
    }

    pub fn walk(&self) -> TreeCursor<'_> {
        TreeCursor::new(&self.layers, self.root)
    }
}

#[derive(Debug)]
pub struct LanguageLayer {
    // mode
    // grammar
    pub config: Arc<HighlightConfiguration>,
    pub(crate) tree: Option<Tree>,
    pub ranges: Vec<Range>,
    pub depth: u32,
    flags: LayerUpdateFlags,
    parent: Option<LayerId>,
}

/// This PartialEq implementation only checks if that
/// two layers are theoretically identical (meaning they highlight the same text range with the same language).
/// It does not check whether the layers have the same internal treesitter
/// state.
impl PartialEq for LanguageLayer {
    fn eq(&self, other: &Self) -> bool {
        self.depth == other.depth
            && self.config.language == other.config.language
            && self.ranges == other.ranges
    }
}

/// Hash implementation belongs to PartialEq implementation above.
/// See its documentation for details.
impl Hash for LanguageLayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.depth.hash(state);
        self.config.language.hash(state);
        self.ranges.hash(state);
    }
}

impl LanguageLayer {
    pub fn tree(&self) -> &Tree {
        // TODO: no unwrap
        self.tree.as_ref().unwrap()
    }

    /// Whether the layer contains the given byte range.
    ///
    /// If the layer has multiple ranges (i.e. combined injections), the
    /// given range is considered contained if it is within the start and
    /// end bytes of the first and last ranges **and** if the given range
    /// starts or ends within any of the layer's ranges.
    fn contains_byte_range(&self, start: usize, end: usize) -> bool {
        let layer_start = self
            .ranges
            .first()
            .expect("ranges should not be empty")
            .start_byte;
        let layer_end = self
            .ranges
            .last()
            .expect("ranges should not be empty")
            .end_byte;

        layer_start <= start
            && layer_end >= end
            && self.ranges.iter().any(|range| {
                let byte_range = range.start_byte..range.end_byte;
                byte_range.contains(&start) || byte_range.contains(&end)
            })
    }
}

#[derive(Debug, Clone)]
pub enum InjectionLanguageMarker<'a> {
    Name(Cow<'a, str>),
    Filename(Cow<'a, Path>),
    Shebang(String),
}

const SHEBANG: &str = r"#!\s*(?:\S*[/\\](?:env\s+(?:\-\S+\s+)*)?)?([^\s\.\d]+)";

#[derive(Debug)]
pub enum CapturedNode<'a> {
    Single(Node<'a>),
    /// Guaranteed to be not empty
    Grouped(Vec<Node<'a>>),
}

impl<'a> CapturedNode<'a> {
    pub fn start_byte(&self) -> usize {
        match self {
            Self::Single(n) => n.start_byte(),
            Self::Grouped(ns) => ns[0].start_byte(),
        }
    }

    pub fn end_byte(&self) -> usize {
        match self {
            Self::Single(n) => n.end_byte(),
            Self::Grouped(ns) => ns.last().unwrap().end_byte(),
        }
    }

    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.start_byte()..self.end_byte()
    }
}

/// The maximum number of in-progress matches a TS cursor can consider at once.
/// This is set to a constant in order to avoid performance problems for medium to large files. Set with `set_match_limit`.
/// Using such a limit means that we lose valid captures, so there is fundamentally a tradeoff here.
///
///
/// Old tree sitter versions used a limit of 32 by default until this limit was removed in version `0.19.5` (must now be set manually).
/// However, this causes performance issues for medium to large files.
/// In helix, this problem caused treesitter motions to take multiple seconds to complete in medium-sized rust files (3k loc).
///
///
/// Neovim also encountered this problem and reintroduced this limit after it was removed upstream
/// (see <https://github.com/neovim/neovim/issues/14897> and <https://github.com/neovim/neovim/pull/14915>).
/// The number used here is fundamentally a tradeoff between breaking some obscure edge cases and performance.
///
///
/// Neovim chose 64 for this value somewhat arbitrarily (<https://github.com/neovim/neovim/pull/18397>).
/// 64 is too low for some languages though. In particular, it breaks some highlighting for record fields in Erlang record definitions.
/// This number can be increased if new syntax highlight breakages are found, as long as the performance penalty is not too high.
const TREE_SITTER_MATCH_LIMIT: u32 = 256;

#[derive(Debug)]
pub struct TextObjectQuery {
    pub query: Query,
}

impl TextObjectQuery {
    /// Run the query on the given node and return sub nodes which match given
    /// capture ("function.inside", "class.around", etc).
    ///
    /// Captures may contain multiple nodes by using quantifiers (+, *, etc),
    /// and support for this is partial and could use improvement.
    ///
    /// ```query
    /// (comment)+ @capture
    ///
    /// ; OR
    /// (
    ///   (comment)*
    ///   .
    ///   (function)
    /// ) @capture
    /// ```
    pub fn capture_nodes<'a>(
        &'a self,
        capture_name: &str,
        node: Node<'a>,
        slice: RopeSlice<'a>,
        cursor: &'a mut QueryCursor,
    ) -> Option<impl Iterator<Item = CapturedNode<'a>>> {
        self.capture_nodes_any(&[capture_name], node, slice, cursor)
    }

    /// Find the first capture that exists out of all given `capture_names`
    /// and return sub nodes that match this capture.
    pub fn capture_nodes_any<'a>(
        &'a self,
        capture_names: &[&str],
        node: Node<'a>,
        slice: RopeSlice<'a>,
        cursor: &'a mut QueryCursor,
    ) -> Option<impl Iterator<Item = CapturedNode<'a>>> {
        let capture_idx = capture_names
            .iter()
            .find_map(|cap| self.query.capture_index_for_name(cap))?;

        cursor.set_match_limit(TREE_SITTER_MATCH_LIMIT);

        let nodes = cursor
            .captures(&self.query, node, RopeProvider(slice))
            .filter_map(move |(mat, _)| {
                let nodes: Vec<_> = mat
                    .captures
                    .iter()
                    .filter_map(|cap| (cap.index == capture_idx).then_some(cap.node))
                    .collect();

                if nodes.len() > 1 {
                    Some(CapturedNode::Grouped(nodes))
                } else {
                    nodes.into_iter().map(CapturedNode::Single).next()
                }
            });

        Some(nodes)
    }
}

/// Represents the reason why syntax highlighting failed.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    InvalidLanguage,
    InvalidRanges,
    Unknown,
}

#[derive(Clone)]
enum IncludedChildren {
    None,
    All,
    Unnamed,
}

impl Default for IncludedChildren {
    fn default() -> Self {
        Self::None
    }
}

fn byte_range_to_str(range: std::ops::Range<usize>, source: RopeSlice) -> Cow<str> {
    Cow::from(source.byte_slice(range))
}

struct TsParser {
    parser: ::tree_sitter::Parser,
    pub cursors: Vec<QueryCursor>,
}

// could also just use a pool, or a single instance?
thread_local! {
    static PARSER: RefCell<TsParser> = RefCell::new(TsParser {
        parser: Parser::new(),
        cursors: Vec::new(),
    })
}

pub fn with_cursor<T>(f: impl FnOnce(&mut QueryCursor) -> T) -> T {
    PARSER.with(|parser| {
        let mut parser = parser.borrow_mut();
        let mut cursor = parser.cursors.pop().unwrap_or_else(QueryCursor::new);
        let res = f(&mut cursor);
        parser.cursors.push(cursor);
        res
    })
}
