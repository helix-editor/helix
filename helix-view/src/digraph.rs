use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trie implementation for storing and searching input
/// strings -> unicode characters defined by the user.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct DigraphStore {
    head: DigraphNode,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Error {
    EmptyInput,
    DuplicateEntry { current: String, existing: String },
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct DigraphNode {
    output: Option<UnicodeOutput>,
    children: Option<HashMap<char, DigraphNode>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct UnicodeOutput {
    pub output: String,
    pub description: Option<String>,
}

impl DigraphStore {
    /// Inserts a new unicode string into the store
    pub fn insert(&mut self, input_seq: &str, unicode: UnicodeOutput) -> Result<(), Error> {
        if input_seq.len() <= 0 {
            return Err(Error::EmptyInput);
        }

        self.head.insert(&input_seq, unicode)
    }

    /// Attempts to retrieve a stored unicode string if it exists
    pub fn get(&self, exact_seq: &str) -> Option<&UnicodeOutput> {
        self.head
            .get(exact_seq)
            .map(|n| n.output.as_ref())
            .flatten()
    }

    /// Returns an iterator of closest matches to the input string
    pub fn search(&self, input_seq: &str) -> impl Iterator<Item = &UnicodeOutput> {
        self.head
            .get(input_seq)
            .into_iter()
            .map(|x| x.iter())
            .flatten()
    }
}

impl DigraphNode {
    fn insert(&mut self, input_seq: &str, unicode: UnicodeOutput) -> Result<(), Error> {
        // see if we found the spot to insert our unicode
        if input_seq.len() == 0 {
            if let Some(existing) = &self.output {
                return Err(Error::DuplicateEntry {
                    existing: existing.output.clone(),
                    current: unicode.output.clone(),
                });
            } else {
                self.output = Some(unicode);
                return Ok(());
            }
        }

        // continue searching
        let node = self
            .children
            .get_or_insert(Default::default())
            .entry(input_seq.chars().next().unwrap())
            .or_default();

        node.insert(&input_seq[1..], unicode)
    }

    fn get(&self, exact_seq: &str) -> Option<&Self> {
        if exact_seq.len() == 0 {
            return Some(&self);
        }

        self.children
            .as_ref()
            .and_then(|cm| cm.get(&exact_seq.chars().next().unwrap()))
            .and_then(|node| node.get(&exact_seq[1..]))
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = &UnicodeOutput> + 'a {
        DigraphIter::new(&self)
    }
}

pub struct DigraphIter<'a, 'b>
where
    'a: 'b,
{
    element_iter: Box<dyn Iterator<Item = &'a UnicodeOutput> + 'b>,
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
            element_iter: Box::new(node.output.iter().chain(Self::get_child_elements(&node))),
            node_iter: Box::new(Self::get_child_nodes(&node)),
        }
    }

    fn get_child_elements(node: &'a DigraphNode) -> impl Iterator<Item = &'a UnicodeOutput> + 'b {
        node.children
            .iter()
            .flat_map(|hm| hm.iter())
            .map(|(_, node)| node.output.as_ref())
            .filter_map(|b| b)
    }

    fn get_child_nodes(node: &'a DigraphNode) -> impl Iterator<Item = &'a DigraphNode> + 'b {
        node.children
            .iter()
            .map(|x| x.iter().map(|(_, node)| node))
            .flatten()
    }
}
impl<'a, 'b> Iterator for DigraphIter<'a, 'b>
where
    'a: 'b,
{
    type Item = &'a UnicodeOutput;

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
                        Box::new(new_nodes.chain(Self::get_child_nodes(&node)));
                    std::mem::swap(&mut new_nodes, &mut self.node_iter);

                    self.element_iter = Box::new(Self::get_child_elements(&node));
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
            UnicodeOutput {
                output: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            UnicodeOutput {
                output: "deadbeef".into(),
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
                .output
                .clone(),
            "testbug".to_string()
        );
    }

    #[test]
    fn digraph_insert_and_get() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abc",
            UnicodeOutput {
                output: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            UnicodeOutput {
                output: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            dg.get("abc").map(|x| x.output.clone()),
            Some("testbug".to_string())
        );
        assert_eq!(
            dg.get("abd").map(|x| x.output.clone()),
            Some("deadbeef".to_string())
        );
        assert_eq!(dg.get("abe").map(|x| x.output.clone()), None);
    }

    #[test]
    fn digraph_node_iter() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abc",
            UnicodeOutput {
                output: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            UnicodeOutput {
                output: "deadbeef".into(),
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
            UnicodeOutput {
                output: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            UnicodeOutput {
                output: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();
        dg.insert(
            "azz",
            UnicodeOutput {
                output: "qwerty".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(dg.search("ab").count(), 2);
        assert_eq!(dg.search("az").next().unwrap().output, "qwerty");
    }

    #[test]
    fn digraph_search_breadth() {
        let mut dg = DigraphStore::default();
        dg.insert(
            "abccccc",
            UnicodeOutput {
                output: "testbug".into(),
                ..Default::default()
            },
        )
        .unwrap();

        dg.insert(
            "abd",
            UnicodeOutput {
                output: "deadbeef".into(),
                ..Default::default()
            },
        )
        .unwrap();
        dg.insert(
            "abee",
            UnicodeOutput {
                output: "qwerty".into(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(dg.search("ab").count(), 3);
        assert_eq!(dg.search("ab").next().unwrap().output, "deadbeef");
    }
}
