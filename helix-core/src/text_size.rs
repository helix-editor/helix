//! A fork of text_size from rust-analyzer

//! Newtypes for working with text sizes/ranges in a more type-safe manner.
//!
//! This library can help with two things:
//!   * Reducing storage requirements for offsets and ranges, under the
//!     assumption that 32 bits is enough.
//!   * Providing standard vocabulary types for applications where text ranges
//!     are pervasive.
//!
//! However, you should not use this library simply because you work with
//! strings. In the overwhelming majority of cases, using `usize` and
//! `std::ops::Range<usize>` is better. In particular, if you are publishing a
//! library, using only std types in the interface would make it more
//! interoperable. Similarly, if you are writing something like a lexer, which
//! produces, but does not *store* text ranges, then sticking to `usize` would
//! be better.
//!
//! Minimal Supported Rust Version: latest stable./ scipt doctests

// scipt doctests
#[cfg(not(doctest))]
mod range;
#[cfg(not(doctest))]
mod traits;

pub use range::TextRange;
pub type TextSize = u32;

#[cfg(target_pointer_width = "16")]
compile_error!("text-size assumes usize >= u32 and does not work on 16-bit targets");
