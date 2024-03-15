use std::collections::VecDeque;
use std::mem::replace;
use std::sync::Arc;

use ahash::RandomState;
use bitflags::bitflags;
use hashbrown::raw::RawTable;
use ropey::RopeSlice;
use tree_sitter::{Node, Parser, Point, QueryCursor, Range};

use crate::ropey::RopeProvider;
use crate::{
    Error, HighlightConfiguration, IncludedChildren, InjectionLanguageMarker, LanguageLayer,
    Syntax, PARSER, TREE_SITTER_MATCH_LIMIT,
};

bitflags! {
    /// Flags that track the status of a layer
    /// in the `Sytaxn::update` function
    #[derive(Debug)]
    pub(crate) struct LayerUpdateFlags : u32{
        const MODIFIED = 0b001;
        const MOVED = 0b010;
        const TOUCHED = 0b100;
    }
}

impl Syntax {
    pub fn update(
        &mut self,
        source: RopeSlice,
        edits: Vec<tree_sitter::InputEdit>,
        injection_callback: impl Fn(&InjectionLanguageMarker) -> Option<Arc<HighlightConfiguration>>,
    ) -> Result<(), Error> {
        let mut queue = VecDeque::new();
        queue.push_back(self.root);

        // This table allows inverse indexing of `layers`.
        // That is by hashing a `Layer` you can find
        // the `LayerId` of an existing equivalent `Layer` in `layers`.
        //
        // It is used to determine if a new layer exists for an injection
        // or if an existing layer needs to be updated.
        let mut layers_table = RawTable::with_capacity(self.layers.len());
        let layers_hasher = RandomState::new();
        // Use the edits to update all layers markers
        fn point_add(a: Point, b: Point) -> Point {
            if b.row > 0 {
                Point::new(a.row.saturating_add(b.row), b.column)
            } else {
                Point::new(0, a.column.saturating_add(b.column))
            }
        }
        fn point_sub(a: Point, b: Point) -> Point {
            if a.row > b.row {
                Point::new(a.row.saturating_sub(b.row), a.column)
            } else {
                Point::new(0, a.column.saturating_sub(b.column))
            }
        }

        for (layer_id, layer) in self.layers.iter_mut() {
            // The root layer always covers the whole range (0..usize::MAX)
            if layer.depth == 0 {
                layer.flags = LayerUpdateFlags::MODIFIED;
                continue;
            }

            if !edits.is_empty() {
                for range in &mut layer.ranges {
                    // Roughly based on https://github.com/tree-sitter/tree-sitter/blob/ddeaa0c7f534268b35b4f6cb39b52df082754413/lib/src/subtree.c#L691-L720
                    for edit in edits.iter().rev() {
                        let is_pure_insertion = edit.old_end_byte == edit.start_byte;

                        // if edit is after range, skip
                        if edit.start_byte > range.end_byte {
                            // TODO: || (is_noop && edit.start_byte == range.end_byte)
                            continue;
                        }

                        // if edit is before range, shift entire range by len
                        if edit.old_end_byte < range.start_byte {
                            range.start_byte =
                                edit.new_end_byte + (range.start_byte - edit.old_end_byte);
                            range.start_point = point_add(
                                edit.new_end_position,
                                point_sub(range.start_point, edit.old_end_position),
                            );

                            range.end_byte = edit
                                .new_end_byte
                                .saturating_add(range.end_byte - edit.old_end_byte);
                            range.end_point = point_add(
                                edit.new_end_position,
                                point_sub(range.end_point, edit.old_end_position),
                            );

                            layer.flags |= LayerUpdateFlags::MOVED;
                        }
                        // if the edit starts in the space before and extends into the range
                        else if edit.start_byte < range.start_byte {
                            range.start_byte = edit.new_end_byte;
                            range.start_point = edit.new_end_position;

                            range.end_byte = range
                                .end_byte
                                .saturating_sub(edit.old_end_byte)
                                .saturating_add(edit.new_end_byte);
                            range.end_point = point_add(
                                edit.new_end_position,
                                point_sub(range.end_point, edit.old_end_position),
                            );
                            layer.flags = LayerUpdateFlags::MODIFIED;
                        }
                        // If the edit is an insertion at the start of the tree, shift
                        else if edit.start_byte == range.start_byte && is_pure_insertion {
                            range.start_byte = edit.new_end_byte;
                            range.start_point = edit.new_end_position;
                            layer.flags |= LayerUpdateFlags::MOVED;
                        } else {
                            range.end_byte = range
                                .end_byte
                                .saturating_sub(edit.old_end_byte)
                                .saturating_add(edit.new_end_byte);
                            range.end_point = point_add(
                                edit.new_end_position,
                                point_sub(range.end_point, edit.old_end_position),
                            );
                            layer.flags = LayerUpdateFlags::MODIFIED;
                        }
                    }
                }
            }

            let hash = layers_hasher.hash_one(layer);
            // Safety: insert_no_grow is unsafe because it assumes that the table
            // has enough capacity to hold additional elements.
            // This is always the case as we reserved enough capacity above.
            unsafe { layers_table.insert_no_grow(hash, layer_id) };
        }

        PARSER.with(|ts_parser| {
            let ts_parser = &mut ts_parser.borrow_mut();
            ts_parser.parser.set_timeout_micros(1000 * 500); // half a second is pretty generours
            let mut cursor = ts_parser.cursors.pop().unwrap_or_else(QueryCursor::new);
            // TODO: might need to set cursor range
            cursor.set_byte_range(0..usize::MAX);
            cursor.set_match_limit(TREE_SITTER_MATCH_LIMIT);

            let source_slice = source.slice(..);

            while let Some(layer_id) = queue.pop_front() {
                let layer = &mut self.layers[layer_id];

                // Mark the layer as touched
                layer.flags |= LayerUpdateFlags::TOUCHED;

                // If a tree already exists, notify it of changes.
                if let Some(tree) = &mut layer.tree {
                    if layer
                        .flags
                        .intersects(LayerUpdateFlags::MODIFIED | LayerUpdateFlags::MOVED)
                    {
                        for edit in edits.iter().rev() {
                            // Apply the edits in reverse.
                            // If we applied them in order then edit 1 would disrupt the positioning of edit 2.
                            tree.edit(edit);
                        }
                    }

                    if layer.flags.contains(LayerUpdateFlags::MODIFIED) {
                        // Re-parse the tree.
                        layer.parse(&mut ts_parser.parser, source)?;
                    }
                } else {
                    // always parse if this layer has never been parsed before
                    layer.parse(&mut ts_parser.parser, source)?;
                }

                // Switch to an immutable borrow.
                let layer = &self.layers[layer_id];

                // Process injections.
                let matches = cursor.matches(
                    &layer.config.injections_query,
                    layer.tree().root_node(),
                    RopeProvider(source_slice),
                );
                let mut combined_injections = vec![
                    (None, Vec::new(), IncludedChildren::default());
                    layer.config.combined_injections_patterns.len()
                ];
                let mut injections = Vec::new();
                let mut last_injection_end = 0;
                for mat in matches {
                    let (injection_capture, content_node, included_children) = layer
                        .config
                        .injection_for_match(&layer.config.injections_query, &mat, source_slice);

                    // in case this is a combined injection save it for more processing later
                    if let Some(combined_injection_idx) = layer
                        .config
                        .combined_injections_patterns
                        .iter()
                        .position(|&pattern| pattern == mat.pattern_index)
                    {
                        let entry = &mut combined_injections[combined_injection_idx];
                        if injection_capture.is_some() {
                            entry.0 = injection_capture;
                        }
                        if let Some(content_node) = content_node {
                            if content_node.start_byte() >= last_injection_end {
                                entry.1.push(content_node);
                                last_injection_end = content_node.end_byte();
                            }
                        }
                        entry.2 = included_children;
                        continue;
                    }

                    // Explicitly remove this match so that none of its other captures will remain
                    // in the stream of captures.
                    mat.remove();

                    // If a language is found with the given name, then add a new language layer
                    // to the highlighted document.
                    if let (Some(injection_capture), Some(content_node)) =
                        (injection_capture, content_node)
                    {
                        if let Some(config) = (injection_callback)(&injection_capture) {
                            let ranges =
                                intersect_ranges(&layer.ranges, &[content_node], included_children);

                            if !ranges.is_empty() {
                                if content_node.start_byte() < last_injection_end {
                                    continue;
                                }
                                last_injection_end = content_node.end_byte();
                                injections.push((config, ranges));
                            }
                        }
                    }
                }

                for (lang_name, content_nodes, included_children) in combined_injections {
                    if let (Some(lang_name), false) = (lang_name, content_nodes.is_empty()) {
                        if let Some(config) = (injection_callback)(&lang_name) {
                            let ranges =
                                intersect_ranges(&layer.ranges, &content_nodes, included_children);
                            if !ranges.is_empty() {
                                injections.push((config, ranges));
                            }
                        }
                    }
                }

                let depth = layer.depth + 1;
                // TODO: can't inline this since matches borrows self.layers
                for (config, ranges) in injections {
                    let parent = Some(layer_id);
                    let new_layer = LanguageLayer {
                        tree: None,
                        config,
                        depth,
                        ranges,
                        flags: LayerUpdateFlags::empty(),
                        parent: None,
                    };

                    // Find an identical existing layer
                    let layer = layers_table
                        .get(layers_hasher.hash_one(&new_layer), |&it| {
                            self.layers[it] == new_layer
                        })
                        .copied();

                    // ...or insert a new one.
                    let layer_id = layer.unwrap_or_else(|| self.layers.insert(new_layer));
                    self.layers[layer_id].parent = parent;

                    queue.push_back(layer_id);
                }

                // TODO: pre-process local scopes at this time, rather than highlight?
                // would solve problems with locals not working across boundaries
            }

            // Return the cursor back in the pool.
            ts_parser.cursors.push(cursor);

            // Reset all `LayerUpdateFlags` and remove all untouched layers
            self.layers.retain(|_, layer| {
                replace(&mut layer.flags, LayerUpdateFlags::empty())
                    .contains(LayerUpdateFlags::TOUCHED)
            });

            Ok(())
        })
    }
}

/// Compute the ranges that should be included when parsing an injection.
/// This takes into account three things:
/// * `parent_ranges` - The ranges must all fall within the *current* layer's ranges.
/// * `nodes` - Every injection takes place within a set of nodes. The injection ranges
///   are the ranges of those nodes.
/// * `includes_children` - For some injections, the content nodes' children should be
///   excluded from the nested document, so that only the content nodes' *own* content
///   is reparsed. For other injections, the content nodes' entire ranges should be
///   reparsed, including the ranges of their children.
fn intersect_ranges(
    parent_ranges: &[Range],
    nodes: &[Node],
    included_children: IncludedChildren,
) -> Vec<Range> {
    let mut cursor = nodes[0].walk();
    let mut result = Vec::new();
    let mut parent_range_iter = parent_ranges.iter();
    let mut parent_range = parent_range_iter
        .next()
        .expect("Layers should only be constructed with non-empty ranges vectors");
    for node in nodes.iter() {
        let mut preceding_range = Range {
            start_byte: 0,
            start_point: Point::new(0, 0),
            end_byte: node.start_byte(),
            end_point: node.start_position(),
        };
        let following_range = Range {
            start_byte: node.end_byte(),
            start_point: node.end_position(),
            end_byte: usize::MAX,
            end_point: Point::new(usize::MAX, usize::MAX),
        };

        for excluded_range in node
            .children(&mut cursor)
            .filter_map(|child| match included_children {
                IncludedChildren::None => Some(child.range()),
                IncludedChildren::All => None,
                IncludedChildren::Unnamed => {
                    if child.is_named() {
                        Some(child.range())
                    } else {
                        None
                    }
                }
            })
            .chain([following_range].iter().cloned())
        {
            let mut range = Range {
                start_byte: preceding_range.end_byte,
                start_point: preceding_range.end_point,
                end_byte: excluded_range.start_byte,
                end_point: excluded_range.start_point,
            };
            preceding_range = excluded_range;

            if range.end_byte < parent_range.start_byte {
                continue;
            }

            while parent_range.start_byte <= range.end_byte {
                if parent_range.end_byte > range.start_byte {
                    if range.start_byte < parent_range.start_byte {
                        range.start_byte = parent_range.start_byte;
                        range.start_point = parent_range.start_point;
                    }

                    if parent_range.end_byte < range.end_byte {
                        if range.start_byte < parent_range.end_byte {
                            result.push(Range {
                                start_byte: range.start_byte,
                                start_point: range.start_point,
                                end_byte: parent_range.end_byte,
                                end_point: parent_range.end_point,
                            });
                        }
                        range.start_byte = parent_range.end_byte;
                        range.start_point = parent_range.end_point;
                    } else {
                        if range.start_byte < range.end_byte {
                            result.push(range);
                        }
                        break;
                    }
                }

                if let Some(next_range) = parent_range_iter.next() {
                    parent_range = next_range;
                } else {
                    return result;
                }
            }
        }
    }
    result
}

impl LanguageLayer {
    fn parse(&mut self, parser: &mut Parser, source: RopeSlice) -> Result<(), Error> {
        parser
            .set_included_ranges(&self.ranges)
            .map_err(|_| Error::InvalidRanges)?;

        parser
            .set_language(&self.config.language)
            .map_err(|_| Error::InvalidLanguage)?;

        // unsafe { syntax.parser.set_cancellation_flag(cancellation_flag) };
        let tree = parser
            .parse_with(
                &mut |byte, _| {
                    if byte <= source.len_bytes() {
                        let (chunk, start_byte, _, _) = source.chunk_at_byte(byte);
                        &chunk.as_bytes()[byte - start_byte..]
                    } else {
                        // out of range
                        &[]
                    }
                },
                self.tree.as_ref(),
            )
            .ok_or(Error::Cancelled)?;
        // unsafe { ts_parser.parser.set_cancellation_flag(None) };
        self.tree = Some(tree);
        Ok(())
    }
}
