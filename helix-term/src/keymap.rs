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


pub type Keymap = HashMap<KeyEvent, KeyNode>;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum KeyNode {
    KeyCommand(Command),
    SubKeymap(Keymap),
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
        use KeyNode::*;
        let normal = hashmap!(
            key!('h') => KeyCommand(Command::move_char_left),
            key!('j') => KeyCommand(Command::move_line_down),
            key!('k') => KeyCommand(Command::move_line_up),
            key!('l') => KeyCommand(Command::move_char_right),

            key!(Left) => KeyCommand(Command::move_char_left),
            key!(Down) => KeyCommand(Command::move_line_down),
            key!(Up) => KeyCommand(Command::move_line_up),
            key!(Right) => KeyCommand(Command::move_char_right),

            key!('t') => KeyCommand(Command::find_till_char),
            key!('f') => KeyCommand(Command::find_next_char),
            key!('T') => KeyCommand(Command::till_prev_char),
            key!('F') => KeyCommand(Command::find_prev_char),
            // and matching set for select mode (extend)
            //
            key!('r') => KeyCommand(Command::replace),
            key!('R') => KeyCommand(Command::replace_with_yanked),

            key!(Home) => KeyCommand(Command::goto_line_start),
            key!(End) => KeyCommand(Command::goto_line_end),

            key!('w') => KeyCommand(Command::move_next_word_start),
            key!('b') => KeyCommand(Command::move_prev_word_start),
            key!('e') => KeyCommand(Command::move_next_word_end),

            key!('W') => KeyCommand(Command::move_next_long_word_start),
            key!('B') => KeyCommand(Command::move_prev_long_word_start),
            key!('E') => KeyCommand(Command::move_next_long_word_end),

            key!('v') => KeyCommand(Command::select_mode),
            key!('g') => KeyCommand(Command::goto_mode),
            key!(':') => KeyCommand(Command::command_mode),

            key!('i') => KeyCommand(Command::insert_mode),
            key!('I') => KeyCommand(Command::prepend_to_line),
            key!('a') => KeyCommand(Command::append_mode),
            key!('A') => KeyCommand(Command::append_to_line),
            key!('o') => KeyCommand(Command::open_below),
            key!('O') => KeyCommand(Command::open_above),
            // [<space>  ]<space> equivalents too (add blank new line, no edit)


            key!('d') => KeyCommand(Command::delete_selection),
            // TODO: also delete without yanking
            key!('c') => KeyCommand(Command::change_selection),
            // TODO: also change delete without yanking

            // key!('r') KeyCommand(ommand)::replace_with_char,

            key!('s') => KeyCommand(Command::select_regex),
            alt!('s') => KeyCommand(Command::split_selection_on_newline),
            key!('S') => KeyCommand(Command::split_selection),
            key!(';') => KeyCommand(Command::collapse_selection),
            alt!(';') => KeyCommand(Command::flip_selections),
            key!('%') => KeyCommand(Command::select_all),
            key!('x') => KeyCommand(Command::extend_line),
            key!('X') => KeyCommand(Command::extend_to_line_bounds),
            // crop_to_whole_line


            key!('m') => KeyCommand(Command::match_mode),
            key!('[') => KeyCommand(Command::left_bracket_mode),
            key!(']') => KeyCommand(Command::right_bracket_mode),

            key!('/') => KeyCommand(Command::search),
            // ? for search_reverse
            key!('n') => KeyCommand(Command::search_next),
            key!('N') => KeyCommand(Command::extend_search_next),
            // N for search_prev
            key!('*') => KeyCommand(Command::search_selection),

            key!('u') => KeyCommand(Command::undo),
            key!('U') => KeyCommand(Command::redo),

            key!('y') => KeyCommand(Command::yank),
            // yank_all
            key!('p') => KeyCommand(Command::paste_after),
            // paste_all
            key!('P') => KeyCommand(Command::paste_before),

            key!('>') => KeyCommand(Command::indent),
            key!('<') => KeyCommand(Command::unindent),
            key!('=') => KeyCommand(Command::format_selections),
            key!('J') => KeyCommand(Command::join_selections),
            // TODO: conflicts hover/doc
            key!('K') => KeyCommand(Command::keep_selections),
            // TODO: and another method for inverse

            // TODO: clashes with space mode
            key!(' ') => KeyCommand(Command::keep_primary_selection),

            // key!('q') KeyCommand(ommand)::record_macro,
            // key!('Q') KeyCommand(ommand)::replay_macro,

            // ~ / apostrophe => change case
            // & align selections
            // _ trim selections

            // C / altC = copy (repeat) selections on prev/next lines

            key!(Esc) => KeyCommand(Command::normal_mode),
            key!(PageUp) => KeyCommand(Command::page_up),
            key!(PageDown) => KeyCommand(Command::page_down),
            ctrl!('b') => KeyCommand(Command::page_up),
            ctrl!('f') => KeyCommand(Command::page_down),
            ctrl!('u') => KeyCommand(Command::half_page_up),
            ctrl!('d') => KeyCommand(Command::half_page_down),

            ctrl!('w') => KeyCommand(Command::window_mode),

            // move under <space>c
            ctrl!('c') => KeyCommand(Command::toggle_comments),
            key!('K') => KeyCommand(Command::hover),

            // z family for save/restore/combine from/to sels from register

            // supposedly ctrl!('i') but did not work
            key!(Tab) => KeyCommand(Command::jump_forward),
            ctrl!('o') => KeyCommand(Command::jump_backward),
            // ctrl!('s') KeyCommand(ommand)::save_selection,

            key!(' ') => KeyCommand(Command::space_mode),
            key!('z') => KeyCommand(Command::view_mode),

            key!('"') => KeyCommand(Command::select_register),
        );
        // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
        // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
        // because some selection operations can now be done from normal mode, some from select mode.
        let mut select = normal.clone();
        select.extend(
            hashmap!(
                key!('h') => KeyCommand(Command::extend_char_left),
                key!('j') => KeyCommand(Command::extend_line_down),
                key!('k') => KeyCommand(Command::extend_line_up),
                key!('l') => KeyCommand(Command::extend_char_right),

                key!(Left) => KeyCommand(Command::extend_char_left),
                key!(Down) => KeyCommand(Command::extend_line_down),
                key!(Up) => KeyCommand(Command::extend_line_up),
                key!(Right) => KeyCommand(Command::extend_char_right),

                key!('w') => KeyCommand(Command::extend_next_word_start),
                key!('b') => KeyCommand(Command::extend_prev_word_start),
                key!('e') => KeyCommand(Command::extend_next_word_end),

                key!('t') => KeyCommand(Command::extend_till_char),
                key!('f') => KeyCommand(Command::extend_next_char),

                key!('T') => KeyCommand(Command::extend_till_prev_char),
                key!('F') => KeyCommand(Command::extend_prev_char),
                key!(Home) => KeyCommand(Command::goto_line_start),
                key!(End) => KeyCommand(Command::goto_line_end),
                key!(Esc) => KeyCommand(Command::exit_select_mode),
            )
            .into_iter(),
        );

        Keymaps(hashmap!(
            Mode::Normal => normal,
            Mode::Select => select,
            Mode::Insert => hashmap!(
                key!(Esc) => KeyCommand(Command::normal_mode),
                key!(Backspace) => KeyCommand(Command::delete_char_backward),
                key!(Delete) => KeyCommand(Command::delete_char_forward),
                key!(Enter) => KeyCommand(Command::insert_newline),
                key!(Tab) => KeyCommand(Command::insert_tab),
                key!(Left) => KeyCommand(Command::move_char_left),
                key!(Down) => KeyCommand(Command::move_line_down),
                key!(Up) => KeyCommand(Command::move_line_up),
                key!(Right) => KeyCommand(Command::move_char_right),
                key!(PageUp) => KeyCommand(Command::page_up),
                key!(PageDown) => KeyCommand(Command::page_down),
                key!(Home) => KeyCommand(Command::goto_line_start),
                key!(End) => KeyCommand(Command::goto_line_end_newline),
                ctrl!('x') => KeyCommand(Command::completion),
                ctrl!('w') => KeyCommand(Command::delete_word_backward),
            ),
        ))
    }
}

/// Merge default config keys with user overwritten keys for custom
/// user config.
pub fn merge_keys(mut config: Config) -> Config {
    let mut delta = std::mem::take(&mut config.keys);
    for (mode, keys) in &mut *config.keys {
        keys.extend(delta.remove(mode).unwrap_or_default());
    }
    config
}

#[test]
fn merge_partial_keys() {
    use KeyNode::*;
    use helix_view::keyboard::{KeyCode, KeyModifiers};
    let config = Config {
        keys: Keymaps(hashmap! {
            Mode::Normal => hashmap! {
                KeyEvent {
                    code: KeyCode::Char('i'),
                    modifiers: KeyModifiers::NONE,
                } => KeyCommand(Command::normal_mode),
                KeyEvent { // key that does not exist
                    code: KeyCode::Char('无'),
                    modifiers: KeyModifiers::NONE,
                } => KeyCommand(Command::insert_mode),
            },
        }),
        ..Default::default()
    };
    let merged_config = merge_keys(config.clone());
    assert_ne!(config, merged_config);
    assert_eq!(
        *merged_config
            .keys
            .0
            .get(&Mode::Normal)
            .unwrap()
            .get(&KeyEvent {
                code: KeyCode::Char('i'),
                modifiers: KeyModifiers::NONE
            })
            .unwrap(),
        KeyCommand(Command::normal_mode)
    );
    assert_eq!(
        *merged_config
            .keys
            .0
            .get(&Mode::Normal)
            .unwrap()
            .get(&KeyEvent {
                code: KeyCode::Char('无'),
                modifiers: KeyModifiers::NONE
            })
            .unwrap(),
        KeyCommand(Command::insert_mode)
    );
    assert!(merged_config.keys.0.get(&Mode::Normal).unwrap().len() > 1);
    assert!(merged_config.keys.0.get(&Mode::Insert).unwrap().len() > 0);
}
