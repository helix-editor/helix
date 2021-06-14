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

pub type Remap = HashMap<KeyEvent, KeyEvent>;
pub type Remaps = HashMap<Mode, Remap>;

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
        key!('h') => commands::Command::MOVE_CHAR_LEFT,
        key!('j') => commands::Command::MOVE_LINE_DOWN,
        key!('k') => commands::Command::MOVE_LINE_UP,
        key!('l') => commands::Command::MOVE_CHAR_RIGHT,

        KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE
        } => commands::Command::MOVE_CHAR_LEFT,
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE
        } => commands::Command::MOVE_LINE_DOWN,
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE
        } => commands::Command::MOVE_LINE_UP,
        KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE
        } => commands::Command::MOVE_CHAR_RIGHT,

        key!('t') => commands::Command::FIND_TILL_CHAR,
        key!('f') => commands::Command::FIND_NEXT_CHAR,
        key!('T') => commands::Command::TILL_PREV_CHAR,
        key!('F') => commands::Command::FIND_PREV_CHAR,
        // and matching set for select mode (extend)
        //
        key!('r') => commands::Command::REPLACE,
        key!('R') => commands::Command::REPLACE_WITH_YANKED,

        KeyEvent {
            code: KeyCode::Home,
            modifiers: KeyModifiers::NONE
        } => commands::Command::MOVE_LINE_START,

        KeyEvent {
            code: KeyCode::End,
            modifiers: KeyModifiers::NONE
        } => commands::Command::MOVE_LINE_END,

        key!('w') => commands::Command::MOVE_NEXT_WORD_START,
        key!('b') => commands::Command::MOVE_PREV_WORD_START,
        key!('e') => commands::Command::MOVE_NEXT_WORD_END,

        key!('v') => commands::Command::SELECT_MODE,
        key!('g') => commands::Command::GOTO_MODE,
        key!(':') => commands::Command::COMMAND_MODE,

        key!('i') => commands::Command::INSERT_MODE,
        key!('I') => commands::Command::PREPEND_TO_LINE,
        key!('a') => commands::Command::APPEND_MODE,
        key!('A') => commands::Command::APPEND_TO_LINE,
        key!('o') => commands::Command::OPEN_BELOW,
        key!('O') => commands::Command::OPEN_ABOVE,
        // [<space>  ]<space> equivalents too (add blank new line, no edit)


        key!('d') => commands::Command::DELETE_SELECTION,
        // TODO: also delete without yanking
        key!('c') => commands::Command::CHANGE_SELECTION,
        // TODO: also change delete without yanking

        // key!('r') => commands::Command::REPLACE_WITH_CHAR,

        key!('s') => commands::Command::SELECT_REGEX,
        alt!('s') => commands::Command::SPLIT_SELECTION_ON_NEWLINE,
        key!('S') => commands::Command::SPLIT_SELECTION,
        key!(';') => commands::Command::COLLAPSE_SELECTION,
        alt!(';') => commands::Command::FLIP_SELECTIONS,
        key!('%') => commands::Command::SELECT_ALL,
        key!('x') => commands::Command::SELECT_LINE,
        key!('X') => commands::Command::EXTEND_LINE,
        // or select mode X?
        // extend_to_whole_line, crop_to_whole_line


        key!('m') => commands::Command::MATCH_BRACKETS,
        // TODO: refactor into
        // key!('m') => commands::select_to_matching,
        // key!('M') => commands::back_select_to_matching,
        // select mode extend equivalents

        // key!('.') => commands::repeat_insert,
        // repeat_select

        // TODO: figure out what key to use
        // key!('[') => commands::Command::EXPAND_SELECTION, ??
        key!('[') => commands::Command::LEFT_BRACKET_MODE,
        key!(']') => commands::Command::RIGHT_BRACKET_MODE,

        key!('/') => commands::Command::SEARCH,
        // ? for search_reverse
        key!('n') => commands::Command::SEARCH_NEXT,
        key!('N') => commands::Command::EXTEND_SEARCH_NEXT,
        // N for search_prev
        key!('*') => commands::Command::SEARCH_SELECTION,

        key!('u') => commands::Command::UNDO,
        key!('U') => commands::Command::REDO,

        key!('y') => commands::Command::YANK,
        // yank_all
        key!('p') => commands::Command::PASTE_AFTER,
        // paste_all
        key!('P') => commands::Command::PASTE_BEFORE,

        key!('>') => commands::Command::INDENT,
        key!('<') => commands::Command::UNINDENT,
        key!('=') => commands::Command::FORMAT_SELECTIONS,
        key!('J') => commands::Command::JOIN_SELECTIONS,
        // TODO: conflicts hover/doc
        key!('K') => commands::Command::KEEP_SELECTIONS,
        // TODO: and another method for inverse

        // TODO: clashes with space mode
        key!(' ') => commands::Command::KEEP_PRIMARY_SELECTION,

        // key!('q') => commands::Command::RECORD_MACRO,
        // key!('Q') => commands::Command::REPLAY_MACRO,

        // ~ / apostrophe => change case
        // & align selections
        // _ trim selections

        // C / altC = copy (repeat) selections on prev/next lines

        KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE
        } => commands::Command::NORMAL_MODE,
        KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::NONE
        } => commands::Command::PAGE_UP,
        ctrl!('b') => commands::Command::PAGE_UP,
        KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE
        } => commands::Command::PAGE_DOWN,
        ctrl!('f') => commands::Command::PAGE_DOWN,
        ctrl!('u') => commands::Command::HALF_PAGE_UP,
        ctrl!('d') => commands::Command::HALF_PAGE_DOWN,

        ctrl!('w') => commands::Command::WINDOW_MODE,

        // move under <space>c
        ctrl!('c') => commands::Command::TOGGLE_COMMENTS,
        key!('K') => commands::Command::HOVER,

        // z family for save/restore/combine from/to sels from register

        KeyEvent { // supposedly ctrl!('i') but did not work
            code: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
        } => commands::Command::JUMP_FORWARD,
        ctrl!('o') => commands::Command::JUMP_BACKWARD,
        // ctrl!('s') => commands::Command::SAVE_SELECTION,

        key!(' ') => commands::Command::SPACE_MODE,
        key!('z') => commands::Command::VIEW_MODE,

        key!('"') => commands::Command::SELECT_REGISTER,
    );
    // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
    // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
    // because some selection operations can now be done from normal mode, some from select mode.
    let mut select = normal.clone();
    select.extend(
        hashmap!(
            key!('h') => commands::Command::EXTEND_CHAR_LEFT,
            key!('j') => commands::Command::EXTEND_LINE_DOWN,
            key!('k') => commands::Command::EXTEND_LINE_UP,
            key!('l') => commands::Command::EXTEND_CHAR_RIGHT,

            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE
            } => commands::Command::EXTEND_CHAR_LEFT,
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE
            } => commands::Command::EXTEND_LINE_DOWN,
            KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE
            } => commands::Command::EXTEND_LINE_UP,
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE
            } => commands::Command::EXTEND_CHAR_RIGHT,

            key!('w') => commands::Command::EXTEND_NEXT_WORD_START,
            key!('b') => commands::Command::EXTEND_PREV_WORD_START,
            key!('e') => commands::Command::EXTEND_NEXT_WORD_END,

            key!('t') => commands::Command::EXTEND_TILL_CHAR,
            key!('f') => commands::Command::EXTEND_NEXT_CHAR,

            key!('T') => commands::Command::EXTEND_TILL_PREV_CHAR,
            key!('F') => commands::Command::EXTEND_PREV_CHAR,
            KeyEvent {
                code: KeyCode::Home,
                modifiers: KeyModifiers::NONE
            } => commands::Command::EXTEND_LINE_START,
            KeyEvent {
                code: KeyCode::End,
                modifiers: KeyModifiers::NONE
            } => commands::Command::EXTEND_LINE_END,
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE
            } => commands::Command::EXIT_SELECT_MODE,
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
            } => commands::Command::NORMAL_MODE,
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE
            } => commands::Command::DELETE_CHAR_BACKWARD,
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE
            } => commands::Command::DELETE_CHAR_FORWARD,
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE
            } => commands::Command::INSERT_NEWLINE,
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE
            } => commands::Command::INSERT_TAB,

            ctrl!('x') => commands::Command::COMPLETION,
            ctrl!('w') => commands::Command::DELETE_WORD_BACKWARD,
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
            KeyCode::Backspace => f.write_str("Bs")?,
            KeyCode::Enter => f.write_str("Enter")?,
            KeyCode::Left => f.write_str("Left")?,
            KeyCode::Right => f.write_str("Right")?,
            KeyCode::Up => f.write_str("Up")?,
            KeyCode::Down => f.write_str("Down")?,
            KeyCode::Home => f.write_str("Home")?,
            KeyCode::End => f.write_str("End")?,
            KeyCode::PageUp => f.write_str("PageUp")?,
            KeyCode::PageDown => f.write_str("PageDown")?,
            KeyCode::Tab => f.write_str("Tab")?,
            KeyCode::BackTab => f.write_str("BackTab")?,
            KeyCode::Delete => f.write_str("Del")?,
            KeyCode::Insert => f.write_str("Insert")?,
            KeyCode::F(i) => f.write_fmt(format_args!("F{}", i))?,
            KeyCode::Char(c) => f.write_fmt(format_args!("{}", c))?,
            KeyCode::Null => f.write_str("Null")?,
            KeyCode::Esc => f.write_str("Esc")?,
        };
        Ok(())
    }
}

impl FromStr for RepresentableKeyEvent {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens: Vec<_> = s.split('-').collect();
        let code = match tokens.pop().ok_or_else(|| anyhow!("Missing key code"))? {
            "Bs" => KeyCode::Backspace,
            "Enter" => KeyCode::Enter,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Up" => KeyCode::Down,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PageUp" => KeyCode::PageUp,
            "PageDown" => KeyCode::PageDown,
            "Tab" => KeyCode::Tab,
            "BackTab" => KeyCode::BackTab,
            "Del" => KeyCode::Delete,
            "Insert" => KeyCode::Insert,
            "Null" => KeyCode::Null,
            "Esc" => KeyCode::Esc,
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

pub fn parse_remaps(remaps: &str) -> Result<Remaps> {
    type TomlCompatibleRemaps = HashMap<String, HashMap<String, String>>;
    let toml_remaps: TomlCompatibleRemaps = toml::from_str(remaps)?;
    let mut remaps = Remaps::new();

    for (mode, map) in toml_remaps {
        let mode = Mode::from_str(&mode)?;
        let mut remap = Remap::new();

        for (source_key, target_key) in map {
            let source_key = str::parse::<RepresentableKeyEvent>(&source_key)?;
            let target_key = str::parse::<RepresentableKeyEvent>(&target_key)?;
            remap.insert(source_key.0, target_key.0);
        }
        remaps.insert(mode, remap);
    }
    Ok(remaps)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parsing_remaps_file() {
        let sample_remaps = r#"
            [Insert]
            y = "x"
            S-C-a = "F12"

            [Normal]
            A-F12 = "S-C-w"
        "#;

        let parsed = parse_remaps(sample_remaps).unwrap();
        assert_eq!(
            parsed,
            hashmap!(
                Mode::Insert => hashmap!(
                    KeyEvent { code: KeyCode::Char('y'), modifiers: KeyModifiers::NONE }
                        => KeyEvent { code: KeyCode::Char('x'), modifiers: KeyModifiers::NONE },
                    KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL }
                        => KeyEvent { code: KeyCode::F(12), modifiers: KeyModifiers::NONE },
                ),
                Mode::Normal => hashmap!(
                    KeyEvent { code: KeyCode::F(12), modifiers: KeyModifiers::ALT }
                        => KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::SHIFT | KeyModifiers::CONTROL },
                )
            )
        )
    }

    #[test]
    fn parsing_unmodified_keys() {
        assert_eq!(
            str::parse::<RepresentableKeyEvent>("Bs").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE
            })
        );

        assert_eq!(
            str::parse::<RepresentableKeyEvent>("Left").unwrap(),
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
            str::parse::<RepresentableKeyEvent>("S-Bs").unwrap(),
            RepresentableKeyEvent(KeyEvent {
                code: KeyCode::Backspace,
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
    }
}
