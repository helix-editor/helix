use crate::{command_histfile, file_histfile, search_histfile};
use bincode::serialize_into;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
};

// TODO: should this contain a ViewPosition?
// it would require exposing that type in a new crate, re-exporting in helix-view,
// since this crate is a dependency of helix-view
#[derive(Debug, Serialize, Deserialize)]
pub struct FileHistoryEntry {
    path: PathBuf,
    anchor: usize,
    vertical_offset: usize,
    horizontal_offset: usize,
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
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_histfile())
        // TODO: do something about this unwrap
        .unwrap();

    // TODO: do something about this unwrap
    serialize_into(file, &entry).unwrap();
}

pub fn push_history(register: char, line: &str) {
    let filepath = match register {
        ':' => command_histfile(),
        '/' => search_histfile(),
        _ => return,
    };

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filepath)
        // TODO: do something about this unwrap
        .unwrap();

    // TODO: do something about this unwrap
    writeln!(file, "{}", line).unwrap();
}

fn read_histfile(filepath: PathBuf) -> Vec<String> {
    match File::open(filepath) {
        Ok(file) => {
            BufReader::new(file)
                .lines()
                .collect::<io::Result<Vec<String>>>()
                // TODO: do something about this unwrap
                .unwrap()
        }
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Vec::new(),
            // TODO: do something about this panic
            _ => panic!(),
        },
    }
}

pub fn read_command_history() -> Vec<String> {
    read_histfile(command_histfile())
}

pub fn read_search_history() -> Vec<String> {
    read_histfile(search_histfile())
}
