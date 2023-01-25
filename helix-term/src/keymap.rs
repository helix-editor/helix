pub mod default;
pub mod macros;
// NOTE: Only pub becuase of their use in macros
pub mod keytrie;
pub mod keytrienode;
#[cfg(test)]
mod tests;

use self::{keytrie::KeyTrie, keytrienode::KeyTrieNode, macros::key};

use crate::commands::MappableCommand;
use arc_swap::{
    access::{DynAccess, DynGuard},
    ArcSwap,
};

use helix_view::{document::Mode, input::KeyEvent};
use std::{collections::HashMap, sync::Arc};


#[derive(Debug, Clone, PartialEq)]
pub enum KeymapResult {
    Pending(KeyTrie),
    Matched(MappableCommand),
    MatchedCommandSequence(Vec<MappableCommand>),
    NotFound,
    /// Contains pressed KeyEvents leading up to the cancellation.
    Cancelled(Vec<KeyEvent>),
}

pub struct Keymap {
    pub keytries: Box<dyn DynAccess<HashMap<Mode, KeyTrie>>>,
    /// Relative to a sticky node if Some.
    pending_keys: Vec<KeyEvent>,
    pub sticky_keytrie: Option<KeyTrie>,
}

pub type CommandList = HashMap<String, Vec<String>>;
impl Keymap {
    pub fn new(keymaps: Box<dyn DynAccess<HashMap<Mode, KeyTrie>>>) -> Self {
        Self {
            keytries: keymaps,
            pending_keys: Vec::new(),
            sticky_keytrie: None,
        }
    }

    pub fn load_keymaps(&self) -> DynGuard<HashMap<Mode, KeyTrie>> {
        self.keytries.load()
    }

    /// Returns list of keys waiting to be disambiguated in current mode.
    pub fn pending(&self) -> &[KeyEvent] {
        &self.pending_keys
    }

    pub fn sticky_keytrie(&self) -> Option<&KeyTrie> {
        self.sticky_keytrie.as_ref()
    }

    /// Lookup `key` in the keymap to try and find a command to execute.
    /// Escape key represents cancellation.
    /// This means clearing pending keystrokes, or the sticky_keytrie if none were present.
    pub fn get(&mut self, mode: Mode, key: KeyEvent) -> KeymapResult {
        // TODO: remove the sticky part and look up manually
        let keymaps = &*self.load_keymaps();
        let active_keymap = &keymaps[&mode];

        if key == key!(Esc) {
            if !self.pending_keys.is_empty() {
                // NOTE: Esc is not included here
                return KeymapResult::Cancelled(self.pending_keys.drain(..).collect());
            }
            // TODO: Shouldn't we return here also?
            self.sticky_keytrie = None;
        }

        // Check if sticky keytrie is to be used.
        let starting_keytrie = match self.sticky_keytrie {
            None => active_keymap,
            Some(ref active_sticky_keytrie) => active_sticky_keytrie,
        };

        // TODO: why check either pending or regular key?
        let first_key = self.pending_keys.get(0).unwrap_or(&key);

        let pending_keytrie: KeyTrie = match starting_keytrie.traverse(&[*first_key]) {
            Some(KeyTrieNode::KeyTrie(sub_keytrie)) => sub_keytrie,
            Some(KeyTrieNode::MappableCommand(cmd)) => {
                return KeymapResult::Matched(cmd);
            }
            Some(KeyTrieNode::CommandSequence(cmds)) => {
                return KeymapResult::MatchedCommandSequence(cmds);
            }
            None => return KeymapResult::NotFound,
        };

        self.pending_keys.push(key);
        match pending_keytrie.traverse(&self.pending_keys[1..]) {
            Some(KeyTrieNode::KeyTrie(map)) => {
                if map.is_sticky {
                    self.pending_keys.clear();
                    self.sticky_keytrie = Some(map.clone());
                }
                KeymapResult::Pending(map)
            }
            Some(KeyTrieNode::MappableCommand(cmd)) => {
                self.pending_keys.clear();
                KeymapResult::Matched(cmd)
            }
            Some(KeyTrieNode::CommandSequence(cmds)) => {
                self.pending_keys.clear();
                KeymapResult::MatchedCommandSequence(cmds)
            }
            None => KeymapResult::Cancelled(self.pending_keys.drain(..).collect()),
        }
    }

    fn get_keytrie(&self, mode: &Mode) -> KeyTrie {
        // HELP: Unsure how I should handle this Option
        self.keytries.load().get(mode).unwrap().clone()
    }

    /// Returns a key-value list of all commands associated to a given Keymap.
    /// Keys are the node names (see KeyTrieNode documentation)
    /// Values are lists of stringified KeyEvents that triger the command.
    /// Each element in the KeyEvent list is prefixed with prefixed the ancestor KeyEvents.
    /// For example: Stringified KeyEvent element for the 'goto_next_window' command could be "space>w>w".
    /// Ancestor KeyEvents are in this case "space" and "w".
    pub fn command_list(&self, mode: &Mode) -> CommandList {
        let mut list = HashMap::new();
        _command_list(
            &mut list,
            &KeyTrieNode::KeyTrie(self.get_keytrie(mode)),
            &mut String::new(),
        );
        return list;

        fn _command_list(list: &mut CommandList, node: &KeyTrieNode, prefix: &mut String) {
            match node {
                KeyTrieNode::KeyTrie(trie_node) => {
                    for (key_event, index) in trie_node.get_child_order() {
                        let mut temp_prefix: String = prefix.to_string();
                        if !&temp_prefix.is_empty() {
                            temp_prefix.push('→');
                        }
                        temp_prefix.push_str(&key_event.to_string());
                        _command_list(list, &trie_node.get_children()[*index], &mut temp_prefix);
                    }
                }
                KeyTrieNode::MappableCommand(mappable_command) => {
                    if mappable_command.name() == "no_op" {
                        return;
                    }
                    list.entry(mappable_command.name().to_string())
                        .or_default()
                        .push(prefix.to_string());
                }
                KeyTrieNode::CommandSequence(_) => {}
            };
        }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(Box::new(ArcSwap::new(Arc::new(default::default()))))
    }

    #[test]
    fn escaped_keymap() {
        use crate::commands::MappableCommand;
        use helix_view::input::{KeyCode, KeyEvent, KeyModifiers};

        let keys = r#"
"+" = [
    "select_all",
    ":pipe sed -E 's/\\s+$//g'",
]
        "#;

        let key = KeyEvent {
            code: KeyCode::Char('+'),
            modifiers: KeyModifiers::NONE,
        };

        let expectation = Keymap::new(KeyTrie::Node(KeyTrieNode::new(
            "",
            hashmap! {
                key => KeyTrie::Sequence(vec!{
                    MappableCommand::select_all,
                    MappableCommand::Typable {
                        name: "pipe".to_string(),
                        args: vec!{
                            "sed".to_string(),
                            "-E".to_string(),
                            "'s/\\s+$//g'".to_string()
                        },
                        doc: "".to_string(),
                    },
                })
            },
            vec![key],
        )));

        assert_eq!(toml::from_str(keys), Ok(expectation));
    }
}
