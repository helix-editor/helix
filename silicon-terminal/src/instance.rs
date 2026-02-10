use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;

use alacritty_terminal::event::{Event as AlacEvent, EventListener, Notify, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, Msg, Notifier};
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::term::{self, Term, TermMode};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::CursorShape as AlacCursorShape;

use silicon_core::Position;
use silicon_view::graphics::{CursorKind, Rect};
use silicon_view::input::KeyEvent;

use tui::buffer::Buffer as Surface;

use crate::colors;
use crate::keys;

/// Internal event captured from the alacritty event loop.
enum TermEvent {
    Title(String),
    ChildExit(i32),
    Wakeup,
    Bell,
}

/// Notification relay from alacritty's I/O thread to the UI.
///
/// Captures title changes and child exit events in a queue, and sends
/// a lightweight wakeup signal to the application's event loop.
pub struct SiliconListener {
    wakeup_tx: tokio::sync::mpsc::UnboundedSender<()>,
    events: Arc<parking_lot::Mutex<Vec<TermEvent>>>,
}

impl EventListener for SiliconListener {
    fn send_event(&self, event: AlacEvent) {
        let queued = match event {
            AlacEvent::Title(title) => Some(TermEvent::Title(title)),
            AlacEvent::ChildExit(code) => Some(TermEvent::ChildExit(code)),
            AlacEvent::Wakeup => Some(TermEvent::Wakeup),
            AlacEvent::Bell => Some(TermEvent::Bell),
            _ => None,
        };
        if let Some(ev) = queued {
            self.events.lock().push(ev);
            let _ = self.wakeup_tx.send(());
        }
    }
}

impl Clone for SiliconListener {
    fn clone(&self) -> Self {
        Self {
            wakeup_tx: self.wakeup_tx.clone(),
            events: Arc::clone(&self.events),
        }
    }
}

/// Terminal size for alacritty's `Dimensions` trait.
#[derive(Clone, Copy)]
struct TermSize {
    cols: u16,
    lines: u16,
}

impl alacritty_terminal::grid::Dimensions for TermSize {
    fn columns(&self) -> usize {
        self.cols as usize
    }

    fn screen_lines(&self) -> usize {
        self.lines as usize
    }

    fn total_lines(&self) -> usize {
        self.screen_lines()
    }
}

/// A single terminal instance backed by `alacritty_terminal`.
pub struct TerminalInstance {
    term: Arc<FairMutex<Term<SiliconListener>>>,
    pty_tx: Notifier,
    events: Arc<parking_lot::Mutex<Vec<TermEvent>>>,
    _event_loop_handle: JoinHandle<(
        EventLoop<tty::Pty, SiliconListener>,
        alacritty_terminal::event_loop::State,
    )>,
    title: String,
    exited: Option<i32>,
    cols: u16,
    rows: u16,
}

impl TerminalInstance {
    /// Spawn a new terminal instance.
    ///
    /// `cols` and `rows` are the initial terminal dimensions.
    /// `shell` optionally specifies the shell program and its arguments
    /// (e.g. `["zsh", "-l"]`). When `None`, falls back to `$SHELL` or `/bin/sh`.
    pub fn new(
        cols: u16,
        rows: u16,
        wakeup_tx: tokio::sync::mpsc::UnboundedSender<()>,
        shell: Option<&[String]>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (program, args) = match shell {
            Some(parts) if !parts.is_empty() => {
                (parts[0].clone(), parts[1..].to_vec())
            }
            _ => {
                let s = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
                (s, Vec::new())
            }
        };

        let pty_config = tty::Options {
            shell: Some(tty::Shell::new(program, args)),
            working_directory: std::env::current_dir().ok(),
            drain_on_exit: true,
            env: HashMap::new(),
        };

        let window_size = WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 1,
            cell_height: 1,
        };

        let config = term::Config {
            scrolling_history: 10_000,
            ..term::Config::default()
        };

        let events: Arc<parking_lot::Mutex<Vec<TermEvent>>> =
            Arc::new(parking_lot::Mutex::new(Vec::new()));

        let listener = SiliconListener {
            wakeup_tx,
            events: Arc::clone(&events),
        };

        let size = TermSize { cols, lines: rows };

        let term = Term::new(config, &size, listener.clone());
        let term = Arc::new(FairMutex::new(term));

        let pty = tty::new(&pty_config, window_size, 0)?;

        let event_loop = EventLoop::new(
            Arc::clone(&term),
            listener,
            pty,
            pty_config.drain_on_exit,
            false,
        )?;

        let pty_tx = Notifier(event_loop.channel());
        let handle = event_loop.spawn();

        Ok(Self {
            term,
            pty_tx,
            events,
            _event_loop_handle: handle,
            title: String::from("terminal"),
            exited: None,
            cols,
            rows,
        })
    }

    /// Send raw bytes to the PTY.
    pub fn input(&self, bytes: &[u8]) {
        self.pty_tx.notify(Cow::Owned(bytes.to_vec()));
    }

    /// Handle a key event. Returns `true` if the key was sent to the PTY.
    pub fn handle_key(&self, key: &KeyEvent) -> bool {
        let mode = *self.term.lock().mode();
        if let Some(bytes) = keys::to_esc_str(key, mode) {
            self.input(&bytes);
            true
        } else {
            false
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;

        let size = TermSize { cols, lines: rows };

        let window_size = WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 1,
            cell_height: 1,
        };

        self.term.lock().resize(size);

        // Notify the PTY of the new size.
        let _ = self.pty_tx.0.send(Msg::Resize(window_size));
    }

    /// Scroll the terminal display.
    pub fn scroll(&self, delta: Scroll) {
        self.term.lock().scroll_display(delta);
    }

    /// Drain pending events from the event queue.
    ///
    /// Returns `true` if a redraw is needed.
    pub fn poll_events(&mut self) -> bool {
        let events: Vec<TermEvent> = {
            let mut lock = self.events.lock();
            std::mem::take(&mut *lock)
        };

        let mut needs_redraw = false;
        for event in events {
            match event {
                TermEvent::Title(title) => {
                    self.title = title;
                    needs_redraw = true;
                }
                TermEvent::ChildExit(code) => {
                    self.exited = Some(code);
                    needs_redraw = true;
                }
                TermEvent::Wakeup | TermEvent::Bell => {
                    needs_redraw = true;
                }
            }
        }
        needs_redraw
    }

    /// Get the current terminal title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Whether the child process has exited.
    pub fn has_exited(&self) -> bool {
        self.exited.is_some()
    }

    /// Get the exit code if the child process has exited.
    pub fn exit_code(&self) -> Option<i32> {
        self.exited
    }

    /// Get the current `TermMode` flags.
    pub fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    /// Render the terminal content onto a Silicon surface.
    pub fn render_to_surface(&self, area: Rect, surface: &mut Surface) {
        let term = self.term.lock();
        let content = term.renderable_content();

        // Iterate over visible cells.
        for indexed in content.display_iter {
            let point = indexed.point;
            let cell = &indexed.cell;

            let col = point.column.0 as u16;
            let line = point.line.0 as u16;

            // Skip cells outside our render area.
            if col >= area.width || line >= area.height {
                continue;
            }

            let x = area.x + col;
            let y = area.y + line;

            // Skip wide char spacers.
            if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
                continue;
            }

            let style =
                colors::cell_to_style(cell.fg, cell.bg, cell.flags, cell.underline_color());

            let ch = cell.c;
            let buf_cell = &mut surface[(x, y)];

            // Set the character.
            if ch == ' ' || ch == '\0' {
                buf_cell.set_char(' ');
            } else {
                buf_cell.set_char(ch);
            }

            // Apply style.
            buf_cell.set_style(style);

            // Handle zero-width characters.
            if let Some(zw) = cell.zerowidth() {
                for &zwc in zw {
                    buf_cell.symbol.push(zwc);
                }
            }
        }
    }

    /// Get cursor position and kind for the terminal.
    pub fn cursor(&self, area: Rect) -> (Option<Position>, CursorKind) {
        let term = self.term.lock();
        let content = term.renderable_content();

        let cursor = content.cursor;
        let col = cursor.point.column.0 as u16;
        let line = cursor.point.line.0 as u16;

        if col >= area.width || line >= area.height {
            return (None, CursorKind::Hidden);
        }

        let pos = Position::new((area.y + line) as usize, (area.x + col) as usize);

        let kind = match cursor.shape {
            AlacCursorShape::Block | AlacCursorShape::HollowBlock => CursorKind::Block,
            AlacCursorShape::Underline => CursorKind::Underline,
            AlacCursorShape::Beam => CursorKind::Bar,
            AlacCursorShape::Hidden => CursorKind::Hidden,
        };

        (Some(pos), kind)
    }
}

impl Drop for TerminalInstance {
    fn drop(&mut self) {
        // Send shutdown to the event loop.
        let _ = self.pty_tx.0.send(Msg::Shutdown);
    }
}
