pub use crate::commands::Command;
use crate::config::Config;
use helix_core::hashmap;
use helix_view::{document::Mode, info::Info, input::KeyEvent};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
};

#[macro_export]
macro_rules! key {
    ($key:ident) => {
        KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
            modifiers: ::helix_view::keyboard::KeyModifiers::NONE,
        }
    };
    ($($ch:tt)*) => {
        KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::NONE,
        }
    };
}

/// Macro for defining the root of a `Keymap` object. Example:
///
/// ```
/// let normal_mode = keymap!({ "Normal mode"
///     "i" => insert_mode,
///     "g" => { "Goto mode"
///         "g" => goto_top,
///         "e" => goto_end
///     },
///     "j" | "down" => move_down
/// });
/// let keymap = Keymap::new(normal_mode);
/// ```
#[macro_export]
macro_rules! keymap {
    (@trie $cmd:ident) => { KeyTrie::Leaf(Command::$cmd) };
    (@trie
        { $label:literal $($($key:literal)|+ => $value:tt),* }
    ) => {
        keymap!({ $label $($($key)|+ => $value),* })
    };
    (
        { $label:literal $($($key:literal)|+ => $value:tt),* }
    ) => {
        {
            // taken from the hashmap! macro since a macro cannot
            // take output of another macro as input
            let _cap = hashmap!(@count $($($key),+),*);
            let mut _map = ::std::collections::HashMap::with_capacity(_cap);
            $(
                $(
                    let _ = _map.insert($key.parse::<KeyEvent>().unwrap(), keymap!(@trie $value));
                )+
            )*
            KeyTrie::Node(KeyTrieNode::new($label, _map))
        }
    };
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct KeyTrieNode {
    /// A label for keys coming under this node, like "Goto mode"
    #[serde(skip)]
    name: String,
    #[serde(flatten)]
    map: HashMap<KeyEvent, KeyTrie>,
}

impl KeyTrieNode {
    pub fn new(name: &str, map: HashMap<KeyEvent, KeyTrie>) -> Self {
        Self {
            name: name.to_string(),
            map,
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
                    let mut n = std::mem::take(node);
                    n.merge(other_node);
                    let _ = std::mem::replace(node, n);
                }
            } else {
                self.map.insert(key, trie.clone());
            }
        }
    }
}

impl From<KeyTrieNode> for Info {
    fn from(node: KeyTrieNode) -> Self {
        let mut body = BTreeMap::new();
        for (&key, trie) in node.iter() {
            let desc = match trie {
                KeyTrie::Leaf(cmd) => cmd.doc(),
                KeyTrie::Node(n) => n.name(),
            };
            // FIXME: multiple keys are ordered randomly (use BTreeSet)
            body.entry(desc).or_insert_with(Vec::new).push(key);
        }
        Info::key(node.name(), body)
    }
}

impl Default for KeyTrieNode {
    fn default() -> Self {
        Self::new("", HashMap::new())
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
    Leaf(Command),
    Node(KeyTrieNode),
}

impl KeyTrie {
    pub fn node(&self) -> Option<&KeyTrieNode> {
        match *self {
            KeyTrie::Node(ref node) => Some(node),
            KeyTrie::Leaf(_) => None,
        }
    }

    pub fn node_mut(&mut self) -> Option<&mut KeyTrieNode> {
        match *self {
            KeyTrie::Node(ref mut node) => Some(node),
            KeyTrie::Leaf(_) => None,
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
                KeyTrie::Leaf(_) => None,
            }?
        }
        Some(trie)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeymapResult {
    /// Needs more keys to execute a command. Contains valid keys for next keystroke.
    Pending(KeyTrieNode),
    Matched(Command),
    /// Key was not found in the root keymap
    NotFound,
    /// Key is invalid in combination with previous keys. Contains keys leading upto
    /// and including current (invalid) key.
    Cancelled(Vec<KeyEvent>),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Keymap {
    /// Always a Node
    #[serde(flatten)]
    pub root: KeyTrie,
    #[serde(skip)]
    state: Vec<KeyEvent>,
}

impl Keymap {
    pub fn new(root: KeyTrie) -> Self {
        Keymap {
            root,
            state: Vec::new(),
        }
    }

    /// Lookup `key` in the keymap to try and find a command to execute
    pub fn get(&mut self, key: KeyEvent) -> KeymapResult {
        let &first = self.state.get(0).unwrap_or(&key);
        let trie = match self.root.search(&[first]) {
            Some(&KeyTrie::Leaf(cmd)) => return KeymapResult::Matched(cmd),
            None => return KeymapResult::NotFound,
            Some(t) => t,
        };
        self.state.push(key);
        match trie.search(&self.state[1..]) {
            Some(&KeyTrie::Node(ref map)) => KeymapResult::Pending(map.clone()),
            Some(&KeyTrie::Leaf(command)) => {
                self.state.clear();
                KeymapResult::Matched(command)
            }
            None => KeymapResult::Cancelled(self.state.drain(..).collect()),
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.root.merge_nodes(other.root);
        // let node = std::mem::take(other.root.node_mut().unwrap());
        // self.root.node_mut().unwrap().merge(node);
    }
}

impl Deref for Keymap {
    type Target = KeyTrieNode;

    fn deref(&self) -> &Self::Target {
        &self.root.node().unwrap()
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(KeyTrie::Node(KeyTrieNode::default()))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Keymaps(pub HashMap<Mode, Keymap>);

impl Deref for Keymaps {
    type Target = HashMap<Mode, Keymap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Keymaps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Keymaps {
    fn default() -> Keymaps {
        let normal = keymap!({ "Normal mode"
            "h" | "left" => move_char_left,
            "j" | "down" => move_line_down,
            "k" | "up" => move_line_up,
            "l" | "right" => move_char_right,

            "t" => find_till_char,
            "f" => find_next_char,
            "T" => till_prev_char,
            "F" => find_prev_char,
            "r" => replace,
            "R" => replace_with_yanked,

            "home" => goto_line_start,
            "end" => goto_line_end,

            "w" => move_next_word_start,
            "b" => move_prev_word_start,
            "e" => move_next_word_end,

            "W" => move_next_long_word_start,
            "B" => move_prev_long_word_start,
            "E" => move_next_long_word_end,

            "v" => select_mode,
            "g" => { "Goto mode"
                "g" => goto_file_start,
                "e" => goto_file_end,
                "h" => goto_line_start,
                "l" => goto_line_end,
                "s" => goto_first_nonwhitespace,
                "d" => goto_definition,
                "y" => goto_type_definition,
                "r" => goto_reference,
                "i" => goto_implementation,
                "t" => goto_window_top,
                "m" => goto_window_middle,
                "b" => goto_window_bottom,
                "a" => goto_last_accessed_file
            },
            ":" => command_mode,

            "i" => insert_mode,
            "I" => prepend_to_line,
            "a" => append_mode,
            "A" => append_to_line,
            "o" => open_below,
            "O" => open_above,
            // [<space>  ]<space> equivalents too (add blank new line, no edit)

            "d" => delete_selection,
            // TODO: also delete without yanking
            "c" => change_selection,
            // TODO: also change delete without yanking

            "s" => select_regex,
            "A-s" => split_selection_on_newline,
            "S" => split_selection,
            ";" => collapse_selection,
            "A-;" => flip_selections,
            "%" => select_all,
            "x" => extend_line,
            "X" => extend_to_line_bounds,
            // crop_to_whole_line

            "m" => { "Match mode"
                "m" => match_brackets,
                "s" => surround_add,
                "r" => surround_replace,
                "d" => surround_delete,
                "a" => select_textobject_around,
                "i" => select_textobject_inner
            },
            "[" => { "Bracket mode"
                "d" => goto_prev_diag,
                "D" => goto_first_diag
            },
            "]" => { "Bracket mode"
                "d" => goto_next_diag,
                "D" => goto_last_diag
            },

            "/" => search,
            // ? for search_reverse
            "n" => search_next,
            "N" => extend_search_next,
            // N for search_prev
            "*" => search_selection,

            "u" => undo,
            "U" => redo,

            "y" => yank,
            // yank_all
            "p" => paste_after,
            // paste_all
            "P" => paste_before,

            ">" => indent,
            "<" => unindent,
            "=" => format_selections,
            "J" => join_selections,
            // TODO: conflicts hover/doc
            "K" => keep_selections,
            // TODO: and another method for inverse

            // TODO: clashes with space mode
            "space" => keep_primary_selection,

            // "q" => record_macro,
            // "Q" => replay_macro,

            // & align selections
            // _ trim selections

            // C / altC = copy (repeat) selections on prev/next lines

            "esc" => normal_mode,
            "C-b" | "pageup" => page_up,
            "C-f" | "pagedown" => page_down,
            "C-u" => half_page_up,
            "C-d" => half_page_down,

            "C-w" => { "Window mode"
                "C-w" | "w" => rotate_view,
                "C-h" | "h" => hsplit,
                "C-v" | "v" => vsplit,
                "C-q" | "q" => wclose
            },

            // move under <space>c
            "C-c" => toggle_comments,
            "K" => hover,

            // z family for save/restore/combine from/to sels from register

            // supposedly "C-i" but did not work
            "tab" => jump_forward,
            "C-o" => jump_backward,
            // "C-s" => save_selection,

            "space" => { "Space mode"
                "f" => file_picker,
                "b" => buffer_picker,
                "s" => symbol_picker,
                "w" => { "Window mode"
                    "C-w" | "w" => rotate_view,
                    "C-h" | "h" => hsplit,
                    "C-v" | "v" => vsplit,
                    "C-q" | "q" => wclose
                },
                "y" => yank_joined_to_clipboard,
                "Y" => yank_main_selection_to_clipboard,
                "p" => paste_clipboard_after,
                "P" => paste_clipboard_before,
                "R" => replace_selections_with_clipboard,
                "space" => keep_primary_selection
            },
            "z" => { "View mode"
                "t" => align_view_top,
                "z" | "c" => align_view_center,
                "b" => align_view_bottom,
                "m" => align_view_middle,
                "k" => scroll_up,
                "j" => scroll_down
            },

            "\"" => select_register
        });
        // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
        // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
        // because some selection operations can now be done from normal mode, some from select mode.
        let mut select = normal.clone();
        select.merge_nodes(keymap!({ "Select mode"
            "h" | "left" => extend_char_left,
            "j" | "down" => extend_line_down,
            "k" | "up" => extend_line_up,
            "l" | "right" => extend_char_right,

            "w" => extend_next_word_start,
            "b" => extend_prev_word_start,
            "e" => extend_next_word_end,

            "t" => extend_till_char,
            "f" => extend_next_char,
            "T" => extend_till_prev_char,
            "F" => extend_prev_char,

            "home" => goto_line_start,
            "end" => goto_line_end,
            "esc" => exit_select_mode
        }));
        let insert = keymap!({ "Insert mode"
            "esc" => normal_mode,

            "backspace" => delete_char_backward,
            "del" => delete_char_forward,
            "ret" => insert_newline,
            "tab" => insert_tab,
            "C-w" => delete_word_backward,

            "left" => move_char_left,
            "down" => move_line_down,
            "up" => move_line_up,
            "right" => move_char_right,
            "pageup" => page_up,
            "pagedown" => page_down,
            "home" => goto_line_start,
            "end" => goto_line_end_newline,

            "C-x" => completion
        });
        Keymaps(hashmap!(
            Mode::Normal => Keymap::new(normal),
            Mode::Select => Keymap::new(select),
            Mode::Insert => Keymap::new(insert),
        ))
    }
}

/// Merge default config keys with user overwritten keys for custom
/// user config.
pub fn merge_keys(mut config: Config) -> Config {
    // FIXME: requires recursive merging (defining "gp" in user mapping
    // will disable all other default mappings under "g")
    let mut delta = std::mem::take(&mut config.keys);
    for (mode, keys) in &mut *config.keys {
        keys.merge(delta.remove(mode).unwrap_or_default())
    }
    config
}

#[test]
fn merge_partial_keys() {
    use helix_view::keyboard::{KeyCode, KeyModifiers};
    let config = Config {
        keys: Keymaps(hashmap! {
            Mode::Normal => Keymap::new(
                keymap!({ "Normal mode"
                    "i" => normal_mode,
                    "无" => insert_mode
                })
            )
        }),
        ..Default::default()
    };
    let mut merged_config = merge_keys(config.clone());
    assert_ne!(config, merged_config);
    assert_eq!(
        merged_config
            .keys
            .0
            .get_mut(&Mode::Normal)
            .unwrap()
            .get(KeyEvent {
                code: KeyCode::Char('i'),
                modifiers: KeyModifiers::NONE
            }),
        KeymapResult::Matched(Command::normal_mode)
    );
    assert_eq!(
        merged_config
            .keys
            .0
            .get_mut(&Mode::Normal)
            .unwrap()
            .get(KeyEvent {
                code: KeyCode::Char('无'),
                modifiers: KeyModifiers::NONE
            }),
        KeymapResult::Matched(Command::insert_mode)
    );
    assert!(merged_config.keys.0.get(&Mode::Normal).unwrap().len() > 1);
    assert!(merged_config.keys.0.get(&Mode::Insert).unwrap().len() > 0);
}
