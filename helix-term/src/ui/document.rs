use std::cmp::min;

use helix_core::doc_formatter::{DocumentFormatter, GraphemeSource, TextFormat};
use helix_core::graphemes::Grapheme;
use helix_core::str_utils::char_to_byte_idx;
use helix_core::syntax::Highlight;
use helix_core::syntax::HighlightEvent;
use helix_core::text_annotations::TextAnnotations;
use helix_core::{visual_offset_from_block, Position, RopeSlice};
use helix_stdx::rope::RopeSliceExt;
use helix_view::editor::WhitespaceFeature;
use helix_view::graphics::Rect;
use helix_view::theme::Style;
use helix_view::view::ViewPosition;
use helix_view::{Document, Theme};
use tui::buffer::Buffer as Surface;

use super::trailing_whitespace::{TrailingWhitespaceTracker, WhitespaceKind};

use crate::ui::text_decorations::DecorationManager;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum StyleIterKind {
    /// base highlights (usually emitted by TS), byte indices (potentially not codepoint aligned)
    BaseHighlights,
    /// overlay highlights (emitted by custom code from selections), char indices
    Overlay,
}

/// A wrapper around a HighlightIterator
/// that merges the layered highlights to create the final text style
/// and yields the active text style and the char_idx where the active
/// style will have to be recomputed.
///
/// TODO(ropey2): hopefully one day helix and ropey will operate entirely
/// on byte ranges and we can remove this
struct StyleIter<'a, H: Iterator<Item = HighlightEvent>> {
    text_style: Style,
    active_highlights: Vec<Highlight>,
    highlight_iter: H,
    kind: StyleIterKind,
    text: RopeSlice<'a>,
    theme: &'a Theme,
}

impl<H: Iterator<Item = HighlightEvent>> Iterator for StyleIter<'_, H> {
    type Item = (Style, usize);
    fn next(&mut self) -> Option<(Style, usize)> {
        while let Some(event) = self.highlight_iter.next() {
            match event {
                HighlightEvent::HighlightStart(highlights) => {
                    self.active_highlights.push(highlights)
                }
                HighlightEvent::HighlightEnd => {
                    self.active_highlights.pop();
                }
                HighlightEvent::Source { mut end, .. } => {
                    let style = self
                        .active_highlights
                        .iter()
                        .fold(self.text_style, |acc, span| {
                            acc.patch(self.theme.highlight(span.0))
                        });
                    if self.kind == StyleIterKind::BaseHighlights {
                        end = self.text.byte_to_next_char(end);
                    }
                    return Some((style, end));
                }
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct LinePos {
    /// Indicates whether the given visual line
    /// is the first visual line of the given document line
    pub first_visual_line: bool,
    /// The line index of the document line that contains the given visual line
    pub doc_line: usize,
    /// Vertical offset from the top of the inner view area
    pub visual_line: u16,
}

#[allow(clippy::too_many_arguments)]
pub fn render_document(
    surface: &mut Surface,
    viewport: Rect,
    doc: &Document,
    offset: ViewPosition,
    doc_annotations: &TextAnnotations,
    syntax_highlight_iter: impl Iterator<Item = HighlightEvent>,
    overlay_highlight_iter: impl Iterator<Item = HighlightEvent>,
    theme: &Theme,
    decorations: DecorationManager,
) {
    let mut renderer = TextRenderer::new(
        surface,
        doc,
        theme,
        Position::new(offset.vertical_offset, offset.horizontal_offset),
        viewport,
    );
    render_text(
        &mut renderer,
        doc.text().slice(..),
        offset.anchor,
        &doc.text_format(viewport.width, Some(theme)),
        doc_annotations,
        syntax_highlight_iter,
        overlay_highlight_iter,
        theme,
        decorations,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render_text(
    renderer: &mut TextRenderer,
    text: RopeSlice<'_>,
    anchor: usize,
    text_fmt: &TextFormat,
    text_annotations: &TextAnnotations,
    syntax_highlight_iter: impl Iterator<Item = HighlightEvent>,
    overlay_highlight_iter: impl Iterator<Item = HighlightEvent>,
    theme: &Theme,
    mut decorations: DecorationManager,
) {
    let row_off = visual_offset_from_block(text, anchor, anchor, text_fmt, text_annotations)
        .0
        .row;

    let mut formatter =
        DocumentFormatter::new_at_prev_checkpoint(text, text_fmt, text_annotations, anchor);
    let mut syntax_styles = StyleIter {
        text_style: renderer.text_style,
        active_highlights: Vec::with_capacity(64),
        highlight_iter: syntax_highlight_iter,
        kind: StyleIterKind::BaseHighlights,
        theme,
        text,
    };
    let mut overlay_styles = StyleIter {
        text_style: Style::default(),
        active_highlights: Vec::with_capacity(64),
        highlight_iter: overlay_highlight_iter,
        kind: StyleIterKind::Overlay,
        theme,
        text,
    };

    let mut last_line_pos = LinePos {
        first_visual_line: false,
        doc_line: usize::MAX,
        visual_line: u16::MAX,
    };
    let mut last_line_end = 0;
    let mut is_in_indent_area = true;
    let mut last_line_indent_level = 0;
    let mut syntax_style_span = syntax_styles
        .next()
        .unwrap_or_else(|| (Style::default(), usize::MAX));
    let mut overlay_style_span = overlay_styles
        .next()
        .unwrap_or_else(|| (Style::default(), usize::MAX));
    let mut reached_view_top = false;

    loop {
        let Some(mut grapheme) = formatter.next() else {
            break;
        };

        // skip any graphemes on visual lines before the block start
        if grapheme.visual_pos.row < row_off {
            continue;
        }
        grapheme.visual_pos.row -= row_off;
        if !reached_view_top {
            decorations.prepare_for_rendering(grapheme.char_idx);
            reached_view_top = true;
        }

        // if the end of the viewport is reached stop rendering
        if grapheme.visual_pos.row as u16 >= renderer.viewport.height + renderer.offset.row as u16 {
            break;
        }

        // apply decorations before rendering a new line
        if grapheme.visual_pos.row as u16 != last_line_pos.visual_line {
            // we initiate doc_line with usize::MAX because no file
            // can reach that size (memory allocations are limited to isize::MAX)
            // initially there is no "previous" line (so doc_line is set to usize::MAX)
            // in that case we don't need to draw indent guides/virtual text
            if last_line_pos.doc_line != usize::MAX {
                // draw indent guides for the last line
                renderer.draw_indent_guides(last_line_indent_level, last_line_pos.visual_line);
                is_in_indent_area = true;
                decorations.render_virtual_lines(renderer, last_line_pos, last_line_end)
            }
            last_line_pos = LinePos {
                first_visual_line: grapheme.line_idx != last_line_pos.doc_line,
                doc_line: grapheme.line_idx,
                visual_line: grapheme.visual_pos.row as u16,
            };
            decorations.decorate_line(renderer, last_line_pos);
        }

        // acquire the correct grapheme style
        while grapheme.char_idx >= syntax_style_span.1 {
            syntax_style_span = syntax_styles
                .next()
                .unwrap_or((Style::default(), usize::MAX));
        }
        while grapheme.char_idx >= overlay_style_span.1 {
            overlay_style_span = overlay_styles
                .next()
                .unwrap_or((Style::default(), usize::MAX));
        }

        let grapheme_style = if let GraphemeSource::VirtualText { highlight } = grapheme.source {
            let mut style = renderer.text_style;
            if let Some(highlight) = highlight {
                style = style.patch(theme.highlight(highlight.0));
            }
            GraphemeStyle {
                syntax_style: style,
                overlay_style: Style::default(),
            }
        } else {
            GraphemeStyle {
                syntax_style: syntax_style_span.0,
                overlay_style: overlay_style_span.0,
            }
        };
        decorations.decorate_grapheme(renderer, &grapheme);

        let virt = grapheme.is_virtual();
        let grapheme_width = renderer.draw_grapheme(
            grapheme.raw,
            grapheme_style,
            virt,
            &mut last_line_indent_level,
            &mut is_in_indent_area,
            grapheme.visual_pos,
        );
        last_line_end = grapheme.visual_pos.col + grapheme_width;
    }

    renderer.draw_indent_guides(last_line_indent_level, last_line_pos.visual_line);
    decorations.render_virtual_lines(renderer, last_line_pos, last_line_end)
}

#[derive(Debug)]
pub struct TextRenderer<'a> {
    surface: &'a mut Surface,
    pub text_style: Style,
    pub whitespace_style: Style,
    pub trailing_whitespace_style: Style,
    pub indent_guide_char: String,
    pub indent_guide_style: Style,
    pub newline: String,
    pub nbsp: String,
    pub nnbsp: String,
    pub space: String,
    pub tab: String,
    pub virtual_tab: String,
    pub indent_width: u16,
    pub starting_indent: usize,
    pub draw_indent_guides: bool,
    pub viewport: Rect,
    pub offset: Position,
    pub trailing_whitespace_tracker: TrailingWhitespaceTracker,
}

pub struct GraphemeStyle {
    syntax_style: Style,
    overlay_style: Style,
}

impl<'a> TextRenderer<'a> {
    pub fn new(
        surface: &'a mut Surface,
        doc: &Document,
        theme: &Theme,
        offset: Position,
        viewport: Rect,
    ) -> TextRenderer<'a> {
        let editor_config = doc.config.load();

        let tab_width = doc.tab_width();
        let text_style = theme.get("ui.text");
        let indent_width = doc.indent_style.indent_width(tab_width) as u16;

        let ws = &editor_config.whitespace;
        let regular_ws = WhitespaceFeature::Regular.palette(ws, tab_width);
        let trailing_ws = WhitespaceFeature::Trailing.palette(ws, tab_width);
        let trailing_whitespace_tracker = TrailingWhitespaceTracker::new(ws.render, trailing_ws);

        TextRenderer {
            surface,
            indent_guide_char: editor_config.indent_guides.character.into(),
            newline: regular_ws.newline,
            nbsp: regular_ws.nbsp,
            nnbsp: regular_ws.nnbsp,
            space: regular_ws.space,
            tab: regular_ws.tab,
            virtual_tab: regular_ws.virtual_tab,
            whitespace_style: theme.get("ui.virtual.whitespace"),
            trailing_whitespace_style: theme.get("ui.virtual.trailing_whitespace"),
            indent_width,
            starting_indent: offset.col / indent_width as usize
                + (offset.col % indent_width as usize != 0) as usize
                + editor_config.indent_guides.skip_levels as usize,
            indent_guide_style: text_style.patch(
                theme
                    .try_get("ui.virtual.indent-guide")
                    .unwrap_or_else(|| theme.get("ui.virtual.whitespace")),
            ),
            text_style,
            draw_indent_guides: editor_config.indent_guides.render,
            viewport,
            offset,
            trailing_whitespace_tracker,
        }
    }
    /// Draws a single `grapheme` at the current render position with a specified `style`.
    pub fn draw_decoration_grapheme(
        &mut self,
        grapheme: Grapheme,
        mut style: Style,
        mut row: u16,
        col: u16,
    ) -> bool {
        if (row as usize) < self.offset.row
            || row >= self.viewport.height
            || col >= self.viewport.width
        {
            return false;
        }
        row -= self.offset.row as u16;
        // TODO is it correct to apply the whitspace style to all unicode white spaces?
        if grapheme.is_whitespace() {
            style = style.patch(self.whitespace_style);
        }

        let grapheme = match grapheme {
            Grapheme::Tab { width } => {
                let grapheme_tab_width = char_to_byte_idx(&self.virtual_tab, width);
                &self.virtual_tab[..grapheme_tab_width]
            }
            Grapheme::Other { ref g } if g == "\u{00A0}" => " ",
            Grapheme::Other { ref g } => g,
            Grapheme::Newline => " ",
        };

        self.surface.set_string(
            self.viewport.x + col,
            self.viewport.y + row,
            grapheme,
            style,
        );
        true
    }

    /// Draws a single `grapheme` at the current render position with a specified `style`.
    pub fn draw_grapheme(
        &mut self,
        grapheme: Grapheme,
        grapheme_style: GraphemeStyle,
        is_virtual: bool,
        last_indent_level: &mut usize,
        is_in_indent_area: &mut bool,
        mut position: Position,
    ) -> usize {
        if position.row < self.offset.row {
            return 0;
        }
        position.row -= self.offset.row;
        let cut_off_start = self.offset.col.saturating_sub(position.col);
        let is_whitespace = grapheme.is_whitespace();

        // TODO is it correct to apply the whitespace style to all unicode white spaces?
        let mut style = grapheme_style.syntax_style;
        if is_whitespace {
            style = style.patch(self.whitespace_style);
        }
        style = style.patch(grapheme_style.overlay_style);

        let width = grapheme.width();
        let space = if is_virtual { " " } else { &self.space };
        let nbsp = if is_virtual { " " } else { &self.nbsp };
        let nnbsp = if is_virtual { " " } else { &self.nnbsp };
        let tab = if is_virtual {
            &self.virtual_tab
        } else {
            &self.tab
        };
        let mut whitespace_kind = WhitespaceKind::None;
        let grapheme = match grapheme {
            Grapheme::Tab { width } => {
                whitespace_kind = WhitespaceKind::Tab;
                let grapheme_tab_width = char_to_byte_idx(tab, width);
                &tab[..grapheme_tab_width]
            }
            // TODO special rendering for other whitespaces?
            Grapheme::Other { ref g } if g == " " => {
                whitespace_kind = WhitespaceKind::Space;
                space
            }
            Grapheme::Other { ref g } if g == "\u{00A0}" => {
                whitespace_kind = WhitespaceKind::NonBreakingSpace;
                nbsp
            }
            Grapheme::Other { ref g } if g == "\u{202F}" => {
                whitespace_kind = WhitespaceKind::NarrowNonBreakingSpace;
                nnbsp
            }
            Grapheme::Other { ref g } => g,
            Grapheme::Newline => {
                whitespace_kind = WhitespaceKind::Newline;
                &self.newline
            }
        };

        let viewport_right_edge = self.viewport.width as usize + self.offset.col - 1;
        let in_bounds = self.column_in_bounds(position.col, width);

        if in_bounds {
            let in_bounds_col = position.col - self.offset.col;
            self.surface.set_string(
                self.viewport.x + in_bounds_col as u16,
                self.viewport.y + position.row as u16,
                grapheme,
                style,
            );

            if self
                .trailing_whitespace_tracker
                .track(in_bounds_col, whitespace_kind)
                || position.col == viewport_right_edge
            {
                self.trailing_whitespace_tracker.render(
                    &mut |trailing_whitespace: &str, from: usize| {
                        self.surface.set_string(
                            self.viewport.x + from as u16,
                            self.viewport.y + position.row as u16,
                            trailing_whitespace,
                            style.patch(self.trailing_whitespace_style),
                        );
                    },
                );
            }
        } else if cut_off_start != 0 && cut_off_start < width {
            // partially on screen
            let rect = Rect::new(
                self.viewport.x,
                self.viewport.y + position.row as u16,
                (width - cut_off_start) as u16,
                1,
            );
            self.surface.set_style(rect, style);
        }
        if *is_in_indent_area && !is_whitespace {
            *last_indent_level = position.col;
            *is_in_indent_area = false;
        }

        width
    }

    pub fn column_in_bounds(&self, colum: usize, width: usize) -> bool {
        self.offset.col <= colum && colum + width <= self.offset.col + self.viewport.width as usize
    }

    /// Overlay indentation guides ontop of a rendered line
    /// The indentation level is computed in `draw_lines`.
    /// Therefore this function must always be called afterwards.
    pub fn draw_indent_guides(&mut self, indent_level: usize, mut row: u16) {
        if !self.draw_indent_guides || self.offset.row > row as usize {
            return;
        }
        row -= self.offset.row as u16;

        // Don't draw indent guides outside of view
        let end_indent = min(
            indent_level,
            // Add indent_width - 1 to round up, since the first visible
            // indent might be a bit after offset.col
            self.offset.col + self.viewport.width as usize + (self.indent_width as usize - 1),
        ) / self.indent_width as usize;

        for i in self.starting_indent..end_indent {
            let x = (self.viewport.x as usize + (i * self.indent_width as usize) - self.offset.col)
                as u16;
            let y = self.viewport.y + row;
            debug_assert!(self.surface.in_bounds(x, y));
            self.surface
                .set_string(x, y, &self.indent_guide_char, self.indent_guide_style);
        }
    }

    pub fn set_string(&mut self, x: u16, y: u16, string: impl AsRef<str>, style: Style) {
        if (y as usize) < self.offset.row {
            return;
        }
        self.surface
            .set_string(x, y + self.viewport.y, string, style)
    }

    pub fn set_stringn(
        &mut self,
        x: u16,
        y: u16,
        string: impl AsRef<str>,
        width: usize,
        style: Style,
    ) {
        if (y as usize) < self.offset.row {
            return;
        }
        self.surface
            .set_stringn(x, y + self.viewport.y, string, width, style);
    }

    /// Sets the style of an area **within the text viewport* this accounts
    /// both for the renderers vertical offset and its viewport
    pub fn set_style(&mut self, mut area: Rect, style: Style) {
        area = area.clip_top(self.offset.row as u16);
        area.y += self.viewport.y;
        self.surface.set_style(area, style);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_string_truncated(
        &mut self,
        x: u16,
        y: u16,
        string: &str,
        width: usize,
        style: impl Fn(usize) -> Style, // Map a grapheme's string offset to a style
        ellipsis: bool,
        truncate_start: bool,
    ) -> (u16, u16) {
        if (y as usize) < self.offset.row {
            return (x, y);
        }
        self.surface.set_string_truncated(
            x,
            y + self.viewport.y,
            string,
            width,
            style,
            ellipsis,
            truncate_start,
        )
    }
}
