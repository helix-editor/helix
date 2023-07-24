use crossterm::{event::Event, ErrorKind};
use helix_view::graphics::{CursorKind, Rect};
use std::io;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tui::{
    backend::Backend,
    buffer::{Buffer, Cell},
    terminal::Config,
};

#[derive(Debug)]
pub struct TestBackend {
    event_stream: UnboundedReceiverStream<Result<Event, ErrorKind>>,
    width: u16,
    buffer: Buffer,
    height: u16,
    cursor: bool,
    pos: (u16, u16),
}

impl TestBackend {
    pub fn new(
        width: u16,
        height: u16,
        event_stream: UnboundedReceiverStream<Result<Event, ErrorKind>>,
    ) -> TestBackend {
        TestBackend {
            event_stream,
            width,
            height,
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            cursor: false,
            pos: (0, 0),
        }
    }
}

impl Backend for TestBackend {
    type Stream = UnboundedReceiverStream<Result<Event, io::Error>>;

    fn claim(&mut self, _config: Config) -> Result<(), io::Error> {
        Ok(())
    }

    fn reconfigure(&mut self, _config: Config) -> Result<(), io::Error> {
        Ok(())
    }

    fn restore(&mut self, _config: Config) -> Result<(), io::Error> {
        Ok(())
    }

    fn force_restore() -> Result<(), io::Error> {
        Ok(())
    }

    fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        for (x, y, c) in content {
            self.buffer[(x, y)] = c.clone();
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), io::Error> {
        self.cursor = false;
        Ok(())
    }

    fn show_cursor(&mut self, _kind: CursorKind) -> Result<(), io::Error> {
        self.cursor = true;
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), io::Error> {
        Ok(self.pos)
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), io::Error> {
        self.pos = (x, y);
        Ok(())
    }

    fn clear(&mut self) -> Result<(), io::Error> {
        self.buffer.reset();
        Ok(())
    }

    fn size(&self) -> Result<Rect, io::Error> {
        Ok(Rect::new(0, 0, self.width, self.height))
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }

    fn event_stream(&mut self) -> &mut Self::Stream {
        &mut self.event_stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui::terminal::Terminal;

    #[test]
    fn terminal_buffer_size_should_not_be_limited() {
        let rx_stream = UnboundedReceiverStream::new(tokio::sync::mpsc::unbounded_channel().1);
        let backend = TestBackend::new(400, 400, rx_stream);
        let terminal = Terminal::new(backend).unwrap();
        let size = terminal.backend().size().unwrap();
        assert_eq!(size.width, 400);
        assert_eq!(size.height, 400);
    }
}
