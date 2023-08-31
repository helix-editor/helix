//! `helix-event` contains systems that allow (often async) communication between
//! different editor components without strongly coupling them. Currently this
//! crate only contains some smaller facilities but the intend is to add more
//! functionality in the future ( like a generic hook system)

pub use redraw::{lock_frame, redraw_requested, request_redraw, start_frame, RenderLockGuard};

mod redraw;
