use crate::commands;
pub use crate::commands::Command;
use anyhow::{anyhow, Error, Result};
use helix_core::hashmap;
use helix_view::document::Mode;
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

// Kakoune-inspired:
// mode = {
//      normal = {
//          q = record_macro
//          w = (next) word
//          W = next WORD
//          e = end of word
//          E = end of WORD
//          r = replace
//          R = replace with yanked
//          t = 'till char
//          y = yank
//          u = undo
//          U = redo
//          i = insert
//          I = INSERT (start of line)
//          o = open below (insert on new line below)
//          O = open above (insert on new line above)
//          p = paste (before cursor)
//          P = PASTE (after cursor)
//          ` =
//          [ = select to text object start (alt = select whole object)
//          ] = select to text object end
//          { = extend to inner object start
//          } = extend to inner object end
//          a = append
//          A = APPEND (end of line)
//          s = split
//          S = select
//          d = delete()
//          f = find_char()
//          g = goto (gg, G, gc, gd, etc)
//
//          h = move_char_left(n)   || arrow-left  = move_char_left(n)
//          j = move_line_down(n)   || arrow-down  = move_line_down(n)
//          k = move_line_up(n)     || arrow_up    = move_line_up(n)
//          l = move_char_right(n)  || arrow-right = move_char_right(n)
//          : = command line
//          ; = collapse selection to cursor
//          " = use register
//          ` = convert case? (to lower) (alt = swap case)
//          ~ = convert to upper case
//          . = repeat last command
//          \ = disable hook?
//          / = search
//          > = indent
//          < = deindent
//          % = select whole buffer (in vim = jump to matching bracket)
//          * = search pattern in selection
//          ( = rotate main selection backward
//          ) = rotate main selection forward
//          - = trim selections? (alt = merge contiguous sel together)
//          @ = convert tabs to spaces
//          & = align cursor
//          ? = extend to next given regex match (alt = to prev)
//
//          in kakoune these are alt-h alt-l / gh gl
//                              select from curs to begin end / move curs to begin end
//          0 = start of line
//          ^ = start of line(first non blank char) || Home  = start of line(first non blank char)
//          $ = end of line                         || End   = end of line
//
//          z = save selections
//          Z = restore selections
//          x = select line
//          X = extend line
//          c = change selected text
//          C = copy selection?
//          v = view menu (viewport manipulation)
//          b = select to previous word start
//          B = select to previous WORD start
//
//
//
//
//
//
//          = = align?
//          + =
//      }
//
//      gd = goto definition
//      gr = goto reference
//      [d = previous diagnostic
//      d] = next diagnostic
//      [D = first diagnostic
//      D] = last diagnostic
// }

// #[cfg(feature = "term")]
pub use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Debug)]
pub struct Keymap(pub HashMap<KeyEvent, Command>);
#[derive(Clone, Debug)]
pub struct Keymaps(pub HashMap<Mode, Keymap>);

#[macro_export]
macro_rules! key {
    ($key:ident) => {
        KeyEvent {
            code: KeyCode::$key,
            modifiers: KeyModifiers::NONE,
        }
    };
    ($($ch:tt)*) => {
        KeyEvent {
            code: KeyCode::Char($($ch)*),
            modifiers: KeyModifiers::NONE,
        }
    };
}

macro_rules! ctrl {
    ($($ch:tt)*) => {
        KeyEvent {
            code: KeyCode::Char($($ch)*),
            modifiers: KeyModifiers::CONTROL,
        }
    };
}

macro_rules! alt {
    ($($ch:tt)*) => {
        KeyEvent {
            code: KeyCode::Char($($ch)*),
            modifiers: KeyModifiers::ALT,
        }
    };
}

impl Default for Keymaps {
    fn default() -> Self {
        let normal = Keymap(hashmap!(
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

            key!(Home) => Command::move_line_start,
            key!(End) => Command::move_line_end,

            key!('w') => Command::move_next_word_start,
            key!('b') => Command::move_prev_word_start,
            key!('e') => Command::move_next_word_end,

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
            key!('x') => Command::select_line,
            key!('X') => Command::extend_line,
            // or select mode X?
            // extend_to_whole_line, crop_to_whole_line


            key!('m') => Command::match_brackets,
            // TODO: refactor into
            // key!('m') => commands::select_to_matching,
            // key!('M') => commands::back_select_to_matching,
            // select mode extend equivalents

            // key!('.') => commands::repeat_insert,
            // repeat_select

            // TODO: figure out what key to use
            // key!('[') => Command::expand_selection, ??
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
        ));
        // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
        // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
        // because some selection operations can now be done from normal mode, some from select mode.
        let mut select = normal.clone();
        select.0.extend(
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
                key!(Home) => Command::extend_line_start,
                key!(End) => Command::extend_line_end,
                key!(Esc) => Command::exit_select_mode,
            )
            .into_iter(),
        );

        Keymaps(hashmap!(
            // as long as you cast the first item, rust is able to infer the other cases
            // TODO: select could be normal mode with some bindings merged over
            Mode::Normal => normal,
            Mode::Select => select,
            Mode::Insert => Keymap(hashmap!(
                key!(Esc) => Command::normal_mode as Command,
                key!(Backspace) => Command::delete_char_backward,
                key!(Delete) => Command::delete_char_forward,
                key!(Enter) => Command::insert_newline,
                key!(Tab) => Command::insert_tab,
                ctrl!('x') => Command::completion,
                ctrl!('w') => Command::delete_word_backward,
            )),
        ))
    }
}

// Newtype wrapper over keys to allow toml serialization/parsing
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub struct RepresentableKeyEvent(pub KeyEvent);
impl Display for RepresentableKeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(key) = self;
        f.write_fmt(format_args!(
            "{}{}{}",
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                "S-"
            } else {
                ""
            },
            if key.modifiers.contains(KeyModifiers::ALT) {
                "A-"
            } else {
                ""
            },
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                "C-"
            } else {
                ""
            },
        ))?;
        match key.code {
            KeyCode::Backspace => f.write_str("backspace")?,
            KeyCode::Enter => f.write_str("ret")?,
            KeyCode::Left => f.write_str("left")?,
            KeyCode::Right => f.write_str("right")?,
            KeyCode::Up => f.write_str("up")?,
            KeyCode::Down => f.write_str("down")?,
            KeyCode::Home => f.write_str("home")?,
            KeyCode::End => f.write_str("end")?,
            KeyCode::PageUp => f.write_str("pageup")?,
            KeyCode::PageDown => f.write_str("pagedown")?,
            KeyCode::Tab => f.write_str("tab")?,
            KeyCode::BackTab => f.write_str("backtab")?,
            KeyCode::Delete => f.write_str("del")?,
            KeyCode::Insert => f.write_str("ins")?,
            KeyCode::Null => f.write_str("null")?,
            KeyCode::Esc => f.write_str("esc")?,
            KeyCode::Char('<') => f.write_str("lt")?,
            KeyCode::Char('>') => f.write_str("gt")?,
            KeyCode::Char('+') => f.write_str("plus")?,
            KeyCode::Char('-') => f.write_str("minus")?,
            KeyCode::Char(';') => f.write_str("semicolon")?,
            KeyCode::Char('%') => f.write_str("percent")?,
            KeyCode::F(i) => f.write_fmt(format_args!("F{}", i))?,
            KeyCode::Char(c) => f.write_fmt(format_args!("{}", c))?,
        };
        Ok(())
    }
}

impl FromStr for RepresentableKeyEvent {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens: Vec<_> = s.split('-').collect();
        let code = match tokens.pop().ok_or_else(|| anyhow!("Missing key code"))? {
            "backspace" => KeyCode::Backspace,
            "space" => KeyCode::Char(' '),
            "ret" => KeyCode::Enter,
            "lt" => KeyCode::Char('<'),
            "gt" => KeyCode::Char('>'),
            "plus" => KeyCode::Char('+'),
            "minus" => KeyCode::Char('-'),
            "semicolon" => KeyCode::Char(';'),
            "percent" => KeyCode::Char('%'),
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "del" => KeyCode::Delete,
            "ins" => KeyCode::Insert,
            "null" => KeyCode::Null,
            "esc" => KeyCode::Esc,
            single if single.len() == 1 => KeyCode::Char(single.chars().next().unwrap()),
            function if function.len() > 1 && function.starts_with('F') => {
                let function: String = function.chars().skip(1).collect();
                let function = str::parse::<u8>(&function)?;
                (function > 0 && function < 13)
                    .then(|| KeyCode::F(function))
                    .ok_or_else(|| anyhow!("Invalid function key '{}'", function))?
            }
            invalid => return Err(anyhow!("Invalid key code '{}'", invalid)),
        };

        let mut modifiers = KeyModifiers::empty();
        for token in tokens {
            let flag = match token {
                "S" => KeyModifiers::SHIFT,
                "A" => KeyModifiers::ALT,
                "C" => KeyModifiers::CONTROL,
                _ => return Err(anyhow!("Invalid key modifier '{}-'", token)),
            };

            if modifiers.contains(flag) {
                return Err(anyhow!("Repeated key modifier '{}-'", token));
            }
            modifiers.insert(flag);
        }

        Ok(RepresentableKeyEvent(KeyEvent { code, modifiers }))
    }
}

pub fn parse_keymaps(toml_keymaps: &HashMap<String, HashMap<String, String>>) -> Result<Keymaps> {
    let mut keymaps = Keymaps::default();

    for (mode, map) in toml_keymaps {
        let mode = Mode::from_str(&mode)?;
        for (key, command) in map {
            let key = str::parse::<RepresentableKeyEvent>(&key)?;
            let command = str::parse::<Command>(&command)?;
            keymaps.0.get_mut(&mode).unwrap().0.insert(key.0, command);
        }
    }
    Ok(keymaps)
}

impl Deref for Keymap {
    type Target = HashMap<KeyEvent, Command>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for Keymaps {
    type Target = HashMap<Mode, Keymap>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Keymap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for Keymaps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod test {
    use crate::config::Config;

    use super::*;

    impl PartialEq for Command {
        fn eq(&self, other: &Self) -> bool {
            self.name() == other.name()
        }
    }

    #[test]
    fn parsing_keymaps_config_file() {
        let sample_keymaps = r#"
            [keys.insert]
            y = "move_line_down"
            S-C-a = "delete_selection"

            [keys.normal]
            A-F12 = "move_next_word_end"
        "#;

        let config = Config::from_str(sample_keymaps).unwrap();
        assert_eq!(
            *config
                .keymaps
                .0
                .get(&Mode::Insert)
                .unwrap()
                .0
                .get(&KeyEvent {
                    code: KeyCode::Char('y'),
                    modifiers: KeyModifiers::NONE
                })
                .unwrap(),
            Command::move_line_down
        );
        assert_eq!(
            *config
                .keymaps
                .0
                .get(&Mode::Insert)
                .unwrap()
                .0
                .get(&KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL
                })
                .unwrap(),
            Command::delete_selection
        );
        assert_eq!(
            *config
                .keymaps
                .0
                .get(&Mode::Normal)
                .unwrap()
                .0
                .get(&KeyEvent {
                    code: KeyCode::F(12),
                    modifiers: KeyModifiers::ALT
                })
                .unwrap(),
            Command::move_next_word_end
        );
    }

    #[test]
    fn parsing_unmodified_keys() {
        assert_eq!(
            str::parse::<RepresentableKeyEvent>("backspace").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE
            })
        );

        assert_eq!(
            str::parse::<RepresentableKeyEvent>("left").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE
            })
        );

        assert_eq!(
            str::parse::<RepresentableKeyEvent>(",").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::Char(','),
                modifiers: KeyModifiers::NONE
            })
        );

        assert_eq!(
            str::parse::<RepresentableKeyEvent>("w").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::NONE
            })
        );

        assert_eq!(
            str::parse::<RepresentableKeyEvent>("F12").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::F(12),
                modifiers: KeyModifiers::NONE
            })
        );
    }

    fn parsing_modified_keys() {
        assert_eq!(
            str::parse::<RepresentableKeyEvent>("S-minus").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::Char('-'),
                modifiers: KeyModifiers::SHIFT
            })
        );

        assert_eq!(
            str::parse::<RepresentableKeyEvent>("C-A-S-F12").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::F(12),
                modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT
            })
        );

        assert_eq!(
            str::parse::<RepresentableKeyEvent>("S-C-2").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::F(2),
                modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL
            })
        );
    }

    #[test]
    fn parsing_nonsensical_keys_fails() {
        assert!(str::parse::<RepresentableKeyEvent>("F13").is_err());
        assert!(str::parse::<RepresentableKeyEvent>("F0").is_err());
        assert!(str::parse::<RepresentableKeyEvent>("aaa").is_err());
        assert!(str::parse::<RepresentableKeyEvent>("S-S-a").is_err());
        assert!(str::parse::<RepresentableKeyEvent>("C-A-S-C-1").is_err());
        assert!(str::parse::<RepresentableKeyEvent>("FU").is_err());
        assert!(str::parse::<RepresentableKeyEvent>("123").is_err());
        assert!(str::parse::<RepresentableKeyEvent>("S--").is_err());
    }
}
