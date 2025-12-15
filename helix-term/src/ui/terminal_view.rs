//! Terminal UI component.
//!
//! This module provides the UI component for rendering an integrated terminal
//! within the Helix editor.
//!
//! Author: Huzeyfe Coşkun <huzeyfecoskun@hotmail.com>
//! GitHub: https://github.com/huzeyfecoskun

use crate::{
    compositor::{Component, Context, Event, EventResult},
    terminal::{Terminal, TerminalId},
};

use helix_core::Position;
use helix_view::{
    graphics::{CursorKind, Modifier, Rect, Style},
    input::{KeyEvent, MouseEvent, MouseEventKind},
    keyboard::{KeyCode, KeyModifiers},
    Editor,
};

use tui::buffer::Buffer as Surface;

/// Terminal view component for rendering a single terminal instance
pub struct TerminalView {
    /// The terminal instance
    terminal: Terminal,
    /// Whether this terminal view is focused
    focused: bool,
}

impl TerminalView {
    /// Create a new terminal view
    pub fn new(terminal: Terminal) -> Self {
        Self {
            terminal,
            focused: true,
        }
    }

    /// Get the terminal ID
    pub fn id(&self) -> TerminalId {
        self.terminal.id
    }

    /// Get the terminal title
    pub fn title(&self) -> &str {
        &self.terminal.title
    }

    /// Check if terminal has exited
    pub fn has_exited(&self) -> bool {
        self.terminal.exited
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Process any pending PTY output
    /// Returns true if any output was processed (for triggering redraw)
    pub fn process_pty_events(&mut self) -> bool {
        use crate::terminal::PtyEvent;

        // Process ALL pending events for instant response - no limit
        let mut had_output = false;

        while let Some(event) = self.terminal.pty.try_recv() {
            match event {
                PtyEvent::Output(data) => {
                    self.terminal.process_output(&data);
                    had_output = true;
                }
                PtyEvent::Exit(_code) => {
                    self.terminal.mark_exited();
                    had_output = true;
                    break;
                }
            }
        }

        had_output
    }

    /// Handle a key event, returning the bytes to write to PTY
    fn key_to_bytes(&self, key: &KeyEvent) -> Option<Vec<u8>> {
        let mut buf = Vec::new();

        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+letter produces control characters
                    if c.is_ascii_lowercase() {
                        buf.push((c as u8) - b'a' + 1);
                    } else if c.is_ascii_uppercase() {
                        buf.push((c as u8) - b'A' + 1);
                    } else {
                        match c {
                            '[' | '3' => buf.push(0x1b),       // Escape
                            '\\' | '4' => buf.push(0x1c),      // File separator
                            ']' | '5' => buf.push(0x1d),       // Group separator
                            '^' | '6' => buf.push(0x1e),       // Record separator
                            '_' | '7' | '/' => buf.push(0x1f), // Unit separator
                            '@' | '2' | ' ' => buf.push(0x00), // Null
                            _ => {
                                let mut s = String::new();
                                s.push(c);
                                buf.extend(s.as_bytes());
                            }
                        }
                    }
                } else if key.modifiers.contains(KeyModifiers::ALT) {
                    buf.push(0x1b); // ESC prefix for Alt
                    let mut s = String::new();
                    s.push(c);
                    buf.extend(s.as_bytes());
                } else {
                    let mut s = String::new();
                    s.push(c);
                    buf.extend(s.as_bytes());
                }
            }
            KeyCode::Enter => buf.push(b'\r'),
            KeyCode::Backspace => buf.push(0x7f),
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    buf.extend(b"\x1b[Z"); // Shift-Tab
                } else {
                    buf.push(b'\t');
                }
            }
            KeyCode::Esc => buf.push(0x1b),
            KeyCode::Up => buf.extend(b"\x1b[A"),
            KeyCode::Down => buf.extend(b"\x1b[B"),
            KeyCode::Right => buf.extend(b"\x1b[C"),
            KeyCode::Left => buf.extend(b"\x1b[D"),
            KeyCode::Home => buf.extend(b"\x1b[H"),
            KeyCode::End => buf.extend(b"\x1b[F"),
            KeyCode::PageUp => buf.extend(b"\x1b[5~"),
            KeyCode::PageDown => buf.extend(b"\x1b[6~"),
            KeyCode::Insert => buf.extend(b"\x1b[2~"),
            KeyCode::Delete => buf.extend(b"\x1b[3~"),
            KeyCode::F(n) => {
                let seq = match n {
                    1 => b"\x1bOP".as_slice(),
                    2 => b"\x1bOQ",
                    3 => b"\x1bOR",
                    4 => b"\x1bOS",
                    5 => b"\x1b[15~",
                    6 => b"\x1b[17~",
                    7 => b"\x1b[18~",
                    8 => b"\x1b[19~",
                    9 => b"\x1b[20~",
                    10 => b"\x1b[21~",
                    11 => b"\x1b[23~",
                    12 => b"\x1b[24~",
                    _ => return None,
                };
                buf.extend(seq);
            }
            _ => return None,
        }

        Some(buf)
    }

    /// Render the terminal content
    fn render_content(&self, area: Rect, surface: &mut Surface, theme_bg: Style, _theme_fg: Style) {
        let emulator = &self.terminal.emulator;
        let (rows, cols) = emulator.size();

        // Clear the area first
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = surface.get_mut(x, y) {
                    cell.reset();
                    cell.set_style(theme_bg);
                }
            }
        }

        // Render each cell from the terminal
        for row in 0..area.height.min(rows) {
            for col in 0..area.width.min(cols) {
                if let Some(term_cell) = emulator.cell(row, col) {
                    let x = area.x + col;
                    let y = area.y + row;

                    if let Some(surface_cell) = surface.get_mut(x, y) {
                        let mut style = Style::default();

                        // Apply foreground color
                        if let Some(fg) = term_cell.fg.to_helix_color(true) {
                            style = style.fg(fg);
                        }

                        // Apply background color
                        if let Some(bg) = term_cell.bg.to_helix_color(false) {
                            style = style.bg(bg);
                        }

                        // Apply modifiers
                        let mut modifiers = Modifier::empty();
                        if term_cell.bold {
                            modifiers |= Modifier::BOLD;
                        }
                        if term_cell.italic {
                            modifiers |= Modifier::ITALIC;
                        }
                        // Note: underline is handled via UnderlineStyle in helix,
                        // simplified here for now
                        if term_cell.inverse {
                            modifiers |= Modifier::REVERSED;
                        }
                        style = style.add_modifier(modifiers);

                        // Set the cell content
                        let content = if term_cell.contents.is_empty() {
                            " "
                        } else {
                            &term_cell.contents
                        };

                        surface_cell.set_symbol(content);
                        surface_cell.set_style(style);
                    }
                }
            }
        }
    }
}

impl Component for TerminalView {
    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> EventResult {
        // Don't process PTY events here - let idle/render handle it for better typing performance

        match event {
            Event::Key(key) => {
                if !self.focused {
                    return EventResult::Ignored(None);
                }

                // Check for special keybindings to exit terminal mode
                // Ctrl-\ or Ctrl-Shift-6 returns focus to editor
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('\\') | KeyCode::Char('6') => {
                            return EventResult::Ignored(None);
                        }
                        _ => {}
                    }
                }

                // Send key to PTY
                if let Some(bytes) = self.key_to_bytes(key) {
                    if let Err(e) = self.terminal.write(&bytes) {
                        log::error!("Failed to write to terminal: {}", e);
                    }
                    // Request redraw to show echo quickly
                    helix_event::request_redraw();
                }

                EventResult::Consumed(None)
            }
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(_),
                ..
            }) => {
                self.focused = true;
                EventResult::Consumed(None)
            }
            Event::Resize(_width, _height) => {
                // Terminal will be resized by the parent panel
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, ctx: &mut Context) {
        // Process pending events before rendering
        self.process_pty_events();

        // Resize terminal if needed
        if area.width > 0 && area.height > 0 {
            let (current_rows, current_cols) = self.terminal.emulator.size();
            if current_rows != area.height || current_cols != area.width {
                self.terminal.resize(area.height, area.width);
            }
        }

        let theme = &ctx.editor.theme;
        let bg_style = theme.get("ui.background");
        let fg_style = theme.get("ui.text");

        self.render_content(area, surface, bg_style, fg_style);
    }

    fn cursor(&self, area: Rect, _ctx: &Editor) -> (Option<Position>, CursorKind) {
        if !self.focused || !self.terminal.emulator.cursor_visible() {
            return (None, CursorKind::Hidden);
        }

        let (row, col) = self.terminal.emulator.cursor_position();
        let pos = Position::new(
            area.y as usize + row as usize,
            area.x as usize + col as usize,
        );

        (Some(pos), CursorKind::Block)
    }
}
