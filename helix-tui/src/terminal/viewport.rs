use helix_view::graphics::Rect;

use super::resize::ResizeBehavior;

#[derive(Debug, Clone, PartialEq)]
/// UNSTABLE
pub struct Viewport {
    pub area: Rect,
    pub resize_behavior: ResizeBehavior,
}

impl Viewport {
    /// UNSTABLE
    pub fn fixed(area: Rect) -> Viewport {
        Viewport {
            area,
            resize_behavior: ResizeBehavior::Fixed,
        }
    }
}
