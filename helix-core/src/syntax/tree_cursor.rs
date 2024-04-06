use std::{cmp::Reverse, ops::Range};

use super::{LanguageLayer, LayerId};

use slotmap::HopSlotMap;
use tree_sitter::Node;

/// The byte range of an injection layer.
///
/// Injection ranges may overlap, but all overlapping parts are subsets of their parent ranges.
/// This allows us to sort the ranges ahead of time in order to efficiently find a range that
/// contains a point with maximum depth.
#[derive(Debug)]
struct InjectionRange {
    start: usize,
    end: usize,
    layer_id: LayerId,
    depth: u32,
}

pub struct TreeCursor<'a> {
    layers: &'a HopSlotMap<LayerId, LanguageLayer>,
    root: LayerId,
    current: LayerId,
    injection_ranges: Vec<InjectionRange>,
    // TODO: Ideally this would be a `tree_sitter::TreeCursor<'a>` but
    // that returns very surprising results in testing.
    cursor: Node<'a>,
}

impl<'a> TreeCursor<'a> {
    pub(super) fn new(layers: &'a HopSlotMap<LayerId, LanguageLayer>, root: LayerId) -> Self {
        let mut injection_ranges = Vec::new();

        for (layer_id, layer) in layers.iter() {
            // Skip the root layer
            if layer.parent.is_none() {
                continue;
            }
            for byte_range in layer.ranges.iter() {
                let range = InjectionRange {
                    start: byte_range.start_byte,
                    end: byte_range.end_byte,
                    layer_id,
                    depth: layer.depth,
                };
                injection_ranges.push(range);
            }
        }

        injection_ranges.sort_unstable_by_key(|range| (range.end, Reverse(range.depth)));

        let cursor = layers[root].tree().root_node();

        Self {
            layers,
            root,
            current: root,
            injection_ranges,
            cursor,
        }
    }

    pub fn node(&self) -> Node<'a> {
        self.cursor
    }

    pub fn goto_parent(&mut self) -> bool {
        if let Some(parent) = self.node().parent() {
            self.cursor = parent;
            return true;
        }

        // If we are already on the root layer, we cannot ascend.
        if self.current == self.root {
            return false;
        }

        // Ascend to the parent layer.
        let range = self.node().byte_range();
        let parent_id = self.layers[self.current]
            .parent
            .expect("non-root layers have a parent");
        self.current = parent_id;
        let root = self.layers[self.current].tree().root_node();
        self.cursor = root
            .descendant_for_byte_range(range.start, range.end)
            .unwrap_or(root);

        true
    }

    /// Finds the injection layer that has exactly the same range as the given `range`.
    fn layer_id_of_byte_range(&self, search_range: Range<usize>) -> Option<LayerId> {
        let start_idx = self
            .injection_ranges
            .partition_point(|range| range.end < search_range.end);

        self.injection_ranges[start_idx..]
            .iter()
            .take_while(|range| range.end == search_range.end)
            .find_map(|range| (range.start == search_range.start).then_some(range.layer_id))
    }

    pub fn goto_first_child(&mut self) -> bool {
        // Check if the current node's range is an exact injection layer range.
        if let Some(layer_id) = self
            .layer_id_of_byte_range(self.node().byte_range())
            .filter(|&layer_id| layer_id != self.current)
        {
            // Switch to the child layer.
            self.current = layer_id;
            self.cursor = self.layers[self.current].tree().root_node();
            true
        } else if let Some(child) = self.cursor.child(0) {
            // Otherwise descend in the current tree.
            self.cursor = child;
            true
        } else {
            false
        }
    }

    pub fn goto_next_sibling(&mut self) -> bool {
        if let Some(sibling) = self.cursor.next_sibling() {
            self.cursor = sibling;
            true
        } else {
            false
        }
    }

    pub fn goto_prev_sibling(&mut self) -> bool {
        if let Some(sibling) = self.cursor.prev_sibling() {
            self.cursor = sibling;
            true
        } else {
            false
        }
    }

    /// Finds the injection layer that contains the given start-end range.
    fn layer_id_containing_byte_range(&self, start: usize, end: usize) -> LayerId {
        let start_idx = self
            .injection_ranges
            .partition_point(|range| range.end < end);

        self.injection_ranges[start_idx..]
            .iter()
            .take_while(|range| range.start < end)
            .find_map(|range| (range.start <= start).then_some(range.layer_id))
            .unwrap_or(self.root)
    }

    pub fn reset_to_byte_range(&mut self, start: usize, end: usize) {
        self.current = self.layer_id_containing_byte_range(start, end);
        let root = self.layers[self.current].tree().root_node();
        self.cursor = root.descendant_for_byte_range(start, end).unwrap_or(root);
    }
}
