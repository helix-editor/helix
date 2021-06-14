use crate::text::{Span, Spans};

use helix_view::graphics::{Color, Modifier, Rect, Style};

pub use termwiz::surface::Surface as Buffer;
use termwiz::{cell::*, surface::*};

pub struct Cell<'a> {
    surface: &'a mut Surface,
}

impl<'a> Cell<'a> {
    pub fn set_symbol<'b>(self, symbol: &'b str) -> Cell<'a> {
        self.surface.add_change(Change::Text(symbol.into()));
        self
    }
    pub fn set_style(self, style: Style) -> Cell<'a> {
        if let Some(fg) = style.fg {
            self.surface
                .add_change(Change::Attribute(AttributeChange::Foreground(fg.into())));
        }
        if let Some(bg) = style.bg {
            self.surface
                .add_change(Change::Attribute(AttributeChange::Background(bg.into())));
        }

        self.surface
            .add_change(Change::Attribute(AttributeChange::Intensity(
                if style.add_modifier.contains(Modifier::BOLD) {
                    Intensity::Bold
                } else if style.add_modifier.contains(Modifier::DIM) {
                    Intensity::Half
                } else {
                    Intensity::Normal
                },
            )));

        self.surface
            .add_change(Change::Attribute(AttributeChange::Italic(
                style.add_modifier.contains(Modifier::ITALIC),
            )));

        self.surface
            .add_change(Change::Attribute(AttributeChange::Underline(
                if style.add_modifier.contains(Modifier::UNDERLINED) {
                    Underline::Single
                } else {
                    Underline::None
                },
            )));

        self.surface
            .add_change(Change::Attribute(AttributeChange::Reverse(
                style.add_modifier.contains(Modifier::REVERSED),
            )));

        self.surface
            .add_change(Change::Attribute(AttributeChange::Invisible(
                style.add_modifier.contains(Modifier::HIDDEN),
            )));

        self.surface
            .add_change(Change::Attribute(AttributeChange::StrikeThrough(
                style.add_modifier.contains(Modifier::CROSSED_OUT),
            )));

        self.surface
            .add_change(Change::Attribute(AttributeChange::Blink(
                if style.add_modifier.contains(Modifier::SLOW_BLINK) {
                    Blink::Slow
                } else if style.add_modifier.contains(Modifier::RAPID_BLINK) {
                    Blink::Rapid
                } else {
                    Blink::None
                },
            )));
        self
    }
}

pub trait SurfaceExt {
    //
    fn set_style(&mut self, area: Rect, style: Style) {}

    fn clear_with(&mut self, area: Rect, style: Style) {}

    fn set_string<S>(&mut self, x: u16, y: u16, string: S, style: Style)
    where
        S: AsRef<str>,
    {
        self.set_stringn(x, y, string, usize::MAX, style);
    }

    fn set_stringn<S>(
        &mut self,
        x: u16,
        y: u16,
        string: S,
        width: usize,
        style: Style,
    ) -> (u16, u16)
    where
        S: AsRef<str>;

    fn set_spans<'a>(&mut self, x: u16, y: u16, spans: &Spans<'a>, width: u16) -> (u16, u16) {
        let mut remaining_width = width;
        let mut x = x;
        for span in &spans.0 {
            if remaining_width == 0 {
                break;
            }
            let pos = self.set_stringn(
                x,
                y,
                span.content.as_ref(),
                remaining_width as usize,
                span.style,
            );
            let w = pos.0.saturating_sub(x);
            x = pos.0;
            remaining_width = remaining_width.saturating_sub(w);
        }
        (x, y)
    }

    fn set_span<'a>(&mut self, x: u16, y: u16, span: &Span<'a>, width: u16) -> (u16, u16) {
        self.set_stringn(x, y, span.content.as_ref(), width as usize, span.style)
    }

    fn set_string_truncated(
        &mut self,
        x: u16,
        y: u16,
        string: &str,
        width: usize,
        style: impl Fn(usize) -> Style, // Map a grapheme's string offset to a style
        ellipsis: bool,
        truncate_start: bool,
    ) {
        // TODO
    }

    fn get_mut(&mut self, x: u16, y: u16) -> Cell;
}

impl SurfaceExt for termwiz::surface::Surface {
    //
    //fn set_style(&mut self, area: Rect, style: Style) {
    //    //
    //}

    fn set_stringn<S>(
        &mut self,
        x: u16,
        y: u16,
        string: S,
        width: usize,
        style: Style,
    ) -> (u16, u16)
    where
        S: AsRef<str>,
    {
        // TODO: style and limit to width
        self.add_change(Change::CursorPosition {
            x: Position::Absolute(x as usize),
            y: Position::Absolute(y as usize),
        });
        let fg = style.fg.unwrap_or(Color::Reset);
        self.add_change(Change::Attribute(AttributeChange::Foreground(fg.into())));
        let bg = style.bg.unwrap_or(Color::Reset);
        self.add_change(Change::Attribute(AttributeChange::Background(bg.into())));
        self.add_change(Change::Text(string.as_ref().to_owned()));

        (0, 0)
    }

    fn get_mut(&mut self, x: u16, y: u16) -> Cell {
        self.add_change(Change::CursorPosition {
            x: Position::Absolute(x as usize),
            y: Position::Absolute(y as usize),
        });
        Cell { surface: self }
    }
}
