use serde::Deserialize;
use std::{collections::HashMap, ops::{Deref, DerefMut}};
use helix_view::{info::Info, input::KeyEvent};
use super::keytrienode::KeyTrieNode;

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
    /// Shows the children listed by possible KeyEvents 
    /// and thier associated documentation.
    pub fn infobox(&self) -> Info {
        let mut body: Vec<(String, &str)> = Vec::with_capacity(self.len());
        for (&key_event, key_trie) in self.iter() {
            let documentation: &str = match key_trie {
                KeyTrieNode::MappableCommand(command) => {
                    if command.name() == "no_op" {
                        continue;
                    }
                    command.doc()
                },
                KeyTrieNode::KeyTrie(key_trie) => &key_trie.documentation,
                // FIX: default to a join of all command names
                // NOTE: Giving same documentation for all sequences will place all sequence keyvents together.
                // Regardless if the command sequence is different.
                KeyTrieNode::CommandSequence(_) => "[Multiple commands]",
            };
            match body.iter().position(|(_, existing_documentation)| &documentation == existing_documentation) {
                Some(position) =>  body[position].0 += &format!(", {}", &key_event.to_string()),
                None => body.push((key_event.to_string(), documentation)),
            }
        }

        Info::new(&self.documentation, &body)
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
