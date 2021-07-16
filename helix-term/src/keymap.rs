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

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Keymaps(pub HashMap<Mode, HashMap<KeyEvent, Command>>);

impl Deref for Keymaps {
    type Target = HashMap<Mode, HashMap<KeyEvent, Command>>;

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
        let normal = hashmap!(
            key!('h') => Command::move_char_left,
            key!('j') => Command::move_line_down,
            key!('k') => Command::move_line_up,
            key!('l') => Command::move_char_right,

            key!(Left) => Command::move_char_left,
            key!(Down) => Command::move_line_down,
            key!(Up) => Command::move_line_up,
            key!(Right) => Command::move_char_right,

            key!('t') => Command::find_till_char,
            key!('f') => Command::find_next_char,
            key!('T') => Command::till_prev_char,
            key!('F') => Command::find_prev_char,
            // and matching set for select mode (extend)
            //
            key!('r') => Command::replace,
            key!('R') => Command::replace_with_yanked,

            key!(Home) => Command::goto_line_start,
            key!(End) => Command::goto_line_end,

            key!('w') => Command::move_next_word_start,
            key!('b') => Command::move_prev_word_start,
            key!('e') => Command::move_next_word_end,

            key!('W') => Command::move_next_long_word_start,
            key!('B') => Command::move_prev_long_word_start,
            key!('E') => Command::move_next_long_word_end,

            key!('v') => Command::select_mode,
            key!('g') => Command::goto_mode,
            key!(':') => Command::command_mode,

            key!('i') => Command::insert_mode,
            key!('I') => Command::prepend_to_line,
            key!('a') => Command::append_mode,
            key!('A') => Command::append_to_line,
            key!('o') => Command::open_below,
            key!('O') => Command::open_above,
            // [<space>  ]<space> equivalents too (add blank new line, no edit)


            key!('d') => Command::delete_selection,
            // TODO: also delete without yanking
            key!('c') => Command::change_selection,
            // TODO: also change delete without yanking

            // key!('r') => Command::replace_with_char,

            key!('s') => Command::select_regex,
            alt!('s') => Command::split_selection_on_newline,
            key!('S') => Command::split_selection,
            key!(';') => Command::collapse_selection,
            alt!(';') => Command::flip_selections,
            key!('%') => Command::select_all,
            key!('x') => Command::extend_line,
            key!('X') => Command::extend_to_line_bounds,
            // crop_to_whole_line


            key!('m') => Command::match_mode,
            key!('[') => Command::left_bracket_mode,
            key!(']') => Command::right_bracket_mode,

            key!('/') => Command::search,
            // ? for search_reverse
            key!('n') => Command::search_next,
            key!('N') => Command::extend_search_next,
            // N for search_prev
            key!('*') => Command::search_selection,

            key!('u') => Command::undo,
            key!('U') => Command::redo,

            key!('y') => Command::yank,
            // yank_all
            key!('p') => Command::paste_after,
            // paste_all
            key!('P') => Command::paste_before,

            key!('>') => Command::indent,
            key!('<') => Command::unindent,
            key!('=') => Command::format_selections,
            key!('J') => Command::join_selections,
            // TODO: conflicts hover/doc
            key!('K') => Command::keep_selections,
            // TODO: and another method for inverse

            // TODO: clashes with space mode
            key!(' ') => Command::keep_primary_selection,

            // key!('q') => Command::record_macro,
            // key!('Q') => Command::replay_macro,

            // ~ / apostrophe => change case
            // & align selections
            // _ trim selections

            // C / altC = copy (repeat) selections on prev/next lines

            key!(Esc) => Command::normal_mode,
            key!(PageUp) => Command::page_up,
            key!(PageDown) => Command::page_down,
            ctrl!('b') => Command::page_up,
            ctrl!('f') => Command::page_down,
            ctrl!('u') => Command::half_page_up,
            ctrl!('d') => Command::half_page_down,

            ctrl!('w') => Command::window_mode,

            // move under <space>c
            ctrl!('c') => Command::toggle_comments,
            key!('K') => Command::hover,

            // z family for save/restore/combine from/to sels from register

            // supposedly ctrl!('i') but did not work
            key!(Tab) => Command::jump_forward,
            ctrl!('o') => Command::jump_backward,
            // ctrl!('s') => Command::save_selection,

            key!(' ') => Command::space_mode,
            key!('z') => Command::view_mode,

            key!('"') => Command::select_register,
        );
        // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
        // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
        // because some selection operations can now be done from normal mode, some from select mode.
        let mut select = normal.clone();
        select.extend(
            hashmap!(
                key!('h') => Command::extend_char_left,
                key!('j') => Command::extend_line_down,
                key!('k') => Command::extend_line_up,
                key!('l') => Command::extend_char_right,

                key!(Left) => Command::extend_char_left,
                key!(Down) => Command::extend_line_down,
                key!(Up) => Command::extend_line_up,
                key!(Right) => Command::extend_char_right,

                key!('w') => Command::extend_next_word_start,
                key!('b') => Command::extend_prev_word_start,
                key!('e') => Command::extend_next_word_end,

                key!('t') => Command::extend_till_char,
                key!('f') => Command::extend_next_char,

                key!('T') => Command::extend_till_prev_char,
                key!('F') => Command::extend_prev_char,
                key!(Home) => Command::goto_line_start,
                key!(End) => Command::goto_line_end,
                key!(Esc) => Command::exit_select_mode,
            )
            .into_iter(),
        );

        Keymaps(hashmap!(
            // as long as you cast the first item, rust is able to infer the other cases
            // TODO: select could be normal mode with some bindings merged over
            Mode::Normal => normal,
            Mode::Select => select,
            Mode::Insert => hashmap!(
                key!(Esc) => Command::normal_mode as Command,
                key!(Backspace) => Command::delete_char_backward,
                key!(Delete) => Command::delete_char_forward,
                key!(Enter) => Command::insert_newline,
                key!(Tab) => Command::insert_tab,
                key!(Left) => Command::move_char_left,
                key!(Down) => Command::move_line_down,
                key!(Up) => Command::move_line_up,
                key!(Right) => Command::move_char_right,
                key!(PageUp) => Command::page_up,
                key!(PageDown) => Command::page_down,
                key!(Home) => Command::goto_line_start,
                key!(End) => Command::goto_line_end_newline,
                ctrl!('x') => Command::completion,
                ctrl!('w') => Command::delete_word_backward,
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
    use helix_view::keyboard::{KeyCode, KeyModifiers};
    let config = Config {
        keys: Keymaps(hashmap! {
            Mode::Normal => hashmap! {
                KeyEvent {
                    code: KeyCode::Char('i'),
                    modifiers: KeyModifiers::NONE,
                } => Command::normal_mode,
                KeyEvent { // key that does not exist
                    code: KeyCode::Char('无'),
                    modifiers: KeyModifiers::NONE,
                } => Command::insert_mode,
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
        Command::normal_mode
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
        Command::insert_mode
    );
    assert!(merged_config.keys.0.get(&Mode::Normal).unwrap().len() > 1);
    assert!(merged_config.keys.0.get(&Mode::Insert).unwrap().len() > 0);
}
