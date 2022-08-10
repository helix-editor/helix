use std::collections::HashMap;

use super::macros::keymap;
use super::{Keymap, Mode};
use helix_core::hashmap;

pub fn default() -> HashMap<Mode, Keymap> {
    let normal = keymap!({ "Normal mode"
        "h" | "left" => move_char_left,
        "j" | "down" => move_line_down,
        "k" | "up" => move_line_up,
        "l" | "right" => move_char_right,

        "t" => find_till_char,
        "f" => find_next_char,
        "T" => till_prev_char,
        "F" => find_prev_char,
        "r" => replace,
        "R" => replace_with_yanked,
        "A-." =>  repeat_last_motion,

        "~" => switch_case,
        "`" => switch_to_lowercase,
        "A-`" => switch_to_uppercase,

        "home" => goto_line_start,
        "end" => goto_line_end,

        "w" => move_next_word_start,
        "b" => move_prev_word_start,
        "e" => move_next_word_end,

        "W" => move_next_long_word_start,
        "B" => move_prev_long_word_start,
        "E" => move_next_long_word_end,

        "v" => select_mode,
        "G" => goto_line,
        "g" => { "Goto"
            "g" => goto_file_start,
            "e" => goto_last_line,
            "f" => goto_file,
            "h" => goto_line_start,
            "l" => goto_line_end,
            "s" => goto_first_nonwhitespace,
            "d" => goto_definition,
            "y" => goto_type_definition,
            "r" => goto_reference,
            "i" => goto_implementation,
            "t" => goto_window_top,
            "c" => goto_window_center,
            "b" => goto_window_bottom,
            "a" => goto_last_accessed_file,
            "m" => goto_last_modified_file,
            "n" => goto_next_buffer,
            "p" => goto_previous_buffer,
            "." => goto_last_modification,
        },
        ":" => command_mode,

        "i" => insert_mode,
        "I" => prepend_to_line,
        "a" => append_mode,
        "A" => append_to_line,
        "o" => open_below,
        "O" => open_above,

        "d" => delete_selection,
        "A-d" => delete_selection_noyank,
        "c" => change_selection,
        "A-c" => change_selection_noyank,

        "C" => copy_selection_on_next_line,
        "A-C" => copy_selection_on_prev_line,


        "s" => select_regex,
        "A-s" => split_selection_on_newline,
        "S" => split_selection,
        ";" => collapse_selection,
        "A-;" => flip_selections,
        "A-o" | "A-up" => expand_selection,
        "A-i" | "A-down" => shrink_selection,
        "A-p" | "A-left" => select_prev_sibling,
        "A-n" | "A-right" => select_next_sibling,

        "%" => select_all,
        "x" => extend_line_below,
        "X" => extend_to_line_bounds,
        "A-x" => shrink_to_line_bounds,

        "m" => { "Match"
            "m" => match_brackets,
            "s" => surround_add,
            "r" => surround_replace,
            "d" => surround_delete,
            "a" => select_textobject_around,
            "i" => select_textobject_inner,
        },
        "[" => { "Left bracket"
            "d" => goto_prev_diag,
            "D" => goto_first_diag,
            "f" => goto_prev_function,
            "c" => goto_prev_class,
            "a" => goto_prev_parameter,
            "o" => goto_prev_comment,
            "t" => goto_prev_test,
            "p" => goto_prev_paragraph,
            "space" => add_newline_above,
        },
        "]" => { "Right bracket"
            "d" => goto_next_diag,
            "D" => goto_last_diag,
            "f" => goto_next_function,
            "c" => goto_next_class,
            "a" => goto_next_parameter,
            "o" => goto_next_comment,
            "t" => goto_next_test,
            "p" => goto_next_paragraph,
            "space" => add_newline_below,
        },

        "/" => search,
        "?" => rsearch,
        "n" => search_next,
        "N" => search_prev,
        "*" => search_selection,

        "u" => undo,
        "U" => redo,
        "A-u" => earlier,
        "A-U" => later,

        "y" => yank,
        // yank_all
        "p" => paste_after,
        // paste_all
        "P" => paste_before,

        "Q" => record_macro,
        "q" => replay_macro,

        ">" => indent,
        "<" => unindent,
        "=" => format_selections,
        "J" => join_selections,
        "K" => keep_selections,
        "A-K" => remove_selections,

        "," => keep_primary_selection,
        "A-," => remove_primary_selection,

        // "q" => record_macro,
        // "Q" => replay_macro,

        "&" => align_selections,
        "_" => trim_selections,

        "(" => rotate_selections_backward,
        ")" => rotate_selections_forward,
        "A-(" => rotate_selection_contents_backward,
        "A-)" => rotate_selection_contents_forward,

        "A-:" => ensure_selections_forward,

        "esc" => normal_mode,
        "C-b" | "pageup" => page_up,
        "C-f" | "pagedown" => page_down,
        "C-u" => half_page_up,
        "C-d" => half_page_down,

        "C-w" => { "Window"
            "C-w" | "w" => rotate_view,
            "C-s" | "s" => hsplit,
            "C-v" | "v" => vsplit,
            "C-t" | "t" => transpose_view,
            "f" => goto_file_hsplit,
            "F" => goto_file_vsplit,
            "C-q" | "q" => wclose,
            "C-o" | "o" => wonly,
            "C-h" | "h" | "left" => jump_view_left,
            "C-j" | "j" | "down" => jump_view_down,
            "C-k" | "k" | "up" => jump_view_up,
            "C-l" | "l" | "right" => jump_view_right,
            "L" => swap_view_right,
            "K" => swap_view_up,
            "H" => swap_view_left,
            "J" => swap_view_down,
            "n" => { "New split scratch buffer"
                "C-s" | "s" => hsplit_new,
                "C-v" | "v" => vsplit_new,
            },
        },

        // move under <space>c
        "C-c" => toggle_comments,

        // z family for save/restore/combine from/to sels from register

        "tab" => jump_forward, // tab == <C-i>
        "C-o" => jump_backward,
        "C-s" => save_selection,

        "space" => { "Space"
            "f" => file_picker,
            "F" => file_picker_in_current_directory,
            "b" => buffer_picker,
            "j" => jumplist_picker,
            "s" => symbol_picker,
            "S" => workspace_symbol_picker,
            "g" => diagnostics_picker,
            "G" => workspace_diagnostics_picker,
            "a" => code_action,
            "'" => last_picker,
            "d" => { "Debug (experimental)" sticky=true
                "l" => dap_launch,
                "b" => dap_toggle_breakpoint,
                "c" => dap_continue,
                "h" => dap_pause,
                "i" => dap_step_in,
                "o" => dap_step_out,
                "n" => dap_next,
                "v" => dap_variables,
                "t" => dap_terminate,
                "C-c" => dap_edit_condition,
                "C-l" => dap_edit_log,
                "s" => { "Switch"
                    "t" => dap_switch_thread,
                    "f" => dap_switch_stack_frame,
                    // sl, sb
                },
                "e" => dap_enable_exceptions,
                "E" => dap_disable_exceptions,
            },
            "w" => { "Window"
                "C-w" | "w" => rotate_view,
                "C-s" | "s" => hsplit,
                "C-v" | "v" => vsplit,
                "C-t" | "t" => transpose_view,
                "f" => goto_file_hsplit,
                "F" => goto_file_vsplit,
                "C-q" | "q" => wclose,
                "C-o" | "o" => wonly,
                "C-h" | "h" | "left" => jump_view_left,
                "C-j" | "j" | "down" => jump_view_down,
                "C-k" | "k" | "up" => jump_view_up,
                "C-l" | "l" | "right" => jump_view_right,
                "H" => swap_view_left,
                "J" => swap_view_down,
                "K" => swap_view_up,
                "L" => swap_view_right,
                "n" => { "New split scratch buffer"
                    "C-s" | "s" => hsplit_new,
                    "C-v" | "v" => vsplit_new,
                },
            },
            "y" => yank_joined_to_clipboard,
            "Y" => yank_main_selection_to_clipboard,
            "p" => paste_clipboard_after,
            "P" => paste_clipboard_before,
            "R" => replace_selections_with_clipboard,
            "/" => global_search,
            "k" => hover,
            "r" => rename_symbol,
            "h" => select_references_to_symbol_under_cursor,
            "?" => command_palette,
        },
        "z" => { "View"
            "z" | "c" => align_view_center,
            "t" => align_view_top,
            "b" => align_view_bottom,
            "m" => align_view_middle,
            "k" | "up" => scroll_up,
            "j" | "down" => scroll_down,
            "C-b" | "pageup" => page_up,
            "C-f" | "pagedown" => page_down,
            "C-u" | "backspace" => half_page_up,
            "C-d" | "space" => half_page_down,

            "/" => search,
            "?" => rsearch,
            "n" => search_next,
            "N" => search_prev,
        },
        "Z" => { "View" sticky=true
            "z" | "c" => align_view_center,
            "t" => align_view_top,
            "b" => align_view_bottom,
            "m" => align_view_middle,
            "k" | "up" => scroll_up,
            "j" | "down" => scroll_down,
            "C-b" | "pageup" => page_up,
            "C-f" | "pagedown" => page_down,
            "C-u" | "backspace" => half_page_up,
            "C-d" | "space" => half_page_down,

            "/" => search,
            "?" => rsearch,
            "n" => search_next,
            "N" => search_prev,
        },

        "\"" => select_register,
        "|" => shell_pipe,
        "A-|" => shell_pipe_to,
        "!" => shell_insert_output,
        "A-!" => shell_append_output,
        "$" => shell_keep_pipe,
        "C-z" => suspend,

        "C-a" => increment,
        "C-x" => decrement,
    });
    let mut select = normal.clone();
    select.merge_nodes(keymap!({ "Select mode"
        "h" | "left" => extend_char_left,
        "j" | "down" => extend_line_down,
        "k" | "up" => extend_line_up,
        "l" | "right" => extend_char_right,

        "w" => extend_next_word_start,
        "b" => extend_prev_word_start,
        "e" => extend_next_word_end,
        "W" => extend_next_long_word_start,
        "B" => extend_prev_long_word_start,
        "E" => extend_next_long_word_end,

        "n" => extend_search_next,
        "N" => extend_search_prev,

        "t" => extend_till_char,
        "f" => extend_next_char,
        "T" => extend_till_prev_char,
        "F" => extend_prev_char,

        "home" => extend_to_line_start,
        "end" => extend_to_line_end,
        "esc" => exit_select_mode,

        "v" => normal_mode,
    }));
    let insert = keymap!({ "Insert mode"
        "esc" => normal_mode,

        "backspace" => delete_char_backward,
        "C-h" => delete_char_backward,
        "del" => delete_char_forward,
        "C-d" => delete_char_forward,
        "ret" => insert_newline,
        "C-j" => insert_newline,
        "tab" => insert_tab,
        "C-w" => delete_word_backward,
        "A-backspace" => delete_word_backward,
        "A-d" => delete_word_forward,
        "A-del" => delete_word_forward,
        "C-s" => commit_undo_checkpoint,

        "left" => move_char_left,
        "C-b" => move_char_left,
        "down" => move_line_down,
        "up" => move_line_up,
        "right" => move_char_right,
        "C-f" => move_char_right,
        "A-b" => move_prev_word_end,
        "C-left" => move_prev_word_end,
        "A-f" => move_next_word_start,
        "C-right" => move_next_word_start,
        "A-<" => goto_file_start,
        "A->" => goto_file_end,
        "pageup" => page_up,
        "pagedown" => page_down,
        "home" => goto_line_start,
        "C-a" => goto_line_start,
        "end" => goto_line_end_newline,
        "C-e" => goto_line_end_newline,

        "C-k" => kill_to_line_end,
        "C-u" => kill_to_line_start,

        "C-x" => completion,
        "C-r" => insert_register,
    });
    hashmap!(
        Mode::Normal => Keymap::new(normal),
        Mode::Select => Keymap::new(select),
        Mode::Insert => Keymap::new(insert),
    )
}
