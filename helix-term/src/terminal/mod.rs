//! Integrated terminal emulator module.
//!
//! This module provides embedded terminal functionality within Helix,
//! allowing users to run shell commands without leaving the editor.
//!
//! Author: Huzeyfe Coşkun <huzeyfecoskun@hotmail.com>
//! GitHub: https://github.com/huzeyfecoskun

mod emulator;
mod pty;

pub use emulator::{TerminalCell, TerminalColor, TerminalEmulator};
pub use pty::{Pty, PtyEvent, PtySize};

use std::sync::atomic::{AtomicU32, Ordering};

/// Unique identifier for terminal instances
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerminalId(u32);

impl TerminalId {
    /// Generate a new unique terminal ID
    pub fn next() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl std::fmt::Display for TerminalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Terminal {}", self.0)
    }
}

/// Terminal instance containing both PTY and emulator state
pub struct Terminal {
    /// Unique identifier for this terminal
    pub id: TerminalId,
    /// Terminal title (usually from shell or running process)
    pub title: String,
    /// The pseudo-terminal handling process I/O
    pub pty: Pty,
    /// Terminal emulator state (screen buffer, cursor, etc.)
    pub emulator: TerminalEmulator,
    /// Whether this terminal has exited
    pub exited: bool,
}

impl Terminal {
    /// Create a new terminal instance
    pub fn new(cwd: Option<std::path::PathBuf>, shell: Option<&[String]>) -> anyhow::Result<Self> {
        let id = TerminalId::next();
        let size = PtySize::default();

        // Get title from cwd - use last component (folder name)
        let title = cwd
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            })
            .unwrap_or_else(|| String::from("terminal"));

        let pty = Pty::new(cwd, shell, size)?;
        let emulator = TerminalEmulator::new(size.rows, size.cols);

        let mut terminal = Self {
            id,
            title,
            pty,
            emulator,
            exited: false,
        };

        // Set up automatic title updating for zsh
        // Send precmd hook that updates title to current directory name
        let shell_cmd = shell.map(|s| s.to_vec()).unwrap_or_else(|| {
            vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())]
        });
        let shell_name = shell_cmd[0].rsplit('/').next().unwrap_or(&shell_cmd[0]);

        if shell_name == "zsh" {
            // For zsh, define precmd function that updates title on each prompt
            // We use a subshell and clear to hide the initialization
            let init_cmd = r#"precmd() { printf "\033]0;%s\007" "${PWD##*/}" }; clear; precmd"#;
            let _ = terminal.write(format!("{}\n", init_cmd).as_bytes());
        }

        Ok(terminal)
    }

    /// Resize the terminal
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let size = PtySize { rows, cols };
        if let Err(e) = self.pty.resize(size) {
            log::error!("Failed to resize PTY: {}", e);
        }
        self.emulator.resize(rows, cols);
    }

    /// Process output from the PTY
    pub fn process_output(&mut self, data: &[u8]) {
        self.emulator.process(data);
        // Update title if changed by escape sequence
        if let Some(title) = self.emulator.take_title() {
            self.title = title;
        }
    }

    /// Write input to the PTY
    pub fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.pty.write(data)
    }

    /// Mark the terminal as exited
    pub fn mark_exited(&mut self) {
        self.exited = true;
        self.title = format!("{} (exited)", self.title);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_id_unique() {
        let id1 = TerminalId::next();
        let id2 = TerminalId::next();
        let id3 = TerminalId::next();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_terminal_id_display() {
        let id = TerminalId(42);
        assert_eq!(format!("{}", id), "Terminal 42");
    }

    #[test]
    fn test_pty_size_default() {
        let size = PtySize::default();
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
    }
}
