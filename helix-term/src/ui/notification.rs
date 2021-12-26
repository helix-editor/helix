use crate::compositor::{Component, Context};
use helix_core::diagnostic::Severity;
use helix_view::graphics::{Margin, Rect};
use helix_view::theme::Color;
use tui::buffer::Buffer as Surface;
use tui::widgets::{Block, Borders, Paragraph, Widget};

pub struct NotificationsView {}

impl NotificationsView {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NotificationsView {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for NotificationsView {
    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        let theme = &cx.editor.theme;
        let text_style = theme.get("ui.text");
        let bg_style = theme.get("ui.background");
        let warning = theme.get("warning");
        let error = theme.get("error");
        let info = theme.get("info");
        let hint = theme.get("hint");

        let mut offset = 0;

        let notifications = cx.editor.notifications.to_display();
        for notification in notifications {
            let popup_style = match notification.severity {
                Severity::Error => error,
                Severity::Warning => warning,
                Severity::Info => info,
                Severity::Hint => hint,
            }
            .bg(bg_style.bg.unwrap_or(Color::Black));

            // Calculate the area of the terminal to modify. Because we want to
            // render at the bottom right, we use the viewport's width and height
            // which evaluate to the most bottom right coordinate.
            let width = 64 + 2 + 2; // +2 for border, +2 for margin
            let height = notification.height + 2; // +2 for border

            // skip notifications that would overflow the view
            // NOTE: optinal improvement would be to show how many notifications are waiting to be display or collapse notifications per title
            if height + offset > viewport.height {
                break;
            }

            let area = viewport.intersection(Rect::new(
                viewport.width.saturating_sub(width) - 1,
                offset,
                width,
                height,
            ));
            surface.clear_with(area, popup_style);

            offset += height;

            let block = Block::default()
                .title(notification.title.as_str())
                .style(popup_style)
                .borders(Borders::ALL)
                .border_style(popup_style);

            let margin = Margin {
                vertical: 0,
                horizontal: 1,
            };
            let inner = block.inner(area).inner(&margin);
            block.render(area, surface);

            Paragraph::new(notification.text.as_str())
                .style(text_style)
                .render(inner, surface);
        }
    }
}
