#[macro_use]
extern crate helix_view;

pub mod application;
pub mod args;
pub mod commands;
pub mod compositor;
pub mod config;
pub mod health;
pub mod job;
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

pub fn set_title_from_doc(doc: &helix_view::Document) {
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::SetTitle(format!(
            "settitle hx {}",
            doc.relative_path()
                .as_deref()
                .unwrap_or(std::path::Path::new("[scratch]"))
                .to_str()
                .unwrap() //,
        ))
    )
    .unwrap();
}