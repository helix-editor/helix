//! A module for the [`Buffer`] and [`Cell`] types.

mod cell;
pub use cell::Cell;

#[allow(clippy::module_inception)]
mod buffer;
pub use buffer::Buffer;
