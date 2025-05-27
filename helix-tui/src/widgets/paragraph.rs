use crate::{
    buffer::Buffer,
    layout::Alignment,
    text::{StyledGrapheme, Text},
    widgets::{
        reflow::{LineComposer, LineTruncator, WordWrapper},
        Block, Widget,
    },
};
use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::graphics::{Rect, Style};
use std::iter;

fn get_line_offset(line_width: u16, text_area_width: u16, alignment: Alignment) -> u16 {
    match alignment {
        Alignment::Center => (text_area_width / 2).saturating_sub(line_width / 2),
        Alignment::Right => text_area_width.saturating_sub(line_width),
        Alignment::Left => 0,
    }
}

/// A widget to display some text.
///
/// # Examples
///
/// ```
/// # use helix_tui::text::{Text, Spans, Span};
/// # use helix_tui::widgets::{Block, Borders, Paragraph, Wrap};
/// # use helix_tui::layout::{Alignment};
/// # use helix_view::graphics::{Style, Color, Modifier};
/// let text = Text::from(vec![
///     Spans::from(vec![
///         Span::raw("First"),
///         Span::styled("line",Style::default().add_modifier(Modifier::ITALIC)),
///         Span::raw("."),
///     ]),
///     Spans::from(Span::styled("Second line", Style::default().fg(Color::Red))),
/// ]);
/// Paragraph::new(&text)
///     .block(Block::bordered().title("Paragraph"))
///     .style(Style::default().fg(Color::White).bg(Color::Black))
///     .alignment(Alignment::Center)
///     .wrap(Wrap { trim: true });
/// ```
#[derive(Debug, Clone)]
pub struct Paragraph<'a> {
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
    /// How to wrap the text
    wrap: Option<Wrap>,
    /// The text to display
    text: &'a Text<'a>,
    /// Scroll
    scroll: (u16, u16),
    /// Alignment of the text
    alignment: Alignment,
}

/// Describes how to wrap text across lines.
///
/// ## Examples
///
/// ```
/// # use helix_tui::widgets::{Paragraph, Wrap};
/// # use helix_tui::text::Text;
/// let bullet_points = Text::from(r#"Some indented points:
///     - First thing goes here and is long so that it wraps
///     - Here is another point that is long enough to wrap"#);
///
/// // With leading spaces trimmed (window width of 30 chars):
/// Paragraph::new(&bullet_points).wrap(Wrap { trim: true });
/// // Some indented points:
/// // - First thing goes here and is
/// // long so that it wraps
/// // - Here is another point that
/// // is long enough to wrap
///
/// // But without trimming, indentation is preserved:
/// Paragraph::new(&bullet_points).wrap(Wrap { trim: false });
/// // Some indented points:
/// //     - First thing goes here
/// // and is long so that it wraps
/// //     - Here is another point
/// // that is long enough to wrap
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Wrap {
    /// Should leading whitespace be trimmed
    pub trim: bool,
}

impl<'a> Paragraph<'a> {
    pub fn new(text: &'a Text) -> Paragraph<'a> {
        Paragraph {
            block: None,
            style: Default::default(),
            wrap: None,
            text,
            scroll: (0, 0),
            alignment: Alignment::Left,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Paragraph<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Paragraph<'a> {
        self.style = style;
        self
    }

    pub fn wrap(mut self, wrap: Wrap) -> Paragraph<'a> {
        self.wrap = Some(wrap);
        self
    }

    pub fn scroll(mut self, offset: (u16, u16)) -> Paragraph<'a> {
        self.scroll = offset;
        self
    }

    pub fn alignment(mut self, alignment: Alignment) -> Paragraph<'a> {
        self.alignment = alignment;
        self
    }

    pub fn required_size(&self, max_text_width: u16) -> (u16, u16) {
        let style = self.style;
        let mut styled = self.text.lines.iter().flat_map(|spans| {
            spans
                .0
                .iter()
                .flat_map(|span| span.styled_graphemes(style))
                // Required given the way composers work but might be refactored out if we change
                // composers to operate on lines instead of a stream of graphemes.
                .chain(iter::once(StyledGrapheme {
                    symbol: "\n",
                    style: self.style,
                }))
        });
        let mut line_composer: Box<dyn LineComposer> = if let Some(Wrap { trim }) = self.wrap {
            Box::new(WordWrapper::new(&mut styled, max_text_width, trim))
        } else {
            let mut line_composer = Box::new(LineTruncator::new(&mut styled, max_text_width));
            if self.alignment == Alignment::Left {
                line_composer.set_horizontal_offset(self.scroll.1);
            }
            line_composer
        };
        let mut text_width = 0;
        let mut text_height = 0;
        while let Some((_, line_width)) = line_composer.next_line() {
            text_width = line_width.max(text_width);
            text_height += 1;
        }
        (text_width, text_height)
    }
}

impl Widget for Paragraph<'_> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let text_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if text_area.height < 1 {
            return;
        }

        let style = self.style;
        let mut styled = self.text.lines.iter().flat_map(|spans| {
            spans
                .0
                .iter()
                .flat_map(|span| span.styled_graphemes(style))
                // Required given the way composers work but might be refactored out if we change
                // composers to operate on lines instead of a stream of graphemes.
                .chain(iter::once(StyledGrapheme {
                    symbol: "\n",
                    style: self.style,
                }))
        });

        let mut line_composer: Box<dyn LineComposer> = if let Some(Wrap { trim }) = self.wrap {
            Box::new(WordWrapper::new(&mut styled, text_area.width, trim))
        } else {
            let mut line_composer = Box::new(LineTruncator::new(&mut styled, text_area.width));
            if self.alignment == Alignment::Left {
                line_composer.set_horizontal_offset(self.scroll.1);
            }
            line_composer
        };
        let mut y = 0;
        while let Some((current_line, current_line_width)) = line_composer.next_line() {
            if y >= self.scroll.0 {
                let mut x = get_line_offset(current_line_width, text_area.width, self.alignment);
                for StyledGrapheme { symbol, style } in current_line {
                    buf[(text_area.left() + x, text_area.top() + y - self.scroll.0)]
                        .set_symbol(if symbol.is_empty() {
                            // If the symbol is empty, the last char which rendered last time will
                            // leave on the line. It's a quick fix.
                            " "
                        } else {
                            symbol
                        })
                        .set_style(*style);
                    x += symbol.width() as u16;
                }
            }
            y += 1;
            if y >= text_area.height + self.scroll.0 {
                break;
            }
        }
    }
}
