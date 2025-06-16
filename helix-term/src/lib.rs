#[macro_use]
extern crate helix_view;

pub mod application;
pub mod args;
pub mod commands;
pub mod compositor;
pub mod config;
pub mod events;
pub mod health;
pub mod job;
pub mod keymap;
pub mod ui;

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use futures_util::Future;
mod handlers;

use ignore::DirEntry;
use url::Url;

#[cfg(windows)]
fn true_color() -> bool {
    true
}

#[cfg(not(windows))]
fn true_color() -> bool {
    if matches!(
        std::env::var("COLORTERM").map(|v| matches!(v.as_str(), "truecolor" | "24bit")),
        Ok(true)
    ) {
        return true;
    }

    match termini::TermInfo::from_env() {
        Ok(t) => {
            t.extended_cap("RGB").is_some()
                || t.extended_cap("Tc").is_some()
                || (t.extended_cap("setrgbf").is_some() && t.extended_cap("setrgbb").is_some())
        }
        Err(_) => false,
    }
}

fn is_binary(path: &Path, read_buffer: &mut Vec<u8>) -> io::Result<bool> {
    let content_type = File::open(path).and_then(|file| {
        // Read up to 1kb to detect the content type
        let n = file.take(1024).read_to_end(read_buffer)?;
        let content_type = content_inspector::inspect(&read_buffer[..n]);
        read_buffer.clear();
        Ok(content_type)
    })?;
    Ok(content_type.is_binary())
}

/// Function used for filtering dir entries in the various file pickers.
fn filter_picker_entry(
    entry: &DirEntry,
    root: &Path,
    dedup_symlinks: bool,
    ignore_binary_files: bool,
) -> bool {
    // We always want to ignore popular VCS directories, otherwise if
    // `ignore` is turned off, we end up with a lot of noise
    // in our picker.
    if matches!(
        entry.file_name().to_str(),
        Some(".git" | ".pijul" | ".jj" | ".hg" | ".svn")
    ) {
        return false;
    }

    // We also ignore symlinks that point inside the current directory
    // if `dedup_links` is enabled.
    if dedup_symlinks && entry.path_is_symlink() {
        return entry
            .path()
            .canonicalize()
            .ok()
            .is_some_and(|path| !path.starts_with(root));
    }

    if ignore_binary_files {
        if let Ok(is_binary) = is_binary(entry.path(), &mut Vec::new()) {
            return !is_binary;
        }
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
