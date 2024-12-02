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

    pub fn goto_parent_with<P>(&mut self, predicate: P) -> bool
    where
        P: Fn(&Node) -> bool,
    {
        while self.goto_parent() {
            if predicate(&self.node()) {
                return true;
            }
        }

        false
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

    fn goto_first_child_impl(&mut self, named: bool) -> bool {
        // Check if the current node's range is an exact injection layer range.
        if let Some(layer_id) = self
            .layer_id_of_byte_range(self.node().byte_range())
            .filter(|&layer_id| layer_id != self.current)
        {
            // Switch to the child layer.
            self.current = layer_id;
            self.cursor = self.layers[self.current].tree().root_node();
            return true;
        }

        let child = if named {
            self.cursor.named_child(0)
        } else {
            self.cursor.child(0)
        };

        if let Some(child) = child {
            // Otherwise descend in the current tree.
            self.cursor = child;
            true
        } else {
            false
        }
    }

    pub fn goto_first_child(&mut self) -> bool {
        self.goto_first_child_impl(false)
    }

    pub fn goto_first_named_child(&mut self) -> bool {
        self.goto_first_child_impl(true)
    }

    fn goto_next_sibling_impl(&mut self, named: bool) -> bool {
        let sibling = if named {
            self.cursor.next_named_sibling()
        } else {
            self.cursor.next_sibling()
        };

        if let Some(sibling) = sibling {
            self.cursor = sibling;
            true
        } else {
            false
        }
    }

    pub fn goto_next_sibling(&mut self) -> bool {
        self.goto_next_sibling_impl(false)
    }

    pub fn goto_next_named_sibling(&mut self) -> bool {
        self.goto_next_sibling_impl(true)
    }

    fn goto_prev_sibling_impl(&mut self, named: bool) -> bool {
        let sibling = if named {
            self.cursor.prev_named_sibling()
        } else {
            self.cursor.prev_sibling()
        };

        if let Some(sibling) = sibling {
            self.cursor = sibling;
            true
        } else {
            false
        }
    }

    pub fn goto_prev_sibling(&mut self) -> bool {
        self.goto_prev_sibling_impl(false)
    }

    pub fn goto_prev_named_sibling(&mut self) -> bool {
        self.goto_prev_sibling_impl(true)
    }

    /// Finds the injection layer that contains the given start-end range.
    fn layer_id_containing_byte_range(&self, start: usize, end: usize) -> LayerId {
        let start_idx = self
            .injection_ranges
            .partition_point(|range| range.end < end);

        self.injection_ranges[start_idx..]
            .iter()
            .take_while(|range| range.start < end || range.depth > 1)
            .find_map(|range| (range.start <= start).then_some(range.layer_id))
            .unwrap_or(self.root)
    }

    pub fn reset_to_byte_range(&mut self, start: usize, end: usize) {
        self.current = self.layer_id_containing_byte_range(start, end);
        let root = self.layers[self.current].tree().root_node();
        self.cursor = root.descendant_for_byte_range(start, end).unwrap_or(root);
    }

    /// Returns an iterator over the children of the node the TreeCursor is on
    /// at the time this is called.
    pub fn children(&'a mut self) -> ChildIter<'a> {
        let parent = self.node();

        ChildIter {
            cursor: self,
            parent,
            named: false,
        }
    }

    /// Returns an iterator over the named children of the node the TreeCursor is on
    /// at the time this is called.
    pub fn named_children(&'a mut self) -> ChildIter<'a> {
        let parent = self.node();

        ChildIter {
            cursor: self,
            parent,
            named: true,
        }
    }
}

pub struct ChildIter<'n> {
    cursor: &'n mut TreeCursor<'n>,
    parent: Node<'n>,
    named: bool,
}

impl<'n> Iterator for ChildIter<'n> {
    type Item = Node<'n>;

    fn next(&mut self) -> Option<Self::Item> {
        // first iteration, just visit the first child
        if self.cursor.node() == self.parent {
            self.cursor
                .goto_first_child_impl(self.named)
                .then(|| self.cursor.node())
        } else {
            self.cursor
                .goto_next_sibling_impl(self.named)
                .then(|| self.cursor.node())
        }
    }
}
