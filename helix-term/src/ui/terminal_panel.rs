//! Terminal panel with tab support.
//!
//! This module provides a panel component that can hold multiple terminal tabs
//! and be toggled visible/hidden.
//!
//! Author: Huzeyfe Coşkun <huzeyfecoskun@hotmail.com>
//! GitHub: https://github.com/huzeyfecoskun

use crate::{
    compositor::{Component, Context, Event, EventResult},
    terminal::{Terminal, TerminalId},
    ui::terminal_view::TerminalView,
};

use helix_core::Position;
use helix_view::{
    graphics::{CursorKind, Rect},
    input::MouseEventKind,
    keyboard::{KeyCode, KeyModifiers},
    Editor,
};

use tui::buffer::Buffer as Surface;

use std::path::PathBuf;

/// Panel containing multiple terminal tabs
pub struct TerminalPanel {
    /// Terminal instances (tabs)
    terminals: Vec<TerminalView>,
    /// Currently active terminal index
    active_index: usize,
    /// Whether the panel is visible
    visible: bool,
    /// Panel height as percentage of screen (0-100)
    height_percent: u16,
    /// Whether the panel has focus
    focused: bool,
}

pub const TERMINAL_PANEL_ID: &str = "terminal-panel";
const DEFAULT_HEIGHT_PERCENT: u16 = 30;
const TAB_BAR_HEIGHT: u16 = 1;

impl Default for TerminalPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalPanel {
    /// Create a new empty terminal panel
    pub fn new() -> Self {
        Self {
            terminals: Vec::new(),
            active_index: 0,
            visible: false,
            height_percent: DEFAULT_HEIGHT_PERCENT,
            focused: false,
        }
    }

    /// Check if panel is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get count of terminal tabs
    pub fn terminals_count(&self) -> usize {
        self.terminals.len()
    }

    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible && self.terminals.is_empty() {
            // Auto-create a terminal when showing empty panel
            if let Err(e) = self.new_terminal(None, None) {
                log::error!("Failed to create terminal: {}", e);
                self.visible = false;
            }
        }
        if self.visible {
            self.focused = true;
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
        if self.terminals.is_empty() {
            if let Err(e) = self.new_terminal(None, None) {
                log::error!("Failed to create terminal: {}", e);
                self.visible = false;
            }
        }
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.set_focused(focused);
        }
    }

    /// Check if panel is focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Create a new terminal tab
    pub fn new_terminal(
        &mut self,
        cwd: Option<PathBuf>,
        shell: Option<&[String]>,
    ) -> anyhow::Result<TerminalId> {
        let terminal = Terminal::new(cwd, shell)?;
        let id = terminal.id;
        let view = TerminalView::new(terminal);

        self.terminals.push(view);
        self.active_index = self.terminals.len() - 1;

        // Update focus state for all terminals
        for (i, term) in self.terminals.iter_mut().enumerate() {
            term.set_focused(i == self.active_index && self.focused);
        }

        self.visible = true;
        self.focused = true;

        Ok(id)
    }

    /// Close the current terminal tab
    pub fn close_current(&mut self) {
        if self.terminals.is_empty() {
            return;
        }

        self.terminals.remove(self.active_index);

        if self.terminals.is_empty() {
            self.visible = false;
            self.focused = false;
        } else {
            self.active_index = self
                .active_index
                .saturating_sub(1)
                .min(self.terminals.len() - 1);
            if let Some(terminal) = self.terminals.get_mut(self.active_index) {
                terminal.set_focused(self.focused);
            }
        }
    }

    /// Close terminal by ID
    pub fn close(&mut self, id: TerminalId) {
        if let Some(idx) = self.terminals.iter().position(|t| t.id() == id) {
            self.terminals.remove(idx);

            if self.terminals.is_empty() {
                self.visible = false;
                self.focused = false;
            } else {
                self.active_index = self
                    .active_index
                    .saturating_sub(1)
                    .min(self.terminals.len() - 1);
            }
        }
    }

    /// Switch to next terminal tab
    pub fn next_tab(&mut self) {
        if self.terminals.len() <= 1 {
            return;
        }

        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.set_focused(false);
        }

        self.active_index = (self.active_index + 1) % self.terminals.len();

        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.set_focused(self.focused);
        }
    }

    /// Switch to previous terminal tab
    pub fn prev_tab(&mut self) {
        if self.terminals.len() <= 1 {
            return;
        }

        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.set_focused(false);
        }

        self.active_index = if self.active_index == 0 {
            self.terminals.len() - 1
        } else {
            self.active_index - 1
        };

        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.set_focused(self.focused);
        }
    }

    /// Switch to a specific tab by index
    pub fn goto_tab(&mut self, index: usize) {
        if index >= self.terminals.len() {
            return;
        }

        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.set_focused(false);
        }

        self.active_index = index;

        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.set_focused(self.focused);
        }
    }

    /// Get the number of terminals
    pub fn terminal_count(&self) -> usize {
        self.terminals.len()
    }

    /// Get height based on viewport
    pub fn calculate_height(&self, viewport_height: u16) -> u16 {
        if !self.visible {
            return 0;
        }

        let height = (viewport_height as u32 * self.height_percent as u32 / 100) as u16;
        height.max(5) // Minimum height
    }

    /// Set panel height percentage
    pub fn set_height_percent(&mut self, percent: u16) {
        self.height_percent = percent.clamp(10, 90);
    }

    /// Increase panel height
    pub fn increase_height(&mut self) {
        self.height_percent = (self.height_percent + 5).min(90);
    }

    /// Decrease panel height  
    pub fn decrease_height(&mut self) {
        self.height_percent = (self.height_percent.saturating_sub(5)).max(10);
    }

    /// Render the tab bar
    fn render_tab_bar(&self, area: Rect, surface: &mut Surface, theme: &helix_view::Theme) {
        let tab_style = theme.get("ui.statusline");
        let active_style = theme.get("ui.statusline.select");

        // Clear the tab bar area
        for x in area.x..area.x + area.width {
            if let Some(cell) = surface.get_mut(x, area.y) {
                cell.reset();
                cell.set_style(tab_style);
            }
        }

        if self.terminals.is_empty() {
            return;
        }

        // Render tabs manually
        let mut x_offset = area.x;
        for (i, term) in self.terminals.iter().enumerate() {
            let title = format!(" {} ", term.title());
            let style = if i == self.active_index {
                active_style
            } else {
                tab_style
            };

            // Draw tab content
            for c in title.chars() {
                if x_offset >= area.x + area.width {
                    break;
                }
                if let Some(cell) = surface.get_mut(x_offset, area.y) {
                    cell.set_char(c);
                    cell.set_style(style);
                }
                x_offset += 1;
            }

            // Draw separator
            if i < self.terminals.len() - 1 && x_offset < area.x + area.width {
                if let Some(cell) = surface.get_mut(x_offset, area.y) {
                    cell.set_char('│');
                    cell.set_style(tab_style);
                }
                x_offset += 1;
            }
        }
    }

    /// Process PTY events for all terminals
    /// Returns true if any output was processed
    pub fn process_pty_events(&mut self) -> bool {
        let mut had_output = false;
        for terminal in &mut self.terminals {
            if terminal.process_pty_events() {
                had_output = true;
            }
        }
        // Request redraw if we had output for faster updates
        if had_output {
            helix_event::request_redraw();
        }
        had_output
    }
}

impl Component for TerminalPanel {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> EventResult {
        if !self.visible {
            return EventResult::Ignored(None);
        }

        // Don't process PTY events on key events - let idle/render handle it
        // This improves typing responsiveness

        match event {
            Event::Key(key) => {
                if !self.focused {
                    return EventResult::Ignored(None);
                }

                // Handle panel-level keybindings
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        // Ctrl-\ to return focus to editor
                        KeyCode::Char('\\') => {
                            self.focused = false;
                            if let Some(terminal) = self.terminals.get_mut(self.active_index) {
                                terminal.set_focused(false);
                            }
                            return EventResult::Consumed(None);
                        }
                        // Ctrl-PageDown for next tab
                        KeyCode::PageDown => {
                            self.next_tab();
                            return EventResult::Consumed(None);
                        }
                        // Ctrl-PageUp for previous tab
                        KeyCode::PageUp => {
                            self.prev_tab();
                            return EventResult::Consumed(None);
                        }
                        // Ctrl-Shift-T for new terminal (when Shift is also pressed)
                        KeyCode::Char('t') | KeyCode::Char('T')
                            if key.modifiers.contains(KeyModifiers::SHIFT) =>
                        {
                            if let Err(e) = self.new_terminal(None, None) {
                                log::error!("Failed to create terminal: {}", e);
                            }
                            return EventResult::Consumed(None);
                        }
                        // Ctrl-W to close current terminal
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            self.close_current();
                            return EventResult::Consumed(None);
                        }
                        _ => {}
                    }
                }

                // Pass to active terminal
                if let Some(terminal) = self.terminals.get_mut(self.active_index) {
                    return terminal.handle_event(event, ctx);
                }

                EventResult::Consumed(None)
            }
            Event::Mouse(mouse) => {
                // Handle mouse clicks on tabs
                if mouse.row == 0 && matches!(mouse.kind, MouseEventKind::Down(_)) {
                    // Calculate which tab was clicked
                    // This is a simplified version - would need proper tab width calculation
                    self.focused = true;
                    return EventResult::Consumed(None);
                }

                if let Some(terminal) = self.terminals.get_mut(self.active_index) {
                    return terminal.handle_event(event, ctx);
                }

                EventResult::Consumed(None)
            }
            Event::Resize(_, _) => {
                // Handled by render
                EventResult::Consumed(None)
            }
            Event::IdleTimeout => {
                // Process PTY events on idle to get real-time output
                self.process_pty_events();
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, ctx: &mut Context) {
        if !self.visible || area.height < 2 {
            return;
        }

        // Process PTY events before rendering
        self.process_pty_events();

        let theme = &ctx.editor.theme;
        let border_style = theme.get("ui.window");

        // Draw top border/separator
        let separator_y = area.y;
        for x in area.x..area.x + area.width {
            if let Some(cell) = surface.get_mut(x, separator_y) {
                cell.set_symbol("─");
                cell.set_style(border_style);
            }
        }

        // Tab bar area (after separator)
        let tab_bar_area = Rect::new(area.x, area.y + 1, area.width, TAB_BAR_HEIGHT);
        self.render_tab_bar(tab_bar_area, surface, theme);

        // Terminal content area
        let content_area = Rect::new(
            area.x,
            area.y + 1 + TAB_BAR_HEIGHT,
            area.width,
            area.height.saturating_sub(1 + TAB_BAR_HEIGHT),
        );

        // Render active terminal
        if let Some(terminal) = self.terminals.get_mut(self.active_index) {
            terminal.render(content_area, surface, ctx);
        } else {
            // Empty state
            let empty_style = theme.get("ui.text.inactive");
            let msg = "No terminal. Press Ctrl-` to create one.";
            let x = area.x + (area.width.saturating_sub(msg.len() as u16)) / 2;
            let y = content_area.y + content_area.height / 2;

            for (i, c) in msg.chars().enumerate() {
                if let Some(cell) = surface.get_mut(x + i as u16, y) {
                    cell.set_char(c);
                    cell.set_style(empty_style);
                }
            }
        }
    }

    fn cursor(&self, area: Rect, ctx: &Editor) -> (Option<Position>, CursorKind) {
        if !self.visible || !self.focused {
            return (None, CursorKind::Hidden);
        }

        // Calculate content area offset
        let content_area = Rect::new(
            area.x,
            area.y + 1 + TAB_BAR_HEIGHT,
            area.width,
            area.height.saturating_sub(1 + TAB_BAR_HEIGHT),
        );

        if let Some(terminal) = self.terminals.get(self.active_index) {
            return terminal.cursor(content_area, ctx);
        }

        (None, CursorKind::Hidden)
    }

    fn id(&self) -> Option<&'static str> {
        Some(TERMINAL_PANEL_ID)
    }
}
