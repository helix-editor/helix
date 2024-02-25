use bincode::{deserialize_from, serialize_into};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader},
    path::PathBuf,
};

pub fn write_history<T: Serialize>(filepath: PathBuf, entries: &Vec<T>) {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(filepath)
        // TODO: do something about this unwrap
        .unwrap();

    for entry in entries {
        // TODO: do something about this unwrap
        serialize_into(&file, &entry).unwrap();
    }
}

pub fn push_history<T: Serialize>(filepath: PathBuf, entry: &T) {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filepath)
        // TODO: do something about this unwrap
        .unwrap();

    // TODO: do something about this unwrap
    serialize_into(file, entry).unwrap();
}

pub fn read_history<T: for<'a> Deserialize<'a>>(filepath: PathBuf) -> Vec<T> {
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
