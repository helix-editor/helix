use super::size::TextSize;

pub trait TextLen: Copy {
    fn text_len(&self) -> TextSize;
}

impl TextLen for &'_ str {
    fn text_len(&self) -> TextSize {
        TextSize {
            raw: self.len() as u32,
        }
    }
}

impl TextLen for char {
    fn text_len(&self) -> TextSize {
        (self.len_utf8() as u32).into()
    }
}