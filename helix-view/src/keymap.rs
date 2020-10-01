use crate::commands::{self, Command};
use helix_core::{hashmap, state};
use std::collections::HashMap;

// Kakoune-inspired:
// mode = {
//      normal = {
//          q = record_macro
//          w = (next) word
//          W = next WORD
//          e = end of word
//          E = end of WORD
//          r =
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
//          h = move_char_left(n)
//          j = move_line_down(n)
//          k = move_line_up(n)
//          l = move_char_right(n)
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
//          ^ = start of line (first non blank char)
//          $ = end of line
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
// }

#[cfg(feature = "term")]
pub use crossterm::event::{KeyCode, KeyEvent as Key, KeyModifiers as Modifiers};

// TODO: could be trie based
type Keymap = HashMap<Vec<Key>, Command>;
type Keymaps = HashMap<state::Mode, Keymap>;

pub fn default() -> Keymaps {
    hashmap!(
        state::Mode::Normal =>
            hashmap!(
                vec![Key {
                    code: KeyCode::Char('h'),
                    modifiers: Modifiers::NONE
                }] => commands::move_char_left as Command,
                vec![Key {
                    code: KeyCode::Char('j'),
                    modifiers: Modifiers::NONE
                }] => commands::move_line_down as Command,
                vec![Key {
                    code: KeyCode::Char('k'),
                    modifiers: Modifiers::NONE
                }] => commands::move_line_up as Command,
                vec![Key {
                    code: KeyCode::Char('0'),
                    modifiers: Modifiers::NONE
                }] => commands::move_line_start as Command,
                vec![Key {
                    code: KeyCode::Char('$'),
                    modifiers: Modifiers::NONE
                }] => commands::move_line_end as Command,
                vec![Key {
                    code: KeyCode::Char('l'),
                    modifiers: Modifiers::NONE
                }] => commands::move_char_right as Command,
                vec![Key {
                    code: KeyCode::Char('H'),
                    modifiers: Modifiers::SHIFT
                }] => commands::extend_char_left as Command,
                vec![Key {
                    code: KeyCode::Char('J'),
                    modifiers: Modifiers::SHIFT
                }] => commands::extend_line_down as Command,
                vec![Key {
                    code: KeyCode::Char('K'),
                    modifiers: Modifiers::SHIFT
                }] => commands::extend_line_up as Command,
                vec![Key {
                    code: KeyCode::Char('L'),
                    modifiers: Modifiers::SHIFT
                }] => commands::extend_char_right as Command,
                vec![Key {
                    code: KeyCode::Char('w'),
                    modifiers: Modifiers::NONE
                }] => commands::move_next_word_start as Command,
                vec![Key {
                    code: KeyCode::Char('b'),
                    modifiers: Modifiers::NONE
                }] => commands::move_prev_word_start as Command,
                vec![Key {
                    code: KeyCode::Char('e'),
                    modifiers: Modifiers::NONE
                }] => commands::move_next_word_end as Command,
                vec![Key {
                    code: KeyCode::Char('i'),
                    modifiers: Modifiers::NONE
                }] => commands::insert_mode as Command,
                vec![Key {
                    code: KeyCode::Char('I'),
                    modifiers: Modifiers::SHIFT,
                }] => commands::prepend_to_line as Command,
                vec![Key {
                    code: KeyCode::Char('a'),
                    modifiers: Modifiers::NONE
                }] => commands::append_mode as Command,
                vec![Key {
                    code: KeyCode::Char('A'),
                    modifiers: Modifiers::SHIFT,
                }] => commands::append_to_line as Command,
                vec![Key {
                    code: KeyCode::Char('o'),
                    modifiers: Modifiers::NONE
                }] => commands::open_below as Command,
                vec![Key {
                    code: KeyCode::Char('d'),
                    modifiers: Modifiers::NONE
                }] => commands::delete_selection as Command,
                vec![Key {
                    code: KeyCode::Char('c'),
                    modifiers: Modifiers::NONE
                }] => commands::change_selection as Command,
                vec![Key {
                    code: KeyCode::Char('s'),
                    modifiers: Modifiers::NONE
                }] => commands::split_selection_on_newline as Command,
                vec![Key {
                    code: KeyCode::Esc,
                    modifiers: Modifiers::NONE
                }] => commands::normal_mode as Command,
            ),
            state::Mode::Insert => hashmap!(
                vec![Key {
                    code: KeyCode::Esc,
                    modifiers: Modifiers::NONE
                }] => commands::normal_mode as Command,
                vec![Key {
                    code: KeyCode::Backspace,
                    modifiers: Modifiers::NONE
                }] => commands::delete_char_backward as Command,
                vec![Key {
                    code: KeyCode::Delete,
                    modifiers: Modifiers::NONE
                }] => commands::delete_char_forward as Command,
                vec![Key {
                    code: KeyCode::Enter,
                    modifiers: Modifiers::NONE
                }] => commands::insert_newline as Command,
            )
    )
}
