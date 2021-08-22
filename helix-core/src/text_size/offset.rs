use std::{convert::TryFrom, num::TryFromIntError, ops::{Add, AddAssign, Sub, SubAssign}};

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

impl Add<TextOffset> for TextOffset {
    type Output = TextOffset;

    fn add(self, rhs: TextOffset) -> Self::Output {
        TextOffset {
            raw: self.raw + rhs.raw,
        }
    }
}

impl Sub<TextOffset> for TextOffset {
    type Output = TextOffset;

    fn sub(self, rhs: TextOffset) -> Self::Output {
        TextOffset {
            raw: self.raw - rhs.raw,
        }
    }
}

impl<A> AddAssign<A> for TextOffset 
where
    TextOffset: Add<A, Output = TextOffset>
{
    fn add_assign(&mut self, rhs: A) {
        *self = *self + rhs
    }
}

impl<S> SubAssign<S> for TextOffset
where
    TextOffset: Sub<S, Output = TextOffset>,
{
    #[inline]
    fn sub_assign(&mut self, rhs: S) {
        *self = *self - rhs
    }
}
