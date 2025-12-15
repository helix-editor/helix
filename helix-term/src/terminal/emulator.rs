//! Terminal emulator state machine.
//!
//! This module uses vt100 to parse ANSI escape sequences and maintain
//! terminal screen state (grid, cursor, colors, etc.).
//!
//! Author: Huzeyfe Coşkun <huzeyfecoskun@hotmail.com>
//! GitHub: https://github.com/huzeyfecoskun

use helix_view::graphics::Color;

/// Terminal emulator wrapping vt100 parser
pub struct TerminalEmulator {
    parser: vt100::Parser,
    /// Title set by escape sequence (OSC 0 or OSC 2)
    pending_title: Option<String>,
}

impl TerminalEmulator {
    /// Create a new terminal emulator with given dimensions
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, 1000), // 1000 lines scrollback
            pending_title: None,
        }
    }

    /// Process input bytes from PTY
    pub fn process(&mut self, data: &[u8]) {
        self.parser.process(data);

        // Check for title changes
        let screen = self.parser.screen();
        let title = screen.title();
        if !title.is_empty() {
            self.pending_title = Some(title.to_string());
        }
    }

    /// Resize the terminal
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }

    /// Get the screen contents
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Get the cursor position (row, col)
    pub fn cursor_position(&self) -> (u16, u16) {
        let screen = self.parser.screen();
        (screen.cursor_position().0, screen.cursor_position().1)
    }

    /// Check if cursor is visible
    pub fn cursor_visible(&self) -> bool {
        !self.parser.screen().hide_cursor()
    }

    /// Take pending title (if any)
    pub fn take_title(&mut self) -> Option<String> {
        self.pending_title.take()
    }

    /// Get a cell at the given position
    pub fn cell(&self, row: u16, col: u16) -> Option<TerminalCell> {
        let screen = self.parser.screen();
        screen.cell(row, col).map(TerminalCell::from_vt100)
    }

    /// Get terminal dimensions
    pub fn size(&self) -> (u16, u16) {
        let screen = self.parser.screen();
        (screen.size().0, screen.size().1)
    }

    /// Get scrollback position (0 = no scroll, positive = scrolled up)
    pub fn scrollback_position(&self) -> usize {
        0 // TODO: implement scrollback
    }

    /// Get the number of scrollback lines available
    pub fn scrollback_len(&self) -> usize {
        self.parser.screen().scrollback()
    }
}

/// A single cell in the terminal grid
#[derive(Debug, Clone)]
pub struct TerminalCell {
    pub contents: String,
    pub fg: TerminalColor,
    pub bg: TerminalColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

impl TerminalCell {
    fn from_vt100(cell: &vt100::Cell) -> Self {
        Self {
            contents: cell.contents().to_string(),
            fg: TerminalColor::from_vt100(cell.fgcolor()),
            bg: TerminalColor::from_vt100(cell.bgcolor()),
            bold: cell.bold(),
            italic: cell.italic(),
            underline: cell.underline(),
            inverse: cell.inverse(),
        }
    }
}

/// Terminal color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl TerminalColor {
    fn from_vt100(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => TerminalColor::Default,
            vt100::Color::Idx(idx) => TerminalColor::Indexed(idx),
            vt100::Color::Rgb(r, g, b) => TerminalColor::Rgb(r, g, b),
        }
    }

    /// Convert to helix Color
    pub fn to_helix_color(self, _is_fg: bool) -> Option<Color> {
        match self {
            TerminalColor::Default => None,
            TerminalColor::Indexed(idx) => Some(Color::Indexed(idx)),
            TerminalColor::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
        }
    }
}

/// Convert vt100 colors to standard 16 colors (kept for potential future use)
#[allow(dead_code)]
fn indexed_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0, 0, 0),        // Black
        1 => (128, 0, 0),      // Red
        2 => (0, 128, 0),      // Green
        3 => (128, 128, 0),    // Yellow
        4 => (0, 0, 128),      // Blue
        5 => (128, 0, 128),    // Magenta
        6 => (0, 128, 128),    // Cyan
        7 => (192, 192, 192),  // White
        8 => (128, 128, 128),  // Bright Black
        9 => (255, 0, 0),      // Bright Red
        10 => (0, 255, 0),     // Bright Green
        11 => (255, 255, 0),   // Bright Yellow
        12 => (0, 0, 255),     // Bright Blue
        13 => (255, 0, 255),   // Bright Magenta
        14 => (0, 255, 255),   // Bright Cyan
        15 => (255, 255, 255), // Bright White
        // 216 color cube (16-231)
        idx if (16..232).contains(&idx) => {
            let idx = idx - 16;
            let r = (idx / 36) % 6;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            let to_rgb = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (to_rgb(r), to_rgb(g), to_rgb(b))
        }
        // Grayscale (232-255)
        idx => {
            let gray = 8 + (idx - 232) * 10;
            (gray, gray, gray)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_emulator_new() {
        let emulator = TerminalEmulator::new(24, 80);
        assert_eq!(emulator.size(), (24, 80));
        assert!(emulator.cursor_visible());
    }

    #[test]
    fn test_terminal_emulator_resize() {
        let mut emulator = TerminalEmulator::new(24, 80);
        emulator.resize(40, 120);
        assert_eq!(emulator.size(), (40, 120));
    }

    #[test]
    fn test_terminal_emulator_cursor_position() {
        let emulator = TerminalEmulator::new(24, 80);
        let (row, col) = emulator.cursor_position();
        assert_eq!(row, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_terminal_emulator_process_text() {
        let mut emulator = TerminalEmulator::new(24, 80);
        emulator.process(b"Hello, World!");

        // Check first cell has 'H'
        let cell = emulator.cell(0, 0).expect("Cell should exist");
        assert_eq!(cell.contents, "H");
    }

    #[test]
    fn test_terminal_emulator_process_newline() {
        let mut emulator = TerminalEmulator::new(24, 80);
        emulator.process(b"Line1\r\nLine2");

        // Check cursor moved to second line
        let (row, _col) = emulator.cursor_position();
        assert_eq!(row, 1);
    }

    #[test]
    fn test_terminal_emulator_title() {
        let mut emulator = TerminalEmulator::new(24, 80);
        // OSC 0 ; title ST - set window title
        emulator.process(b"\x1b]0;My Title\x07");

        let title = emulator.take_title();
        assert_eq!(title, Some("My Title".to_string()));

        // Title should be taken (None after take)
        assert!(emulator.take_title().is_none());
    }

    #[test]
    fn test_terminal_color_default() {
        let color = TerminalColor::Default;
        assert_eq!(color.to_helix_color(true), None);
    }

    #[test]
    fn test_terminal_color_indexed() {
        let color = TerminalColor::Indexed(1);
        assert_eq!(color.to_helix_color(true), Some(Color::Indexed(1)));
    }

    #[test]
    fn test_terminal_color_rgb() {
        let color = TerminalColor::Rgb(255, 128, 64);
        assert_eq!(color.to_helix_color(true), Some(Color::Rgb(255, 128, 64)));
    }

    #[test]
    fn test_indexed_to_rgb_basic_colors() {
        assert_eq!(indexed_to_rgb(0), (0, 0, 0)); // Black
        assert_eq!(indexed_to_rgb(1), (128, 0, 0)); // Red
        assert_eq!(indexed_to_rgb(7), (192, 192, 192)); // White
        assert_eq!(indexed_to_rgb(15), (255, 255, 255)); // Bright White
    }

    #[test]
    fn test_indexed_to_rgb_color_cube() {
        // Test some color cube values (16-231)
        let (r, g, b) = indexed_to_rgb(16); // First in cube
        assert_eq!((r, g, b), (0, 0, 0));

        let (r, g, b) = indexed_to_rgb(231); // Last in cube
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_indexed_to_rgb_grayscale() {
        // Grayscale (232-255)
        let (r, g, b) = indexed_to_rgb(232);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert_eq!(r, 8);

        let (r, g, b) = indexed_to_rgb(255);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn test_terminal_cell_attributes() {
        let mut emulator = TerminalEmulator::new(24, 80);
        // SGR 1 (bold) + text
        emulator.process(b"\x1b[1mBold\x1b[0m");

        let cell = emulator.cell(0, 0).expect("Cell should exist");
        assert!(cell.bold);
    }

    #[test]
    fn test_terminal_emulator_scrollback() {
        let emulator = TerminalEmulator::new(24, 80);
        // Initially no scrollback
        assert_eq!(emulator.scrollback_position(), 0);
    }
}
