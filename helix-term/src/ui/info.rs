use crate::compositor::{Component, Context};
use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::graphics::{Margin, Rect};
use helix_view::info::Info;
use tui::buffer::Buffer as Surface;
use tui::widgets::{Block, Borders, Paragraph, Widget};

impl Component for Info {
    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        let text_style = cx.editor.theme.get("ui.text.info");
        let popup_style = cx.editor.theme.get("ui.popup.info");
        let background_style = cx.editor.theme.get("ui.background");

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

        // Render margin to prevent double-width characters (CJK) from flowing
        // into border, but we also do not want to overwrite existing
        // cells so we only overwrite those that are troublesome
        // (double-width).
        let left_margin_area = viewport.intersection(Rect::new(
            viewport.width.saturating_sub(width + 1),   // +1 for margin
            viewport.height.saturating_sub(height + 2), // +2 for status
            1,
            height + 1,
        ));
        // surface.clear_with(left_margin_area, background_style);
        for x in left_margin_area.left()..left_margin_area.right() {
            for y in left_margin_area.top()..left_margin_area.bottom() {
                let cell = &mut surface[(x, y)];
                if cell.symbol().width() > 1 {
                    cell.reset();
                    cell.set_style(background_style);
                }
            }
        }

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
