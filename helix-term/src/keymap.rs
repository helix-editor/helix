use crate::commands::{self, Command};
use helix_core::hashmap;
use helix_view::document::Mode;
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
//
//      gd = goto definition
//      gr = goto reference
// }

// #[cfg(feature = "term")]
pub use crossterm::event::{KeyCode, KeyEvent as Key, KeyModifiers as Modifiers};

// TODO: could be trie based
pub type Keymap = HashMap<Vec<Key>, Command>;
pub type Keymaps = HashMap<Mode, Keymap>;

macro_rules! key {
    ($ch:expr) => {
        Key {
            code: KeyCode::Char($ch),
            modifiers: Modifiers::NONE,
        }
    };
}

macro_rules! shift {
    ($ch:expr) => {
        Key {
            code: KeyCode::Char($ch),
            modifiers: Modifiers::SHIFT,
        }
    };
}

macro_rules! ctrl {
    ($ch:expr) => {
        Key {
            code: KeyCode::Char($ch),
            modifiers: Modifiers::CONTROL,
        }
    };
}

// macro_rules! alt {
//     ($ch:expr) => {
//         Key {
//             code: KeyCode::Char($ch),
//             modifiers: Modifiers::ALT,
//         }
//     };
// }

pub fn default() -> Keymaps {
    hashmap!(
        Mode::Normal =>
            // as long as you cast the first item, rust is able to infer the other cases
            hashmap!(
                vec![key!('h')] => commands::move_char_left as Command,
                vec![key!('j')] => commands::move_line_down,
                vec![key!('k')] => commands::move_line_up,
                vec![key!('0')] => commands::move_line_start,
                vec![key!('$')] => commands::move_line_end,
                vec![key!('l')] => commands::move_char_right,
                vec![shift!('H')] => commands::extend_char_left,
                vec![shift!('J')] => commands::extend_line_down,
                vec![shift!('K')] => commands::extend_line_up,
                vec![shift!('L')] => commands::extend_char_right,
                vec![key!('w')] => commands::move_next_word_start,
                vec![shift!('W')] => commands::extend_next_word_start,
                vec![key!('b')] => commands::move_prev_word_start,
                vec![shift!('B')] => commands::extend_prev_word_start,
                vec![key!('e')] => commands::move_next_word_end,
                // TODO: E
                vec![key!('g')] => commands::goto_mode,
                vec![key!('i')] => commands::insert_mode,
                vec![shift!('I')] => commands::prepend_to_line,
                vec![key!('a')] => commands::append_mode,
                vec![shift!('A')] => commands::append_to_line,
                vec![key!('o')] => commands::open_below,
                vec![key!('d')] => commands::delete_selection,
                vec![key!('c')] => commands::change_selection,
                vec![key!('s')] => commands::split_selection_on_newline,
                vec![shift!('S')] => commands::split_selection,
                vec![key!(';')] => commands::collapse_selection,
                // TODO should be alt(;)
                vec![key!('%')] => commands::flip_selections,
                vec![key!('x')] => commands::select_line,
                vec![key!('u')] => commands::undo,
                vec![shift!('U')] => commands::redo,
                vec![key!('y')] => commands::yank,
                vec![key!('p')] => commands::paste,
                vec![key!('>')] => commands::indent,
                vec![key!('<')] => commands::unindent,
                vec![key!(':')] => commands::command_mode,
                vec![Key {
                    code: KeyCode::Esc,
                    modifiers: Modifiers::NONE
                }] => commands::normal_mode,
                vec![Key {
                    code: KeyCode::PageUp,
                    modifiers: Modifiers::NONE
                }] => commands::page_up,
                vec![Key {
                    code: KeyCode::PageDown,
                    modifiers: Modifiers::NONE
                }] => commands::page_down,
                vec![ctrl!('u')] => commands::half_page_up,
                vec![ctrl!('d')] => commands::half_page_down,

                vec![ctrl!('p')] => commands::file_picker,
                vec![ctrl!('b')] => commands::buffer_picker,
            ),
            Mode::Insert => hashmap!(
                vec![Key {
                    code: KeyCode::Esc,
                    modifiers: Modifiers::NONE
                }] => commands::normal_mode as Command,
                vec![Key {
                    code: KeyCode::Backspace,
                    modifiers: Modifiers::NONE
                }] => commands::insert::delete_char_backward,
                vec![Key {
                    code: KeyCode::Delete,
                    modifiers: Modifiers::NONE
                }] => commands::insert::delete_char_forward,
                vec![Key {
                    code: KeyCode::Enter,
                    modifiers: Modifiers::NONE
                }] => commands::insert::insert_newline,
                vec![Key {
                    code: KeyCode::Tab,
                    modifiers: Modifiers::NONE
                }] => commands::insert::insert_tab,

                vec![ctrl!('x')] => commands::completion,
            ),
            Mode::Goto => hashmap!(
                vec![Key {
                    code: KeyCode::Esc,
                    modifiers: Modifiers::NONE
                }] => commands::normal_mode as Command,
                vec![key!('g')] => commands::move_file_start as Command,
                vec![key!('e')] => commands::move_file_end as Command,
            ),
    )
}
