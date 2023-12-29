use helix_loader::command_histfile;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Lines, Write},
    path::PathBuf,
};

pub fn push_history(register: char, line: &str) {
    let filepath = match register {
        ':' => command_histfile(),
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

fn read_histfile(filepath: PathBuf) -> Lines<BufReader<File>> {
    // TODO: do something about this unwrap
    BufReader::new(File::open(filepath).unwrap()).lines()
}

pub fn read_command_history() -> Vec<String> {
    read_histfile(command_histfile())
        .collect::<io::Result<Vec<String>>>()
        // TODO: do something about this unwrap
        .unwrap()
}
