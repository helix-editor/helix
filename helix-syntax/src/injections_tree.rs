use core::slice;
use std::cell::RefCell;
use std::iter::Peekable;
use std::mem::replace;
use std::sync::Arc;

use hashbrown::HashMap;
use ropey::RopeSlice;
use slotmap::{new_key_type, SlotMap};

use crate::parse::LayerUpdateFlags;
use crate::tree_sitter::{
    self, Capture, InactiveQueryCursor, Parser, Query, QueryCursor, RopeTsInput, SyntaxTree,
    SyntaxTreeNode,
};
use crate::HighlightConfiguration;

// TODO(perf): replace std::ops::Range<usize> with helix_stdx::Range<u32> once added
type Range = std::ops::Range<usize>;

new_key_type! {
    /// The default slot map key type.
    pub struct LayerId;
}

#[derive(Debug)]
pub struct LanguageLayer {
    pub config: Arc<HighlightConfiguration>,
    pub(crate) parse_tree: Option<SyntaxTree>,
    /// internal flags used during parsing to track incremental invalidation
    pub(crate) flags: LayerUpdateFlags,
    ranges: Vec<tree_sitter::Range>,
    pub(crate) parent: Option<LayerId>,
    /// a list of **sorted** non-overlapping injection ranges. Note that
    /// injection ranges are not relative to the start of this layer but the
    /// start of the root layer
    pub(crate) injections: Box<[Injection]>,
}

#[derive(Debug, Clone)]
pub(crate) struct Injection {
    pub byte_range: Range,
    pub layer: LayerId,
}

impl LanguageLayer {
    /// Returns the injection range **within this layers** that contains `idx`.
    /// This function will not descend into nested injections
    pub(crate) fn injection_at_byte_idx(&self, idx: usize) -> Option<&Injection> {
        let i = self
            .injections
            .partition_point(|range| range.byte_range.start <= idx);
        self.injections
            .get(i)
            .filter(|injection| injection.byte_range.end > idx)
    }
}

struct InjectionTree {
    layers: SlotMap<LayerId, LanguageLayer>,
    root: LayerId,
}

impl InjectionTree {
    pub fn layer_for_byte_range(&self, start: usize, end: usize) -> LayerId {
        let mut cursor = self.root;
        loop {
            let layer = &self.layers[cursor];
            let Some(start_injection) = layer.injection_at_byte_idx(start) else {
                break;
            };
            let Some(end_injection) = layer.injection_at_byte_idx(end) else {
                break;
            };
            if start_injection.layer == end_injection.layer {
                cursor = start_injection.layer;
            } else {
                break;
            }
        }
        cursor
    }
}

#[derive(Clone)]
pub struct MatchedNode {
    pub capture: Capture,
    pub byte_range: Range,
}

struct LayerQueryIter<'a> {
    cursor: QueryCursor<'a, 'a, RopeTsInput<'a>>,
    peeked: Option<MatchedNode>,
}

impl<'a> LayerQueryIter<'a> {
    fn peek(&mut self) -> Option<&MatchedNode> {
        if self.peeked.is_none() {
            let (query_match, node_idx) = self.cursor.next_matched_node()?;
            let matched_node = query_match.matched_node(node_idx);
            self.peeked = Some(MatchedNode {
                capture: matched_node.capture,
                byte_range: matched_node.syntax_node.byte_range(),
            });
        }
        self.peeked.as_ref()
    }

    fn consume(&mut self) -> MatchedNode {
        self.peeked.take().unwrap()
    }
}

struct ActiveLayer<'a> {
    query_iter: LayerQueryIter<'a>,
    injections: Peekable<slice::Iter<'a, Injection>>,
}

struct QueryBuilder<'a, 'tree> {
    query: &'a Query,
    node: &'a SyntaxTreeNode<'tree>,
    src: RopeSlice<'a>,
    injection_tree: &'a InjectionTree,
}

pub struct QueryIter<'a, 'tree> {
    query_builder: Box<QueryBuilder<'a, 'tree>>,
    active_layers: HashMap<LayerId, ActiveLayer<'a>>,
    active_injections: Vec<Injection>,
    current_injection: Injection,
}

impl<'a> QueryIter<'a, '_> {
    fn enter_injection(&mut self, injection: Injection) -> bool {
        self.active_layers
            .entry(injection.layer)
            .or_insert_with(|| {
                let layer = &self.query_builder.injection_tree.layers[injection.layer];
                let injection_start = layer
                    .injections
                    .partition_point(|child| child.byte_range.start < injection.byte_range.start);
                let cursor = get_cursor().execute_query(
                    self.query_builder.query,
                    self.query_builder.node,
                    RopeTsInput::new(self.query_builder.src),
                );
                ActiveLayer {
                    query_iter: LayerQueryIter {
                        cursor,
                        peeked: None,
                    },
                    injections: layer.injections[injection_start..].iter().peekable(),
                }
            });
        let old_injection = replace(&mut self.current_injection, injection);
        self.active_injections.push(old_injection);
        true
    }

    fn exit_injection(&mut self) -> Option<Injection> {
        let injection = replace(&mut self.current_injection, self.active_injections.pop()?);
        let finished_layer = self.active_layers[&injection.layer]
            .query_iter
            .peeked
            .is_none();
        if finished_layer {
            let layer = self.active_layers.remove(&injection.layer).unwrap();
            reuse_cursor(layer.query_iter.cursor.reuse());
        }
        Some(injection)
    }
}

pub enum QueryIterEvent {
    EnterInjection(Injection),
    Match(MatchedNode),
    ExitInjection(Injection),
}

impl<'a> Iterator for QueryIter<'a, '_> {
    type Item = QueryIterEvent;

    fn next(&mut self) -> Option<QueryIterEvent> {
        loop {
            let active_layer = self
                .active_layers
                .get_mut(&self.current_injection.layer)
                .unwrap();
            let next_injection = active_layer.injections.peek().filter(|injection| {
                injection.byte_range.start < self.current_injection.byte_range.end
            });
            let next_match = active_layer.query_iter.peek().filter(|matched_node| {
                matched_node.byte_range.start < self.current_injection.byte_range.end
            });

            match (next_match, next_injection) {
                (None, None) => {
                    return self.exit_injection().map(QueryIterEvent::ExitInjection);
                }
                (Some(_), None) => {
                    // consume match
                    let matched_node = active_layer.query_iter.consume();
                    return Some(QueryIterEvent::Match(matched_node));
                }
                (Some(matched_node), Some(injection))
                    if matched_node.byte_range.start <= injection.byte_range.end =>
                {
                    // consume match
                    let matched_node = active_layer.query_iter.consume();
                    // ignore nodes that are overlapped by the injection
                    if matched_node.byte_range.start <= injection.byte_range.start {
                        return Some(QueryIterEvent::Match(matched_node));
                    }
                }
                (Some(_), Some(_)) | (None, Some(_)) => {
                    // consume injection
                    let injection = active_layer.injections.next().unwrap();
                    if self.enter_injection(injection.clone()) {
                        return Some(QueryIterEvent::EnterInjection(injection.clone()));
                    }
                }
            }
        }
    }
}

struct TsParser {
    parser: crate::tree_sitter::Parser,
    pub cursors: Vec<crate::tree_sitter::InactiveQueryCursor>,
}

// could also just use a pool, or a single instance?
thread_local! {
    static PARSER: RefCell<TsParser> = RefCell::new(TsParser {
        parser: Parser::new(),
        cursors: Vec::new(),
    })
}

pub fn with_cursor<T>(f: impl FnOnce(&mut InactiveQueryCursor) -> T) -> T {
    PARSER.with(|parser| {
        let mut parser = parser.borrow_mut();
        let mut cursor = parser
            .cursors
            .pop()
            .unwrap_or_else(InactiveQueryCursor::new);
        let res = f(&mut cursor);
        parser.cursors.push(cursor);
        res
    })
}

pub fn get_cursor() -> InactiveQueryCursor {
    PARSER.with(|parser| {
        let mut parser = parser.borrow_mut();
        parser
            .cursors
            .pop()
            .unwrap_or_else(InactiveQueryCursor::new)
    })
}

pub fn reuse_cursor(cursor: InactiveQueryCursor) {
    PARSER.with(|parser| {
        let mut parser = parser.borrow_mut();
        parser.cursors.push(cursor)
    })
}
