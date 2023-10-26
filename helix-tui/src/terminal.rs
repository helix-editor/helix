use crate::{backend::Backend, buffer::Buffer};
use helix_view::editor::Config as EditorConfig;
use helix_view::graphics::{CursorKind, Rect};
use std::io;

#[derive(Debug, Clone, PartialEq)]
/// UNSTABLE
enum ResizeBehavior {
    Fixed,
    Auto,
}

#[derive(Debug, Clone, PartialEq)]
/// UNSTABLE
pub struct Viewport {
    area: Rect,
    resize_behavior: ResizeBehavior,
}

#[derive(Debug)]
pub struct Config {
    pub enable_mouse_capture: bool,
}

impl From<EditorConfig> for Config {
    fn from(config: EditorConfig) -> Self {
        Self {
            enable_mouse_capture: config.mouse,
        }
    }
}

impl Viewport {
    /// UNSTABLE
    pub fn fixed(area: Rect) -> Viewport {
        Viewport {
            area,
            resize_behavior: ResizeBehavior::Fixed,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Options to pass to [`Terminal::with_options`]
pub struct TerminalOptions {
    /// Viewport used to draw to the terminal
    pub viewport: Viewport,
}

/// Interface to the terminal backed by Termion
#[derive(Debug)]
pub struct Terminal<B>
where
    B: Backend,
{
    backend: B,
    /// Holds the results of the current and previous draw calls. The two are compared at the end
    /// of each draw pass to output the necessary updates to the terminal
    buffers: [Buffer; 2],
    /// Index of the current buffer in the previous array
    current: usize,
    /// Kind of cursor (hidden or others)
    cursor_kind: CursorKind,
    /// Viewport
    viewport: Viewport,
}

impl<B> Terminal<B>
where
    B: Backend,
{
    /// Wrapper around Terminal initialization. Each buffer is initialized with a blank string and
    /// default colors for the foreground and the background
    pub fn new(backend: B) -> io::Result<Terminal<B>> {
        let size = backend.size()?;
        Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport {
                    area: size,
                    resize_behavior: ResizeBehavior::Auto,
                },
            },
        )
    }

    /// UNSTABLE
    pub fn with_options(backend: B, options: TerminalOptions) -> io::Result<Terminal<B>> {
        Ok(Terminal {
            backend,
            buffers: [
                Buffer::empty(options.viewport.area),
                Buffer::empty(options.viewport.area),
            ],
            current: 0,
            cursor_kind: CursorKind::Block,
            viewport: options.viewport,
        })
    }

    pub fn claim(&mut self, config: Config) -> io::Result<()> {
        self.backend.claim(config)
    }

    pub fn reconfigure(&mut self, config: Config) -> io::Result<()> {
        self.backend.reconfigure(config)
    }

    pub fn restore(&mut self, config: Config) -> io::Result<()> {
        self.backend.restore(config)
    }

    // /// Get a Frame object which provides a consistent view into the terminal state for rendering.
    // pub fn get_frame(&mut self) -> Frame<B> {
    //     Frame {
    //         terminal: self,
    //         cursor_position: None,
    //     }
    // }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Obtains a difference between the previous and the current buffer and passes it to the
    /// current backend for drawing.
    pub fn flush(&mut self) -> io::Result<()> {
        let previous_buffer = &self.buffers[1 - self.current];
        let current_buffer = &self.buffers[self.current];
        let updates = previous_buffer.diff(current_buffer);
        self.backend.draw(updates.into_iter())
    }

    /// Updates the Terminal so that internal buffers match the requested size. Requested size will
    /// be saved so the size can remain consistent when rendering.
    /// This leads to a full clear of the screen.
    pub fn resize(&mut self, area: Rect) -> io::Result<()> {
        self.buffers[self.current].resize(area);
        self.buffers[1 - self.current].resize(area);
        self.viewport.area = area;
        self.clear()
    }

    /// Queries the backend for size and resizes if it doesn't match the previous size.
    pub fn autoresize(&mut self) -> io::Result<Rect> {
        let size = self.size()?;
        if size != self.viewport.area {
            self.resize(size)?;
        };
        Ok(size)
    }

    /// Synchronizes terminal size, calls the rendering closure, flushes the current internal state
    /// and prepares for the next draw call.
    pub fn draw(
        &mut self,
        cursor_position: Option<(u16, u16)>,
        cursor_kind: CursorKind,
    ) -> io::Result<()> {
        // // Autoresize - otherwise we get glitches if shrinking or potential desync between widgets
        // // and the terminal (if growing), which may OOB.
        // self.autoresize()?;

        // let mut frame = self.get_frame();
        // f(&mut frame);
        // // We can't change the cursor position right away because we have to flush the frame to
        // // stdout first. But we also can't keep the frame around, since it holds a &mut to
        // // Terminal. Thus, we're taking the important data out of the Frame and dropping it.
        // let cursor_position = frame.cursor_position;

        // Draw to stdout
        self.flush()?;

        if let Some((x, y)) = cursor_position {
            self.set_cursor(x, y)?;
        }

        match cursor_kind {
            CursorKind::Hidden => self.hide_cursor()?,
            kind => self.show_cursor(kind)?,
        }

        // Swap buffers
        self.buffers[1 - self.current].reset();
        self.current = 1 - self.current;

        // Flush
        self.backend.flush()?;
        Ok(())
    }

    #[inline]
    pub fn cursor_kind(&self) -> CursorKind {
        self.cursor_kind
    }

    pub fn hide_cursor(&mut self) -> io::Result<()> {
        self.backend.hide_cursor()?;
        self.cursor_kind = CursorKind::Hidden;
        Ok(())
    }

    pub fn show_cursor(&mut self, kind: CursorKind) -> io::Result<()> {
        self.backend.show_cursor(kind)?;
        self.cursor_kind = kind;
        Ok(())
    }

    pub fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        self.backend.get_cursor()
    }

    pub fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.backend.set_cursor(x, y)
    }

    /// Clear the terminal and force a full redraw on the next draw call.
    pub fn clear(&mut self) -> io::Result<()> {
        self.backend.clear()?;
        // Reset the back buffer to make sure the next update will redraw everything.
        self.buffers[1 - self.current].reset();
        Ok(())
    }

    /// Queries the real size of the backend.
    pub fn size(&self) -> io::Result<Rect> {
        self.backend.size()
    }
}
