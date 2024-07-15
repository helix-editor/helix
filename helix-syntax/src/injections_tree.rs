use core::slice;
use std::iter::Peekable;
use std::sync::Arc;

use hashbrown::HashMap;
use slotmap::{new_key_type, SlotMap};

use crate::parse::LayerUpdateFlags;
use crate::tree_sitter::SyntaxTree;
use crate::{HighlightConfiguration, RopeProvider};

// TODO(perf): replace std::ops::Range with helix_core::Range once added
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
    pub(crate) parent: Option<LayerId>,
    /// a list of **sorted** non-overlapping injection ranges note that
    /// injection ranges are not relative to the start of this layer but the
    /// start of the root layer
    pub(crate) injection_ranges: Box<[InjectionRange]>,
}

#[derive(Debug)]
pub(crate) struct InjectionRange {
    pub byte_range: Range,
    pub layer: LayerId,
}

impl LanguageLayer {
    /// Returns the injection range **within this layers** that contains `idx`.
    /// This function will not descend into nested injections
    pub(crate) fn injection_at_byte_idx(&self, idx: usize) -> Option<&InjectionRange> {
        let i = self
            .injection_ranges
            .partition_point(|range| range.byte_range.start <= idx);
        self.injection_ranges
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

struct ActiveInjection<'a> {
    injections: Peekable<slice::Iter<'a, InjectionTree>>,
    range: InjectionRange,
}

struct ActiveLayer<'a, State> {
    state: State,
    /// the query captures just for this layer
    layer_captures: Peekable<LayerQueryCaptures<'a>>,
}

type LayerQueryCaptures<'a> = tree_sitter::QueryCaptures<'a, 'a, RopeProvider<'a>, &'a [u8]>;

pub struct QueryCaptures<'a> {
    active_layers: HashMap<LayerId, ActiveLayer<'a, ()>>,
    active_injections: Vec<ActiveInjection<'a>>,
}
