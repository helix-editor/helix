//! Pseudo-terminal (PTY) handling.
//!
//! This module wraps portable-pty to provide cross-platform PTY support.
//!
//! Author: Huzeyfe Coşkun <huzeyfecoskun@hotmail.com>
//! GitHub: https://github.com/huzeyfecoskun

use anyhow::{Context, Result};
use portable_pty::{
    native_pty_system, Child, CommandBuilder, MasterPty, PtyPair, PtySize as PtyPtySize,
};
use std::io::{Read, Write};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Size of the terminal in rows and columns
#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

impl From<PtySize> for PtyPtySize {
    fn from(size: PtySize) -> Self {
        PtyPtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

/// Events from the PTY
#[derive(Debug)]
pub enum PtyEvent {
    /// Data received from the PTY
    Output(Vec<u8>),
    /// The child process has exited
    Exit(Option<u32>),
}

/// Pseudo-terminal wrapper
pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    output_rx: mpsc::UnboundedReceiver<PtyEvent>,
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl Pty {
    /// Create a new PTY with the specified working directory and shell
    pub fn new(cwd: Option<PathBuf>, shell: Option<&[String]>, size: PtySize) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair: PtyPair = pty_system
            .openpty(size.into())
            .context("Failed to open PTY")?;

        let shell_cmd = Self::get_shell_command(shell);
        let mut cmd = CommandBuilder::new(&shell_cmd[0]);

        if shell_cmd.len() > 1 {
            cmd.args(&shell_cmd[1..]);
        }

        // Set working directory
        if let Some(cwd) = cwd {
            cmd.cwd(cwd);
        } else if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }

        // Inherit environment
        for (key, value) in std::env::vars() {
            cmd.env(key, value);
        }

        // Set TERM for proper terminal emulation
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell process")?;

        let master = pair.master;
        let writer = master.take_writer().context("Failed to get PTY writer")?;

        // Set up async reader
        let (output_tx, output_rx) = mpsc::unbounded_channel();
        let mut reader = master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;

        let reader_handle = tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 8192]; // Larger buffer for better throughput
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - process exited
                        let _ = output_tx.send(PtyEvent::Exit(None));
                        break;
                    }
                    Ok(n) => {
                        if output_tx.send(PtyEvent::Output(buf[..n].to_vec())).is_err() {
                            break;
                        }
                        // Request redraw immediately when data arrives
                        helix_event::request_redraw();
                    }
                    Err(e) => {
                        log::error!("Error reading from PTY: {}", e);
                        let _ = output_tx.send(PtyEvent::Exit(None));
                        break;
                    }
                }
            }
        });

        Ok(Self {
            master,
            child,
            writer,
            output_rx,
            _reader_handle: reader_handle,
        })
    }

    /// Get the shell command to use
    fn get_shell_command(shell: Option<&[String]>) -> Vec<String> {
        if let Some(shell) = shell {
            if !shell.is_empty() {
                return shell.to_vec();
            }
        }

        // Try SHELL environment variable
        if let Ok(shell) = std::env::var("SHELL") {
            return vec![shell];
        }

        // Platform defaults
        #[cfg(windows)]
        {
            if let Ok(comspec) = std::env::var("COMSPEC") {
                return vec![comspec];
            }
            vec!["cmd.exe".to_string()]
        }

        #[cfg(not(windows))]
        {
            vec!["/bin/sh".to_string()]
        }
    }

    /// Resize the PTY
    pub fn resize(&mut self, size: PtySize) -> Result<()> {
        self.master
            .resize(size.into())
            .context("Failed to resize PTY")
    }

    /// Write data to the PTY (keyboard input)
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer
            .write_all(data)
            .context("Failed to write to PTY")?;
        self.writer.flush().context("Failed to flush PTY writer")
    }

    /// Try to receive output from the PTY (non-blocking)
    pub fn try_recv(&mut self) -> Option<PtyEvent> {
        self.output_rx.try_recv().ok()
    }

    /// Receive output from the PTY (async)
    pub async fn recv(&mut self) -> Option<PtyEvent> {
        self.output_rx.recv().await
    }

    /// Check if the child process is still running
    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Kill the child process
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("Failed to kill PTY process")
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // Try to kill the process gracefully
        let _ = self.kill();
    }
}
