use silicon_core::Position;
use silicon_view::graphics::{Color, CursorKind, Modifier, Rect, Style};
use silicon_view::input::KeyEvent;

use tui::buffer::Buffer as Surface;

use crate::instance::TerminalInstance;

const DEFAULT_HEIGHT_PERCENT: u16 = 30;

/// Multi-tab terminal panel that renders at the bottom of the editor.
pub struct TerminalPanel {
    instances: Vec<TerminalInstance>,
    active_tab: usize,
    pub visible: bool,
    pub height_percent: u16,
    pub focused: bool,
    /// Whether the separator line is highlighted (mouse hover).
    pub separator_highlighted: bool,
    shell: Option<Vec<String>>,
    wakeup_tx: tokio::sync::mpsc::UnboundedSender<()>,
    pub wakeup_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
}

impl Default for TerminalPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalPanel {
    /// Create a new terminal panel. No terminals are spawned until first use.
    pub fn new() -> Self {
        let (wakeup_tx, wakeup_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            instances: Vec::new(),
            active_tab: 0,
            visible: false,
            height_percent: DEFAULT_HEIGHT_PERCENT,
            focused: false,
            separator_highlighted: false,
            shell: None,
            wakeup_tx,
            wakeup_rx,
        }
    }

    /// Toggle the terminal panel visibility and focus.
    ///
    /// Cycles through: hidden → visible+focused → hidden.
    /// If visible but not focused, focuses the panel.
    pub fn toggle(&mut self, area: Rect) {
        if !self.visible {
            // Hidden → show + focus
            self.visible = true;
            self.focused = true;
            if self.instances.is_empty() {
                self.spawn_terminal(area);
            }
        } else if self.focused {
            // Visible + focused → hide
            self.visible = false;
            self.focused = false;
        } else {
            // Visible + not focused → focus
            self.focused = true;
        }
    }

    /// Show the terminal panel and focus it. Unlike `toggle()`, this never hides
    /// the panel — it always ensures it is visible and focused.
    pub fn show(&mut self, area: Rect) {
        self.visible = true;
        self.focused = true;
        if self.instances.is_empty() {
            self.spawn_terminal(area);
        }
    }

    /// Unfocus the terminal panel (keep it visible).
    pub fn unfocus(&mut self) {
        self.focused = false;
    }

    /// Spawn a new terminal tab.
    pub fn spawn_terminal(&mut self, area: Rect) {
        let panel_height = self.panel_height(area);
        let cols = area.width;
        let rows = panel_height.saturating_sub(2); // 2 lines for separator + tab bar

        if cols == 0 || rows == 0 {
            return;
        }

        match TerminalInstance::new(cols, rows, self.wakeup_tx.clone(), self.shell.as_deref()) {
            Ok(instance) => {
                self.instances.push(instance);
                self.active_tab = self.instances.len() - 1;
            }
            Err(err) => {
                log::error!("Failed to spawn terminal: {}", err);
            }
        }
    }

    /// Close the active terminal tab.
    pub fn close_active(&mut self) {
        if self.instances.is_empty() {
            return;
        }
        self.instances.remove(self.active_tab);
        if self.instances.is_empty() {
            self.visible = false;
            self.focused = false;
        } else if self.active_tab >= self.instances.len() {
            self.active_tab = self.instances.len() - 1;
        }
    }

    /// Set the shell to use for new terminal instances.
    pub fn set_shell(&mut self, shell: Vec<String>) {
        self.shell = Some(shell);
    }

    /// Run a command in a new terminal tab.
    ///
    /// `shell` overrides the default shell for this invocation.
    pub fn run_command(&mut self, cmd: &str, shell: &[String], area: Rect) {
        // Use the provided shell for this terminal instance.
        let prev = self.shell.take();
        self.shell = Some(shell.to_vec());

        self.visible = true;
        self.focused = true;
        self.spawn_terminal(area);

        // Restore previous shell setting.
        self.shell = prev;

        if let Some(instance) = self.instances.last() {
            // Write the command followed by Enter so the shell executes it.
            let input = format!("{cmd}\r");
            instance.input(input.as_bytes());
        }
    }

    /// Handle a key event when the panel is focused.
    /// Returns `true` if the event was consumed.
    pub fn handle_key_event(&mut self, key: &KeyEvent) -> bool {
        if let Some(instance) = self.instances.get(self.active_tab) {
            instance.handle_key(key)
        } else {
            false
        }
    }

    /// Handle a mouse scroll event.
    pub fn handle_scroll(&mut self, delta: i32) {
        if let Some(instance) = self.instances.get(self.active_tab) {
            use alacritty_terminal::grid::Scroll;
            instance.scroll(Scroll::Delta(delta));
        }
    }

    /// Poll events from all terminal instances.
    /// Returns `true` if a redraw is needed.
    pub fn poll_events(&mut self) -> bool {
        // Drain all wakeup notifications.
        while self.wakeup_rx.try_recv().is_ok() {}

        let mut needs_redraw = false;
        for instance in &mut self.instances {
            if instance.poll_events() {
                needs_redraw = true;
            }
        }
        needs_redraw
    }

    /// Resize all terminal instances to fit the given area.
    pub fn resize(&mut self, area: Rect) {
        let panel_height = self.panel_height(area);
        let cols = area.width;
        let rows = panel_height.saturating_sub(2); // separator + tab bar

        if cols == 0 || rows == 0 {
            return;
        }

        for instance in &mut self.instances {
            instance.resize(cols, rows);
        }
    }

    /// Grow the terminal panel by 5%, clamped to 80%.
    pub fn grow(&mut self, area: Rect) {
        self.height_percent = (self.height_percent + 5).min(80);
        self.resize(area);
    }

    /// Shrink the terminal panel by 5%, clamped to 10%.
    pub fn shrink(&mut self, area: Rect) {
        self.height_percent = self.height_percent.saturating_sub(5).max(10);
        self.resize(area);
    }

    /// Calculate the panel height in rows for a given total area.
    pub fn panel_height(&self, area: Rect) -> u16 {
        let h = (area.height as u32 * self.height_percent as u32 / 100) as u16;
        h.max(3) // At minimum: separator + tab bar + 1 content line
    }

    /// Number of terminal tabs.
    pub fn tab_count(&self) -> usize {
        self.instances.len()
    }

    /// Active tab index.
    pub fn active_tab(&self) -> usize {
        self.active_tab
    }

    /// Cycle to next tab.
    pub fn next_tab(&mut self) {
        if !self.instances.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.instances.len();
        }
    }

    /// Cycle to previous tab.
    pub fn prev_tab(&mut self) {
        if !self.instances.is_empty() {
            if self.active_tab == 0 {
                self.active_tab = self.instances.len() - 1;
            } else {
                self.active_tab -= 1;
            }
        }
    }

    /// Render the terminal panel.
    ///
    /// `area` is the full area allocated to the terminal panel (bottom portion of screen).
    pub fn render(&self, area: Rect, surface: &mut Surface) {
        if !self.visible || area.height < 2 {
            return;
        }

        // Draw separator line at top (highlighted when mouse hovers for resize).
        let separator_style = if self.separator_highlighted {
            Style::default()
                .fg(Color::White)
                .bg(Color::Indexed(8)) // ANSI dark gray
        } else {
            Style::default()
                .fg(Color::Gray)
                .bg(Color::Reset)
        };
        for x in area.x..area.right() {
            let cell = &mut surface[(x, area.y)];
            cell.set_char('─');
            cell.set_style(separator_style);
        }

        // Draw tab bar on second line.
        let tab_y = area.y + 1;
        let tab_style = Style::default()
            .fg(Color::LightGray)
            .bg(Color::Reset);
        let active_tab_style = Style::default()
            .fg(Color::White)
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD);

        let mut x = area.x;
        for (i, instance) in self.instances.iter().enumerate() {
            let title = instance.title();
            let display = format!(" {} ", if title.is_empty() { "terminal" } else { title });
            let style = if i == self.active_tab {
                active_tab_style
            } else {
                tab_style
            };

            for ch in display.chars() {
                if x >= area.right() {
                    break;
                }
                let cell = &mut surface[(x, tab_y)];
                cell.set_char(ch);
                cell.set_style(style);
                x += 1;
            }

            // Separator between tabs.
            if x < area.right() {
                let cell = &mut surface[(x, tab_y)];
                cell.set_char('│');
                cell.set_style(separator_style);
                x += 1;
            }
        }

        // Fill rest of tab bar.
        while x < area.right() {
            let cell = &mut surface[(x, tab_y)];
            cell.set_char(' ');
            cell.set_style(tab_style);
            x += 1;
        }

        // Render active terminal content below tab bar.
        let content_area = Rect::new(
            area.x,
            area.y + 2,
            area.width,
            area.height.saturating_sub(2),
        );

        if content_area.height > 0 {
            if let Some(instance) = self.instances.get(self.active_tab) {
                instance.render_to_surface(content_area, surface);

                // Show exit message overlay when the shell process has exited.
                if let Some(code) = instance.exit_code() {
                    let msg = format!("[Process exited with code {code}]");
                    let msg_style = Style::default()
                        .fg(if code == 0 { Color::Green } else { Color::Red })
                        .add_modifier(Modifier::BOLD);

                    // Render at the bottom of the content area.
                    let y = content_area.y + content_area.height.saturating_sub(1);
                    let start_x = content_area.x;
                    for (i, ch) in msg.chars().enumerate() {
                        let x = start_x + i as u16;
                        if x >= content_area.right() {
                            break;
                        }
                        let cell = &mut surface[(x, y)];
                        cell.set_char(ch);
                        cell.set_style(msg_style);
                    }
                }
            }
        }
    }

    /// Get cursor position and kind from the active terminal.
    pub fn cursor(&self, area: Rect) -> (Option<Position>, CursorKind) {
        if !self.visible || !self.focused || area.height < 3 {
            return (None, CursorKind::Hidden);
        }

        let content_area = Rect::new(
            area.x,
            area.y + 2,
            area.width,
            area.height.saturating_sub(2),
        );

        if let Some(instance) = self.instances.get(self.active_tab) {
            instance.cursor(content_area)
        } else {
            (None, CursorKind::Hidden)
        }
    }
}
