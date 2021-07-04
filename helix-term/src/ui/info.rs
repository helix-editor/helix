use crate::compositor::{Component, Context};
use helix_view::graphics::Rect;
use helix_view::info::Info;
use tui::buffer::Buffer as Surface;
use tui::widgets::{Block, Borders, Widget};

impl Component for Info {
    fn render(&self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        let style = cx.editor.theme.get("ui.popup");
        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(style);
        let Info { width, height, .. } = self;
        let (w, h) = (*width + 2, *height + 2);
        // -2 to subtract command line + statusline. a bit of a hack, because of splits.
        let area = viewport.intersection(Rect::new(
            viewport.width.saturating_sub(w),
            viewport.height.saturating_sub(h + 2),
            w,
            h,
        ));
        surface.clear_with(area, style);
        let Rect { x, y, .. } = block.inner(area);
        for (y, line) in (y..).zip(self.text.lines()) {
            surface.set_string(x, y, line, style);
        }
        block.render(area, surface);
    }
}
