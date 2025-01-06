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
        .truncate(true)
        .open(filepath)
        .unwrap();

    for entry in entries {
        serialize_into(&file, &entry).unwrap();
    }
}

pub fn push_history<T: Serialize>(filepath: PathBuf, entry: &T) {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filepath)
        .unwrap();

    serialize_into(file, entry).unwrap();
}

pub fn read_history<T: for<'a> Deserialize<'a>>(filepath: &PathBuf) -> Vec<T> {
    match File::open(filepath) {
        Ok(file) => {
            let mut read = BufReader::new(file);
            let mut entries = Vec::new();
            // FIXME: Can we do better error handling here? It's unfortunate that bincode doesn't
            // distinguish an empty reader from an actual error.
            //
            // Perhaps we could use the underlying bufreader to check for emptiness in the while
            // condition, then we could know any errors from bincode should be surfaced or logged.
            // BufRead has a method `has_data_left` that would work for this, but at the time of
            // writing it is nightly-only and experimental :(
            while let Ok(entry) = deserialize_from(&mut read) {
                entries.push(entry);
            }
            entries
        }
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Vec::new(),
            // Going through the potential errors listed from the docs:
            // - `InvalidInput` can't happen since we aren't setting options
            // - `AlreadyExists` can't happen since we aren't setting `create_new`
            // - `PermissionDenied` could happen if someone really borked their file permissions
            //   in `~/.local`, but helix already panics in that case, and I think a panic is
            //   acceptable.
            _ => unreachable!(),
        },
    }
}

pub fn trim_history<T: Clone + Serialize + for<'a> Deserialize<'a>>(
    filepath: PathBuf,
    limit: usize,
) {
    let history: Vec<T> = read_history(&filepath);
    if history.len() > limit {
        let trim_start = history.len() - limit;
        let trimmed_history = history[trim_start..].to_vec();
        write_history(filepath, &trimmed_history);
    }
}
