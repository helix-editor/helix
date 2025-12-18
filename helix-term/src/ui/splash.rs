use crate::compositor::{Callback, Component, Compositor, Context, EventResult};
use helix_view::{graphics::Rect, input::Event};
use tui::{
    buffer::Buffer as Surface,
    text::Text,
    widgets::{Paragraph, Widget},
};

/// A full-screen component that displays centered text and closes on any keypress.
pub struct Splash {
    contents: Text<'static>,
}

impl Splash {
    pub fn new(contents: String) -> Self {
        Self {
            contents: Text::from(contents),
        }
    }
}

impl Component for Splash {
    fn handle_event(&mut self, event: &Event, _cx: &mut Context) -> EventResult {
        match event {
            Event::Key(_) => {
                // Close on any keypress
                let close_fn: Callback = Box::new(|compositor: &mut Compositor, _| {
                    compositor.pop();
                });
                EventResult::Consumed(Some(close_fn))
            }
            _ => EventResult::Ignored(None),
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // Clear the entire area with the popup background
        let background = cx.editor.theme.get("ui.popup");
        surface.clear_with(area, background);

        // Calculate content dimensions
        let content_width = self.contents.width() as u16;
        let content_height = self.contents.height() as u16;

        // Center the content
        let x = area.x + area.width.saturating_sub(content_width) / 2;
        let y = area.y + area.height.saturating_sub(content_height) / 2;

        let centered_area = Rect::new(x, y, content_width, content_height);

        let paragraph = Paragraph::new(&self.contents);
        paragraph.render(centered_area, surface);
    }
}
