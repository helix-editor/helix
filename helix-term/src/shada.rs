use bincode::{encode_into_std_write, Decode, Encode};
use helix_loader::{shada_file, VERSION_AND_GIT_HASH};
// use helix_view::view::ViewPosition;
use std::{
    fs::File,
    time::{SystemTime, UNIX_EPOCH},
};

// TODO: should this be non-exhaustive?
#[derive(Debug, Encode, Decode)]
struct Header {
    generator: String,
    version: String,
    max_kbyte: u32,
    pid: u32,
}

// TODO: should this be non-exhaustive?
#[derive(Debug, Encode, Decode)]
struct FilePosition {
    path: String,
    // position: ViewPosition,
}

// TODO: should this be non-exhaustive?
#[derive(Debug, Encode, Decode)]
enum EntryData {
    Header(Header),
    FilePosition(FilePosition),
}

// TODO: should this be non-exhaustive?
#[derive(Debug, Encode, Decode)]
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
            max_kbyte: 100,
            pid: std::process::id(),
        }),
    }
}

pub fn write_shada_file() {
    // TODO: merge existing file if exists

    // TODO: do something about this unwrap
    let mut shada_file = File::create(shada_file()).unwrap();

    let header = generate_header();

    // TODO: do something about this unwrap
    encode_into_std_write(&header, &mut shada_file, bincode::config::standard()).unwrap();
}
