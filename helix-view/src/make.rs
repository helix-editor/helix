use helix_lsp::lsp::{Position, Range};
use std::path::PathBuf;

// TODO(szulf): better naming cause god damn

#[derive(Debug, Default, Clone)]
pub struct Value {
    // TODO(szulf): not sure about this
    pub err_msg: String,
}

impl Value {
    pub fn new(err_msg: &str) -> Self {
        Self {
            err_msg: err_msg.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Location {
    pub path: PathBuf,
    pub range: Range,
}

// NOTE(szulf): would absolutely love to use helix-term::commands::Location
// but cannot access it from here whyyyyy
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub location: Location,
    pub value: Value,
}

impl Entry {
    pub fn new(path: PathBuf, value: Value) -> Self {
        Self {
            location: Location {
                path: path,
                range: Range::new(Position::new(2, 5), Position::new(2, 6)),
            },
            value: value,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct List {
    entries: Vec<Entry>,
}

impl List {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn into_iter(self) -> impl Iterator<Item = Entry> {
        self.entries.into_iter()
    }
}
