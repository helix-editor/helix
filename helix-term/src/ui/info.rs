use crate::compositor::{Component, RenderContext};
use helix_view::graphics::{Margin, Rect};
use helix_view::info::Info;
use tui::buffer::Buffer as Surface;
use tui::widgets::{Block, Borders, Paragraph, Widget};

impl Component for Info {
    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut RenderContext<'_>) {
        let text_style = cx.editor.theme.get("ui.text.info");
        let popup_style = cx.editor.theme.get("ui.popup.info");

        // Calculate the area of the terminal to modify. Because we want to
        // render at the bottom right, we use the viewport's width and height
        // which evaluate to the most bottom right coordinate.
        let width = self.width + 2 + 2; // +2 for border, +2 for margin
        let height = self.height + 2; // +2 for border
        let area = viewport.intersection(Rect::new(
            viewport.width.saturating_sub(width),
            viewport.height.saturating_sub(height + 2), // +2 for statusline
            width,
            height,
        ));
        surface.clear_with(area, popup_style);

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(popup_style);

        let margin = Margin {
            vertical: 0,
            horizontal: 1,
        };
        let inner = block.inner(area).inner(&margin);
        block.render(area, surface);

        Paragraph::new(self.text.as_str())
            .style(text_style)
            .render(inner, surface);
    }
}
