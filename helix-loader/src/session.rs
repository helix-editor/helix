use crate::{command_histfile, search_histfile};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
};

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
