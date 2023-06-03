pub mod default;
pub mod macros;

pub use crate::commands::MappableCommand;
use arc_swap::{
    access::{DynAccess, DynGuard},
    ArcSwap,
};
use helix_view::{document::Mode, info::Info, input::KeyEvent};
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    convert::TryFrom,
    sync::Arc,
};

pub use default::default;
use macros::key;

#[derive(Debug, Clone, Default)]
pub struct KeyTrieNode {
    /// A label for keys coming under this node, like "Goto mode"
    name: Option<String>,
    // Values represent index in order.
    map: HashMap<KeyEvent, usize>,
    order: Vec<KeyTrie>,
    pub is_sticky: bool,
}

impl<'de> Deserialize<'de> for KeyTrieNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let list = Vec::<(KeyEvent, KeyTrie)>::deserialize(deserializer)?;
        let mut map: HashMap<KeyEvent, usize> = HashMap::with_capacity(list.len());
        let mut order: Vec<KeyTrie> = Vec::with_capacity(list.len());
        for (index, (key_event, key_trie)) in list.into_iter().enumerate() {
            map.insert(key_event, index)
                .expect("Non-duplicate key events");
            order.push(key_trie);
        }

        Ok(Self {
            map,
            order,
            ..Default::default()
        })
    }
}

impl KeyTrieNode {
    pub fn new(name: Option<&str>, map: HashMap<KeyEvent, usize>, order: Vec<KeyTrie>) -> Self {
        Self {
            name: name.map(ToString::to_string),
            map,
            order,
            is_sticky: false,
        }
    }

    fn paired(mut self) -> Vec<(KeyEvent, KeyTrie)> {
        let mut map = self.map.into_iter().collect::<Vec<(KeyEvent, usize)>>();
        map.sort_unstable_by_key(|(_, trie_index)| *trie_index);
        map.into_iter()
            .map(|(key_event, _)| key_event)
            .zip(self.order.drain(..))
            .collect()
    }

    fn paired_ref(&self) -> Vec<(KeyEvent, &KeyTrie)> {
        let mut map = self
            .map
            .clone()
            .into_iter()
            .collect::<Vec<(KeyEvent, usize)>>();
        map.sort_unstable_by_key(|(_, trie_index)| *trie_index);
        map.into_iter()
            .map(|(key_event, _)| key_event)
            .zip(self.order.iter())
            .collect()
    }

    // Order is preserved where order of other takes precedence.
    pub fn merge(self, mut other: Self) -> Self {
        if other.name.is_none() && self.name.is_some() {
            other.name = self.name.clone();
        }

        if !other.is_sticky && self.is_sticky {
            other.is_sticky = true;
        }

        for (self_key, self_trie) in self.paired() {
            match other.map.get(&self_key) {
                None => {
                    other.map.insert(self_key, other.order.len());
                    other.order.push(self_trie);
                }
                Some(other_trie_index) => {
                    if let KeyTrie::Node(other_keytrie_node) = &other.order[*other_trie_index] {
                        if let KeyTrie::Node(self_keytrie_node) = self_trie {
                            other.order[*other_trie_index] =
                                KeyTrie::Node(self_keytrie_node.merge(other_keytrie_node.clone()));
                        }
                    }
                }
            }
        }

        other
    }

    pub fn infobox(&self) -> Info {
        let mut body: Vec<(BTreeSet<&KeyEvent>, &str)> = Vec::with_capacity(self.order.len());
        for (key, key_trie_index) in self.map.iter() {
            let desc = match &self.order[*key_trie_index] {
                KeyTrie::MappableCommand(cmd) => {
                    if cmd.name() == "no_op" {
                        continue;
                    }
                    cmd.doc()
                }
                KeyTrie::Node(n) => n.name.as_deref().unwrap_or_default(),
                KeyTrie::Sequence(_) => "[Multiple commands]",
            };
            match body.iter().position(|(_, d)| d == &desc) {
                Some(pos) => {
                    body[pos].0.insert(key);
                }
                None => body.push((BTreeSet::from([key]), desc)),
            }
        }

        body.sort_unstable_by_key(|(keys, _)| {
            self.map
                .get(keys.iter().next().expect("At least one KeyEvent per row."))
        });

        let body: Vec<_> = body
            .into_iter()
            .map(|(events, desc)| {
                let events = events.iter().map(ToString::to_string).collect::<Vec<_>>();
                (events.join(", "), desc)
            })
            .collect();
        Info::new(self.name.as_deref().unwrap_or_default(), &body)
    }
}

impl PartialEq for KeyTrieNode {
    fn eq(&self, other: &Self) -> bool {
        self.is_sticky == other.is_sticky
            && self.name == other.name
            && self.paired_ref() == other.paired_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyTrie {
    MappableCommand(MappableCommand),
    Sequence(Vec<MappableCommand>),
    Node(KeyTrieNode),
}

impl<'de> Deserialize<'de> for KeyTrie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(KeyTrieVisitor)
    }
}

struct KeyTrieVisitor;

impl<'de> serde::de::Visitor<'de> for KeyTrieVisitor {
    type Value = KeyTrie;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a command, list of commands, or sub-keymap")
    }

    fn visit_str<E>(self, command: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        command
            .parse::<MappableCommand>()
            .map(KeyTrie::MappableCommand)
            .map_err(E::custom)
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: serde::de::SeqAccess<'de>,
    {
        let mut commands = Vec::new();
        while let Some(command) = seq.next_element::<String>()? {
            commands.push(
                command
                    .parse::<MappableCommand>()
                    .map_err(serde::de::Error::custom)?,
            )
        }
        Ok(KeyTrie::Sequence(commands))
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut mapping = HashMap::new();
        let mut order = Vec::new();
        while let Some((key, key_trie)) = map.next_entry::<KeyEvent, KeyTrie>()? {
            mapping.insert(key, order.len());
            order.push(key_trie);
        }
        Ok(KeyTrie::Node(KeyTrieNode::new(None, mapping, order)))
    }
}

impl KeyTrie {
    pub fn reverse_map(&self) -> ReverseKeymap {
        // recursively visit all nodes in keymap
        fn map_node(cmd_map: &mut ReverseKeymap, node: &KeyTrie, keys: &mut Vec<KeyEvent>) {
            match node {
                KeyTrie::MappableCommand(cmd) => {
                    let name = cmd.name();
                    if name != "no_op" {
                        cmd_map.entry(name.into()).or_default().push(keys.clone())
                    }
                }
                KeyTrie::Node(next) => {
                    for (key, key_trie_index) in &next.map {
                        keys.push(*key);
                        map_node(cmd_map, &next.order[*key_trie_index], keys);
                        keys.pop();
                    }
                }
                KeyTrie::Sequence(_) => {}
            };
        }

        let mut res = HashMap::new();
        map_node(&mut res, self, &mut Vec::new());
        res
    }

    pub fn node(&self) -> Option<&KeyTrieNode> {
        match *self {
            KeyTrie::Node(ref node) => Some(node),
            KeyTrie::MappableCommand(_) | KeyTrie::Sequence(_) => None,
        }
    }

    /// Merge another KeyTrie in, assuming that this KeyTrie and the other
    /// are both Nodes. Panics otherwise.
    pub fn merge_nodes(self, other: Self) -> Self {
        KeyTrie::Node(
            KeyTrieNode::try_from(self)
                .unwrap()
                .merge(KeyTrieNode::try_from(other).unwrap()),
        )
    }

    pub fn search(&self, keys: &[KeyEvent]) -> Option<&KeyTrie> {
        let mut trie = self;
        for key in keys {
            trie = match trie {
                KeyTrie::Node(node) => node
                    .map
                    .get(key)
                    .map(|key_trie_index| &node.order[*key_trie_index]),
                // leaf encountered while keys left to process
                KeyTrie::MappableCommand(_) | KeyTrie::Sequence(_) => None,
            }?
        }
        Some(trie)
    }
}

impl TryFrom<KeyTrie> for KeyTrieNode {
    type Error = ();

    fn try_from(key_trie: KeyTrie) -> Result<Self, Self::Error> {
        if let KeyTrie::Node(key_trie_node) = key_trie {
            Ok(key_trie_node)
        } else {
            Err(())
        }
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

/// A map of command names to keybinds that will execute the command.
pub type ReverseKeymap = HashMap<String, Vec<Vec<KeyEvent>>>;

pub struct Keymaps {
    pub map: Box<dyn DynAccess<HashMap<Mode, KeyTrie>>>,
    /// Stores pending keys waiting for the next key. This is relative to a
    /// sticky node if one is in use.
    state: Vec<KeyEvent>,
    /// Stores the sticky node if one is activated.
    pub sticky: Option<KeyTrieNode>,
}

impl Keymaps {
    pub fn new(map: Box<dyn DynAccess<HashMap<Mode, KeyTrie>>>) -> Self {
        Self {
            map,
            state: Vec::new(),
            sticky: None,
        }
    }

    pub fn map(&self) -> DynGuard<HashMap<Mode, KeyTrie>> {
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
            None => Cow::Borrowed(keymap),
        };

        let trie = match trie_node.search(&[*first]) {
            Some(KeyTrie::MappableCommand(ref cmd)) => {
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
            Some(KeyTrie::Node(map)) => {
                if map.is_sticky {
                    self.state.clear();
                    self.sticky = Some(map.clone());
                }
                KeymapResult::Pending(map.clone())
            }
            Some(KeyTrie::MappableCommand(cmd)) => {
                self.state.clear();
                KeymapResult::Matched(cmd.clone())
            }
            Some(KeyTrie::Sequence(cmds)) => {
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
pub fn merge_keys(
    dst: HashMap<Mode, KeyTrie>,
    mut delta: HashMap<Mode, KeyTrie>,
) -> HashMap<Mode, KeyTrie> {
    let mut res = HashMap::with_capacity(dst.len());
    for (mode, keys) in dst {
        res.insert(
            mode,
            keys.merge_nodes(
                delta
                    .remove(&mode)
                    .unwrap_or_else(|| KeyTrie::Node(KeyTrieNode::default())),
            ),
        );
    }

    res
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
        let keymap = hashmap! {
            Mode::Normal => keymap!({ "Normal mode"
                    "i" => normal_mode,
                    "无" => insert_mode,
                    "z" => jump_backward,
                    "g" => { "Merge into goto mode"
                        "$" => goto_line_end,
                        "g" => delete_char_forward,
                    },
                })
        };
        let mut merged_keyamp = merge_keys(default(), keymap.clone());
        assert_ne!(keymap, merged_keyamp);

        let mut keymap = Keymaps::new(Box::new(Constant(merged_keyamp.clone())));
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

        let keymap = merged_keyamp.get_mut(&Mode::Normal).unwrap();
        // Assumes that `g` is a node in default keymap
        assert_eq!(
            keymap.search(&[key!('g'), key!('$')]).unwrap(),
            &KeyTrie::MappableCommand(MappableCommand::goto_line_end),
            "Leaf should be present in merged subnode"
        );
        // Assumes that `gg` is in default keymap
        assert_eq!(
            keymap.search(&[key!('g'), key!('g')]).unwrap(),
            &KeyTrie::MappableCommand(MappableCommand::delete_char_forward),
            "Leaf should replace old leaf in merged subnode"
        );
        // Assumes that `ge` is in default keymap
        assert_eq!(
            keymap.search(&[key!('g'), key!('e')]).unwrap(),
            &KeyTrie::MappableCommand(MappableCommand::goto_last_line),
            "Old leaves in subnode should be present in merged node"
        );

        assert!(
            merged_keyamp
                .get(&Mode::Normal)
                .and_then(|key_trie| key_trie.node())
                .unwrap()
                .order
                .len()
                > 1
        );
        assert!(!merged_keyamp
            .get(&Mode::Insert)
            .and_then(|key_trie| key_trie.node())
            .unwrap()
            .order
            .is_empty());
    }

    #[test]
    fn merge_overrides_node_name_and_sticky() {
        let self_keymap = keymap!({ "Goto something"
            "g" => goto_file_start,
            "e" => goto_file_end,
        });
        let other_keymap = keymap!({ "Goto something new" sticky=true
            "g" => goto_file_start,
            "e" => goto_file_end,
        });
        let expected_keymap = keymap!({ "Goto something new" sticky=true
            "g" => goto_file_start,
            "e" => goto_file_end,
        });
        assert_eq!(expected_keymap, self_keymap.merge_nodes(other_keymap))
    }

    #[test]
    fn merg_preserves_node_name_and_sticky() {
        let self_keymap = keymap!({ "Goto something old" sticky=true
            "g" => goto_file_start,
            "e" => goto_file_end,
        });
        let other_keymap = keymap!({ ""
            "g" => goto_file_start,
            "e" => goto_file_end,
        })
        .node()
        .unwrap()
        .clone();
        let other_keymap = KeyTrie::Node(KeyTrieNode {
            name: None,
            ..other_keymap
        });
        let expected_keymap = keymap!({ "Goto something old" sticky=true
            "g" => goto_file_start,
            "e" => goto_file_end,
        });

        assert_eq!(expected_keymap, self_keymap.merge_nodes(other_keymap))
    }

    #[test]
    fn order_should_be_set() {
        let keymap = hashmap! {
            Mode::Normal => keymap!({ "Normal mode"
                    "space" => { ""
                        "s" => { ""
                            "v" => vsplit,
                            "c" => hsplit,
                        },
                    },
                })
        };
        let mut merged_keyamp = merge_keys(default(), keymap.clone());
        assert_ne!(keymap, merged_keyamp);
        let keymap = merged_keyamp.get_mut(&Mode::Normal).unwrap();
        // Make sure mapping works
        assert_eq!(
            keymap.search(&[key!(' '), key!('s'), key!('v')]).unwrap(),
            &KeyTrie::MappableCommand(MappableCommand::vsplit),
            "Leaf should be present in merged subnode"
        );
        // Make sure an order was set during merge
        let node = keymap.search(&[crate::key!(' ')]).unwrap();
        assert!(!node.node().unwrap().order.as_slice().is_empty())
    }

    #[test]
    fn user_order_takes_precedence() {
        let self_keymap = keymap!({ "Normal mode"
            "i" => insert_mode,
            "g" => { "Goto"
                "g" => goto_file_start,
                "e" => goto_file_end,
                "d" => page_down,
            },
            "j" | "k" => move_line_down,
            "r" => replace,
        });
        let other_keymap = keymap!({ "Normal mode"
            "i" => goto_file_start,
            "j" | "k" => move_line_down,
            "g" => { "Goto"
                "e" => goto_file_end,
                "g" => goto_file_start,
            },
        });
        let expected_keymap = keymap!({ "Normal mode"
            "i" => goto_file_start,
            "j" | "k" => move_line_down,
            "g" => { "Goto"
                "e" => goto_file_end,
                "g" => goto_file_start,
                "d" => page_down,
            },
            "r" => replace,
        });

        assert_eq!(expected_keymap, self_keymap.merge_nodes(other_keymap))
    }

    #[test]
    fn aliased_modes_are_same_in_default_keymap() {
        let keymaps = Keymaps::default().map();
        let root = keymaps.get(&Mode::Normal).unwrap();
        assert_eq!(
            root.search(&[key!(' '), key!('w')]).unwrap(),
            root.search(&["C-w".parse::<KeyEvent>().unwrap()]).unwrap(),
            "Mismatch for window mode on `Space-w` and `Ctrl-w`"
        );

        let z_view = root.search(&[key!('z')]).unwrap().node().unwrap().clone();
        let mut z_sticky_view = root.search(&[key!('Z')]).unwrap().node().unwrap().clone();
        assert!(z_sticky_view.is_sticky);
        z_sticky_view.is_sticky = false;
        assert_eq!(
            z_view, z_sticky_view,
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
        let keymap = normal_mode;
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

        let expectation = KeyTrie::Node(KeyTrieNode::new(
            None,
            hashmap! {
                key => 0
            },
            vec![KeyTrie::Sequence(vec![
                MappableCommand::select_all,
                MappableCommand::Typable {
                    name: "pipe".to_string(),
                    args: vec![
                        "sed".to_string(),
                        "-E".to_string(),
                        "'s/\\s+$//g'".to_string(),
                    ],
                    doc: "".to_string(),
                },
            ])],
        ));

        assert_eq!(toml::from_str(keys), Ok(expectation));
    }
}
