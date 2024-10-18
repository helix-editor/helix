#[allow(deprecated)]
use helix_core::visual_coords_at_pos;

use helix_core::{
    syntax::RopeProvider,
    text_annotations::TextAnnotations,
    tree_sitter::{Node, QueryCursor, QueryMatch},
    Position,
};

use helix_view::{editor::Config, graphics::Rect, Document, DocumentId, Theme, View};

use tui::buffer::Buffer as Surface;

use crate::ui::text_decorations::DecorationManager;

use super::{
    document::{render_text, TextRenderer},
    EditorView,
};

#[derive(Debug, Default, Clone)]
pub struct StickyNode {
    pub line: usize,
    pub visual_line: u16,
    pub byte_range: std::ops::Range<usize>,
    pub indicator: Option<String>,
    pub anchor: usize,
    pub has_context_end: bool,
    pub doc_id: DocumentId,
}

#[derive(Debug)]
struct StickyNodeContext {
    /// The row on which the cursor is placed
    pub visual_row: usize,
    /// This marks the location of which we take possible out of range nodes
    /// e.g. on follows-cursor: 'on', this would be the parent nodes, relative to the *cursor*
    /// on follows-cursor: 'off', this would be the parent nodes, relative to the *topmost screen*
    pub context_location: usize,
    /// The topmost byte that is visible (and not hidden behind sticky nodes)
    pub topmost_byte: usize,
    /// The anchor of the view offset
    pub anchor_line: usize,
    /// The viewport
    pub viewport: Rect,
}

impl StickyNodeContext {
    pub fn from_context(
        last_nodes: Option<&Vec<StickyNode>>,
        doc: &Document,
        view: &View,
        config: &Config,
        cursor_cache: Option<&Position>,
    ) -> Option<Self> {
        let Some(cursor_cache) = cursor_cache else {
            return None;
        };
        let text = doc.text().slice(..);
        let viewport = view.inner_area(doc);
        let cursor_byte = text.char_to_byte(doc.selection(view.id).primary().cursor(text));

        let anchor_line = text.char_to_line(doc.view_offset(view.id).anchor);
        let visual_cursor_row = cursor_cache.row;

        if visual_cursor_row == 0 {
            return None;
        }

        let top_first_byte = text
            .try_line_to_byte(anchor_line + last_nodes.as_ref().map_or(0, |nodes| nodes.len()))
            .ok()?;

        let last_scan_byte = if config.sticky_context.follow_cursor {
            cursor_byte
        } else {
            top_first_byte
        };
        Some(Self {
            visual_row: visual_cursor_row,
            context_location: last_scan_byte,
            topmost_byte: top_first_byte,
            anchor_line,
            viewport,
        })
    }
}

/// Calculates the sticky nodes
pub fn calculate_sticky_nodes(
    nodes: Option<Vec<StickyNode>>,
    doc: &Document,
    view: &View,
    config: &Config,
    cursor_cache: Option<&Position>,
) -> Option<Vec<StickyNode>> {
    let Some(mut context) =
        StickyNodeContext::from_context(nodes.as_ref(), doc, view, config, cursor_cache)
    else {
        return None;
    };

    let syntax = doc.syntax()?;
    let tree = syntax.tree();
    let text = doc.text().slice(..);

    let mut cached_nodes = build_cached_nodes(doc, nodes, view, &mut context).unwrap_or_default();

    if cached_nodes.iter().any(|node| node.doc_id != view.doc) {
        cached_nodes.clear();
    }

    let start_byte_range = cached_nodes
        .last()
        .unwrap_or(&StickyNode::default())
        .byte_range
        .clone();

    let start_byte = if start_byte_range.start != tree.root_node().start_byte() {
        start_byte_range.start
    } else {
        context.context_location
    };

    let mut result: Vec<StickyNode> = Vec::new();
    let mut start_node = tree
        .root_node()
        .descendant_for_byte_range(start_byte, start_byte);

    // When the start_node is the root node... there's no point in searching further
    if let Some(start_node) = start_node {
        if start_node.byte_range() == tree.root_node().byte_range() {
            return None;
        }
    }

    // Traverse to the parent node
    while start_node
        .unwrap_or_else(|| tree.root_node())
        .parent()
        .unwrap_or_else(|| tree.root_node())
        .byte_range()
        != tree.root_node().byte_range()
    {
        let Some(start) = start_node else {
            continue;
        };

        start_node = start.parent();
    }

    let context_nodes = doc
        .language_config()
        .and_then(|lang| lang.context_query())?;

    let start_index = context_nodes.query.capture_index_for_name("context")?;
    let end_index = context_nodes
        .query
        .capture_index_for_name("context.params")
        .unwrap_or(start_index);

    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(start_byte_range.start..context.context_location);

    let query = &context_nodes.query;
    // Collect the query, for further iteration
    let query_matches = cursor.matches(
        query,
        start_node.unwrap_or_else(|| tree.root_node()),
        RopeProvider(text),
    );

    for matched_node in query_matches {
        // find @context.params nodes
        let node_byte_range = get_context_paired_range(
            &matched_node,
            start_index,
            end_index,
            context.topmost_byte,
            context.context_location,
        );

        for node in matched_node.nodes_for_capture_index(start_index) {
            let mut last_node_add = 0;
            if let Some(last_node) = result.last() {
                if last_node.line == (node.start_position().row) {
                    last_node_add += text
                        .line(text.byte_to_line(context.topmost_byte))
                        .len_bytes();
                }
            }

            if node_in_range(
                node,
                context.anchor_line,
                node_byte_range.as_ref(),
                context.context_location,
                context.topmost_byte + last_node_add,
                result.len(),
            ) {
                continue;
            }

            result.push(StickyNode {
                line: node.start_position().row,
                visual_line: 0,
                byte_range: node_byte_range
                    .as_ref()
                    .unwrap_or(&(node.start_byte()..node.end_byte()))
                    .clone(),
                indicator: None,
                anchor: doc.view_offset(view.id).anchor,
                has_context_end: node_byte_range.is_some(),
                doc_id: view.doc,
            });
        }
    }
    // result should be filled by now
    if result.is_empty() {
        if !cached_nodes.is_empty() {
            if config.sticky_context.indicator {
                return Some(add_indicator(doc, &context.viewport, view, cached_nodes));
            }

            return Some(cached_nodes);
        }

        return None;
    }

    let mut res = {
        cached_nodes.append(&mut result);
        cached_nodes
    };

    // Order of commands is important here
    res.sort_unstable_by(|lhs, rhs| lhs.line.cmp(&rhs.line));
    res.dedup_by(|lhs, rhs| lhs.line == rhs.line);

    // always cap the maximum amount of sticky contextes to 1/3 of the viewport
    // unless configured otherwise
    let max_lines = config.sticky_context.max_lines as u16;
    let max_nodes_amount = max_lines.min(context.viewport.height / 3) as usize;

    let skip = res.len().saturating_sub(max_nodes_amount);

    res = res
        .iter()
        // only take the nodes until 1 / 3 of the viewport is reached or the maximum amount of sticky nodes
        .skip(skip)
        .enumerate()
        .take_while(|(i, _)| {
            *i + Into::<usize>::into(config.sticky_context.indicator) != context.visual_row
        }) // also only nodes that don't overlap with the visual cursor position
        .map(|(i, node)| {
            let mut new_node = node.clone();
            new_node.visual_line = i as u16;
            new_node
        })
        .collect();

    if config.sticky_context.indicator {
        res = add_indicator(doc, &context.viewport, view, res);
    }

    Some(res)
}

fn build_cached_nodes(
    doc: &Document,
    nodes: Option<Vec<StickyNode>>,
    view: &View,
    context: &mut StickyNodeContext,
) -> Option<Vec<StickyNode>> {
    if let Some(nodes) = nodes {
        if nodes.iter().any(|node| view.doc != node.doc_id) {
            return None;
        }

        // nothing has changed, so the cached result can be returned
        if nodes
            .iter()
            .any(|node| doc.view_offset(view.id).anchor == node.anchor)
        {
            return Some(nodes.iter().take(context.visual_row).cloned().collect());
        }

        // Nodes are elligible for reuse
        // While the cached nodes are outside our search-range, pop them, too
        let valid_nodes: Vec<StickyNode> = nodes
            .into_iter()
            .filter(|node| node.byte_range.contains(&context.topmost_byte))
            .collect();

        return Some(valid_nodes);
    }

    None
}

fn get_context_paired_range(
    query_match: &QueryMatch,
    start_index: u32,
    end_index: u32,
    top_first_byte: usize,
    last_scan_byte: usize,
) -> Option<std::ops::Range<usize>> {
    // get all the captured @context.params nodes
    let end_nodes = once_cell::unsync::Lazy::new(|| {
        query_match
            .nodes_for_capture_index(end_index)
            .collect::<Vec<_>>()
    });

    query_match
        .nodes_for_capture_index(start_index)
        .find_map(|context| {
            let ctx_start_range = context.byte_range();

            // filter all matches that are out of scope, based on follows-cursor
            let start_range_contains_bytes = ctx_start_range.contains(&top_first_byte)
                && ctx_start_range.contains(&last_scan_byte);
            if !start_range_contains_bytes {
                return None;
            }

            let ctx_start_row = context.start_position().row;
            let ctx_start_byte = ctx_start_range.start;

            end_nodes.iter().find_map(|it| {
                let end = it.end_byte();
                // check whether or not @context.params nodes are on different lines
                (ctx_start_row != it.end_position().row && ctx_start_range.contains(&end))
                    .then_some(ctx_start_byte..end)
            })
        })
}

/// Tests whether or not a given node is in a specific tree-sitter range
fn node_in_range(
    node: Node,
    anchor: usize,
    node_byte_range: Option<&std::ops::Range<usize>>,
    last_scan_byte: usize,
    topmost_byte: usize,
    result_len: usize,
) -> bool {
    (!node.byte_range().contains(&last_scan_byte) || !node.byte_range().contains(&topmost_byte))
        && node.start_position().row != anchor + result_len
        && node_byte_range.is_none()
}

/// Adds an indicator line to the Sticky Context
fn add_indicator(
    doc: &Document,
    viewport: &Rect,
    view: &View,
    res: Vec<StickyNode>,
) -> Vec<StickyNode> {
    let mut res = res;
    let str = "─".repeat(viewport.width as usize);
    res.push(StickyNode {
        line: usize::MAX,
        visual_line: res.len() as u16,
        byte_range: 0..0,
        indicator: Some(str),
        anchor: doc.view_offset(view.id).anchor,
        has_context_end: false,
        doc_id: view.doc,
    });

    res
}

/// Render the sticky context
pub fn render_sticky_context(
    doc: &Document,
    view: &View,
    surface: &mut Surface,
    context: Option<&Vec<StickyNode>>,
    theme: &Theme,
) {
    let Some(context) = context else {
        return;
    };

    let text = doc.text().slice(..);
    let viewport = view.inner_area(doc);

    // backup (status line) shall always exist
    let status_line_style = theme
        .try_get("ui.statusline.context")
        .expect("`ui.statusline.context` exists");

    // define sticky context styles
    let context_style = theme
        .try_get("ui.sticky.context")
        .unwrap_or(status_line_style);
    let indicator_style = theme
        .try_get("ui.sticky.indicator")
        .unwrap_or(status_line_style);

    let mut context_area = viewport;
    context_area.height = 1;

    const DOTS: &str = "...";

    for node in context {
        surface.clear_with(context_area, context_style);

        if let Some(indicator) = node.indicator.as_deref() {
            // set the indicator
            surface.set_stringn(
                context_area.x,
                context_area.y,
                indicator,
                indicator.len(),
                indicator_style,
            );
            continue;
        }

        let node_start = text.byte_to_char(node.byte_range.start);
        let first_node_line = text.line(text.char_to_line(node_start));

        // subtract 1 to handle indexes
        let mut first_node_line_end = first_node_line.len_chars().saturating_sub(1);

        // trim trailing whitespace / newline
        first_node_line_end -= first_node_line
            .chars()
            .reversed()
            .position(|c| !c.is_whitespace())
            .unwrap_or(0);

        #[allow(deprecated)]
        let Position {
            row: _,
            col: first_node_line_end,
        } = visual_coords_at_pos(first_node_line, first_node_line_end, doc.tab_width());

        // get the highlighting of the basic capture
        let syntax_highlights = EditorView::doc_syntax_highlights(doc, node_start, 1, theme);
        let overlay_highlights = EditorView::empty_highlight_iter(doc, node_start, 1);

        let mut offset_area = context_area;

        // Limit scope of borrowed surface
        {
            let mut renderer =
                TextRenderer::new(surface, doc, theme, Position::new(0, 0), context_area);

            // create the formatting for the basic node render
            let mut formatting = doc.text_format(context_area.width, Some(theme));
            formatting.soft_wrap = false;

            render_text(
                &mut renderer,
                text,
                node_start,
                &formatting,
                &TextAnnotations::default(),
                syntax_highlights,
                overlay_highlights,
                theme,
                DecorationManager::default(),
            );
            offset_area.x += first_node_line_end as u16;
        }

        if node.has_context_end {
            let node_end = text.byte_to_char(node.byte_range.end);
            let end_node_line = text.line(text.char_to_line(node_end));
            let whitespace_offset = end_node_line
                .chars()
                .position(|c| !c.is_whitespace())
                .unwrap_or(0);

            #[allow(deprecated)]
            let Position {
                col: end_vis_offset,
                row: _,
            } = visual_coords_at_pos(end_node_line, whitespace_offset, doc.tab_width());

            surface.set_stringn(
                offset_area.x,
                offset_area.y,
                DOTS,
                DOTS.len(),
                theme.get("keyword.operator"),
            );
            offset_area.x += DOTS.len() as u16;

            let mut renderer = TextRenderer::new(
                surface,
                doc,
                theme,
                Position::new(0, end_vis_offset),
                offset_area,
            );

            let syntax_highlights = EditorView::doc_syntax_highlights(doc, node_end, 1, theme);
            let overlay_highlights = EditorView::empty_highlight_iter(doc, node_end, 1);

            let mut formatting = doc.text_format(offset_area.width, Some(theme));
            formatting.soft_wrap = false;

            render_text(
                &mut renderer,
                text,
                node_end,
                &formatting,
                &TextAnnotations::default(),
                syntax_highlights,
                overlay_highlights,
                theme,
                DecorationManager::default(),
            );
        }

        // next node
        context_area.y += 1;
    }
}
