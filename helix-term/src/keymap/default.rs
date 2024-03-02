use std::collections::HashMap;

use super::macros::keymap;
use super::{KeyTrie, Mode};
use helix_core::hashmap;

pub fn default() -> HashMap<Mode, KeyTrie> {
    let normal = keymap!({ "Normal mode"

        "left" => move_char_left,
        "right" => move_char_right,

        "C-S-left" => select_prev_sibling,
        "C-S-right" => select_next_sibling,

        "up" => move_line_up,
        "down" => move_line_down,

        "S-down" => extend_line_down,
        "S-up" => extend_line_up,

        "C-S-up" => expand_selection,
        "C-S-down" => shrink_selection,

        "t" => find_till_char,
        "T" => till_prev_char,

        "f" => find_next_char,
        "F" => find_prev_char,

        "r" => replace,
        "R" => replace_with_yanked,

        "C-A-u" => switch_case,
        "C-u" => switch_to_uppercase,
        "C-l" => switch_to_lowercase,

        "home" => goto_line_start,
        "end" => goto_line_end,

        "w" => move_next_word_start,
        "e" => move_next_word_end,

        "b" => move_prev_word_start,

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

        "A-e" => move_parent_node_end,
        "A-b" => move_parent_node_start,

        "C-a" => select_all,

        "x" => no_op,
        "X" => extend_to_line_bounds,
        "A-x" => shrink_to_line_bounds,

        "/" => search,
        "?" => rsearch,
        "n" => search_next,
        "N" => search_prev,
        "*" => search_selection,

        "C-z" | "u" => undo,
        "C-y" | "U" => redo,

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

        "C-g" => goto_line,

        "C-/" => toggle_comments,
        "C-S-/" => toggle_block_comments,
        "A-c" => toggle_line_comments,

        "C-i" | "tab" => jump_forward,
        "C-o" => jump_backward,
        "C-s" => save_selection,

        "\"" => select_register,
        "|" => shell_pipe,
        "A-|" => shell_pipe_to,
        "!" => shell_insert_output,
        "A-!" => shell_append_output,
        "$" => shell_keep_pipe,
        // "C-z" => suspend,

        // "C-a" => increment,
        "C-x" => decrement,

        // new to normal mode
        "C-tab" => goto_next_buffer,
        "C-S-tab" => goto_previous_buffer,
        "C-home" => goto_file_start,
        "C-end" => goto_last_line,
        "C-," => file_picker,
        "C-." => file_picker,
        "F2" => rename_symbol,
        "F12" => goto_definition,
        "S-F12" => goto_reference,
        "K" => hover,
        "C-up" => scroll_up,
        "C-down" => scroll_down,

        // Menu
        "space" => { " ⭐ Space "

            "🗿" => menu_separator_local,

            "b" => buffer_picker,
            "d" => diagnostics_picker,
            "f" => file_picker,
            "j" => jumplist_picker,
            "s" => symbol_picker,
            "." => last_picker,

            "🌐" => menu_separator_global,

            "F" => file_picker_in_current_directory,
            "S" => workspace_symbol_picker,
            "D" => workspace_diagnostics_picker,
            "/" => global_search,
            "?" => command_palette,

            "⌨" => menu_separator_code,

            "a" => code_action,
            "k" => hover,
            "r" => rename_symbol,
            "h" => select_references_to_symbol_under_cursor,
            "C-space" => add_newline_above,
            "space" => add_newline_below,

            "📋" => menu_separator_clipboard,

            // Submenu
            "y" => { "📤 Yank  " sticky=true
                "y" => yank,
                "j" => yank_joined,
                "P" => paste_before,
                "p" => paste_after,
                "r" => replace_with_yanked,
            },

            // Submenu
            "c" => { "📋 Copy  " sticky=true
                "c" => yank_to_clipboard,
                "j" => yank_joined_to_clipboard,
                "m" => yank_main_selection_to_clipboard,
                "P" => paste_clipboard_before,
                "p" => paste_clipboard_after,
                "r" => replace_selections_with_clipboard,
            },

            // Submenu
            "e" => { "💻 Export  " sticky=true
                "e" => yank_to_primary_clipboard,
                "E" => yank_joined_to_primary_clipboard,
                "A-e" => yank_main_selection_to_primary_clipboard,
                "P" => paste_primary_clipboard_before,
                "p" => paste_primary_clipboard_after,
                "R" => replace_selections_with_primary_clipboard,
            },


            "‒" => menu_separator,

            // Submenu
            "g" => { "🐞 Debug  " sticky=true
                "l" => dap_launch,
                "r" => dap_restart,
                "b" => dap_toggle_breakpoint,
                "c" => dap_continue,

                "p" => dap_pause,

                "i" => dap_step_in,
                "o" => dap_step_out,
                "n" => dap_next,
                "v" => dap_variables,
                "t" => dap_terminate,
                "C-c" => dap_edit_condition,
                "C-l" => dap_edit_log,
                "s" => { " Switch  "
                    "t" => dap_switch_thread,
                    "f" => dap_switch_stack_frame,
                },
                "e" => dap_enable_exceptions,
                "d" => dap_disable_exceptions,
            },

            // Submenu
            "v" => { "🧿 View  "
                "z" | "c" => align_view_center,
                "t" => align_view_top,
                "b" => align_view_bottom,
                "m" => align_view_middle,
                "up" => scroll_up,
                "down" => scroll_down,
                "pageup" => page_up,
                "pagedown" => page_down,
                "backspace" => page_cursor_half_up,
                "space" => page_cursor_half_down,

                "/" => search,
                "?" => rsearch,
                "n" => search_next,
                "N" => search_prev,
            },

            // Submenu
            "w" => { "🪟 Window  "
                "w" => rotate_view,
                "s" => hsplit,
                "v" => vsplit,
                "t" => transpose_view,
                "f" => goto_file_hsplit,
                "F" => goto_file_vsplit,
                "q" => wclose,
                "o" => wonly,
                "left" => jump_view_left,
                "down" => jump_view_down,
                "up" => jump_view_up,
                "right" => jump_view_right,
                "H" => swap_view_left,
                "J" => swap_view_down,
                "K" => swap_view_up,
                "L" => swap_view_right,
                "n" => { "New split scratch buffer"
                    "s" => hsplit_new,
                    "v" => vsplit_new,
                },
            },
        },

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

        "m" => { " 󰾹 Match "
            "m" => match_brackets,
            "s" => surround_add,
            "r" => surround_replace,
            "d" => surround_delete,
            "a" => select_textobject_around,
            "i" => select_textobject_inner,
        },

        "[" => { " ⮬ Goto "
            "D" => goto_first_diag,
            "C" => goto_first_change,

            "d" => goto_prev_diag,
            "c" => goto_prev_change,
            "f" => goto_prev_function,
            "t" => goto_prev_class,
            "a" => goto_prev_parameter,
            "/" => goto_prev_comment,
            "T" => goto_prev_test,
            "p" => goto_prev_paragraph,
        },

        "]" => { " Goto ⮯ "
            "d" => goto_next_diag,
            "c" => goto_next_change,
            "f" => goto_next_function,
            "t" => goto_next_class,
            "a" => goto_next_parameter,
            "/" => goto_next_comment,
            "T" => goto_next_test,
            "p" => goto_next_paragraph,

            "D" => goto_last_diag,
            "G" => goto_last_change,
        },
    });

    let mut select = normal.clone();
    select.merge_nodes(keymap!({ "Select mode"
        "left" => extend_char_left,
        "down" => extend_visual_line_down,
        "up" => extend_visual_line_up,
        "right" => extend_char_right,

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
