use helix_core::Position;
use helix_view::{
    graphics::{CursorKind, Rect},
    Editor,
};
use tui::buffer::Buffer;

use crate::compositor::{Component, Context, Event, EventResult};

/// Contains a component placed in the center of the parent component
pub struct Overlay<T> {
    /// Child component
    pub content: T,
    /// Function to compute the size and position of the child component
    pub calc_child_size: Box<dyn Fn(Rect) -> Rect>,
}

/// Surrounds the component with a margin of 5% on each side, and an additional 2 rows at the bottom
pub fn overlaid<T>(content: T) -> Overlay<T> {
    Overlay {
        content,
        calc_child_size: Box::new(|rect: Rect| rect.overlayed()),
    }
}

impl<T: Component + 'static> Component for Overlay<T> {
    fn render(&mut self, area: Rect, frame: &mut Buffer, ctx: &mut Context) {
        let dimensions = (self.calc_child_size)(area);
        self.content.render(dimensions, frame, ctx)
    }

    fn required_size(&mut self, (width, height): (u16, u16)) -> Option<(u16, u16)> {
        let area = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let dimensions = (self.calc_child_size)(area);
        let viewport = (dimensions.width, dimensions.height);
        let _ = self.content.required_size(viewport)?;
        Some((width, height))
    }

    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> EventResult {
        self.content.handle_event(event, ctx)
    }

    fn cursor(&self, area: Rect, ctx: &Editor) -> (Option<Position>, CursorKind) {
        let dimensions = (self.calc_child_size)(area);
        self.content.cursor(dimensions, ctx)
    }

    fn id(&self) -> Option<&'static str> {
        self.content.id()
    }
}
