pub mod default;
pub mod macros;

pub use crate::commands::MappableCommand;
use crate::config::Config;
use arc_swap::{
    access::{DynAccess, DynGuard},
    ArcSwap,
};
use helix_view::{document::Mode, info::Info, input::KeyEvent};
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use default::default;
use macros::key;

#[derive(Debug, Clone)]
pub struct KeyTrieNode {
    /// A label for keys coming under this node, like "Goto mode"
    name: String,
    map: HashMap<KeyEvent, KeyTrie>,
    order: Vec<KeyEvent>,
    pub is_sticky: bool,
}

impl<'de> Deserialize<'de> for KeyTrieNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = HashMap::<KeyEvent, KeyTrie>::deserialize(deserializer)?;
        let order = map.keys().copied().collect::<Vec<_>>(); // NOTE: map.keys() has arbitrary order
        Ok(Self {
            map,
            order,
            ..Default::default()
        })
    }
}

impl KeyTrieNode {
    pub fn new(name: &str, map: HashMap<KeyEvent, KeyTrie>, order: Vec<KeyEvent>) -> Self {
        Self {
            name: name.to_string(),
            map,
            order,
            is_sticky: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Merge another Node in. Leaves and subnodes from the other node replace
    /// corresponding keyevent in self, except when both other and self have
    /// subnodes for same key. In that case the merge is recursive.
    pub fn merge(&mut self, mut other: Self) {
        for (key, trie) in std::mem::take(&mut other.map) {
            if let Some(KeyTrie::Node(node)) = self.map.get_mut(&key) {
                if let KeyTrie::Node(other_node) = trie {
                    node.merge(other_node);
                    continue;
                }
            }
            self.map.insert(key, trie);
        }
        for &key in self.map.keys() {
            if !self.order.contains(&key) {
                self.order.push(key);
            }
        }
    }

    pub fn infobox(&self) -> Info {
        let mut body: Vec<(&str, BTreeSet<KeyEvent>)> = Vec::with_capacity(self.len());
        for (&key, trie) in self.iter() {
            let desc = match trie {
                KeyTrie::Leaf(cmd) => {
                    if cmd.name() == "no_op" {
                        continue;
                    }
                    cmd.doc()
                }
                KeyTrie::Node(n) => n.name(),
                KeyTrie::Sequence(_) => "[Multiple commands]",
            };
            match body.iter().position(|(d, _)| d == &desc) {
                Some(pos) => {
                    body[pos].1.insert(key);
                }
                None => body.push((desc, BTreeSet::from([key]))),
            }
        }
        body.sort_unstable_by_key(|(_, keys)| {
            self.order
                .iter()
                .position(|&k| k == *keys.iter().next().unwrap())
                .unwrap()
        });
        let prefix = format!("{} ", self.name());
        if body.iter().all(|(desc, _)| desc.starts_with(&prefix)) {
            body = body
                .into_iter()
                .map(|(desc, keys)| (desc.strip_prefix(&prefix).unwrap(), keys))
                .collect();
        }
        Info::from_keymap(self.name(), body)
    }
    /// Get a reference to the key trie node's order.
    pub fn order(&self) -> &[KeyEvent] {
        self.order.as_slice()
    }
}

impl Default for KeyTrieNode {
    fn default() -> Self {
        Self::new("", HashMap::new(), Vec::new())
    }
}

impl PartialEq for KeyTrieNode {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
    }
}

impl Deref for KeyTrieNode {
    type Target = HashMap<KeyEvent, KeyTrie>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for KeyTrieNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum KeyTrie {
    Leaf(MappableCommand),
    Sequence(Vec<MappableCommand>),
    Node(KeyTrieNode),
}

impl KeyTrie {
    pub fn node(&self) -> Option<&KeyTrieNode> {
        match *self {
            KeyTrie::Node(ref node) => Some(node),
            KeyTrie::Leaf(_) | KeyTrie::Sequence(_) => None,
        }
    }

    pub fn node_mut(&mut self) -> Option<&mut KeyTrieNode> {
        match *self {
            KeyTrie::Node(ref mut node) => Some(node),
            KeyTrie::Leaf(_) | KeyTrie::Sequence(_) => None,
        }
    }

    /// Merge another KeyTrie in, assuming that this KeyTrie and the other
    /// are both Nodes. Panics otherwise.
    pub fn merge_nodes(&mut self, mut other: Self) {
        let node = std::mem::take(other.node_mut().unwrap());
        self.node_mut().unwrap().merge(node);
    }

    pub fn search(&self, keys: &[KeyEvent]) -> Option<&KeyTrie> {
        let mut trie = self;
        for key in keys {
            trie = match trie {
                KeyTrie::Node(map) => map.get(key),
                // leaf encountered while keys left to process
                KeyTrie::Leaf(_) | KeyTrie::Sequence(_) => None,
            }?
        }
        Some(trie)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeymapResult {
    /// Needs more keys to execute a command. Contains valid keys for next keystroke.
    Pending(KeyTrieNode),
    Matched(MappableCommand),
    /// Matched a sequence of commands to execute.
    MatchedSequence(Vec<MappableCommand>),
    /// Key was not found in the root keymap
    NotFound,
    /// Key is invalid in combination with previous keys. Contains keys leading upto
    /// and including current (invalid) key.
    Cancelled(Vec<KeyEvent>),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Keymap {
    /// Always a Node
    root: KeyTrie,
}

/// A map of command names to keybinds that will execute the command.
pub type ReverseKeymap = HashMap<String, Vec<Vec<KeyEvent>>>;

impl Keymap {
    pub fn new(root: KeyTrie) -> Self {
        Keymap { root }
    }

    pub fn reverse_map(&self) -> ReverseKeymap {
        // recursively visit all nodes in keymap
        fn map_node(cmd_map: &mut ReverseKeymap, node: &KeyTrie, keys: &mut Vec<KeyEvent>) {
            match node {
                KeyTrie::Leaf(cmd) => match cmd {
                    MappableCommand::Typable { name, .. } => {
                        cmd_map.entry(name.into()).or_default().push(keys.clone())
                    }
                    MappableCommand::Static { name, .. } => cmd_map
                        .entry(name.to_string())
                        .or_default()
                        .push(keys.clone()),
                },
                KeyTrie::Node(next) => {
                    for (key, trie) in &next.map {
                        keys.push(*key);
                        map_node(cmd_map, trie, keys);
                        keys.pop();
                    }
                }
                KeyTrie::Sequence(_) => {}
            };
        }

        let mut res = HashMap::new();
        map_node(&mut res, &self.root, &mut Vec::new());
        res
    }

    pub fn root(&self) -> &KeyTrie {
        &self.root
    }

    pub fn merge(&mut self, other: Self) {
        self.root.merge_nodes(other.root);
    }
}

impl Deref for Keymap {
    type Target = KeyTrieNode;

    fn deref(&self) -> &Self::Target {
        self.root.node().unwrap()
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(KeyTrie::Node(KeyTrieNode::default()))
    }
}

pub struct Keymaps {
    pub map: Box<dyn DynAccess<HashMap<Mode, Keymap>>>,
    /// Stores pending keys waiting for the next key. This is relative to a
    /// sticky node if one is in use.
    state: Vec<KeyEvent>,
    /// Stores the sticky node if one is activated.
    pub sticky: Option<KeyTrieNode>,
}

impl Keymaps {
    pub fn new(map: Box<dyn DynAccess<HashMap<Mode, Keymap>>>) -> Self {
        Self {
            map,
            state: Vec::new(),
            sticky: None,
        }
    }

    pub fn map(&self) -> DynGuard<HashMap<Mode, Keymap>> {
        self.map.load()
    }

    /// Returns list of keys waiting to be disambiguated in current mode.
    pub fn pending(&self) -> &[KeyEvent] {
        &self.state
    }

    pub fn sticky(&self) -> Option<&KeyTrieNode> {
        self.sticky.as_ref()
    }

    /// Lookup `key` in the keymap to try and find a command to execute. Escape
    /// key cancels pending keystrokes. If there are no pending keystrokes but a
    /// sticky node is in use, it will be cleared.
    pub fn get(&mut self, mode: Mode, key: KeyEvent) -> KeymapResult {
        // TODO: remove the sticky part and look up manually
        let keymaps = &*self.map();
        let keymap = &keymaps[&mode];

        if key!(Esc) == key {
            if !self.state.is_empty() {
                // Note that Esc is not included here
                return KeymapResult::Cancelled(self.state.drain(..).collect());
            }
            self.sticky = None;
        }

        let first = self.state.get(0).unwrap_or(&key);
        let trie_node = match self.sticky {
            Some(ref trie) => Cow::Owned(KeyTrie::Node(trie.clone())),
            None => Cow::Borrowed(&keymap.root),
        };

        let trie = match trie_node.search(&[*first]) {
            Some(KeyTrie::Leaf(ref cmd)) => {
                return KeymapResult::Matched(cmd.clone());
            }
            Some(KeyTrie::Sequence(ref cmds)) => {
                return KeymapResult::MatchedSequence(cmds.clone());
            }
            None => return KeymapResult::NotFound,
            Some(t) => t,
        };

        self.state.push(key);
        match trie.search(&self.state[1..]) {
            Some(&KeyTrie::Node(ref map)) => {
                if map.is_sticky {
                    self.state.clear();
                    self.sticky = Some(map.clone());
                }
                KeymapResult::Pending(map.clone())
            }
            Some(&KeyTrie::Leaf(ref cmd)) => {
                self.state.clear();
                KeymapResult::Matched(cmd.clone())
            }
            Some(&KeyTrie::Sequence(ref cmds)) => {
                self.state.clear();
                KeymapResult::MatchedSequence(cmds.clone())
            }
            None => KeymapResult::Cancelled(self.state.drain(..).collect()),
        }
    }
}

impl Default for Keymaps {
    fn default() -> Self {
        Self::new(Box::new(ArcSwap::new(Arc::new(default()))))
    }
}

/// Merge default config keys with user overwritten keys for custom user config.
pub fn merge_keys(mut config: Config) -> Config {
    let mut delta = std::mem::replace(&mut config.keys, default());
    for (mode, keys) in &mut config.keys {
        keys.merge(delta.remove(mode).unwrap_or_default())
    }
    config
}

#[cfg(test)]
mod tests {
    use super::macros::keymap;
    use super::*;
    use arc_swap::access::Constant;
    use helix_core::hashmap;

    #[test]
    #[should_panic]
    fn duplicate_keys_should_panic() {
        keymap!({ "Normal mode"
            "i" => normal_mode,
            "i" => goto_definition,
        });
    }

    #[test]
    fn check_duplicate_keys_in_default_keymap() {
        // will panic on duplicate keys, assumes that `Keymaps` uses keymap! macro
        Keymaps::default();
    }

    #[test]
    fn merge_partial_keys() {
        let config = Config {
            keys: hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "i" => normal_mode,
                        "无" => insert_mode,
                        "z" => jump_backward,
                        "g" => { "Merge into goto mode"
                            "$" => goto_line_end,
                            "g" => delete_char_forward,
                        },
                    })
                )
            },
            ..Default::default()
        };
        let mut merged_config = merge_keys(config.clone());
        assert_ne!(config, merged_config);

        let mut keymap = Keymaps::new(Box::new(Constant(merged_config.keys.clone())));
        assert_eq!(
            keymap.get(Mode::Normal, key!('i')),
            KeymapResult::Matched(MappableCommand::normal_mode),
            "Leaf should replace leaf"
        );
        assert_eq!(
            keymap.get(Mode::Normal, key!('无')),
            KeymapResult::Matched(MappableCommand::insert_mode),
            "New leaf should be present in merged keymap"
        );
        // Assumes that z is a node in the default keymap
        assert_eq!(
            keymap.get(Mode::Normal, key!('z')),
            KeymapResult::Matched(MappableCommand::jump_backward),
            "Leaf should replace node"
        );

        let keymap = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        // Assumes that `g` is a node in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('$')]).unwrap(),
            &KeyTrie::Leaf(MappableCommand::goto_line_end),
            "Leaf should be present in merged subnode"
        );
        // Assumes that `gg` is in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('g')]).unwrap(),
            &KeyTrie::Leaf(MappableCommand::delete_char_forward),
            "Leaf should replace old leaf in merged subnode"
        );
        // Assumes that `ge` is in default keymap
        assert_eq!(
            keymap.root().search(&[key!('g'), key!('e')]).unwrap(),
            &KeyTrie::Leaf(MappableCommand::goto_last_line),
            "Old leaves in subnode should be present in merged node"
        );

        assert!(merged_config.keys.get(&Mode::Normal).unwrap().len() > 1);
        assert!(merged_config.keys.get(&Mode::Insert).unwrap().len() > 0);
    }

    #[test]
    fn order_should_be_set() {
        let config = Config {
            keys: hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "space" => { ""
                            "s" => { ""
                                "v" => vsplit,
                                "c" => hsplit,
                            },
                        },
                    })
                )
            },
            ..Default::default()
        };
        let mut merged_config = merge_keys(config.clone());
        assert_ne!(config, merged_config);
        let keymap = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        // Make sure mapping works
        assert_eq!(
            keymap
                .root()
                .search(&[key!(' '), key!('s'), key!('v')])
                .unwrap(),
            &KeyTrie::Leaf(MappableCommand::vsplit),
            "Leaf should be present in merged subnode"
        );
        // Make sure an order was set during merge
        let node = keymap.root().search(&[crate::key!(' ')]).unwrap();
        assert!(!node.node().unwrap().order().is_empty())
    }

    #[test]
    fn aliased_modes_are_same_in_default_keymap() {
        let keymaps = Keymaps::default().map();
        let root = keymaps.get(&Mode::Normal).unwrap().root();
        assert_eq!(
            root.search(&[key!(' '), key!('w')]).unwrap(),
            root.search(&["C-w".parse::<KeyEvent>().unwrap()]).unwrap(),
            "Mismatch for window mode on `Space-w` and `Ctrl-w`"
        );
        assert_eq!(
            root.search(&[key!('z')]).unwrap(),
            root.search(&[key!('Z')]).unwrap(),
            "Mismatch for view mode on `z` and `Z`"
        );
    }

    #[test]
    fn reverse_map() {
        let normal_mode = keymap!({ "Normal mode"
            "i" => insert_mode,
            "g" => { "Goto"
                "g" => goto_file_start,
                "e" => goto_file_end,
            },
            "j" | "k" => move_line_down,
        });
        let keymap = Keymap::new(normal_mode);
        let mut reverse_map = keymap.reverse_map();

        // sort keybindings in order to have consistent tests
        // HashMaps can be compared but we can still get different ordering of bindings
        // for commands that have multiple bindings assigned
        for v in reverse_map.values_mut() {
            v.sort()
        }

        assert_eq!(
            reverse_map,
            HashMap::from([
                ("insert_mode".to_string(), vec![vec![key!('i')]]),
                (
                    "goto_file_start".to_string(),
                    vec![vec![key!('g'), key!('g')]]
                ),
                (
                    "goto_file_end".to_string(),
                    vec![vec![key!('g'), key!('e')]]
                ),
                (
                    "move_line_down".to_string(),
                    vec![vec![key!('j')], vec![key!('k')]]
                ),
            ]),
            "Mismatch"
        )
    }
}
