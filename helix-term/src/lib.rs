#[macro_use]
extern crate helix_view;

pub mod application;
pub mod args;
pub mod commands;
pub use helix_view::compositor;
pub mod config;
pub mod health;
pub use helix_view::job;
pub mod keymap;
pub mod ui;
pub use keymap::macros::*;

#[cfg(not(windows))]
fn true_color() -> bool {
    std::env::var("COLORTERM")
        .map(|v| matches!(v.as_str(), "truecolor" | "24bit"))
        .unwrap_or(false)
}
#[cfg(windows)]
fn true_color() -> bool {
    true
}
