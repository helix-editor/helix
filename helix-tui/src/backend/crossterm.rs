use crate::{backend::Backend, buffer::Cell};
use crossterm::{
    cursor::{CursorShape, Hide, MoveTo, SetCursorShape, Show},
    execute, queue,
    style::{
        Attribute as CAttribute, Color as CColor, Print, SetAttribute, SetBackgroundColor,
        SetForegroundColor, SetUnderlineColor,
    },
    terminal::{self, Clear, ClearType},
};
use helix_view::graphics::{Color, CursorKind, Modifier, Rect, UnderlineStyle};
use std::io::{self, Write};

fn vte_version() -> Option<usize> {
    std::env::var("VTE_VERSION").ok()?.parse().ok()
}

/// Describes terminal capabilities like extended underline, truecolor, etc.
#[derive(Copy, Clone, Debug, Default)]
struct Capabilities {
    /// Support for undercurled, underdashed, etc.
    has_extended_underlines: bool,
}

impl Capabilities {
    /// Detect capabilities from the terminfo database located based
    /// on the $TERM environment variable. If detection fails, returns
    /// a default value where no capability is supported.
    pub fn from_env_or_default() -> Self {
        match cxterminfo::terminfo::TermInfo::from_env() {
            Err(_) => Capabilities::default(),
            Ok(t) => Capabilities {
                // Smulx, VTE: https://unix.stackexchange.com/a/696253/246284
                // Su (used by kitty): https://sw.kovidgoyal.net/kitty/underlines
                has_extended_underlines: t.get_ext_string("Smulx").is_some()
                    || *t.get_ext_bool("Su").unwrap_or(&false)
                    || vte_version() >= Some(5102),
            },
        }
    }
}

pub struct CrosstermBackend<W: Write> {
    buffer: W,
    capabilities: Capabilities,
}

impl<W> CrosstermBackend<W>
where
    W: Write,
{
    pub fn new(buffer: W) -> CrosstermBackend<W> {
        CrosstermBackend {
            buffer,
            capabilities: Capabilities::from_env_or_default(),
        }
    }
}

impl<W> Write for CrosstermBackend<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush()
    }
}

impl<W> Backend for CrosstermBackend<W>
where
    W: Write,
{
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let mut fg = Color::Reset;
        let mut bg = Color::Reset;
        let mut underline_color = Color::Reset;
        let mut underline_style = UnderlineStyle::Reset;
        let mut modifier = Modifier::empty();
        let mut last_pos: Option<(u16, u16)> = None;
        for (x, y, cell) in content {
            // Move the cursor if the previous location was not (x - 1, y)
            if !matches!(last_pos, Some(p) if x == p.0 + 1 && y == p.1) {
                map_error(queue!(self.buffer, MoveTo(x, y)))?;
            }
            last_pos = Some((x, y));
            if cell.modifier != modifier {
                let diff = ModifierDiff {
                    from: modifier,
                    to: cell.modifier,
                };
                diff.queue(&mut self.buffer)?;
                modifier = cell.modifier;
            }
            if cell.fg != fg {
                let color = CColor::from(cell.fg);
                map_error(queue!(self.buffer, SetForegroundColor(color)))?;
                fg = cell.fg;
            }
            if cell.bg != bg {
                let color = CColor::from(cell.bg);
                map_error(queue!(self.buffer, SetBackgroundColor(color)))?;
                bg = cell.bg;
            }
            if cell.underline_color != underline_color {
                let color = CColor::from(cell.underline_color);
                map_error(queue!(self.buffer, SetUnderlineColor(color)))?;
                underline_color = cell.underline_color;
            }

            let mut new_underline_style = cell.underline_style;
            if !self.capabilities.has_extended_underlines {
                match new_underline_style {
                    UnderlineStyle::Reset => (),
                    _ => new_underline_style = UnderlineStyle::Line,
                }
            }

            if new_underline_style != underline_style {
                let attr = CAttribute::from(cell.underline_style);
                map_error(queue!(self.buffer, SetAttribute(attr)))?;
                underline_style = new_underline_style;
            }

            map_error(queue!(self.buffer, Print(&cell.symbol)))?;
        }

        map_error(queue!(
            self.buffer,
            SetUnderlineColor(CColor::Reset),
            SetForegroundColor(CColor::Reset),
            SetBackgroundColor(CColor::Reset),
            SetAttribute(CAttribute::Reset)
        ))
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        map_error(execute!(self.buffer, Hide))
    }

    fn show_cursor(&mut self, kind: CursorKind) -> io::Result<()> {
        let shape = match kind {
            CursorKind::Block => CursorShape::Block,
            CursorKind::Bar => CursorShape::Line,
            CursorKind::Underline => CursorShape::UnderScore,
            CursorKind::Hidden => unreachable!(),
        };
        map_error(execute!(self.buffer, Show, SetCursorShape(shape)))
    }

    fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        crossterm::cursor::position()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        map_error(execute!(self.buffer, MoveTo(x, y)))
    }

    fn clear(&mut self) -> io::Result<()> {
        map_error(execute!(self.buffer, Clear(ClearType::All)))
    }

    fn size(&self) -> io::Result<Rect> {
        let (width, height) =
            terminal::size().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        Ok(Rect::new(0, 0, width, height))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush()
    }
}

fn map_error(error: crossterm::Result<()>) -> io::Result<()> {
    error.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

#[derive(Debug)]
struct ModifierDiff {
    pub from: Modifier,
    pub to: Modifier,
}

impl ModifierDiff {
    fn queue<W>(&self, mut w: W) -> io::Result<()>
    where
        W: io::Write,
    {
        //use crossterm::Attribute;
        let removed = self.from - self.to;
        if removed.contains(Modifier::REVERSED) {
            map_error(queue!(w, SetAttribute(CAttribute::NoReverse)))?;
        }
        if removed.contains(Modifier::BOLD) {
            map_error(queue!(w, SetAttribute(CAttribute::NormalIntensity)))?;
            if self.to.contains(Modifier::DIM) {
                map_error(queue!(w, SetAttribute(CAttribute::Dim)))?;
            }
        }
        if removed.contains(Modifier::ITALIC) {
            map_error(queue!(w, SetAttribute(CAttribute::NoItalic)))?;
        }
        if removed.contains(Modifier::DIM) {
            map_error(queue!(w, SetAttribute(CAttribute::NormalIntensity)))?;
        }
        if removed.contains(Modifier::CROSSED_OUT) {
            map_error(queue!(w, SetAttribute(CAttribute::NotCrossedOut)))?;
        }
        if removed.contains(Modifier::SLOW_BLINK) || removed.contains(Modifier::RAPID_BLINK) {
            map_error(queue!(w, SetAttribute(CAttribute::NoBlink)))?;
        }

        let added = self.to - self.from;
        if added.contains(Modifier::REVERSED) {
            map_error(queue!(w, SetAttribute(CAttribute::Reverse)))?;
        }
        if added.contains(Modifier::BOLD) {
            map_error(queue!(w, SetAttribute(CAttribute::Bold)))?;
        }
        if added.contains(Modifier::ITALIC) {
            map_error(queue!(w, SetAttribute(CAttribute::Italic)))?;
        }
        if added.contains(Modifier::DIM) {
            map_error(queue!(w, SetAttribute(CAttribute::Dim)))?;
        }
        if added.contains(Modifier::CROSSED_OUT) {
            map_error(queue!(w, SetAttribute(CAttribute::CrossedOut)))?;
        }
        if added.contains(Modifier::SLOW_BLINK) {
            map_error(queue!(w, SetAttribute(CAttribute::SlowBlink)))?;
        }
        if added.contains(Modifier::RAPID_BLINK) {
            map_error(queue!(w, SetAttribute(CAttribute::RapidBlink)))?;
        }

        Ok(())
    }
}
