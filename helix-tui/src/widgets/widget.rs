use crate::buffer::Buffer;
use helix_view::graphics::Rect;

/// Base requirements for a Widget
pub trait Widget {
    /// Draws the current state of the widget in the given buffer. That the only method required to
    /// implement a custom widget.
    fn render(self, area: Rect, buf: &mut Buffer);
}
