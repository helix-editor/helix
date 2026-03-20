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
        return b"\x1b[>0 q".to_vec();
    }
    let mut buf = Vec::with_capacity(8 + cursors.len() * 12);
    buf.extend_from_slice(b"\x1b[>29");
    for &(row, col) in cursors {
        write!(buf, ";2:{}:{}", row + 1, col + 1).unwrap();
    }
    buf.extend_from_slice(b" q");
    buf
}

/// Probe terminal for kitty multi-cursor protocol support.
///
/// Sends the support detection query (CSI > SP q) and checks for a
/// response within a short timeout. Uses /dev/tty directly to avoid
/// interfering with the backend's event system.
///
/// Must be called while the terminal is in raw mode and before the
/// backend's event reader is active.
#[cfg(unix)]
pub(super) fn probe_multi_cursor_support() -> bool {
    use std::io::{Read, Write};
    use std::os::unix::io::AsRawFd;

    let Ok(mut tty) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    else {
        log::debug!("Failed to open /dev/tty for multi-cursor probe");
        return false;
    };

    let fd = tty.as_raw_fd();

    // Drain any pending input.
    loop {
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut pfd, 1, 0) } <= 0 {
            break;
        }
        let mut drain = [0u8; 256];
        if tty.read(&mut drain).unwrap_or(0) == 0 {
            break;
        }
    }

    // Send support detection query.
    if tty.write_all(b"\x1b[> q").is_err() || tty.flush().is_err() {
        log::debug!("Failed to write multi-cursor probe query");
        return false;
    }

    // Wait for response with 100ms timeout.
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    if unsafe { libc::poll(&mut pfd, 1, 100) } <= 0 {
        log::debug!("No response to multi-cursor probe (timeout)");
        return false;
    }

    // Read response.
    let mut buf = [0u8; 128];
    let n = tty.read(&mut buf).unwrap_or(0);
    if n == 0 {
        return false;
    }
    let response = &buf[..n];

    // Valid response: CSI > N;N;... SP q
    let supported =
        response.starts_with(b"\x1b[>") && response.windows(2).any(|w| w == b" q");

    log::debug!(
        "Kitty multi-cursor protocol {}detected (response: {:?})",
        if supported { "" } else { "not " },
        std::str::from_utf8(response).unwrap_or("<binary>"),
    );

    supported
}

#[cfg(not(unix))]
pub(super) fn probe_multi_cursor_support() -> bool {
    log::debug!("Multi-cursor probe not supported on this platform");
    false
}
