use super::keytrienode::KeyTrieNode;
use helix_view::{info::Info, input::KeyEvent};
use std::{collections::HashMap, ops::{Deref, DerefMut}, cmp::Ordering};
use serde::Deserialize;

/// Edges of the trie are KeyEvents and the nodes are descrbibed by KeyTrieNode
#[derive(Debug, Clone)]
pub struct KeyTrie {
    documentation: String,
    /// Used for pre-defined order in infoboxes, values represent the index of the key tries children.
    child_order: HashMap<KeyEvent, usize>,
    children: Vec<KeyTrieNode>,
    pub is_sticky: bool,
}

impl KeyTrie {
    pub fn new(documentation: &str, child_order: HashMap<KeyEvent, usize>, children: Vec<KeyTrieNode>) -> Self {
        Self {
            documentation: documentation.to_string(),
            child_order,
            children,
            is_sticky: false,
        }
    }

    pub fn get_child_order(&self) -> &HashMap<KeyEvent, usize> {
        &self.child_order
    }

    pub fn get_children(&self) -> &Vec<KeyTrieNode> {
        &self.children
    }

    // None symbolizes NotFound
    pub fn traverse(&self, key_events: &[KeyEvent]) -> Option<KeyTrieNode> {
        return _traverse(self, key_events, 0);

        fn _traverse(keytrie: &KeyTrie, key_events: &[KeyEvent], mut depth: usize) -> Option<KeyTrieNode> {
            if depth == key_events.len() {
                return Some(KeyTrieNode::KeyTrie(keytrie.clone()));
            }
            else if let Some(found_index) = keytrie.child_order.get(&key_events[depth]) {               
                match &keytrie.children[*found_index] {
                    KeyTrieNode::KeyTrie(sub_keytrie) => {
                        depth += 1;
                        return _traverse(sub_keytrie, key_events, depth)
                    },
                    _found_child => return Some(_found_child.clone())
                }
            }
            return None;
        }
    }

    pub fn merge_keytrie(&mut self, mut other_keytrie: Self) {
        for (other_key_event, other_index) in other_keytrie.get_child_order() {
            let other_child_keytrie_node = &other_keytrie.get_children()[*other_index];
            match other_child_keytrie_node {
                KeyTrieNode::KeyTrie(ref other_child_keytrie) => {
                    if let Some(self_index) = self.child_order.get(&other_key_event) {
                        if let KeyTrieNode::KeyTrie(ref mut self_clashing_child_key_trie) = self.children[*self_index] {
                            self_clashing_child_key_trie.merge_keytrie(other_child_keytrie.clone());
                        }
                    }
                    else {
                        self.child_order.insert(*other_key_event, self.children.len());
                        self.children.push(KeyTrieNode::KeyTrie(other_child_keytrie.clone()));
                    }
                }
                KeyTrieNode::MappableCommand(_) | KeyTrieNode::CommandSequence(_) => {
                    if let Some(existing_index) = self.child_order.get(other_key_event) {
                        self.children[*existing_index] = other_child_keytrie_node.clone();
                    }
                    else {
                        self.child_order.insert(*other_key_event, self.children.len());
                        self.children.push(other_child_keytrie_node.clone());
                    }
                }
            }
        }
    }
    
    /// Open an Info box for a given KeyTrie
    /// Shows the children as possible KeyEvents and thier associated description.
    pub fn infobox(&self) -> Info {
        let mut body: InfoBoxBody = Vec::with_capacity(self.children.len());
        let mut key_event_order = Vec::with_capacity(self.children.len());
        // child_order and children is of same length
        unsafe { key_event_order.set_len(self.children.len()); }
        for (key_event, index) in &self.child_order {
            key_event_order[*index] = key_event.clone();
        }

        for (index, key_trie) in self.children.iter().enumerate() {
            let documentation: &str = match key_trie {
                KeyTrieNode::MappableCommand(ref command) => {
                    if command.name() == "no_op" {
                        continue;
                    }
                    command.description()
                },
                KeyTrieNode::KeyTrie(ref key_trie) => &key_trie.documentation,
                // FIX: default to a join of all command names
                // NOTE: Giving same documentation for all sequences will place all sequence keyvents together.
                // Regardless if the command sequence is different.
                KeyTrieNode::CommandSequence(_) => "[Multiple commands]",
            };
            let key_event = key_event_order[index];
            match body.iter().position(|(_, existing_documentation)| &documentation == existing_documentation) {
                Some(position) =>  body[position].0.push(key_event.to_string()),
                None => {
                    body.push((vec![key_event.to_string()], documentation))   
                },
            }
        }

        // Shortest keyevent (as string) appears first, unless is a "C-" KeyEvent
        // Those events will always be placed after the one letter KeyEvent
        for (key_events, _) in body.iter_mut() {
            key_events.sort_unstable_by(|a, b| {
                if a.len() == 1 { return Ordering::Less }
                if b.len() > a.len() && b.starts_with("C-") {
                    return Ordering::Greater
                }
                a.len().cmp(&b.len())
            });
        }

        // TODO: conditional sort added here by calling infobox_sort(body)
        let stringified_key_events_body: Vec<(String, &str)> = body
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
        Self::new("", HashMap::new(), Vec::new())
    }
}

impl PartialEq for KeyTrie {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
    }
}

impl<'de> Deserialize<'de> for KeyTrie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // NOTE: no assumption of pre-defined order in config
        let child_collection = HashMap::<KeyEvent, KeyTrieNode>::deserialize(deserializer)?;
        let mut child_order = HashMap::<KeyEvent, usize>::new();
        let mut children = Vec::new();
        for (key_event, keytrie_node) in child_collection {
            child_order.insert(key_event, children.len());
            children.push(keytrie_node);
        }

         Ok(Self {
            child_order,
            children,
            ..Default::default()
        })
    }
}

type InfoBoxBody<'a> = Vec<(Vec<String>, &'a str)>;
fn infobox_sort(mut body: InfoBoxBody) -> InfoBoxBody {
    body.sort_unstable_by(|a, b| a.0[0].to_lowercase().cmp(&b.0[0].to_lowercase()));
    // Consistently place lowercase before uppercase of the same letter.
    if body.len() > 1 {
        let mut x_index = 0;
        let mut y_index = 1;

        while y_index < body.len() {
            let x = &body[x_index].0[0];
            let y = &body[y_index].0[0];
            if x.to_lowercase() == y.to_lowercase() {
                // Uppercase regarded as lower value.
                if x < y {
                    let temp_holder = body[x_index].clone();
                    body[x_index] = body[y_index].clone();
                    body[y_index] = temp_holder;
                }
            }
            x_index = y_index;
            y_index += 1;
        }
    }
    body
}
