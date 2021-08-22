use super::{traits::TextLen, TextOffset, TextRange};
use std::{
    convert::{TryFrom, TryInto},
    fmt, iter,
    num::TryFromIntError,
    ops::{Add, AddAssign, Sub, SubAssign},
};

/// A measure of text length. Also, equivalently, an index into text.
///
/// This is a UTF-8 bytes offset or a char offset stored as `u32`
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextSize {
    pub(crate) raw: u32,
}

impl TextSize {
    /// The text size of some primitive text-like object.
    ///
    /// Accepts `char`, `&str`, and `&String`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use text_size::*;
    /// let char_size = TextSize::of('🦀');
    /// assert_eq!(char_size, TextSize::from(4));
    ///
    /// let str_size = TextSize::of("rust-analyzer");
    /// assert_eq!(str_size, TextSize::from(13));
    /// ```
    #[inline]
    pub fn of<T: TextLen>(text: T) -> TextSize {
        text.text_len()
    }

    /// Checked addition. Returns `None` if overflow occurred.
    #[inline]
    pub fn checked_add(self, rhs: TextSize) -> Option<TextSize> {
        self.raw.checked_add(rhs.raw).map(|raw| TextSize { raw })
    }

    /// Checked subtraction. Returns `None` if overflow occurred.
    #[inline]
    pub fn checked_sub(self, rhs: TextSize) -> Option<TextSize> {
        self.raw.checked_sub(rhs.raw).map(|raw| TextSize { raw })
    }
}

impl fmt::Debug for TextSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl From<u32> for TextSize {
    #[inline]
    fn from(raw: u32) -> TextSize {
        TextSize { raw }
    }
}

impl From<TextSize> for u32 {
    #[inline]
    fn from(value: TextSize) -> Self {
        value.raw
    }
}

impl From<&TextSize> for TextSize {
    fn from(value: &TextSize) -> Self {
        *value
    }
}

impl TryFrom<usize> for TextSize {
    type Error = TryFromIntError;
    #[inline]
    fn try_from(value: usize) -> Result<Self, TryFromIntError> {
        Ok(u32::try_from(value)?.into())
    }
}

impl TryFrom<i32> for TextSize {
    type Error = TryFromIntError;
    #[inline]
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(u32::try_from(value)?.into())
    }
}

impl From<TextSize> for i32 {
    fn from(value: TextSize) -> Self {
        value.raw as i32
    }
}

impl From<TextSize> for usize {
    #[inline]
    fn from(value: TextSize) -> Self {
        value.raw as usize
    }
}

impl TryFrom<TextRange> for TextSize {
    type Error = ();
    fn try_from(value: TextRange) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Ok(TextSize::of(value.start()))
        } else {
            Err(())
        }
    }
}

macro_rules! ops {
    (impl $Op:ident for TextSize by fn $f:ident = $op:tt) => {
        impl $Op<TextSize> for TextSize {
            type Output = TextSize;
            #[inline]
            fn $f(self, other: TextSize) -> TextSize {
                TextSize { raw: self.raw $op other.raw }
            }
        }
        impl $Op<&TextSize> for TextSize {
            type Output = TextSize;
            #[inline]
            fn $f(self, other: &TextSize) -> TextSize {
                self $op *other
            }
        }
        impl<T> $Op<T> for &TextSize
        where
            TextSize: $Op<T, Output=TextSize>,
        {
            type Output = TextSize;
            #[inline]
            fn $f(self, other: T) -> TextSize {
                *self $op other
            }
        }
    };
}

ops!(impl Add for TextSize by fn add = +);
ops!(impl Sub for TextSize by fn sub = -);

impl Add<TextOffset> for TextSize {
    type Output = TextSize;
    fn add(self, rhs: TextOffset) -> Self::Output {
        if rhs.raw.is_negative() {
            self.raw - ((-rhs.raw) as u32)
        } else {
            self.raw + (rhs.raw as u32)
        }
        .try_into()
        .unwrap()
    }
}

impl<A> AddAssign<A> for TextSize
where
    TextSize: Add<A, Output = TextSize>,
{
    #[inline]
    fn add_assign(&mut self, rhs: A) {
        *self = *self + rhs
    }
}

impl<S> SubAssign<S> for TextSize
where
    TextSize: Sub<S, Output = TextSize>,
{
    #[inline]
    fn sub_assign(&mut self, rhs: S) {
        *self = *self - rhs
    }
}

impl<A> iter::Sum<A> for TextSize
where
    TextSize: Add<A, Output = TextSize>,
{
    #[inline]
    fn sum<I: Iterator<Item = A>>(iter: I) -> TextSize {
        iter.fold(0.into(), Add::add)
    }
}
