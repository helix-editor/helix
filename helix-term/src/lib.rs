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
use std::path::Path;

use futures_util::Future;
use ignore::DirEntry;
use url::Url;

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

/// Function used for filtering dir entries in the various file pickers.
fn filter_picker_entry(entry: &DirEntry, root: &Path, dedup_symlinks: bool) -> bool {
    // We always want to ignore the .git directory, otherwise if
    // `ignore` is turned off, we end up with a lot of noise
    // in our picker.
    if entry.file_name() == ".git" {
        return false;
    }

    // We also ignore symlinks that point inside the current directory
    // if `dedup_links` is enabled.
    if dedup_symlinks && entry.path_is_symlink() {
        return entry
            .path()
            .canonicalize()
            .ok()
            .map_or(false, |path| !path.starts_with(root));
    }

    true
}

/// Opens URL in external program.
fn open_external_url_callback(
    url: Url,
) -> impl Future<Output = Result<job::Callback, anyhow::Error>> + Send + 'static {
    let commands = open::commands(url.as_str());
    async {
        for cmd in commands {
            let mut command = tokio::process::Command::new(cmd.get_program());
            command.args(cmd.get_args());
            if command.output().await.is_ok() {
                return Ok(job::Callback::Editor(Box::new(|_| {})));
            }
        }
        Ok(job::Callback::Editor(Box::new(move |editor| {
            editor.set_error("Opening URL in external program failed")
        })))
    }
}
