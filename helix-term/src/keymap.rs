pub mod keymaps;
pub mod default;
pub mod macros;
pub mod tests;
pub mod keytrienode;
pub mod keytrie;

use crate::{
    commands::MappableCommand,
    keymap::{
        keytrie::KeyTrie,
        keytrienode::KeyTrieNode
    }
};
use std::{collections::HashMap, ops::{Deref, DerefMut}};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
/// KeyTrie starting point.
pub struct Keymap {
    pub root_node: KeyTrie
}

pub type CommandList = HashMap<String, Vec<String>>;
impl Keymap {
    pub fn new(root_node: KeyTrie) -> Self {
        Keymap { root_node }
    }

    /// Returns a key-value list of all commands associated to a given Keymap.
    /// Keys are the node names (see KeyTrieNode documentation)
    /// Values are lists of stringified KeyEvents that triger the command.
    /// Each element in the KeyEvent list is prefixed with prefixed the ancestor KeyEvents. 
    /// For example: Stringified KeyEvent element for the 'goto_next_window' command could be "space>w>w".
    /// Ancestor KeyEvents are in this case "space" and "w".
    pub fn command_list(&self) -> CommandList {
        let mut list = HashMap::new();
        _command_list(&mut list, &KeyTrieNode::KeyTrie(self.root_node.clone()), &mut String::new());
        return list;

        fn _command_list(list: &mut CommandList, node: &KeyTrieNode, prefix: &mut String) {
            match node {
                KeyTrieNode::KeyTrie(trie_node) => {
                    for (key_event, subtrie_node) in trie_node.deref() {
                        let mut temp_prefix: String = prefix.to_string();
                        if &temp_prefix != "" { 
                            temp_prefix.push_str(">");
                        }
                        temp_prefix.push_str(&key_event.to_string());
                        _command_list(list, subtrie_node, &mut temp_prefix);
                    }
                },
                KeyTrieNode::MappableCommand(mappable_command) => {
                    if mappable_command.name() == "no_op" { return }
                    list.entry(mappable_command.name().to_string()).or_default().push(prefix.to_string());
                },
                KeyTrieNode::CommandSequence(_) => {}
            };
        }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(KeyTrie::default())
    }
}

/// Returns the Keymap root KeyTrie node.
impl Deref for Keymap {
    type Target = KeyTrie;

    fn deref(&self) -> &Self::Target {
        &self.root_node
    }
}

/// Returns the Keymap root KeyTrie node.
impl DerefMut for Keymap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.root_node
    }
}
