pub use crate::commands::Command;
use crate::config::Config;
use helix_core::hashmap;
use helix_view::{document::Mode, input::KeyEvent};
use serde::Deserialize;
use std::{
    collections::HashMap,
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

macro_rules! ctrl {
    ($($ch:tt)*) => {
        KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::CONTROL,
        }
    };
}

macro_rules! alt {
    ($($ch:tt)*) => {
        KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::ALT,
        }
    };
}

type KeyTrieNode = HashMap<KeyEvent, KeyTrie>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum KeyTrie {
    Leaf(Command),
    Node(KeyTrieNode),
}

impl KeyTrie {
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
    #[serde(flatten)]
    pub root: KeyTrieNode,
    #[serde(skip)]
    state: Vec<KeyEvent>,
}

impl Keymap {
    pub fn new(root: KeyTrieNode) -> Self {
        Keymap {
            root,
            state: Vec::new(),
        }
    }

    /// Lookup `key` in the keymap to try and find a command to execute
    pub fn get(&mut self, key: KeyEvent) -> KeymapResult {
        let first = self.state.get(0).unwrap_or(&key);
        let trie = match self.root.get(first) {
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
}

impl Deref for Keymap {
    type Target = KeyTrieNode;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

impl DerefMut for Keymap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.root
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(HashMap::new())
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
        use KeyTrie::*;
        let normal = hashmap!(
            key!('h') => Leaf(Command::move_char_left),
            key!('j') => Leaf(Command::move_line_down),
            key!('k') => Leaf(Command::move_line_up),
            key!('l') => Leaf(Command::move_char_right),

            key!(Left) => Leaf(Command::move_char_left),
            key!(Down) => Leaf(Command::move_line_down),
            key!(Up) => Leaf(Command::move_line_up),
            key!(Right) => Leaf(Command::move_char_right),

            key!('t') => Leaf(Command::find_till_char),
            key!('f') => Leaf(Command::find_next_char),
            key!('T') => Leaf(Command::till_prev_char),
            key!('F') => Leaf(Command::find_prev_char),
            // and matching set for select mode (extend)
            //
            key!('r') => Leaf(Command::replace),
            key!('R') => Leaf(Command::replace_with_yanked),

            key!(Home) => Leaf(Command::goto_line_start),
            key!(End) => Leaf(Command::goto_line_end),

            key!('w') => Leaf(Command::move_next_word_start),
            key!('b') => Leaf(Command::move_prev_word_start),
            key!('e') => Leaf(Command::move_next_word_end),

            key!('W') => Leaf(Command::move_next_long_word_start),
            key!('B') => Leaf(Command::move_prev_long_word_start),
            key!('E') => Leaf(Command::move_next_long_word_end),

            key!('v') => Leaf(Command::select_mode),
            key!('g') => Leaf(Command::goto_mode),
            key!(':') => Leaf(Command::command_mode),

            key!('i') => Leaf(Command::insert_mode),
            key!('I') => Leaf(Command::prepend_to_line),
            key!('a') => Leaf(Command::append_mode),
            key!('A') => Leaf(Command::append_to_line),
            key!('o') => Leaf(Command::open_below),
            key!('O') => Leaf(Command::open_above),
            // [<space>  ]<space> equivalents too (add blank new line, no edit)


            key!('d') => Leaf(Command::delete_selection),
            // TODO: also delete without yanking
            key!('c') => Leaf(Command::change_selection),
            // TODO: also change delete without yanking

            // key!('r') KeyCommand(ommand)::replace_with_char,

            key!('s') => Leaf(Command::select_regex),
            alt!('s') => Leaf(Command::split_selection_on_newline),
            key!('S') => Leaf(Command::split_selection),
            key!(';') => Leaf(Command::collapse_selection),
            alt!(';') => Leaf(Command::flip_selections),
            key!('%') => Leaf(Command::select_all),
            key!('x') => Leaf(Command::extend_line),
            key!('X') => Leaf(Command::extend_to_line_bounds),
            // crop_to_whole_line


            key!('m') => Leaf(Command::match_mode),
            key!('[') => Leaf(Command::left_bracket_mode),
            key!(']') => Leaf(Command::right_bracket_mode),

            key!('/') => Leaf(Command::search),
            // ? for search_reverse
            key!('n') => Leaf(Command::search_next),
            key!('N') => Leaf(Command::extend_search_next),
            // N for search_prev
            key!('*') => Leaf(Command::search_selection),

            key!('u') => Leaf(Command::undo),
            key!('U') => Leaf(Command::redo),

            key!('y') => Leaf(Command::yank),
            // yank_all
            key!('p') => Leaf(Command::paste_after),
            // paste_all
            key!('P') => Leaf(Command::paste_before),

            key!('>') => Leaf(Command::indent),
            key!('<') => Leaf(Command::unindent),
            key!('=') => Leaf(Command::format_selections),
            key!('J') => Leaf(Command::join_selections),
            // TODO: conflicts hover/doc
            key!('K') => Leaf(Command::keep_selections),
            // TODO: and another method for inverse

            // TODO: clashes with space mode
            key!(' ') => Leaf(Command::keep_primary_selection),

            // key!('q') KeyCommand(ommand)::record_macro,
            // key!('Q') KeyCommand(ommand)::replay_macro,

            // ~ / apostrophe => change case
            // & align selections
            // _ trim selections

            // C / altC = copy (repeat) selections on prev/next lines

            key!(Esc) => Leaf(Command::normal_mode),
            key!(PageUp) => Leaf(Command::page_up),
            key!(PageDown) => Leaf(Command::page_down),
            ctrl!('b') => Leaf(Command::page_up),
            ctrl!('f') => Leaf(Command::page_down),
            ctrl!('u') => Leaf(Command::half_page_up),
            ctrl!('d') => Leaf(Command::half_page_down),

            ctrl!('w') => Leaf(Command::window_mode),

            // move under <space>c
            ctrl!('c') => Leaf(Command::toggle_comments),
            key!('K') => Leaf(Command::hover),

            // z family for save/restore/combine from/to sels from register

            // supposedly ctrl!('i') but did not work
            key!(Tab) => Leaf(Command::jump_forward),
            ctrl!('o') => Leaf(Command::jump_backward),
            // ctrl!('s') KeyCommand(ommand)::save_selection,

            key!(' ') => Leaf(Command::space_mode),
            key!('z') => Leaf(Command::view_mode),

            key!('"') => Leaf(Command::select_register),
        );
        // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
        // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
        // because some selection operations can now be done from normal mode, some from select mode.
        let mut select = normal.clone();
        select.extend(
            hashmap!(
                key!('h') => Leaf(Command::extend_char_left),
                key!('j') => Leaf(Command::extend_line_down),
                key!('k') => Leaf(Command::extend_line_up),
                key!('l') => Leaf(Command::extend_char_right),

                key!(Left) => Leaf(Command::extend_char_left),
                key!(Down) => Leaf(Command::extend_line_down),
                key!(Up) => Leaf(Command::extend_line_up),
                key!(Right) => Leaf(Command::extend_char_right),

                key!('w') => Leaf(Command::extend_next_word_start),
                key!('b') => Leaf(Command::extend_prev_word_start),
                key!('e') => Leaf(Command::extend_next_word_end),

                key!('t') => Leaf(Command::extend_till_char),
                key!('f') => Leaf(Command::extend_next_char),

                key!('T') => Leaf(Command::extend_till_prev_char),
                key!('F') => Leaf(Command::extend_prev_char),
                key!(Home) => Leaf(Command::goto_line_start),
                key!(End) => Leaf(Command::goto_line_end),
                key!(Esc) => Leaf(Command::exit_select_mode),
            )
            .into_iter(),
        );
        let insert = hashmap!(
            key!(Esc) => Leaf(Command::normal_mode),
            key!(Backspace) => Leaf(Command::delete_char_backward),
            key!(Delete) => Leaf(Command::delete_char_forward),
            key!(Enter) => Leaf(Command::insert_newline),
            key!(Tab) => Leaf(Command::insert_tab),
            key!(Left) => Leaf(Command::move_char_left),
            key!(Down) => Leaf(Command::move_line_down),
            key!(Up) => Leaf(Command::move_line_up),
            key!(Right) => Leaf(Command::move_char_right),
            key!(PageUp) => Leaf(Command::page_up),
            key!(PageDown) => Leaf(Command::page_down),
            key!(Home) => Leaf(Command::goto_line_start),
            key!(End) => Leaf(Command::goto_line_end_newline),
            ctrl!('x') => Leaf(Command::completion),
            ctrl!('w') => Leaf(Command::delete_word_backward),
        );
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
    let mut delta = std::mem::take(&mut config.keys);
    for (mode, keys) in &mut *config.keys {
        keys.extend(delta.remove(mode).unwrap_or_default().root)
    }
    config
}

#[test]
fn merge_partial_keys() {
    use helix_view::keyboard::{KeyCode, KeyModifiers};
    use KeyTrie::*;
    let config = Config {
        keys: Keymaps(hashmap! {
            Mode::Normal => Keymap::new(hashmap! {
                KeyEvent {
                    code: KeyCode::Char('i'),
                    modifiers: KeyModifiers::NONE,
                } => Leaf(Command::normal_mode),
                KeyEvent { // key that does not exist
                    code: KeyCode::Char('无'),
                    modifiers: KeyModifiers::NONE,
                } => Leaf(Command::insert_mode),
            }),
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
