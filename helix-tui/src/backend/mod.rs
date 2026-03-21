//! Provides interface for controlling the terminal

use std::io;
use std::io::Write;

use crate::{buffer::Cell, terminal::Config};

use helix_view::{
    graphics::{CursorKind, Rect},
    theme::Color,
};

#[cfg(all(feature = "termina", not(windows)))]
mod termina;
#[cfg(all(feature = "termina", not(windows)))]
pub use self::termina::TerminaBackend;

#[cfg(all(feature = "termina", windows))]
mod crossterm;
#[cfg(all(feature = "termina", windows))]
pub use self::crossterm::CrosstermBackend;

mod test;
pub use self::test::TestBackend;

/// Representation of a terminal backend.
pub trait Backend {
    /// Claims the terminal for TUI use.
    fn claim(&mut self) -> Result<(), io::Error>;
    /// Update terminal configuration.
    fn reconfigure(&mut self, config: Config) -> Result<(), io::Error>;
    /// Restores the terminal to a normal state, undoes `claim`
    fn restore(&mut self) -> Result<(), io::Error>;
    /// Draws styled text to the terminal
    fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>;
    /// Hides the cursor
    fn hide_cursor(&mut self) -> Result<(), io::Error>;
    /// Sets the cursor to the given shape
    fn show_cursor(&mut self, kind: CursorKind) -> Result<(), io::Error>;
    /// Sets the cursor to the given position
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), io::Error>;
    /// Clears the terminal
    fn clear(&mut self) -> Result<(), io::Error>;
    /// Gets the size of the terminal in cells
    fn size(&self) -> Result<Rect, io::Error>;
    /// Flushes the terminal buffer
    fn flush(&mut self) -> Result<(), io::Error>;
    fn supports_true_color(&self) -> bool;
    fn get_theme_mode(&self) -> Option<helix_view::theme::Mode>;
    fn set_background_color(&mut self, color: Option<Color>) -> io::Result<()>;
    /// Write raw bytes to the terminal output.
    fn write_raw(&mut self, bytes: &[u8]) -> io::Result<()>;
    /// Emit kitty multi-cursor protocol sequence (CSI > Ps ; ... SP q).
    /// Backends should gate this on their capability check, then delegate here.
    fn emit_extra_cursors(&mut self, cursors: &[(u16, u16)]) -> io::Result<()> {
        self.write_raw(&build_multi_cursor_sequence(cursors))
    }
}

/// Build the wire-format kitty multi-cursor protocol sequence.
/// Empty cursors produces a clear sequence; non-empty produces shape 29
/// with 1-indexed point coordinates.
fn build_multi_cursor_sequence(cursors: &[(u16, u16)]) -> Vec<u8> {
    if cursors.is_empty() {
        return b"\x1b[>0;4 q".to_vec();
    }
    let mut buf = Vec::with_capacity(8 + cursors.len() * 12);
    buf.extend_from_slice(b"\x1b[>29");
    for &(row, col) in cursors {
        write!(buf, ";2:{}:{}", row + 1, col + 1).unwrap();
    }
    buf.extend_from_slice(b" q");
    buf
}
