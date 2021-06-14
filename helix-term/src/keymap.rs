use crate::commands;
pub use crate::commands::Command;
use anyhow::{anyhow, Error, Result};
use helix_core::hashmap;
use helix_view::document::Mode;
use std::{collections::HashMap, fmt::Display, str::FromStr};

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

pub type Keymap = HashMap<KeyEvent, Command>;
pub type Keymaps = HashMap<Mode, Keymap>;

#[macro_export]
macro_rules! key {
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

pub fn default() -> Keymaps {
    let normal = hashmap!(
        key!('h') => commands::Command::move_char_left,
        key!('j') => commands::Command::move_line_down,
        key!('k') => commands::Command::move_line_up,
        key!('l') => commands::Command::move_char_right,

        KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE
        } => commands::Command::move_char_left,
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE
        } => commands::Command::move_line_down,
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE
        } => commands::Command::move_line_up,
        KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE
        } => commands::Command::move_char_right,

        key!('t') => commands::Command::find_till_char,
        key!('f') => commands::Command::find_next_char,
        key!('T') => commands::Command::till_prev_char,
        key!('F') => commands::Command::find_prev_char,
        // and matching set for select mode (extend)
        //
        key!('r') => commands::Command::replace,
        key!('R') => commands::Command::replace_with_yanked,

        KeyEvent {
            code: KeyCode::Home,
            modifiers: KeyModifiers::NONE
        } => commands::Command::move_line_start,

        KeyEvent {
            code: KeyCode::End,
            modifiers: KeyModifiers::NONE
        } => commands::Command::move_line_end,

        key!('w') => commands::Command::move_next_word_start,
        key!('b') => commands::Command::move_prev_word_start,
        key!('e') => commands::Command::move_next_word_end,

        key!('v') => commands::Command::select_mode,
        key!('g') => commands::Command::goto_mode,
        key!(':') => commands::Command::command_mode,

        key!('i') => commands::Command::insert_mode,
        key!('I') => commands::Command::prepend_to_line,
        key!('a') => commands::Command::append_mode,
        key!('A') => commands::Command::append_to_line,
        key!('o') => commands::Command::open_below,
        key!('O') => commands::Command::open_above,
        // [<space>  ]<space> equivalents too (add blank new line, no edit)


        key!('d') => commands::Command::delete_selection,
        // TODO: also delete without yanking
        key!('c') => commands::Command::change_selection,
        // TODO: also change delete without yanking

        // key!('r') => commands::Command::replace_with_char,

        key!('s') => commands::Command::select_regex,
        alt!('s') => commands::Command::split_selection_on_newline,
        key!('S') => commands::Command::split_selection,
        key!(';') => commands::Command::collapse_selection,
        alt!(';') => commands::Command::flip_selections,
        key!('%') => commands::Command::select_all,
        key!('x') => commands::Command::select_line,
        key!('X') => commands::Command::extend_line,
        // or select mode X?
        // extend_to_whole_line, crop_to_whole_line


        key!('m') => commands::Command::match_brackets,
        // TODO: refactor into
        // key!('m') => commands::select_to_matching,
        // key!('M') => commands::back_select_to_matching,
        // select mode extend equivalents

        // key!('.') => commands::repeat_insert,
        // repeat_select

        // TODO: figure out what key to use
        // key!('[') => commands::Command::expand_selection, ??
        key!('[') => commands::Command::left_bracket_mode,
        key!(']') => commands::Command::right_bracket_mode,

        key!('/') => commands::Command::search,
        // ? for search_reverse
        key!('n') => commands::Command::search_next,
        key!('N') => commands::Command::extend_search_next,
        // N for search_prev
        key!('*') => commands::Command::search_selection,

        key!('u') => commands::Command::undo,
        key!('U') => commands::Command::redo,

        key!('y') => commands::Command::yank,
        // yank_all
        key!('p') => commands::Command::paste_after,
        // paste_all
        key!('P') => commands::Command::paste_before,

        key!('>') => commands::Command::indent,
        key!('<') => commands::Command::unindent,
        key!('=') => commands::Command::format_selections,
        key!('J') => commands::Command::join_selections,
        // TODO: conflicts hover/doc
        key!('K') => commands::Command::keep_selections,
        // TODO: and another method for inverse

        // TODO: clashes with space mode
        key!(' ') => commands::Command::keep_primary_selection,

        // key!('q') => commands::Command::record_macro,
        // key!('Q') => commands::Command::replay_macro,

        // ~ / apostrophe => change case
        // & align selections
        // _ trim selections

        // C / altC = copy (repeat) selections on prev/next lines

        KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE
        } => commands::Command::normal_mode,
        KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::NONE
        } => commands::Command::page_up,
        ctrl!('b') => commands::Command::page_up,
        KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE
        } => commands::Command::page_down,
        ctrl!('f') => commands::Command::page_down,
        ctrl!('u') => commands::Command::half_page_up,
        ctrl!('d') => commands::Command::half_page_down,

        ctrl!('w') => commands::Command::window_mode,

        // move under <space>c
        ctrl!('c') => commands::Command::toggle_comments,
        key!('K') => commands::Command::hover,

        // z family for save/restore/combine from/to sels from register

        KeyEvent { // supposedly ctrl!('i') but did not work
            code: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
        } => commands::Command::jump_forward,
        ctrl!('o') => commands::Command::jump_backward,
        // ctrl!('s') => commands::Command::save_selection,

        key!(' ') => commands::Command::space_mode,
        key!('z') => commands::Command::view_mode,

        key!('"') => commands::Command::select_register,
    );
    // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
    // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
    // because some selection operations can now be done from normal mode, some from select mode.
    let mut select = normal.clone();
    select.extend(
        hashmap!(
            key!('h') => commands::Command::extend_char_left,
            key!('j') => commands::Command::extend_line_down,
            key!('k') => commands::Command::extend_line_up,
            key!('l') => commands::Command::extend_char_right,

            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE
            } => commands::Command::extend_char_left,
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE
            } => commands::Command::extend_line_down,
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE
            } => commands::Command::extend_line_up,
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE
            } => commands::Command::extend_char_right,

            key!('w') => commands::Command::extend_next_word_start,
            key!('b') => commands::Command::extend_prev_word_start,
            key!('e') => commands::Command::extend_next_word_end,

            key!('t') => commands::Command::extend_till_char,
            key!('f') => commands::Command::extend_next_char,

            key!('T') => commands::Command::extend_till_prev_char,
            key!('F') => commands::Command::extend_prev_char,
            KeyEvent {
                code: KeyCode::Home,
                modifiers: KeyModifiers::NONE
            } => commands::Command::extend_line_start,
            KeyEvent {
                code: KeyCode::End,
                modifiers: KeyModifiers::NONE
            } => commands::Command::extend_line_end,
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE
            } => commands::Command::exit_select_mode,
        )
        .into_iter(),
    );

    hashmap!(
        // as long as you cast the first item, rust is able to infer the other cases
        // TODO: select could be normal mode with some bindings merged over
        Mode::Normal => normal,
        Mode::Select => select,
        Mode::Insert => hashmap!(
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE
            } => commands::Command::normal_mode,
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE
            } => commands::Command::delete_char_backward,
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE
            } => commands::Command::delete_char_forward,
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE
            } => commands::Command::insert_newline,
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE
            } => commands::Command::insert_tab,

            ctrl!('x') => commands::Command::completion,
            ctrl!('w') => commands::Command::delete_word_backward,
        ),
    )
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
            function if function.len() > 1 && &function[0..1] == "F" => {
                let function = str::parse::<u8>(&function[1..])?;
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

pub fn parse_remaps(remaps: &str) -> Result<Keymaps> {
    type TomlCompatibleRemaps = HashMap<String, HashMap<String, String>>;
    let toml_remaps: TomlCompatibleRemaps = toml::from_str(remaps)?;
    let mut remaps = Keymaps::new();

    for (mode, map) in toml_remaps {
        let mode = Mode::from_str(&mode)?;
        let mut remap = Keymap::new();

        for (key, command) in map {
            let key = str::parse::<RepresentableKeyEvent>(&key)?;
            let command = str::parse::<Command>(&command)?;
            remap.insert(key.0, command);
        }
        remaps.insert(mode, remap);
    }
    Ok(remaps)
}

#[cfg(test)]
mod test {
    use super::*;

    impl PartialEq for Command {
        fn eq(&self, other: &Self) -> bool {
            self.name() == other.name()
        }
    }

    #[test]
    fn parsing_remaps_file() {
        let sample_remaps = r#"
            [Insert]
            y = "move_line_down"
            S-C-a = "delete_selection"

            [Normal]
            A-F12 = "move_next_word_end"
        "#;

        let parsed = parse_remaps(sample_remaps).unwrap();
        assert_eq!(
            parsed,
            hashmap!(
                Mode::Insert => hashmap!(
                    KeyEvent { code: KeyCode::Char('y'), modifiers: KeyModifiers::NONE }
                        => commands::Command::move_line_down,
                    KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL }
                        => commands::Command::delete_selection,
                ),
                Mode::Normal => hashmap!(
                    KeyEvent { code: KeyCode::F(12), modifiers: KeyModifiers::ALT }
                        => commands::Command::move_next_word_end,
                )
            )
        )
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
