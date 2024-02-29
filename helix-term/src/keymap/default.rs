use std::collections::HashMap;

use super::macros::keymap;
use super::{KeyTrie, Mode};
use helix_core::hashmap;

pub fn default() -> HashMap<Mode, KeyTrie> {
    let normal = keymap!({ "Normal mode"

        "left" => move_char_left,
        "down" => move_visual_line_down,
        "up" => move_visual_line_up,
        "right" => move_char_right,

        "t" => find_till_char,
        "T" => till_prev_char,

        "f" => find_next_char,
        "F" => find_prev_char,

        "r" => replace,
        "R" => replace_with_yanked,

        "A-." => no_op, // disabled until knowing how this actually work

        "C-A-u" => switch_case,

        "`" => no_op,
        "C-l" => switch_to_lowercase,

        "A-`" => no_op,
        "C-u" => switch_to_uppercase,

        "home" => goto_line_start,
        "end" => goto_line_end,

        "w" => move_next_word_start,
        "W" => no_op,
        "C-right" => move_next_long_word_start,

        "b" => move_prev_word_start,
        "B" => no_op,
        "C-left" => move_prev_long_word_start,

        "e" => move_next_word_end,
        "E" => no_op,

        ":" => command_mode,

        "i" => insert_mode,
        "I" => insert_at_line_start,
        "a" => append_mode,
        "A" => insert_at_line_end,
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
        "A-minus" => merge_selections,
        "A-_" => merge_consecutive_selections,
        "S" => split_selection,
        ";" => collapse_selection,
        "A-;" => flip_selections,
        "A-o" | "A-up" => expand_selection,
        "A-i" | "A-down" => shrink_selection,
        "A-p" | "A-left" => select_prev_sibling,
        "A-n" | "A-right" => select_next_sibling,
        "A-e" => move_parent_node_end,
        "A-b" => move_parent_node_start,

        "C-a" => select_all,

        "x" => no_op,
        "S-down" => extend_line_down,
        "S-up" => extend_line_up,
        "X" => extend_to_line_bounds,
        "A-x" => shrink_to_line_bounds,

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
        "p" => paste_after,
        "P" => paste_before,

        "Q" => record_macro,
        "q" => replay_macro,

        ">" => indent,
        "<" => unindent,
        "=" => format_selections,
        "J" => join_selections,
        "A-J" => join_selections_space,
        "k" => keep_selections,
        "A-k" => remove_selections,

        "," => keep_primary_selection,
        "A-," => remove_primary_selection,

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
        "C-d" => page_cursor_half_down,

        "v" => select_mode,
        "G" => goto_line,

        "C-/" => toggle_comments,

        "C-i" | "tab" => jump_forward,
        "C-o" => jump_backward,
        "C-s" => save_selection,

        // new to normal mode
        "C-tab" => goto_next_buffer,
        "C-S-tab" => goto_previous_buffer,
        "C-home" => goto_file_start,
        "C-end" => goto_last_line,
        "C-up" => goto_window_top,
        "C-down" => goto_window_bottom,
        "C-," => file_picker,
        "C-." => file_picker,
        "F2" => rename_symbol,
        "F12" => goto_definition,
        "S-F12" => goto_reference,
        "K" => hover,

        // Menu
        "g" => { " 🚀 Goto "
            "g" => no_op,
            "e" => no_op,
            "f" => no_op,
            "s" => no_op,

            "n" => goto_first_nonwhitespace,

            "d" => goto_definition,
            "D" => goto_declaration,
            "y" => goto_type_definition,
            "r" => goto_reference,
            "i" => goto_implementation,

            "t" => goto_window_top,
            "c" => goto_window_center,
            "b" => goto_window_bottom,

            "a" => goto_last_accessed_file,
            "m" => goto_last_modified_file,

            "p" => no_op,
            "k" => no_op,
            "j" => no_op,

            "." => goto_last_modification,
        },

        "m" => { " 󰾹 Match"
            "m" => match_brackets,
            "s" => surround_add,
            "r" => surround_replace,
            "d" => surround_delete,
            "a" => select_textobject_around,
            "i" => select_textobject_inner,
        },

        "[" => { " 󰅪 Left bracket"
            "d" => goto_prev_diag,
            "D" => goto_first_diag,
            "g" => goto_prev_change,
            "G" => goto_first_change,
            "f" => goto_prev_function,
            "t" => goto_prev_class,
            "a" => goto_prev_parameter,
            "c" => goto_prev_comment,
            "T" => goto_prev_test,
            "p" => goto_prev_paragraph,
            "space" => add_newline_above,
        },

        "]" => { " 󰅪 Right bracket"
            "d" => goto_next_diag,
            "D" => goto_last_diag,
            "g" => goto_next_change,
            "G" => goto_last_change,
            "f" => goto_next_function,
            "t" => goto_next_class,
            "a" => goto_next_parameter,
            "c" => goto_next_comment,
            "T" => goto_next_test,
            "p" => goto_next_paragraph,
            "space" => add_newline_below,
        },

        // Menu
        "C-w" => { " 🪟 Window"
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

        // Menu
        "space" => { " ⭐ Space"

            "🔎" => menu_separator_open,

            "b" => buffer_picker,
            "d" => diagnostics_picker,
            "f" => file_picker,
            "j" => jumplist_picker,
            "s" => symbol_picker,
            "'" => last_picker,

            "📋" => menu_separator_clipboard,

            "y" => no_op,
            "Y" => no_op,

            "C-c" => yank_to_clipboard,
            "C-C" => yank_joined_to_clipboard,
            "C-A-c" => yank_main_selection_to_clipboard,
            "C-P" => paste_clipboard_before,
            "C-p" => paste_clipboard_after,
            "C-R" => replace_selections_with_clipboard,

            "🖉" => menu_separator_code,

            "a" => code_action,
            "k" => hover,
            "r" => rename_symbol,
            "h" => select_references_to_symbol_under_cursor,

            "🌐" => menu_separator_global,

            "F" => file_picker_in_current_directory,
            "S" => workspace_symbol_picker,
            "D" => workspace_diagnostics_picker,
            "/" => global_search,
            "?" => command_palette,

            "🧰" => menu_separator_more,

            // Submenu
            "g" => { "🐞 debug " sticky=true
                "l" => dap_launch,
                "r" => dap_restart,
                "b" => dap_toggle_breakpoint,
                "c" => dap_continue,

                // "h" => dap_pause,
                "h" => no_op,
                "p" => dap_pause,

                "i" => dap_step_in,
                "o" => dap_step_out,
                "n" => dap_next,
                "v" => dap_variables,
                "t" => dap_terminate,
                "C-c" => dap_edit_condition,
                "C-l" => dap_edit_log,
                "s" => { " Switch "
                    "t" => dap_switch_thread,
                    "f" => dap_switch_stack_frame,
                    // sl, sb
                },
                "e" => dap_enable_exceptions,
                // "E" => dap_disable_exceptions,
                "E" => no_op,
                "d" => dap_disable_exceptions,
            },

            // Submenu
            "w" => { "🪟 window "
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
        },

        // Menu
        "z" => { " 󰛐 View"
            "z" | "c" => align_view_center,
            "t" => align_view_top,
            "b" => align_view_bottom,
            "m" => align_view_middle,
            "k" | "up" => scroll_up,
            "j" | "down" => scroll_down,
            "C-b" | "pageup" => page_up,
            "C-f" | "pagedown" => page_down,
            "C-u" | "backspace" => page_cursor_half_up,
            "C-d" | "space" => page_cursor_half_down,

            "/" => search,
            "?" => rsearch,
            "n" => search_next,
            "N" => search_prev,
        },

        // Menu
        "Z" => { "  VIEW" sticky=true
            "z" | "c" => align_view_center,
            "t" => align_view_top,
            "b" => align_view_bottom,
            "m" => align_view_middle,
            "k" | "up" => scroll_up,
            "j" | "down" => scroll_down,
            "C-b" | "pageup" => page_up,
            "C-f" | "pagedown" => page_down,
            "C-u" | "backspace" => page_cursor_half_up,
            "C-d" | "space" => page_cursor_half_down,

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

        // "C-a" => increment,
        "C-x" => decrement,
    });

    let mut select = normal.clone();
    select.merge_nodes(keymap!({ "Select mode"
        "h" | "left" => extend_char_left,
        "j" | "down" => extend_visual_line_down,
        "k" | "up" => extend_visual_line_up,
        "l" | "right" => extend_char_right,

        "w" => extend_next_word_start,
        "b" => extend_prev_word_start,
        "e" => extend_next_word_end,
        "W" => extend_next_long_word_start,
        "B" => extend_prev_long_word_start,
        "E" => extend_next_long_word_end,

        "A-e" => extend_parent_node_end,
        "A-b" => extend_parent_node_start,

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
        "g" => { "Goto"
            "k" => extend_line_up,
            "j" => extend_line_down,
        },
    }));

    let insert = keymap!({ "Insert mode"
        "esc" => normal_mode,

        "C-s" => commit_undo_checkpoint,
        "C-x" => completion,
        "C-r" => insert_register,

        "C-w" | "A-backspace" => delete_word_backward,
        "A-d" | "A-del" => delete_word_forward,
        "C-u" => kill_to_line_start,
        "C-k" => kill_to_line_end,
        "C-h" | "backspace" | "S-backspace" => delete_char_backward,
        "C-d" | "del" => delete_char_forward,
        "C-j" | "ret" => insert_newline,
        "tab" => smart_tab,
        "S-tab" => insert_tab,

        "up" => move_visual_line_up,
        "down" => move_visual_line_down,
        "left" => move_char_left,
        "right" => move_char_right,
        "pageup" => page_up,
        "pagedown" => page_down,
        "home" => goto_line_start,
        "end" => goto_line_end_newline,
    });
    hashmap!(
        Mode::Normal => normal,
        Mode::Select => select,
        Mode::Insert => insert,
    )
}
