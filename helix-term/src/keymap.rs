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
pub type Keymap = HashMap<Key, Command>;
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

macro_rules! alt {
    ($ch:expr) => {
        Key {
            code: KeyCode::Char($ch),
            modifiers: Modifiers::ALT,
        }
    };
}

pub fn default() -> Keymaps {
    let normal = hashmap!(
        key!('h') => commands::move_char_left as Command,
        key!('j') => commands::move_line_down,
        key!('k') => commands::move_line_up,
        key!('l') => commands::move_char_right,

        // key!('t') => commands::till_next_char,
        // key!('f') => commands::find_next_char,
        // key!('T') => commands::till_prev_char,
        // key!('f') => commands::find_prev_char,
        // and matching set for select mode (extend)

        key!('0') => commands::move_line_start,
        key!('$') => commands::move_line_end,

        key!('w') => commands::move_next_word_start,
        key!('b') => commands::move_prev_word_start,
        key!('e') => commands::move_next_word_end,

        key!('v') => commands::select_mode,
        key!('g') => commands::goto_mode,
        key!(':') => commands::command_mode,

        key!('i') => commands::insert_mode,
        shift!('I') => commands::prepend_to_line,
        key!('a') => commands::append_mode,
        shift!('A') => commands::append_to_line,
        key!('o') => commands::open_below,
        // key!('O') => commands::open_above,
        // [<space>  ]<space> equivalents too (add blank new line, no edit)


        key!('d') => commands::delete_selection,
        // TODO: also delete without yanking
        key!('c') => commands::change_selection,
        // TODO: also change delete without yanking

        // key!('r') => commands::replace_with_char,

        key!('s') => commands::select_regex,
        alt!('s') => commands::split_selection_on_newline,
        shift!('S') => commands::split_selection,
        key!(';') => commands::collapse_selection,
        alt!(';') => commands::flip_selections,
        key!('%') => commands::select_all,
        key!('x') => commands::select_line,
        shift!('X') => commands::extend_line,
        // or select mode X?
        // extend_to_whole_line, crop_to_whole_line

        // key!('m') => commands::select_to_matching,
        // key!('M') => commands::back_select_to_matching,
        // select mode extend equivalents

        // key!('.') => commands::repeat_insert,
        // repeat_select

        // TODO: figure out what key to use
        key!('[') => commands::expand_selection,

        key!('/') => commands::search,
        key!('n') => commands::search_next,
        key!('*') => commands::search_selection,

        key!('u') => commands::undo,
        shift!('U') => commands::redo,

        key!('y') => commands::yank,
        // yank_all
        key!('p') => commands::paste,
        // paste_all

        key!('>') => commands::indent,
        key!('<') => commands::unindent,
        key!('=') => commands::format_selections,
        shift!('J') => commands::join_selections,
        shift!('K') => commands::keep_selections,

        // key!('q') => commands::record_macro,
        // key!('Q') => commands::replay_macro,

        // ~ / apostrophe => change case
        // & align selections
        // _ trim selections

        // C / altC = copy (repeat) selections on prev/next lines

        Key {
            code: KeyCode::Esc,
            modifiers: Modifiers::NONE
        } => commands::normal_mode,
        Key {
            code: KeyCode::PageUp,
            modifiers: Modifiers::NONE
        } => commands::page_up,
        Key {
            code: KeyCode::PageDown,
            modifiers: Modifiers::NONE
        } => commands::page_down,
        ctrl!('u') => commands::half_page_up,
        ctrl!('d') => commands::half_page_down,

        ctrl!('p') => commands::file_picker,
        ctrl!('b') => commands::buffer_picker,
        Key {
            code: KeyCode::Tab,
            modifiers: Modifiers::NONE
        } => commands::next_view,

        // move under <space>c
        ctrl!('c') => commands::toggle_comments,
        shift!('K') => commands::hover,

        // z family for save/restore/combine from/to sels from register
    );
    // TODO: decide whether we want normal mode to also be select mode (kakoune-like), or whether
    // we keep this separate select mode. More keys can fit into normal mode then, but it's weird
    // because some selection operations can now be done from normal mode, some from select mode.
    let mut select = normal.clone();
    select.extend(
        hashmap!(
            key!('h') => commands::extend_char_left as Command,
            key!('j') => commands::extend_line_down,
            key!('k') => commands::extend_line_up,
            key!('l') => commands::extend_char_right,

            key!('w') => commands::extend_next_word_start,
            key!('b') => commands::extend_prev_word_start,
            key!('e') => commands::extend_next_word_end,

            Key {
                code: KeyCode::Esc,
                modifiers: Modifiers::NONE
            } => commands::exit_select_mode as Command,
        )
        .into_iter(),
    );

    hashmap!(
        // as long as you cast the first item, rust is able to infer the other cases
        // TODO: select could be normal mode with some bindings merged over
        Mode::Normal => normal,
        Mode::Select => select,
        Mode::Insert => hashmap!(
            Key {
                code: KeyCode::Esc,
                modifiers: Modifiers::NONE
            } => commands::normal_mode as Command,
            Key {
                code: KeyCode::Backspace,
                modifiers: Modifiers::NONE
            } => commands::insert::delete_char_backward,
            Key {
                code: KeyCode::Delete,
                modifiers: Modifiers::NONE
            } => commands::insert::delete_char_forward,
            Key {
                code: KeyCode::Enter,
                modifiers: Modifiers::NONE
            } => commands::insert::insert_newline,
            Key {
                code: KeyCode::Tab,
                modifiers: Modifiers::NONE
            } => commands::insert::insert_tab,

            ctrl!('x') => commands::completion,
        ),
        Mode::Goto => hashmap!(
            Key {
                code: KeyCode::Esc,
                modifiers: Modifiers::NONE
            } => commands::normal_mode as Command,
            key!('g') => commands::move_file_start as Command,
            key!('e') => commands::move_file_end as Command,
        ),
    )
}
