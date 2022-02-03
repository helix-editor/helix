use crossterm::event::Event;
use helix_core::Position;
use helix_view::{
    graphics::{CursorKind, Rect},
    Editor,
};
use tui::buffer::Buffer;

use crate::compositor::{Component, Context, EventResult};

/// Contains a component placed in the center of the parent component
#[derive(Debug)]
pub struct Overlay<T> {
    /// Child component
    pub content: T,
    /// Value between 0 and 100 that indicates how much of the vertical space is used
    pub vertical_size: u8,
    /// Value between 0 and 100 that indicates how much of the horizontal space is used
    pub horizontal_size: u8,
}

impl<T> Overlay<T> {
    const MARGIN_BOTTOM: u16 = 2;

    fn get_dimensions(&self, viewport: (u16, u16)) -> Rect {
        fn mul_and_cast(size: u16, factor: u8) -> u16 {
            ((size as u32) * (factor as u32) / 100).try_into().unwrap()
        }

        let (outer_w, outer_h) = viewport;
        let outer_h = outer_h.saturating_sub(Self::MARGIN_BOTTOM);

        let inner_w = mul_and_cast(outer_w, self.horizontal_size);
        let inner_h = mul_and_cast(outer_h, self.vertical_size);

        let pos_x = outer_w.saturating_sub(inner_w) / 2;
        let pos_y = outer_h.saturating_sub(inner_h) / 2;

        Rect {
            x: pos_x,
            y: pos_y,
            width: inner_w,
            height: inner_h,
        }
    }

    fn get_outer_size_from_inner_size(&self, inner: (u16, u16)) -> (u16, u16) {
        fn div_and_cast(size: u16, divisor: u8) -> u16 {
            ((size as u32) * 100 / (divisor as u32)).try_into().unwrap()
        }

        let (inner_w, inner_h) = inner;
        (
            div_and_cast(inner_w, self.horizontal_size),
            div_and_cast(inner_h, self.vertical_size) + Self::MARGIN_BOTTOM,
        )
    }
}

impl<T: Component + 'static> Component for Overlay<T> {
    fn render(&mut self, area: Rect, frame: &mut Buffer, ctx: &mut Context) {
        let dimensions = self.get_dimensions((area.width, area.height));
        self.content.render(dimensions, frame, ctx)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let dimensions = self.get_dimensions(viewport);
        let viewport = (dimensions.width, dimensions.height);
        let required = self.content.required_size(viewport)?;
        Some(self.get_outer_size_from_inner_size(required))
    }

    fn handle_event(&mut self, event: Event, ctx: &mut Context) -> EventResult {
        self.content.handle_event(event, ctx)
    }

    fn cursor(&self, area: Rect, ctx: &Editor) -> (Option<Position>, CursorKind) {
        let dimensions = self.get_dimensions((area.width, area.height));
        self.content.cursor(dimensions, ctx)
    }
}
