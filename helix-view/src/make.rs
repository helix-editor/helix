use helix_core::diagnostic::Severity;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub location: Location,
    pub msg: String,
    pub severity: Severity,
}

impl Entry {
    pub fn new(location: Location, msg: String, severity: Severity) -> Self {
        Self {
            location: location,
            msg: msg,
            severity: severity,
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

    pub fn set(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Index<usize> for List {
    type Output = Entry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for List {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl IntoIterator for List {
    type Item = Entry;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

// TODO(szulf): dont know if this is the right way to iterate over this collection without
// consuming it
impl<'a> IntoIterator for &'a List {
    type Item = &'a Entry;
    type IntoIter = std::slice::Iter<'a, Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormatType {
    Rust,
    Gcc,
    Clang,
    Msvc,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Command {
    pub command: String,
    pub format_type: FormatType,
}
