use std::io;

use crate::{buffer::Cell, terminal::Config};

use helix_view::graphics::{CursorKind, Rect};

#[cfg(feature = "crossterm")]
mod crossterm;
#[cfg(feature = "crossterm")]
pub use self::crossterm::CrosstermBackend;

mod test;
pub use self::test::TestBackend;

pub trait Backend {
    fn claim(&mut self, config: Config) -> Result<(), io::Error>;
    fn reconfigure(&mut self, config: Config) -> Result<(), io::Error>;
    fn restore(&mut self, config: Config) -> Result<(), io::Error>;
    fn force_restore() -> Result<(), io::Error>;
    fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>;
    fn hide_cursor(&mut self) -> Result<(), io::Error>;
    fn show_cursor(&mut self, kind: CursorKind) -> Result<(), io::Error>;
    fn get_cursor(&mut self) -> Result<(u16, u16), io::Error>;
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), io::Error>;
    fn clear(&mut self) -> Result<(), io::Error>;
    fn size(&self) -> Result<Rect, io::Error>;
    fn flush(&mut self) -> Result<(), io::Error>;
}
