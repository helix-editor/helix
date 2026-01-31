use serde::{Deserialize, Serialize};
use smartstring::{SmartString, SmartStringMode};
use std::{
    borrow::{Borrow, Cow},
    error::Error,
    fmt::Display,
    ops::Deref,
};

use arrayvec::ArrayString;

const INLINE: usize = 28;

/// String type that stores at most 28 bytes inline.
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct StackString(ArrayString<INLINE>);

impl StackString {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        const { assert!(size_of::<Self>() == 32) }
        Self(ArrayString::new())
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    pub fn push(&mut self, c: char) {
        self.0.push(c);
    }

    #[inline]
    pub fn try_push(&mut self, c: char) -> Result<(), CapacityError> {
        self.0.try_push(c).map_err(|_| CapacityError)
    }

    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.0.push_str(s);
    }

    #[inline]
    pub fn try_push_str(&mut self, s: &str) -> Result<(), CapacityError> {
        self.0.try_push_str(s).map_err(|_| CapacityError)
    }

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Only meant to be used with compile-time known strings. For runtime known
    /// use `try_from`.
    ///
    /// # Panics
    ///
    /// Panics if `s.len()` > `28`.
    #[inline]
    #[must_use]
    pub fn from(s: &'static str) -> Self {
        assert!(s.len() <= INLINE, "`StackString` can only be used with `&str` that is at most {INLINE} bytes: {s} is too large");
        Self(ArrayString::from(s).unwrap())
    }
}

impl Deref for StackString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl Display for StackString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Borrow<str> for StackString {
    #[inline]
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for StackString {
    type Error = CapacityError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self(ArrayString::from(s).map_err(|_| CapacityError)?))
    }
}

impl AsRef<str> for StackString {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Write for StackString {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.try_push_str(s).map_err(|_| std::fmt::Error)
    }
}

#[macro_export]
macro_rules! stack_format {
    ($($arg:tt)*) => {{
        use std::fmt::Write;
        let mut s = $crate::string::StackString::new();
        match write!(&mut s, $($arg)*) {
            Ok(_) => s,
            Err(_) => panic!("stack_format! exceeded capacity of 28 bytes"),
        }
    }};
}

#[derive(Debug)]
pub struct CapacityError;
impl Error for CapacityError {}
impl Display for CapacityError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CapacityError: not enough capacity to hold string data in `ArrayString`"
        )
    }
}

impl From<StackString> for Cow<'_, str> {
    #[inline]
    fn from(s: StackString) -> Self {
        Cow::Owned(s.to_string())
    }
}

impl<T: SmartStringMode> From<StackString> for SmartString<T> {
    #[inline]
    fn from(s: StackString) -> Self {
        Self::from(s.as_str())
    }
}
