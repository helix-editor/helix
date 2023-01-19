use super::keytrienode::KeyTrieNode;
use helix_view::{info::Info, input::KeyEvent};
use std::{collections::HashMap, ops::{Deref, DerefMut}};
use serde::Deserialize;

/// Edges of the trie are KeyEvents and the nodes are descrbibed by KeyTrieNode
#[derive(Debug, Clone)]
pub struct KeyTrie {
    documentation: String,
    children: HashMap<KeyEvent, KeyTrieNode>,
    pub is_sticky: bool,
}

impl KeyTrie {
    pub fn new(documentation: &str, children: HashMap<KeyEvent, KeyTrieNode>) -> Self {
        Self {
            documentation: documentation.to_string(),
            children,
            is_sticky: false,
        }
    }

    // None symbolizes NotFound
    pub fn traverse(&self, key_events: &[KeyEvent]) -> Option<KeyTrieNode> {
        return _traverse(self, key_events, 0);

        fn _traverse(keytrie: &KeyTrie, key_events: &[KeyEvent], mut depth: usize) -> Option<KeyTrieNode> {
            if depth == key_events.len() {
                return Some(KeyTrieNode::KeyTrie(keytrie.clone()));
            }
            else if let Some(found_child) = keytrie.get(&key_events[depth]) {
                match found_child {
                    KeyTrieNode::KeyTrie(sub_keytrie) => {
                        depth += 1;
                        return _traverse(sub_keytrie, key_events, depth)
                    },
                    _ => return Some(found_child.clone())
                }
            }
            return None;
        }
    }

    pub fn merge_keytrie(&mut self, mut other_keytrie: Self) {
        for (other_key_event, other_child_node) in std::mem::take(&mut other_keytrie.children) {
            match other_child_node {
                KeyTrieNode::KeyTrie(other_child_key_trie) => {
                    if let Some(KeyTrieNode::KeyTrie(self_clashing_child_key_trie)) = self.children.get_mut(&other_key_event) {
                        self_clashing_child_key_trie.merge_keytrie(other_child_key_trie);
                    }
                    else {
                        self.children.insert(other_key_event, KeyTrieNode::KeyTrie(other_child_key_trie));
                    }
                }
                KeyTrieNode::MappableCommand(_) | KeyTrieNode::CommandSequence(_) => {
                    self.children.insert(other_key_event, other_child_node);
                }
            }
        }
    }

    /// Open an Info box for a given KeyTrie
    /// Shows the children as possible KeyEvents and thier associated description.
    pub fn infobox(&self) -> Info {
        let mut body: Vec<(Vec<String>, &str)> = Vec::with_capacity(self.len());
        for (&key_event, key_trie) in self.iter() {
            let documentation: &str = match key_trie {
                KeyTrieNode::MappableCommand(command) => {
                    if command.name() == "no_op" {
                        continue;
                    }
                    command.description()
                },
                KeyTrieNode::KeyTrie(key_trie) => &key_trie.documentation,
                // FIX: default to a join of all command names
                // NOTE: Giving same documentation for all sequences will place all sequence keyvents together.
                // Regardless if the command sequence is different.
                KeyTrieNode::CommandSequence(_) => "[Multiple commands]",
            };
            match body.iter().position(|(_, existing_documentation)| &documentation == existing_documentation) {
                Some(position) =>  body[position].0.push(key_event.to_string()),
                None => {
                    let mut temp_vec: Vec<String> = Vec::new();
                    temp_vec.push(key_event.to_string());
                    body.push((temp_vec, documentation))   
                },
            }
        }

        // Shortest keyevent (as string) appears first, unless is a "C-" KeyEvent
        // Those events will always be placed after the one letter KeyEvent
        let mut sorted_body = body
            .iter()
            .map(|(key_events, description)| {
                let mut temp_key_events = key_events.clone();
                temp_key_events.sort_unstable_by(|a, b| a.len().cmp(&b.len()));
                (temp_key_events, *description)
            })
            .collect::<Vec<(Vec<String>, &str)>>();
        sorted_body.sort_unstable_by(|a, b| a.0[0].to_lowercase().cmp(&b.0[0].to_lowercase()));
        // Consistently place lowercase before uppercase of the same letter.
        if sorted_body.len() > 1 {
            let mut x_index = 0;
            let mut y_index = 1;

            while y_index < sorted_body.len() {
                let x = &sorted_body[x_index].0[0];
                let y = &sorted_body[y_index].0[0];
                if x.to_lowercase() == y.to_lowercase() {
                    // Uppercase regarded as lower value.
                    if x < y {
                        let temp_holder = sorted_body[x_index].clone();
                        sorted_body[x_index] = sorted_body[y_index].clone();
                        sorted_body[y_index] = temp_holder;
                    }
                }
                x_index = y_index;
                y_index += 1;
            }
        }

        let stringified_key_events_body: Vec<(String, &str)> = sorted_body
            .iter()
            .map(|(key_events, description)| {
                let key_events_string: String = key_events.iter().fold(String::new(), |mut acc, key_event| {
                    if !acc.is_empty() { acc.push_str(", "); }
                    acc.push_str(key_event);
                    acc
                });
                (key_events_string, *description)
            })
            .collect();

        Info::new(&self.documentation, &stringified_key_events_body)
    }
}

impl Default for KeyTrie {
    fn default() -> Self {
        Self::new("", HashMap::new())
    }
}

impl PartialEq for KeyTrie {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
    }
}

/// Returns the children of the KeyTrie
impl Deref for KeyTrie {
    type Target = HashMap<KeyEvent, KeyTrieNode>;

    fn deref(&self) -> &Self::Target {
        &self.children
    }
}

/// Returns the children of the KeyTrie
impl DerefMut for KeyTrie {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.children
    }
}

impl<'de> Deserialize<'de> for KeyTrie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
         Ok(Self {
            children: HashMap::<KeyEvent, KeyTrieNode>::deserialize(deserializer)?,
            ..Default::default()
        })
    }
}
