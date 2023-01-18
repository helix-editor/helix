#[macro_use]
extern crate helix_view;

pub mod application;
pub mod args;
pub use helix_view::commands;
pub use helix_view::compositor;
pub use helix_view::config;
pub mod health;
pub use helix_view::job;
pub use helix_view::keymap;
pub use helix_view::ui;
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
