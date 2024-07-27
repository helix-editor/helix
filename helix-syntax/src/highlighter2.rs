use std::borrow::Cow;
use std::iter::{self, Peekable};
use std::mem::{replace, take};
use std::slice;

use hashbrown::HashMap;

use crate::query_iter::{MatchedNode, QueryIter, QueryIterEvent};
use crate::{Injection, LayerId, Range, Syntax};

/// Indicates which highlight should be applied to a region of source code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Highlight(pub u32);
impl Highlight{
    pub(crate) const NONE = Highlight(u32::MAX);
}

#[derive(Debug)]
struct LocalDef<'a> {
    name: Cow<'a, str>,
    value_range: Range,
    highlight: Option<Highlight>,
}

#[derive(Debug)]
struct LocalScope<'a> {
    inherits: bool,
    range: Range,
    local_defs: Vec<LocalDef<'a>>,
}

#[derive(Debug)]
struct HighlightedNode {
    end: u32,
    highlight: Highlight,
}

#[derive(Debug, Default)]
struct LayerData<'a> {
    parent_highlights: usize,
    dormant_highlights: Vec<HighlightedNode>,
    scope_stack: Vec<LocalDef<'a>>,
}

struct HighlighterConfig<'a> {
    new_precedance: bool,
    highlight_indices: &'a [Highlight],
}

pub struct Highligther<'a> {
    query: QueryIter<'a, LayerData<'a>>,
    next_query_event: Option<QueryIterEvent<LayerData<'a>>>,
    active_highlights: Vec<HighlightedNode>,
    next_highlight_end: u32,
    next_highlight_start: u32,
    config: HighlighterConfig<'a>,
}

pub struct HighlightList<'a>(slice::Iter<'a, HighlightedNode>);

impl<'a> Iterator for HighlightList<'a> {
    type Item = Highlight;

    fn next(&mut self) -> Option<Highlight> {
        self.0.next().map(|node| node.highlight)
    }
}

pub enum HighlighEvent<'a> {
    RefreshHiglights(HighlightList<'a>),
    PushHighlights(HighlightList<'a>),
}

impl<'a> Highligther<'a> {
    pub fn active_highlights(&self) -> HighlightList<'_> {
        HighlightList(self.active_highlights.iter())
    }

    pub fn next_event_offset(&self) -> u32 {
        self.next_highlight_start.min(self.next_highlight_end)
    }

    pub fn advance(&mut self) -> HighlighEvent<'_> {
        let mut refresh = false;
        let prev_stack_size = self.active_highlights.len();

        let pos = self.next_event_offset();
        if self.next_highlight_end == pos {
            self.process_injection_ends();
            self.process_higlight_end();
            refresh = true;
        }

        let mut first_highlight = true;
        while self.next_highlight_start == pos {
            let Some(query_event) = self.adance_query_iter() else {
                break;
            };
            match query_event {
                QueryIterEvent::EnterInjection(_) => self.enter_injection(),
                QueryIterEvent::Match(node) => self.start_highlight(node, &mut first_highlight),
                QueryIterEvent::ExitInjection { injection, state } => {
                    // state is returned if the layer is finifhed, if it isn't we have
                    // a combined injection and need to deactive its highlights
                    if state.is_none() {
                        self.deactive_layer(injection.layer);
                        refresh = true;
                    }
                }
            }
        }
        self.next_highlight_end = self
            .active_highlights
            .last()
            .map_or(u32::MAX, |node| node.end);

        if refresh {
            HighlighEvent::RefreshHiglights(HighlightList(self.active_highlights.iter()))
        } else {
            HighlighEvent::PushHighlights(HighlightList(
                self.active_highlights[prev_stack_size..].iter(),
            ))
        }
    }

    fn adance_query_iter(&mut self) -> Option<QueryIterEvent<LayerData<'a>>> {
        let event = replace(&mut self.next_query_event, self.query.next());
        self.next_highlight_start = self
            .next_query_event
            .as_ref()
            .map_or(u32::MAX, |event| event.start());
        event
    }

    fn process_higlight_end(&mut self) {
        let i = self
            .active_highlights
            .iter()
            .rposition(|highlight| highlight.end != self.next_highlight_end)
            .unwrap();
        self.active_highlights.truncate(i);
    }

    /// processes injections that end at the same position as highlights first.
    fn process_injection_ends(&mut self) {
        while self.next_highlight_end == self.next_highlight_start {
            match self.next_query_event.as_ref() {
                Some(QueryIterEvent::ExitInjection { injection, state }) => {
                    if state.is_none() {
                        self.deactive_layer(injection.layer);
                    }
                }
                Some(QueryIterEvent::Match(matched_node)) if matched_node.byte_range.is_empty() => {
                }
                _ => break,
            }
        }
    }

    fn enter_injection(&mut self) {
        self.query.current_layer_state().parent_highlights = self.active_highlights.len();
    }

    fn deactive_layer(&mut self, layer: LayerId) {
        let LayerData {
            parent_highlights,
            ref mut dormant_highlights,
            ..
        } = *self.query.layer_state(layer);
        let i = self.active_highlights[parent_highlights..]
            .iter()
            .rposition(|highlight| highlight.end != self.next_highlight_end)
            .unwrap();
        self.active_highlights.truncate(parent_highlights + i);
        dormant_highlights.extend(self.active_highlights.drain(parent_highlights..))
    }

    fn start_highlight(&mut self, node: MatchedNode, first_highlight: &mut bool) {
        if node.byte_range.is_empty() {
            return;
        }

        // if there are multiple matches for the exact same node
        // only use one of the (the last with new/nvim precedance)
        if !*first_highlight
            && self.active_highlights.last().map_or(false, |prev_node| {
                prev_node.end == node.byte_range.end as u32
            })
        {
            if self.config.new_precedance {
                self.active_highlights.pop();
            } else {
                return;
            }
        }
        let highlight = self.config.highlight_indices[node.capture.idx()];
        if highlight.0 == u32::MAX {
            return;
        }
        self.active_highlights.push(HighlightedNode {
            end: node.byte_range.end as u32,
            highlight,
        });
        *first_highlight = false;
    }
}
