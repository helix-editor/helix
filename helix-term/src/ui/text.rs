use crate::compositor::{Component, Compositor, Context, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::buffer::Buffer as Surface;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

use std::borrow::Cow;

use helix_core::Position;
use helix_view::Editor;

pub struct Text {
    contents: String,
}

impl Text {
    pub fn new(contents: String) -> Self {
        Self { contents }
    }
}
impl Component for Text {
    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        use tui::widgets::{Paragraph, Widget, Wrap};
        let contents = tui::text::Text::from(self.contents.clone());

        let style = Style::default().fg(Color::Rgb(164, 160, 232)); // lavender

        let par = Paragraph::new(contents).wrap(Wrap { trim: false });
        // .scroll(x, y) offsets

        par.render(area, surface);
    }

    fn size_hint(&self, area: Rect) -> Option<(usize, usize)> {
        let contents = tui::text::Text::from(self.contents.clone());
        Some((contents.width(), contents.height()))
    }
}
