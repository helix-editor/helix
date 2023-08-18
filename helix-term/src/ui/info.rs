use crate::compositor::{Component, Context};
use helix_view::graphics::{Margin, Rect};
use helix_view::info::Info;
use tui::buffer::Buffer as Surface;
use tui::widgets::{Block, Borders, Paragraph, Widget};

impl Component for Info {
    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        let text_style = cx.editor.theme.get("ui.text.info");
        let popup_style = cx.editor.theme.get("ui.popup.info");

        let area = self.get_intersecting_area(viewport);
        surface.clear_with(area, popup_style);

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(popup_style);

        let margin = Margin::horizontal(1);
        let inner = block.inner(area).inner(&margin);
        block.render(area, surface);

        Paragraph::new(self.text.as_str())
            .style(text_style)
            .render(inner, surface);
    }
}
