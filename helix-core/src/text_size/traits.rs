use std::convert::TryInto;

use super::TextSize;
pub trait TextLen: Copy {
    fn text_len(&self) -> TextSize;
}

impl TextLen for &'_ str {
    fn text_len(&self) -> TextSize {
        self.chars().count().try_into().unwrap()
    }
}

impl TextLen for char {
    fn text_len(&self) -> TextSize {
        1
    }
}

impl TextLen for TextSize {
    fn text_len(&self) -> TextSize {
        *self
    }
}
