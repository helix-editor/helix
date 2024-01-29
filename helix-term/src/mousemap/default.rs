use std::collections::HashMap;

use helix_core::hashmap;
use helix_view::{document::Mode, input::MouseEvent};

use crate::mousemap;

use super::MouseTrie;

pub fn default() -> HashMap<Mode, HashMap<MouseEvent, MouseTrie>> {
    let normal = mousemap!({
        "1-left" => handle_main_button_mouse,
        "2-left" => select_word_mouse,
        "3-left" => select_long_word_mouse,
        "A-1-left" => add_selection_mouse,
        "1-right" => yank_main_selection_to_primary_clipboard_mouse,
        "A-1-middle" => replace_selections_with_primary_clipboard_mouse,
        "1-middle" => paste_primary_clipboard_before_mouse,
        "scroll_up" => scroll_up_mouse,
        "scroll_down" => scroll_down_mouse,

    });
    let insert = normal.clone();
    let select = normal.clone();
    hashmap!(
        Mode::Normal => normal,
        Mode::Insert => insert,
        Mode::Select => select,
    )
}
