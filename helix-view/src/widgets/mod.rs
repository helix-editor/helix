pub mod menu;
pub mod popup;
pub mod prompt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}
