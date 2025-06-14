//! Utilities for working with strings and specialized string types.

use std::{
    alloc,
    borrow::{Borrow, Cow},
    fmt, hash,
    mem::{size_of, ManuallyDrop},
    ptr::{self, NonNull},
    slice, str,
};

/// A very very small owned string type.
///
/// This type is like a `Box<str>` and is similarly two `usize`s large. It can only fit strings
/// with a byte length smaller than 256. On 64-bit machines this type stores up to 15 bytes inline
/// (7 bytes on 32-bit machines). One byte is used to store the length. For strings short enough
/// to be stored inline, the remaining 15 (or 7) bytes store the content inline. Otherwise the
/// second `usize` of memory is a thin pointer to the string content.
///
/// Unlike `Box<str>` this type is not null-pointer optimized.
#[repr(C)]
pub struct TinyBoxedStr {
    len: u8,
    prefix: [u8; Self::PREFIX_LEN],
    trailing: TinyBoxedStrTrailing,
}

#[repr(C)]
union TinyBoxedStrTrailing {
    suffix: [u8; TinyBoxedStr::SUFFIX_LEN],
    ptr: ManuallyDrop<NonNull<u8>>,
}

impl TinyBoxedStr {
    // 1 usize minus the byte to store the length.
    const PREFIX_LEN: usize = size_of::<usize>() - size_of::<u8>();
    // The other `usize` is a pointer or the end parts of an inline string.
    const SUFFIX_LEN: usize = size_of::<usize>();
    // ... for a grand total of 15 bytes for 64-bit machines or 7 for 32-bit.
    const INLINE_LEN: u8 = (Self::PREFIX_LEN + Self::SUFFIX_LEN) as u8;

    pub const MAX_LEN: usize = u8::MAX as usize;

    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        let ptr = if self.len <= Self::INLINE_LEN {
            let ptr = ptr::from_ref(self);
            unsafe { ptr::addr_of!((*ptr).prefix) }.cast()
        } else {
            unsafe { self.trailing.ptr }.as_ptr()
        };
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Exposes the bytes as a mutable slice.
    ///
    /// When a string is short enough to be inline, this slice points to the `prefix` and `suffix`
    /// parts of the struct. Otherwise the slice wraps the pointer to the allocation.
    ///
    /// SAFETY: As such, if the string is allocated then it is the caller's responsibility to
    /// ensure that any modifications made to `&s.as_bytes_mut[..Self::PREFIX_LEN]` are written
    /// to `s.prefix` as well if the string is allocated.
    ///
    /// SAFETY: It is also the caller's responsibility to ensure that edits to the bytes do not
    /// make the bytes invalid UTF-8.
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        let ptr = if self.len <= Self::INLINE_LEN {
            let ptr = ptr::from_mut(self);
            unsafe { ptr::addr_of_mut!((*ptr).prefix) }.cast()
        } else {
            unsafe { self.trailing.ptr }.as_ptr()
        };
        unsafe { slice::from_raw_parts_mut(ptr, self.len()) }
    }

    fn layout(len: u8) -> alloc::Layout {
        alloc::Layout::array::<u8>(len as usize)
            .expect("a valid layout for an array")
            .pad_to_align()
    }

    /// Creates a new `TinyBoxedStr` of the given length with all bytes zeroed.
    ///
    /// While this is used to create uninitialized strings which are later filled, note that the
    /// zero byte is valid UTF-8 so the zeroed representation is always valid.
    fn zeroed(len: u8) -> Self {
        let trailing = if len <= Self::INLINE_LEN {
            TinyBoxedStrTrailing {
                suffix: [0; Self::SUFFIX_LEN],
            }
        } else {
            let layout = Self::layout(len);
            let nullable = unsafe { alloc::alloc_zeroed(layout) };
            let Some(ptr) = NonNull::new(nullable) else {
                alloc::handle_alloc_error(layout);
            };
            TinyBoxedStrTrailing {
                ptr: ManuallyDrop::new(ptr),
            }
        };
        Self {
            len,
            prefix: [0; Self::PREFIX_LEN],
            trailing,
        }
    }
}

#[derive(Debug)]
pub struct TooLongError;

impl fmt::Display for TooLongError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("string was too long to be stored as a `TinyBoxedStr` (max 256 bytes)")
    }
}

impl std::error::Error for TooLongError {}

impl TryFrom<&str> for TinyBoxedStr {
    type Error = TooLongError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.len() > Self::MAX_LEN {
            return Err(TooLongError);
        }

        let mut this = Self::zeroed(s.len() as u8);
        // SAFETY: if `s` is valid UTF-8, `this`'s bytes will be valid UTF-8.
        unsafe { this.as_bytes_mut() }.copy_from_slice(s.as_bytes());
        if this.len > Self::INLINE_LEN {
            this.prefix
                .copy_from_slice(&s.as_bytes()[..Self::PREFIX_LEN]);
        }
        Ok(this)
    }
}

// NOTE: converting from a `String` to a `TinyBoxedStr` is cheap when the string's length is equal
// to its capacity.
impl TryFrom<String> for TinyBoxedStr {
    type Error = TooLongError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        // Inline strings must be cloned. It's a constant number of bytes to copy though.
        if s.len() <= Self::INLINE_LEN as usize {
            return s.as_str().try_into();
        }

        // Otherwise we can sometimes steal the `String`'s allocation if the string is allocated
        // exactly (i.e. `s.len() == s.capacity()`). A `Box<str>` is defined as being allocated
        // exactly so we first convert to `Box<str>` (which will reallocate if the capacity is not
        // the same as the length) and then steal its pointer.

        if s.len() > Self::MAX_LEN {
            return Err(TooLongError);
        }

        let len = s.len() as u8;
        let mut prefix = [0; Self::PREFIX_LEN];
        prefix.copy_from_slice(&s.as_bytes()[..Self::PREFIX_LEN]);
        let ptr = Box::into_raw(s.into_boxed_str()).cast::<u8>();
        // SAFETY: `Box::into_raw` docs guarantee non-null.
        let ptr = ManuallyDrop::new(unsafe { NonNull::new_unchecked(ptr) });
        let trailing = TinyBoxedStrTrailing { ptr };

        Ok(Self {
            len,
            prefix,
            trailing,
        })
    }
}

impl TryFrom<Cow<'_, str>> for TinyBoxedStr {
    type Error = TooLongError;

    fn try_from(s: Cow<'_, str>) -> Result<Self, Self::Error> {
        match s {
            Cow::Borrowed(s) => s.try_into(),
            Cow::Owned(s) => s.try_into(),
        }
    }
}

impl TryFrom<ropey::RopeSlice<'_>> for TinyBoxedStr {
    type Error = TooLongError;

    fn try_from(slice: ropey::RopeSlice<'_>) -> Result<Self, Self::Error> {
        // `impl From<RopeSlice> for String` uses `String::with_capacity` so we can reuse its
        // allocation whenever it allocates `slice.len_bytes()`.
        let s: Cow<str> = slice.into();
        s.try_into()
    }
}

impl Drop for TinyBoxedStr {
    fn drop(&mut self) {
        if self.len > Self::INLINE_LEN {
            let ptr = unsafe { self.trailing.ptr }.as_ptr();
            let layout = Self::layout(self.len);
            unsafe { alloc::dealloc(ptr, layout) }
        }
    }
}

impl Clone for TinyBoxedStr {
    fn clone(&self) -> Self {
        let mut this = Self::zeroed(self.len);
        // SAFETY: if `self` is valid UTF-8 then `this` will be too.
        unsafe { this.as_bytes_mut() }.copy_from_slice(self.as_bytes());
        if this.len > Self::INLINE_LEN {
            this.prefix
                .copy_from_slice(&self.as_bytes()[..Self::PREFIX_LEN]);
        }
        this
    }
}

impl Default for TinyBoxedStr {
    fn default() -> Self {
        Self::zeroed(0)
    }
}

impl AsRef<str> for TinyBoxedStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for TinyBoxedStr {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

// NOTE: this could be specialized to optimize the number of comparison operations. We could cast
// the first `usize` of memory together to do a single comparison (and same for the suffixes).
// This optimization would only matter if we compared these strings very frequently however.
impl PartialEq for TinyBoxedStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for TinyBoxedStr {}

impl PartialEq<str> for TinyBoxedStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl hash::Hash for TinyBoxedStr {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl fmt::Debug for TinyBoxedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Display for TinyBoxedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

unsafe impl Send for TinyBoxedStr {}
unsafe impl Sync for TinyBoxedStr {}
