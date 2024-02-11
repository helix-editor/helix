use helix_loader::{
    command_histfile, file_histfile,
    persistence::{push_history, read_history},
    search_histfile,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// TODO: should this contain a ViewPosition?
#[derive(Debug, Serialize, Deserialize)]
pub struct FileHistoryEntry {
    pub path: PathBuf,
    pub anchor: usize,
    pub vertical_offset: usize,
    pub horizontal_offset: usize,
}

impl FileHistoryEntry {
    pub fn new(
        path: PathBuf,
        anchor: usize,
        vertical_offset: usize,
        horizontal_offset: usize,
    ) -> Self {
        Self {
            path,
            anchor,
            vertical_offset,
            horizontal_offset,
        }
    }
}

pub fn push_file_history(entry: FileHistoryEntry) {
    push_history(file_histfile(), entry)
}

pub fn read_file_history() -> Vec<FileHistoryEntry> {
    read_history(file_histfile())
}

pub fn push_reg_history(register: char, line: &str) {
    let filepath = match register {
        ':' => command_histfile(),
        '/' => search_histfile(),
        _ => return,
    };

    push_history(filepath, line)
}

fn read_reg_history(filepath: PathBuf) -> Vec<String> {
    read_history(filepath)
}

pub fn read_command_history() -> Vec<String> {
    let mut hist = read_reg_history(command_histfile());
    hist.reverse();
    hist
}

pub fn read_search_history() -> Vec<String> {
    let mut hist = read_reg_history(search_histfile());
    hist.reverse();
    hist
}
