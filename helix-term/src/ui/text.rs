use crate::compositor::{Component, Context};
use tui::buffer::Buffer as Surface;

use helix_view::graphics::Rect;

pub struct Text {
    contents: String,
}

impl Text {
    pub fn new(contents: String) -> Self {
        Self { contents }
    }
}
impl Component for Text {
    fn render(&self, area: Rect, surface: &mut Surface, _cx: &mut Context) {
        use tui::widgets::{Paragraph, Widget, Wrap};
        let contents = tui::text::Text::from(self.contents.clone());

        let par = Paragraph::new(contents).wrap(Wrap { trim: false });
        // .scroll(x, y) offsets

        par.render(area, surface);
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let contents = tui::text::Text::from(self.contents.clone());
        let width = std::cmp::min(contents.width() as u16, viewport.0);
        let height = std::cmp::min(contents.height() as u16, viewport.1);
        Some((width, height))
    }
}
