use super::Direction;
use helix_core::Position;

pub trait PopupItem {}

pub trait Popup<T: PopupItem> {
    fn set_position(&mut self, pos: Option<Position>);
    fn scroll(&mut self, offset: usize, direction: Direction);
    fn contents(&self) -> &T;
    fn contents_mut(&mut self) -> &mut T;
}
