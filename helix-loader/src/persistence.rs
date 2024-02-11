use crate::{command_histfile, file_histfile, search_histfile};
use bincode::{deserialize_from, serialize_into};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader},
    path::PathBuf,
};

// TODO: should this contain a ViewPosition?
// it would require exposing that type in a new crate, re-exporting in helix-view,
// since this crate is a dependency of helix-view
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

fn push_history<T: Serialize>(filepath: PathBuf, entry: T) {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filepath)
        // TODO: do something about this unwrap
        .unwrap();

    // TODO: do something about this unwrap
    serialize_into(file, &entry).unwrap();
}

fn read_history<T: for<'a> Deserialize<'a>>(filepath: PathBuf) -> Vec<T> {
    match File::open(filepath) {
        Ok(file) => {
            let mut read = BufReader::new(file);
            let mut entries = Vec::new();
            // TODO: more sophisticated error handling
            while let Ok(entry) = deserialize_from(&mut read) {
                entries.push(entry);
            }
            entries
        }
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Vec::new(),
            // TODO: do something about this panic
            _ => panic!(),
        },
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
