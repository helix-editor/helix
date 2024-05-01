use helix_core::Selection;
use helix_loader::{
    clipboard_file, command_histfile, file_histfile,
    persistence::{push_history, read_history, trim_history, write_history},
    search_histfile,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::view::ViewPosition;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileHistoryEntry {
    pub path: PathBuf,
    pub view_position: ViewPosition,
    pub selection: Selection,
}

impl FileHistoryEntry {
    pub fn new(path: PathBuf, view_position: ViewPosition, selection: Selection) -> Self {
        Self {
            path,
            view_position,
            selection,
        }
    }
}

pub fn push_file_history(entry: &FileHistoryEntry) {
    push_history(file_histfile(), entry)
}

pub fn read_file_history() -> Vec<FileHistoryEntry> {
    read_history(file_histfile())
}

pub fn trim_file_history(limit: usize) {
    trim_history::<FileHistoryEntry>(file_histfile(), limit)
}

pub fn push_reg_history(register: char, line: &String) {
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

pub fn trim_command_history(limit: usize) {
    trim_history::<String>(command_histfile(), limit)
}

pub fn read_search_history() -> Vec<String> {
    let mut hist = read_reg_history(search_histfile());
    hist.reverse();
    hist
}

pub fn trim_search_history(limit: usize) {
    trim_history::<String>(search_histfile(), limit)
}

pub fn write_clipboard_file(values: &Vec<String>) {
    write_history(clipboard_file(), values)
}

pub fn read_clipboard_file() -> Vec<String> {
    read_history(clipboard_file())
}
