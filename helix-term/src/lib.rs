#[macro_use]
extern crate helix_view;

pub mod application;
pub mod args;
pub mod commands;
pub mod compositor;
pub mod config;
pub mod job;
pub mod keymap;
pub mod ui;

fn true_color() -> bool {
    std::env::var("COLORTERM")
        .map(|v| matches!(v.as_str(), "truecolor" | "24bit"))
        .unwrap_or(false)
}
