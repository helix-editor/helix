use crate::compositor::{Component, Context};
use helix_view::graphics::Rect;
use helix_view::info::Info;
use tui::buffer::Buffer as Surface;
use tui::widgets::{Block, Borders, Widget};

impl Component for Info {
    fn render(&self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        let style = cx.editor.theme.get("ui.popup");

        // Calculate the area of the terminal to modify. Because we want to
        // render at the bottom right, we use the viewport's width and height
        // which evaluate to the most bottom right coordinate.
        let (width, height) = (self.width + 2, self.height + 2);
        let area = viewport.intersection(Rect::new(
            viewport.width.saturating_sub(width),
            viewport.height.saturating_sub(height + 2),
            width,
            height,
        ));
        surface.clear_with(area, style);

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(style);
        let inner = block.inner(area);
        block.render(area, surface);

        // Only write as many lines as there are rows available.
        for (y, line) in (inner.y..)
            .zip(self.text.lines())
            .take(inner.height as usize)
        {
            surface.set_string(inner.x, y, line, style);
        }
    }
}
