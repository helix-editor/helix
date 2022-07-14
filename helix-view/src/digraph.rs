use anyhow::Result;
use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::collections::HashMap;

// Errors
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Error {
    EmptyInput(String),
    DuplicateEntry {
        seq: String,
        current: String,
        existing: String,
    },
    Custom(String),
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Custom(msg.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::EmptyInput(s) => {
                f.write_str(&format!("No symbols were given for key sequence {}", s))
            }
            Error::DuplicateEntry {
                seq,
                current,
                existing,
            } => f.write_str(&format!(
                "Attempted to bind {} to symbols ({}) when already bound to ({})",
                seq, current, existing
            )),
            Error::Custom(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

/// Trie implementation for storing and searching input
/// strings -> unicode characters defined by the user.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct DigraphStore {
    head: DigraphNode,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct DigraphNode {
    output: Option<FullDigraphEntry>,
    children: Option<HashMap<char, DigraphNode>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigraphEntry {
    pub symbols: String,
    pub description: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct FullDigraphEntry {
    pub sequence: String,
    pub symbols: String,
    pub description: Option<String>,
}

impl<'de> Deserialize<'de> for DigraphStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum EntryDef {
            Full(DigraphEntry),
            Symbols(String),
        }

        let mut store = Self::default();
        HashMap::<String, EntryDef>::deserialize(deserializer)?
            .into_iter()
            .map(|(k, d)| match d {
                EntryDef::Symbols(symbols) => (
                    k,
                    DigraphEntry {
                        symbols,
                        description: None,
                    },
                ),
                EntryDef::Full(entry) => (k, entry),
            })
            .try_for_each(|(k, v)| store.insert(&k, v))
            .map_err(serde::de::Error::custom)?;

        Ok(store)
    }
}

impl Serialize for DigraphStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut m = serializer.serialize_map(None)?;

        self.search("").try_for_each(|entry| {
            m.serialize_entry(
                &entry.sequence,
                &DigraphEntry {
                    symbols: entry.symbols.clone(),
                    description: entry.description.clone(),
                },
            )
        })?;
        m.end()
    }
}

/// A Store of input -> unicode strings that can be quickly looked up and
/// searched.
impl DigraphStore {
    /// Inserts a new unicode string into the store
    pub fn insert(&mut self, input_seq: &str, entry: DigraphEntry) -> Result<(), Error> {
        if input_seq.is_empty() {
            return Err(Error::EmptyInput(input_seq.to_string()));
        }

        self.head.insert(
            input_seq,
            FullDigraphEntry {
                sequence: input_seq.to_string(),
                symbols: entry.symbols,
                description: entry.description,
            },
        )
    }

    /// Attempts to retrieve a stored unicode string if it exists
    pub fn get(&self, exact_seq: &str) -> Option<&FullDigraphEntry> {
        self.head.get(exact_seq).and_then(|n| n.output.as_ref())
    }

    /// Returns an iterator of closest matches to the input string
    pub fn search(&self, input_seq: &str) -> impl Iterator<Item = &FullDigraphEntry> {
        self.head.get(input_seq).into_iter().flat_map(|x| x.iter())
    }
}

impl DigraphNode {
    fn insert(&mut self, input_seq: &str, entry: FullDigraphEntry) -> Result<(), Error> {
        // see if we found the spot to insert our unicode
        if input_seq.is_empty() {
            if let Some(existing) = &self.output {
                return Err(Error::DuplicateEntry {
                    seq: entry.sequence,
                    existing: existing.symbols.clone(),
                    current: entry.symbols,
                });
            } else {
                self.output = Some(entry);
                return Ok(());
            }
        }

        // continue searching
        let node = self
            .children
            .get_or_insert(Default::default())
            .entry(input_seq.chars().next().unwrap())
            .or_default();

        node.insert(&input_seq[1..], entry)
    }

    fn get(&self, exact_seq: &str) -> Option<&Self> {
        if exact_seq.is_empty() {
            return Some(self);
        }

        self.children
            .as_ref()
            .and_then(|cm| cm.get(&exact_seq.chars().next().unwrap()))
            .and_then(|node| node.get(&exact_seq[1..]))
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = &FullDigraphEntry> + 'a {
        DigraphIter::new(self)
    }
}

pub struct DigraphIter<'a, 'b>
where
    'a: 'b,
{
    element_iter: Box<dyn Iterator<Item = &'a FullDigraphEntry> + 'b>,
    node_iter: Box<dyn Iterator<Item = &'a DigraphNode> + 'b>,
}

impl<'a, 'b> DigraphIter<'a, 'b>
where
    'a: 'b,
{
    fn new(node: &'a DigraphNode) -> Self {
        // do a lazy breadth-first search by keeping track of the next 'rung' of
        // elements to produce, and the next 'rung' of nodes to refill the element
        // iterator when empty
        Self {
            element_iter: Box::new(node.output.iter().chain(Self::get_child_elements(node))),
            node_iter: Box::new(Self::get_child_nodes(node)),
        }
    }

    fn get_child_elements(
        node: &'a DigraphNode,
    ) -> impl Iterator<Item = &'a FullDigraphEntry> + 'b {
        node.children
            .iter()
            .flat_map(|hm| hm.iter())
            .flat_map(|(_, node)| node.output.as_ref())
    }

    fn get_child_nodes(node: &'a DigraphNode) -> impl Iterator<Item = &'a DigraphNode> + 'b {
        node.children
            .iter()
            .flat_map(|x| x.iter().map(|(_, node)| node))
    }
}
impl<'a, 'b> Iterator for DigraphIter<'a, 'b>
where
    'a: 'b,
{
    type Item = &'a FullDigraphEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(e) = self.element_iter.next() {
                return Some(e);
            }

            // We ran out of elements, fetch more by traversing the next rung of nodes
            match self.node_iter.next() {
                Some(node) => {
                    // todo: figure out a better way to update self's nodes
                    let mut new_nodes: Box<dyn Iterator<Item = &DigraphNode>> =
                        Box::new(std::iter::empty());
                    std::mem::swap(&mut new_nodes, &mut self.node_iter);
                    let mut new_nodes: Box<dyn Iterator<Item = &DigraphNode>> =
                        Box::new(new_nodes.chain(Self::get_child_nodes(node)));
                    std::mem::swap(&mut new_nodes, &mut self.node_iter);

                    self.element_iter = Box::new(Self::get_child_elements(node));
                }
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digraph_insert() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abc",
            DigraphEntry {
                symbols: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            DigraphEntry {
                symbols: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            dg.head
                .children
                .as_ref()
                .unwrap()
                .get(&'a')
                .unwrap()
                .children
                .as_ref()
                .unwrap()
                .get(&'b')
                .unwrap()
                .children
                .as_ref()
                .unwrap()
                .get(&'c')
                .unwrap()
                .output
                .clone()
                .unwrap()
                .symbols
                .clone(),
            "testbug".to_string()
        );
    }

    #[test]
    fn digraph_insert_and_get() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abc",
            DigraphEntry {
                symbols: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            DigraphEntry {
                symbols: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            dg.get("abc").map(|x| x.symbols.clone()),
            Some("testbug".to_string())
        );
        assert_eq!(
            dg.get("abd").map(|x| x.symbols.clone()),
            Some("deadbeef".to_string())
        );
        assert_eq!(dg.get("abe").map(|x| x.symbols.clone()), None);
    }

    #[test]
    fn digraph_node_iter() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abc",
            DigraphEntry {
                symbols: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            DigraphEntry {
                symbols: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(dg.head.iter().count(), 2);
    }

    #[test]
    fn digraph_search() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abc",
            DigraphEntry {
                symbols: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            DigraphEntry {
                symbols: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();
        dg.insert(
            "azz",
            DigraphEntry {
                symbols: "qwerty".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(dg.search("ab").count(), 2);
        assert_eq!(dg.search("az").next().unwrap().symbols, "qwerty");
    }

    #[test]
    fn digraph_search_breadth() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abccccc",
            DigraphEntry {
                symbols: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            DigraphEntry {
                symbols: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();
        dg.insert(
            "abee",
            DigraphEntry {
                symbols: "qwerty".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(dg.search("ab").count(), 3);
        assert_eq!(dg.search("ab").next().unwrap().symbols, "deadbeef");
    }
}
