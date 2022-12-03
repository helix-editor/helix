use std::borrow::Cow;
use std::cmp::min;

use helix_core::doc_cursor::{AnnotationSource, DocumentCursor};
use helix_core::graphemes::Grapheme;
use helix_core::str_utils::char_to_byte_idx;
use helix_core::syntax::Highlight;
use helix_core::syntax::HighlightEvent;
use helix_core::{Position, RopeSlice};
use helix_view::editor::{WhitespaceConfig, WhitespaceRenderValue};
use helix_view::graphics::Rect;
use helix_view::theme::Style;
use helix_view::Theme;
use helix_view::{editor, Document};
use tui::buffer::Buffer as Surface;

pub struct DocumentRender<'a, H: Iterator<Item = HighlightEvent>, A: AnnotationSource<'a>> {
    pub config: &'a editor::Config,
    pub theme: &'a Theme,

    text: RopeSlice<'a>,
    pub cursor: DocumentCursor<'a, Style, A>,

    highlights: H,
    spans: Vec<Highlight>,
    highlight_scope: (usize, Style),

    is_finished: bool,
}

impl<'a, H: Iterator<Item = HighlightEvent>, A: AnnotationSource<'a>> DocumentRender<'a, H, A> {
    pub fn new(
        config: &'a editor::Config,
        theme: &'a Theme,
        text: RopeSlice<'a>,
        cursor: DocumentCursor<'a, Style, A>,
        highlights: H,
        text_render: &mut TextRender,
    ) -> Self {
        let mut render = DocumentRender {
            config,
            theme,
            highlights,
            cursor,
            // render: TextRender::new(surface, render_config, offset.col, viewport),
            spans: Vec::with_capacity(64),
            is_finished: false,
            highlight_scope: (0, Style::default()),
            text,
        };

        // advance to first highlight scope
        render.advance_highlight_scope(text_render);
        render
    }

    /// Advance to the next treesitter highlight range
    /// if the last one is exhaused
    fn advance_highlight_scope(&mut self, text_render: &mut TextRender) {
        while let Some(event) = self.highlights.next() {
            match event {
                HighlightEvent::HighlightStart(span) => self.spans.push(span),
                HighlightEvent::HighlightEnd => {
                    self.spans.pop();
                }
                HighlightEvent::Source { start, end } => {
                    if start == end {
                        continue;
                    }
                    // TODO cursor end
                    let style = self
                        .spans
                        .iter()
                        .fold(text_render.config.text_style, |acc, span| {
                            acc.patch(self.theme.highlight(span.0))
                        });
                    self.highlight_scope = (end, style);
                    return;
                }
            }
        }
        self.is_finished = true;
    }

    /// Returns whether this document renderer finished rendering
    /// either because the viewport end or EOF was reached
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// Renders the next line of the document.
    /// If softwrapping is enabled this may only correspond to rendering a part of the line
    ///
    /// # Returns
    ///
    /// Whether the rendered line was only partially rendered because the viewport end was reached
    pub fn render_line<const SOFTWRAP: bool>(&mut self, text_render: &mut TextRender) {
        if self.is_finished {
            return;
        }

        loop {
            while let Some(word) = self.cursor.advance::<SOFTWRAP>(self.highlight_scope) {
                text_render.posititon = word.visual_position;
                for grapheme in word.graphmes {
                    text_render.draw_grapheme(grapheme);
                }

                let line_break = if let Some(line_break) = word.terminating_linebreak {
                    line_break
                } else {
                    continue;
                };

                if self.config.indent_guides.render {
                    text_render.draw_indent_guides();
                }
                if !line_break.is_softwrap {
                    // render EOL space
                    text_render.draw_grapheme(StyledGrapheme {
                        grapheme: Grapheme::Space,
                        style: self.highlight_scope.1,
                    });
                }

                if self.config.indent_guides.render {
                    text_render.draw_indent_guides()
                }
                text_render.posititon.row += 1;
                self.is_finished = text_render.reached_viewport_end();
                return;
            }

            self.advance_highlight_scope(text_render);

            // we properly reached the text end, this is the end of the last line
            // render remaining text
            if self.is_finished {
                // the last word is garunteed to fit on the last line
                // and to not wrap (otherwise it would have been yielded before)
                // render it
                for grapheme in self.cursor.finish() {
                    text_render.draw_grapheme(grapheme);
                }

                if self.highlight_scope.0 > self.text.len_chars() {
                    // trailing cursor is rendered as a whitespace
                    text_render.draw_grapheme(StyledGrapheme {
                        grapheme: Grapheme::Space,
                        style: self.highlight_scope.1,
                    });
                }

                return;
            }

            // we reached the viewport end but the line was only partially rendered
            if text_render.reached_viewport_end() {
                if self.config.indent_guides.render {
                    text_render.draw_indent_guides()
                }
                self.is_finished = true;
                return;
            }
        }
    }
}

pub type StyledGrapheme<'a> = helix_core::graphemes::StyledGrapheme<'a, Style>;

/// A TextRender Basic grapheme rendering and visual position tracking
#[derive(Debug)]
pub struct TextRender<'a> {
    /// Surface to render to
    surface: &'a mut Surface,
    /// Various constants required for rendering
    pub config: &'a TextRenderConfig,
    viewport: Rect,
    col_offset: usize,
    indent_known: bool,
    indent_level: usize,
    pub posititon: Position,
}

impl<'a> TextRender<'a> {
    pub fn new(
        surface: &'a mut Surface,
        config: &'a TextRenderConfig,
        col_offset: usize,
        viewport: Rect,
    ) -> TextRender<'a> {
        TextRender {
            surface,
            config,
            viewport,
            col_offset,
            posititon: Position { row: 0, col: 0 },
            indent_level: 0,
            indent_known: false,
        }
    }

    /// Draws a single `grapheme` at the current render position with a specified `style`.
    pub fn draw_grapheme(&mut self, styled_grapheme: StyledGrapheme) {
        let cut_off_start = self.col_offset.saturating_sub(self.posititon.row as usize);
        let is_whitespace = styled_grapheme.is_whitespace();

        let style = if is_whitespace {
            styled_grapheme.style.patch(self.config.whitespace_style)
        } else {
            styled_grapheme.style
        };
        let (width, grapheme) = match styled_grapheme.grapheme {
            Grapheme::Tab { width } => {
                let grapheme_tab_width = char_to_byte_idx(&self.config.tab, width as usize);
                (width, Cow::from(&self.config.tab[..grapheme_tab_width]))
            }

            Grapheme::Space => (1, Cow::from(&self.config.space)),
            Grapheme::Nbsp => (1, Cow::from(&self.config.nbsp)),
            Grapheme::Other { width, raw: str } => (width, str),
            Grapheme::Newline => (1, Cow::from(&self.config.newline)),
        };

        if self.in_bounds() {
            self.surface.set_string(
                self.viewport.x + (self.posititon.col - self.col_offset) as u16,
                self.viewport.y + self.posititon.row as u16,
                grapheme,
                style,
            );
        } else if cut_off_start != 0 && cut_off_start < width as usize {
            // partially on screen
            let rect = Rect::new(
                self.viewport.x as u16,
                self.viewport.y + self.posititon.row as u16,
                width - cut_off_start as u16,
                1,
            );
            self.surface.set_style(rect, style);
        }

        if !is_whitespace && !self.indent_known {
            self.indent_known = true;
            self.indent_level = self.posititon.col;
        }
        self.posititon.col += width as usize;
    }

    /// Returns whether the current column is in bounds
    fn in_bounds(&self) -> bool {
        self.col_offset <= (self.posititon.col as usize)
            && (self.posititon.col as usize) < self.viewport.width as usize + self.col_offset
    }

    /// Overlay indentation guides ontop of a rendered line
    /// The indentation level is computed in `draw_lines`.
    /// Therefore this function must always be called afterwards.
    pub fn draw_indent_guides(&mut self) {
        // Don't draw indent guides outside of view
        let end_indent = min(
            self.indent_level,
            // Add tab_width - 1 to round up, since the first visible
            // indent might be a bit after offset.col
            self.col_offset + self.viewport.width as usize + (self.config.tab_width - 1) as usize,
        ) / self.config.tab_width as usize;

        for i in self.config.starting_indent..end_indent {
            let x = (self.viewport.x as usize + (i * self.config.tab_width as usize)
                - self.col_offset) as u16;
            let y = self.viewport.y + self.posititon.row as u16;
            debug_assert!(self.surface.in_bounds(x, y));
            self.surface.set_string(
                x,
                y,
                &self.config.indent_guide_char,
                self.config.indent_guide_style,
            );
        }

        // reset indentation level for next line
        self.indent_known = false;
    }

    pub fn reached_viewport_end(&mut self) -> bool {
        self.posititon.row as u16 >= self.viewport.height
    }
}

#[derive(Debug)]
/// Various constants required for text rendering.
pub struct TextRenderConfig {
    pub text_style: Style,
    pub whitespace_style: Style,
    pub indent_guide_char: String,
    pub indent_guide_style: Style,
    pub newline: String,
    pub nbsp: String,
    pub space: String,
    pub tab: String,
    pub tab_width: u16,
    pub starting_indent: usize,
}

impl TextRenderConfig {
    pub fn new(
        doc: &Document,
        editor_config: &editor::Config,
        theme: &Theme,
        offset: &Position,
    ) -> TextRenderConfig {
        let WhitespaceConfig {
            render: ws_render,
            characters: ws_chars,
        } = &editor_config.whitespace;

        let tab_width = doc.tab_width();
        let tab = if ws_render.tab() == WhitespaceRenderValue::All {
            std::iter::once(ws_chars.tab)
                .chain(std::iter::repeat(ws_chars.tabpad).take(tab_width - 1))
                .collect()
        } else {
            " ".repeat(tab_width)
        };
        let newline = if ws_render.newline() == WhitespaceRenderValue::All {
            ws_chars.newline.into()
        } else {
            " ".to_owned()
        };

        let space = if ws_render.space() == WhitespaceRenderValue::All {
            ws_chars.space.into()
        } else {
            " ".to_owned()
        };
        let nbsp = if ws_render.nbsp() == WhitespaceRenderValue::All {
            ws_chars.nbsp.into()
        } else {
            " ".to_owned()
        };

        let text_style = theme.get("ui.text");

        TextRenderConfig {
            indent_guide_char: editor_config.indent_guides.character.into(),
            newline,
            nbsp,
            space,
            tab_width: tab_width as u16,
            tab,
            whitespace_style: theme.get("ui.virtual.whitespace"),
            starting_indent: (offset.col / tab_width)
                + editor_config.indent_guides.skip_levels as usize,
            indent_guide_style: text_style.patch(
                theme
                    .try_get("ui.virtual.indent-guide")
                    .unwrap_or_else(|| theme.get("ui.virtual.whitespace")),
            ),
            text_style,
        }
    }
}
