use helix_loader::{shada_file, VERSION_AND_GIT_HASH};
use helix_view::view::ViewPosition;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    time::{SystemTime, UNIX_EPOCH},
};

// TODO: should this be non-exhaustive?
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "H")]
struct Header {
    generator: String,
    version: String,
    encoding: String,
    max_kbyte: u32,
    pid: u32,
}

// TODO: should this be non-exhaustive?
#[derive(Debug, Deserialize, Serialize)]
struct FilePosition {
    path: String,
    position: ViewPosition,
}

// TODO: should this be non-exhaustive?
#[derive(Debug, Deserialize, Serialize)]
enum EntryData {
    Header(Header),
    FilePosition(FilePosition),
}

// TODO: should this be non-exhaustive?
#[derive(Debug, Deserialize, Serialize)]
struct Entry {
    timestamp: u64,
    data: EntryData,
}

fn timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_header() -> Entry {
    Entry {
        timestamp: timestamp_now(),
        data: EntryData::Header(Header {
            generator: "helix".to_string(),
            version: VERSION_AND_GIT_HASH.to_string(),
            // TODO: is this necessary? helix doesn't seem to expose an option
            // for internal encoding like nvim does
            encoding: "utf-8".to_string(),
            max_kbyte: 100,
            pid: std::process::id(),
        }),
    }
}

pub fn write_shada_file() {
    // TODO: merge existing file if exists

    // TODO: do something about this unwrap
    let shada_file = File::create(shada_file()).unwrap();
    let mut serializer = Serializer::new(shada_file);

    let header = generate_header();

    // TODO: do something about this unwrap
    header.serialize(&mut serializer).unwrap();
}
