use crate::compositor::{Component, Context};
use helix_view::graphics::{Margin, Rect};
use helix_view::info::Info;
use helix_view::theme::Style;
use tui::buffer::Buffer as Surface;
use tui::text::Span;
use tui::widgets::{Block, Widget};

impl Component for Info {
    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        let popup_bg = cx.editor.theme.get("ui.popup.info");
        let border_style = Style::new()
            .fg(crate::ui::primary_color(&cx.editor.theme))
            .bg(popup_bg.bg.unwrap_or(helix_view::theme::Color::Reset));
        let key_style = Style::default().fg(crate::ui::secondary_color(&cx.editor.theme));
        let desc_style = Style::default().fg(crate::ui::tertiary_color(&cx.editor.theme));

        // Calculate the area of the terminal to modify. Because we want to
        // render at the bottom right, we use the viewport's width and height
        // which evaluate to the most bottom right coordinate.
        let width = self.width + 2 + 2; // +2 for border, +2 for margin
        let height = self.height + 2; // +2 for border
        let area = viewport.clip_bottom(1).intersection(Rect::new(
            viewport.width.saturating_sub(width),
            viewport.height.saturating_sub(height + 1), // +1 for statusline
            width,
            height,
        ));
        surface.clear_with(area, popup_bg);

        let block = Block::bordered()
            .title(Span::styled(self.title.as_ref().to_owned(), border_style))
            .border_style(border_style);

        let margin = Margin::horizontal(1);
        let inner = block.inner(area).inner(margin);
        block.render(area, surface);

        // Render body with shortcut in secondary and description in tertiary.
        // `Info::new` formats each line as `"{key:width$}  {desc}"`.
        for (idx, line) in self.text.lines().enumerate() {
            let y = inner.y + idx as u16;
            if y >= inner.bottom() {
                break;
            }
            // Find the separator `  ` between key and desc.
            if let Some(sep_pos) = line.find("  ") {
                let key = &line[..sep_pos];
                let desc = &line[sep_pos + 2..];
                // Key is right-padded to item_width; trim trailing spaces for style but keep width.
                let key_trimmed = key.trim_end();
                let key_width = key.len() as u16;
                surface.set_stringn(inner.x, y, key_trimmed, key_width as usize, key_style);
                // Two spaces separator in popup bg
                surface.set_stringn(inner.x + key_width, y, "  ", 2, popup_bg);
                surface.set_stringn(
                    inner.x + key_width + 2,
                    y,
                    desc,
                    inner.width.saturating_sub(key_width + 2) as usize,
                    desc_style,
                );
            } else {
                surface.set_stringn(inner.x, y, line, inner.width as usize, desc_style);
            }
        }
    }
}
