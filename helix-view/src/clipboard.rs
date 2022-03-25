// Implementation reference: https://github.com/neovim/neovim/blob/f2906a4669a2eef6d7bf86a29648793d63c98949/runtime/autoload/provider/clipboard.vim#L68-L152

mod config;
mod provider;

use anyhow::Result;

pub use config::ClipboardConfig;

pub enum ClipboardType {
    Clipboard,
    Selection,
}

pub trait ClipboardProvider: std::fmt::Debug + std::fmt::Display {
    fn get_contents(&self, clipboard_type: ClipboardType) -> Result<String>;
    fn set_contents(&mut self, contents: String, clipboard_type: ClipboardType) -> Result<()>;
}
