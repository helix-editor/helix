use core::slice;
use std::iter::Peekable;
use std::mem::replace;

use hashbrown::HashMap;
use ropey::RopeSlice;

use crate::tree_sitter::{
    Capture, InactiveQueryCursor, Query, QueryCursor, RopeTsInput, SyntaxTreeNode,
};
use crate::{Injection, LayerId, Range, Syntax};

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

struct ActiveLayer<'a, S> {
    state: S,
    query_iter: LayerQueryIter<'a>,
    injections: Peekable<slice::Iter<'a, Injection>>,
}

// data only needed when entering and exiting injections
// seperate struck to keep the QueryIter reasonably small
struct QueryIterLayerManager<'a, S> {
    query: &'a Query,
    node: SyntaxTreeNode<'a>,
    src: RopeSlice<'a>,
    syntax: &'a Syntax,
    active_layers: HashMap<LayerId, Box<ActiveLayer<'a, S>>>,
    active_injections: Vec<Injection>,
}

impl<'a, S: Default> QueryIterLayerManager<'a, S> {
    fn init_layer(&mut self, injection: Injection) -> Box<ActiveLayer<'a, S>> {
        self.active_layers
            .remove(&injection.layer)
            .unwrap_or_else(|| {
                let layer = &self.syntax.layers[injection.layer];
                let injection_start = layer
                    .injections
                    .partition_point(|child| child.byte_range.start < injection.byte_range.start);
                let cursor = InactiveQueryCursor::new().execute_query(
                    self.query,
                    &self.node,
                    RopeTsInput::new(self.src),
                );
                Box::new(ActiveLayer {
                    state: S::default(),
                    query_iter: LayerQueryIter {
                        cursor,
                        peeked: None,
                    },
                    injections: layer.injections[injection_start..].iter().peekable(),
                })
            })
    }
}

pub struct QueryIter<'a, LayerState: Default = ()> {
    layer_manager: Box<QueryIterLayerManager<'a, LayerState>>,
    current_layer: Box<ActiveLayer<'a, LayerState>>,
    current_injection: Injection,
}

impl<'a, LayerState: Default> QueryIter<'a, LayerState> {
    pub fn new(syntax: &'a Syntax, src: RopeSlice<'a>, query: &'a Query) -> Self {
        Self::at(syntax, src, query, syntax.tree().root_node(), syntax.root)
    }

    pub fn at(
        syntax: &'a Syntax,
        src: RopeSlice<'a>,
        query: &'a Query,
        node: SyntaxTreeNode<'a>,
        layer: LayerId,
    ) -> Self {
        // create fake injection for query root
        let injection = Injection {
            byte_range: node.byte_range(),
            layer,
        };
        let mut layer_manager = Box::new(QueryIterLayerManager {
            query,
            node,
            src,
            syntax,
            // TODO: reuse allocations with an allocation pool
            active_layers: HashMap::with_capacity(8),
            active_injections: Vec::with_capacity(8),
        });
        Self {
            current_layer: layer_manager.init_layer(injection),
            current_injection: injection,
            layer_manager,
        }
    }

    pub fn current_layer_state(&mut self) -> &mut LayerState {
        &mut self.current_layer.state
    }

    pub fn layer_state(&mut self, layer: LayerId) -> &mut LayerState {
        if layer == self.current_injection.layer {
            self.current_layer_state()
        } else {
            &mut self
                .layer_manager
                .active_layers
                .get_mut(&layer)
                .unwrap()
                .state
        }
    }

    fn enter_injection(&mut self, injection: Injection) {
        let active_layer = self.layer_manager.init_layer(injection);
        let old_injection = replace(&mut self.current_injection, injection);
        let old_layer = replace(&mut self.current_layer, active_layer);
        self.layer_manager
            .active_layers
            .insert(old_injection.layer, old_layer);
        self.layer_manager.active_injections.push(old_injection);
    }

    fn exit_injection(&mut self) -> Option<(Injection, Option<LayerState>)> {
        let injection = replace(
            &mut self.current_injection,
            self.layer_manager.active_injections.pop()?,
        );
        let layer = replace(
            &mut self.current_layer,
            self.layer_manager
                .active_layers
                .remove(&self.current_injection.layer)?,
        );
        let layer_unfinished = layer.query_iter.peeked.is_some();
        if layer_unfinished {
            self.layer_manager
                .active_layers
                .insert(injection.layer, layer)
                .unwrap();
            Some((injection, None))
        } else {
            Some((injection, Some(layer.state)))
        }
    }
}

impl<'a, S: Default> Iterator for QueryIter<'a, S> {
    type Item = QueryIterEvent<S>;

    fn next(&mut self) -> Option<QueryIterEvent<S>> {
        loop {
            let next_injection = self.current_layer.injections.peek().filter(|injection| {
                injection.byte_range.start < self.current_injection.byte_range.end
            });
            let next_match = self.current_layer.query_iter.peek().filter(|matched_node| {
                matched_node.byte_range.start < self.current_injection.byte_range.end
            });

            match (next_match, next_injection) {
                (None, None) => {
                    return self.exit_injection().map(|(injection, state)| {
                        QueryIterEvent::ExitInjection { injection, state }
                    });
                }
                (Some(_), None) => {
                    // consume match
                    let matched_node = self.current_layer.query_iter.consume();
                    return Some(QueryIterEvent::Match(matched_node));
                }
                (Some(matched_node), Some(injection))
                    if matched_node.byte_range.start <= injection.byte_range.end =>
                {
                    // consume match
                    let matched_node = self.current_layer.query_iter.consume();
                    // ignore nodes that are overlapped by the injection
                    if matched_node.byte_range.start <= injection.byte_range.start {
                        return Some(QueryIterEvent::Match(matched_node));
                    }
                }
                (Some(_), Some(_)) | (None, Some(_)) => {
                    // consume injection
                    let injection = self.current_layer.injections.next().unwrap();
                    self.enter_injection(injection.clone());
                    return Some(QueryIterEvent::EnterInjection(injection.clone()));
                }
            }
        }
    }
}

pub enum QueryIterEvent<State = ()> {
    EnterInjection(Injection),
    Match(MatchedNode),
    ExitInjection {
        injection: Injection,
        state: Option<State>,
    },
}

impl<S> QueryIterEvent<S> {
    pub fn start(&self) -> u32 {
        match self {
            QueryIterEvent::EnterInjection(injection) => injection.byte_range.start as u32,
            QueryIterEvent::Match(mat) => mat.byte_range.start as u32,
            QueryIterEvent::ExitInjection { injection, .. } => injection.byte_range.start as u32,
        }
    }
}
