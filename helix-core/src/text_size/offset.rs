use std::{convert::TryFrom, num::TryFromIntError};

use super::TextSize;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextOffset {
    pub(crate) raw: i32,
}

impl From<i32> for TextOffset {
    fn from(raw: i32) -> Self {
        TextOffset { raw }
    }
}

impl From<TextOffset> for i32 {
    fn from(value: TextOffset) -> Self {
        value.raw
    }
}

impl TryFrom<usize> for TextOffset {
    type Error = TryFromIntError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(TextOffset {
            raw: i32::try_from(value)?,
        })
    }
}

impl TryFrom<TextSize> for TextOffset {
    type Error = TryFromIntError;
    fn try_from(value: TextSize) -> Result<Self, Self::Error> {
        Ok(TextOffset {
            raw: i32::try_from(value.raw)?,
        })
    }
}

