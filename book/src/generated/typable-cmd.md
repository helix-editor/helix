| Name | Description |
| --- | --- |
| `:quit`, `:q` | Close the current view. |
| `:quit!`, `:q!` | Force close the current view, ignoring unsaved changes. |
| `:open`, `:o` | Open a file from disk into the current view. |
| `:buffer-close`, `:bc`, `:bclose` | Close the current buffer. |
| `:buffer-close!`, `:bc!`, `:bclose!` | Close the current buffer forcefully, ignoring unsaved changes. |
| `:buffer-close-others`, `:bco`, `:bcloseother` | Close all buffers but the currently focused one. |
| `:buffer-close-others!`, `:bco!`, `:bcloseother!` | Force close all buffers but the currently focused one. |
| `:buffer-close-all`, `:bca`, `:bcloseall` | Close all buffers without quitting. |
| `:buffer-close-all!`, `:bca!`, `:bcloseall!` | Force close all buffers ignoring unsaved changes without quitting. |
| `:buffer-next`, `:bn`, `:bnext` | Goto next buffer. |
| `:buffer-previous`, `:bp`, `:bprev` | Goto previous buffer. |
| `:write`, `:w` | Write changes to disk. Accepts an optional path (:write some/path.txt) |
| `:write!`, `:w!` | Force write changes to disk creating necessary subdirectories. Accepts an optional path (:write! some/path.txt) |
| `:write-buffer-close`, `:wbc` | Write changes to disk and closes the buffer. Accepts an optional path (:write-buffer-close some/path.txt) |
| `:write-buffer-close!`, `:wbc!` | Force write changes to disk creating necessary subdirectories and closes the buffer. Accepts an optional path (:write-buffer-close! some/path.txt) |
| `:new`, `:n` | Create a new scratch buffer. |
| `:format`, `:fmt` | Format the file using the LSP formatter. |
| `:indent-style` | Set the indentation style for editing. ('t' for tabs or 1-16 for number of spaces.) |
| `:line-ending` | Set the document's default line ending. Options: crlf, lf. |
| `:earlier`, `:ear` | Jump back to an earlier point in edit history. Accepts a number of steps or a time span. |
| `:later`, `:lat` | Jump to a later point in edit history. Accepts a number of steps or a time span. |
| `:write-quit`, `:wq`, `:x` | Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt) |
| `:write-quit!`, `:wq!`, `:x!` | Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt) |
| `:write-all`, `:wa` | Write changes from all buffers to disk. |
| `:write-all!`, `:wa!` | Forcefully write changes from all buffers to disk creating necessary subdirectories. |
| `:write-quit-all`, `:wqa`, `:xa` | Write changes from all buffers to disk and close all views. |
| `:write-quit-all!`, `:wqa!`, `:xa!` | Write changes from all buffers to disk and close all views forcefully (ignoring unsaved changes). |
| `:quit-all`, `:qa` | Close all views. |
| `:quit-all!`, `:qa!` | Force close all views ignoring unsaved changes. |
| `:cquit`, `:cq` | Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2). |
| `:cquit!`, `:cq!` | Force quit with exit code (default 1) ignoring unsaved changes. Accepts an optional integer exit code (:cq! 2). |
| `:theme` | Change the editor theme (show current theme if no name specified). |
| `:yank-join` | Yank joined selections. A separator can be provided as first argument. Default value is newline. |
| `:clipboard-yank` | Yank main selection into system clipboard. |
| `:clipboard-yank-join` | Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline. |
| `:primary-clipboard-yank` | Yank main selection into system primary clipboard. |
| `:primary-clipboard-yank-join` | Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline. |
| `:clipboard-paste-after` | Paste system clipboard after selections. |
| `:clipboard-paste-before` | Paste system clipboard before selections. |
| `:clipboard-paste-replace` | Replace selections with content of system clipboard. |
| `:primary-clipboard-paste-after` | Paste primary clipboard after selections. |
| `:primary-clipboard-paste-before` | Paste primary clipboard before selections. |
| `:primary-clipboard-paste-replace` | Replace selections with content of system primary clipboard. |
| `:show-clipboard-provider` | Show clipboard provider name in status bar. |
| `:change-current-directory`, `:cd` | Change the current working directory. |
| `:show-directory`, `:pwd` | Show the current working directory. |
| `:encoding` | Set encoding. Based on `https://encoding.spec.whatwg.org`. |
| `:character-info`, `:char` | Get info about the character under the primary cursor. |
| `:reload`, `:rl` | Discard changes and reload from the source file. |
| `:reload-all`, `:rla` | Discard changes and reload all documents from the source files. |
| `:update`, `:u` | Write changes only if the file has been modified. |
| `:lsp-workspace-command` | Open workspace command picker |
| `:lsp-restart` | Restarts the language servers used by the current doc |
| `:lsp-stop` | Stops the language servers that are used by the current doc |
| `:tree-sitter-scopes` | Display tree sitter scopes, primarily for theming and development. |
| `:tree-sitter-highlight-name` | Display name of tree-sitter highlight scope under the cursor. |
| `:debug-start`, `:dbg` | Start a debug session from a given template with given parameters. |
| `:debug-remote`, `:dbg-tcp` | Connect to a debug adapter by TCP address and start a debugging session from a given template with given parameters. |
| `:debug-eval` | Evaluate expression in current debug context. |
| `:vsplit`, `:vs` | Open the file in a vertical split. |
| `:vsplit-new`, `:vnew` | Open a scratch buffer in a vertical split. |
| `:hsplit`, `:hs`, `:sp` | Open the file in a horizontal split. |
| `:hsplit-new`, `:hnew` | Open a scratch buffer in a horizontal split. |
| `:tutor` | Open the tutorial. |
| `:goto`, `:g` | Goto line number. |
| `:set-language`, `:lang` | Set the language of current buffer (show current language if no value specified). |
| `:set-option`, `:set` | Set a config option at runtime.<br>For example to disable smart case search, use `:set search.smart-case false`. |
| `:toggle-option`, `:toggle` | Toggle a boolean config option at runtime.<br>For example to toggle smart case search, use `:toggle search.smart-case`. |
| `:get-option`, `:get` | Get the current value of a config option. |
| `:sort` | Sort ranges in selection. |
| `:rsort` | Sort ranges in selection in reverse order. |
| `:reflow` | Hard-wrap the current selection of lines to a given width. |
| `:tree-sitter-subtree`, `:ts-subtree` | Display tree sitter subtree under cursor, primarily for debugging queries. |
| `:config-reload` | Refresh user config. |
| `:config-open` | Open the user config.toml file. |
| `:config-open-workspace` | Open the workspace config.toml file. |
| `:log-open` | Open the helix log file. |
| `:insert-output` | Run shell command, inserting output before each selection. |
| `:append-output` | Run shell command, appending output after each selection. |
| `:pipe` | Pipe each selection to the shell command. |
| `:pipe-to` | Pipe each selection to the shell command, ignoring output. |
| `:run-shell-command`, `:sh` | Run a shell command |
| `:reset-diff-change`, `:diffget`, `:diffg` | Reset the diff change at the cursor position. |
| `:clear-register` | Clear given register. If no argument is provided, clear all registers. |
| `:redraw` | Clear and re-render the whole UI |
| `:move` | Move the current buffer and its corresponding file to a different path |
| `:yank-diagnostic` | Yank diagnostic(s) under primary cursor to register, or clipboard by default |
| `:read`, `:r` | Load a file into buffer |
| `no_op` | Do nothing |
| `move_char_left` | Move left |
| `move_char_right` | Move right |
| `move_line_up` | Move up |
| `move_line_down` | Move down |
| `move_visual_line_up` | Move up |
| `move_visual_line_down` | Move down |
| `extend_char_left` | Extend left |
| `extend_char_right` | Extend right |
| `extend_line_up` | Extend up |
| `extend_line_down` | Extend down |
| `extend_visual_line_up` | Extend up |
| `extend_visual_line_down` | Extend down |
| `copy_selection_on_next_line` | Copy selection on next line |
| `copy_selection_on_prev_line` | Copy selection on previous line |
| `move_next_word_start` | Move to start of next word |
| `move_prev_word_start` | Move to start of previous word |
| `move_next_word_end` | Move to end of next word |
| `move_prev_word_end` | Move to end of previous word |
| `move_next_long_word_start` | Move to start of next long word |
| `move_prev_long_word_start` | Move to start of previous long word |
| `move_next_long_word_end` | Move to end of next long word |
| `move_prev_long_word_end` | Move to end of previous long word |
| `move_parent_node_end` | Move to end of the parent node |
| `move_parent_node_start` | Move to beginning of the parent node |
| `extend_next_word_start` | Extend to start of next word |
| `extend_prev_word_start` | Extend to start of previous word |
| `extend_next_word_end` | Extend to end of next word |
| `extend_prev_word_end` | Extend to end of previous word |
| `extend_next_long_word_start` | Extend to start of next long word |
| `extend_prev_long_word_start` | Extend to start of previous long word |
| `extend_next_long_word_end` | Extend to end of next long word |
| `extend_prev_long_word_end` | Extend to end of prev long word |
| `extend_parent_node_end` | Extend to end of the parent node |
| `extend_parent_node_start` | Extend to beginning of the parent node |
| `find_till_char` | Move till next occurrence of char |
| `find_next_char` | Move to next occurrence of char |
| `extend_till_char` | Extend till next occurrence of char |
| `extend_next_char` | Extend to next occurrence of char |
| `till_prev_char` | Move till previous occurrence of char |
| `find_prev_char` | Move to previous occurrence of char |
| `extend_till_prev_char` | Extend till previous occurrence of char |
| `extend_prev_char` | Extend to previous occurrence of char |
| `repeat_last_motion` | Repeat last motion |
| `replace` | Replace with new char |
| `switch_case` | Switch (toggle) case |
| `switch_to_uppercase` | Switch to uppercase |
| `switch_to_lowercase` | Switch to lowercase |
| `page_up` | Move page up |
| `page_down` | Move page down |
| `half_page_up` | Move half page up |
| `half_page_down` | Move half page down |
| `page_cursor_up` | Move page and cursor up |
| `page_cursor_down` | Move page and cursor down |
| `page_cursor_half_up` | Move page and cursor half up |
| `page_cursor_half_down` | Move page and cursor half down |
| `select_all` | Select whole document |
| `select_regex` | Select all regex matches inside selections |
| `split_selection` | Split selections on regex matches |
| `split_selection_on_newline` | Split selection on newlines |
| `merge_selections` | Merge selections |
| `merge_consecutive_selections` | Merge consecutive selections |
| `search` | Search for regex pattern |
| `rsearch` | Reverse search for regex pattern |
| `search_next` | Select next search match |
| `search_prev` | Select previous search match |
| `extend_search_next` | Add next search match to selection |
| `extend_search_prev` | Add previous search match to selection |
| `search_selection` | Use current selection as search pattern |
| `make_search_word_bounded` | Modify current search to make it word bounded |
| `global_search` | Global search in workspace folder |
| `extend_line` | Select current l |
| `extend_line_below` | Select current l |
| `extend_line_above` | Select current l |
| `select_line_above` | Select current l |
| `select_line_below` | Select current l |
| `extend_to_line_bounds` | Extend selection to line bounds |
| `shrink_to_line_bounds` | Shrink selection to line bounds |
| `delete_selection` | Delete selection |
| `delete_selection_noyank` | Delete selection without yanking |
| `change_selection` | Change selection |
| `change_selection_noyank` | Change selection without yanking |
| `collapse_selection` | Collapse selection into single cursor |
| `flip_selections` | Flip selection cursor and anchor |
| `ensure_selections_forward` | Ensure all selections face forward |
| `insert_mode` | Insert before selection |
| `append_mode` | Append after selection |
| `command_mode` | Enter command mode |
| `file_picker` | Open file picker |
| `file_picker_in_current_buffer_directory` | Open file picker at current buffer's directory |
| `file_picker_in_current_directory` | Open file picker at current working directory |
| `code_action` | Perform code action |
| `buffer_picker` | Open buffer picker |
| `jumplist_picker` | Open jumplist picker |
| `symbol_picker` | Open symbol picker |
| `changed_file_picker` | Open changed file picker |
| `select_references_to_symbol_under_cursor` | Select symbol references |
| `workspace_symbol_picker` | Open workspace symbol picker |
| `diagnostics_picker` | Open diagnostic picker |
| `workspace_diagnostics_picker` | Open workspace diagnostic picker |
| `last_picker` | Open last picker |
| `insert_at_line_start` | Insert at start of line |
| `insert_at_line_end` | Insert at end of line |
| `open_below` | Open new line below selection |
| `open_above` | Open new line above selection |
| `normal_mode` | Enter normal mode |
| `select_mode` | Enter selection extend mode |
| `exit_select_mode` | Exit selection mode |
| `goto_definition` | Goto definition |
| `goto_declaration` | Goto declaration |
| `add_newline_above` | Add newline above |
| `add_newline_below` | Add newline below |
| `goto_type_definition` | Goto type definition |
| `goto_implementation` | Goto implementation |
| `goto_file_start` | Goto line number <n> else file start |
| `goto_file_end` | Goto file end |
| `goto_file` | Goto files/URLs in selections |
| `goto_file_hsplit` | Goto files in selections (hsplit) |
| `goto_file_vsplit` | Goto files in selections (vsplit) |
| `goto_reference` | Goto references |
| `goto_window_top` | Goto window top |
| `goto_window_center` | Goto window center |
| `goto_window_bottom` | Goto window bottom |
| `goto_last_accessed_file` | Goto last accessed file |
| `goto_last_modified_file` | Goto last modified file |
| `goto_last_modification` | Goto last modification |
| `goto_line` | Goto line |
| `goto_last_line` | Goto last line |
| `goto_first_diag` | Goto first diagnostic |
| `goto_last_diag` | Goto last diagnostic |
| `goto_next_diag` | Goto next diagnostic |
| `goto_prev_diag` | Goto previous diagnostic |
| `goto_next_change` | Goto next change |
| `goto_prev_change` | Goto previous change |
| `goto_first_change` | Goto first change |
| `goto_last_change` | Goto last change |
| `goto_line_start` | Goto line start |
| `goto_line_end` | Goto line end |
| `goto_next_buffer` | Goto next buffer |
| `goto_previous_buffer` | Goto previous buffer |
| `goto_line_end_newline` | Goto newline at line end |
| `goto_first_nonwhitespace` | Goto first non-blank in line |
| `trim_selections` | Trim whitespace from selections |
| `extend_to_line_start` | Extend to line start |
| `extend_to_first_nonwhitespace` | Extend to first non-blank in line |
| `extend_to_line_end` | Extend to line end |
| `extend_to_line_end_newline` | Extend to line end |
| `signature_help` | Show signature help |
| `smart_tab` | Insert tab if all cursors have all whitespace to their left; otherw |
| `insert_tab` | Insert tab char |
| `insert_newline` | Insert newline char |
| `delete_char_backward` | Delete previous char |
| `delete_char_forward` | Delete next char |
| `delete_word_backward` | Delete previous word |
| `delete_word_forward` | Delete next word |
| `kill_to_line_start` | Delete till start of line |
| `kill_to_line_end` | Delete till end of line |
| `undo` | Undo change |
| `redo` | Redo change |
| `earlier` | Move backward in history |
| `later` | Move forward in history |
| `commit_undo_checkpoint` | Commit changes to new checkpoint |
| `yank` | Yank selection |
| `yank_to_clipboard` | Yank selections to clipboard |
| `yank_to_primary_clipboard` | Yank selections to primary clipboard |
| `yank_joined` | Join and yank selections |
| `yank_joined_to_clipboard` | Join and yank selections to clipboard |
| `yank_main_selection_to_clipboard` | Yank main selection to clipboard |
| `yank_joined_to_primary_clipboard` | Join and yank selections to primary clipboard |
| `yank_main_selection_to_primary_clipboard` | Yank main selection to primary clipboard |
| `replace_with_yanked` | Replace with yanked text |
| `replace_selections_with_clipboard` | Replace selections by clipboard content |
| `replace_selections_with_primary_clipboard` | Replace selections by primary clipboard |
| `paste_after` | Paste after selection |
| `paste_before` | Paste before selection |
| `paste_clipboard_after` | Paste clipboard after selections |
| `paste_clipboard_before` | Paste clipboard before selections |
| `paste_primary_clipboard_after` | Paste primary clipboard after selections |
| `paste_primary_clipboard_before` | Paste primary clipboard before selections |
| `indent` | Indent selection |
| `unindent` | Unindent selection |
| `format_selections` | Format selection |
| `join_selections` | Join lines inside selection |
| `join_selections_space` | Join lines inside selection and select spaces |
| `keep_selections` | Keep selections matching regex |
| `remove_selections` | Remove selections matching regex |
| `align_selections` | Align selections in column |
| `keep_primary_selection` | Keep primary selection |
| `remove_primary_selection` | Remove primary selection |
| `completion` | Invoke completion popup |
| `hover` | Show docs for item under cursor |
| `toggle_comments` | Comment/uncomment selections |
| `toggle_line_comments` | Line comment/uncomment selections |
| `toggle_block_comments` | Block comment/uncomment selections |
| `rotate_selections_forward` | Rotate selections forward |
| `rotate_selections_backward` | Rotate selections backward |
| `rotate_selection_contents_forward` | Rotate selection contents forward |
| `rotate_selection_contents_backward` | Rotate selections contents backward |
| `reverse_selection_contents` | Reverse selections contents |
| `expand_selection` | Expand selection to parent syntax node |
| `shrink_selection` | Shrink selection to previously expanded syntax node |
| `select_next_sibling` | Select next sibling in the syntax tree |
| `select_prev_sibling` | Select previous sibling the in syntax tree |
| `select_all_siblings` | Select all siblings of the current node |
| `select_all_children` | Select all children of the current node |
| `jump_forward` | Jump forward on jumplist |
| `jump_backward` | Jump backward on jumplist |
| `save_selection` | Save current selection to jumplist |
| `jump_view_right` | Jump to right split |
| `jump_view_left` | Jump to left split |
| `jump_view_up` | Jump to split above |
| `jump_view_down` | Jump to split below |
| `swap_view_right` | Swap with right split |
| `swap_view_left` | Swap with left split |
| `swap_view_up` | Swap with split above |
| `swap_view_down` | Swap with split below |
| `transpose_view` | Transpose splits |
| `rotate_view` | Goto next window |
| `rotate_view_reverse` | Goto previous window |
| `hsplit` | Horizontal bottom split |
| `hsplit_new` | Horizontal bottom split scratch buffer |
| `vsplit` | Vertical right split |
| `vsplit_new` | Vertical right split scratch buffer |
| `wclose` | Close window |
| `wonly` | Close windows except current |
| `select_register` | Select register |
| `insert_register` | Insert register |
| `align_view_middle` | Align view middle |
| `align_view_top` | Align view top |
| `align_view_center` | Align view center |
| `align_view_bottom` | Align view bottom |
| `scroll_up` | Scroll view up |
| `scroll_down` | Scroll view down |
| `match_brackets` | Goto matching bracket |
| `surround_add` | Surround add |
| `surround_replace` | Surround replace |
| `surround_delete` | Surround delete |
| `select_textobject_around` | Select around object |
| `select_textobject_inner` | Select inside object |
| `goto_next_function` | Goto next function |
| `goto_prev_function` | Goto previous function |
| `goto_next_class` | Goto next type definition |
| `goto_prev_class` | Goto previous type definition |
| `goto_next_parameter` | Goto next parameter |
| `goto_prev_parameter` | Goto previous parameter |
| `goto_next_comment` | Goto next comment |
| `goto_prev_comment` | Goto previous comment |
| `goto_next_test` | Goto next test |
| `goto_prev_test` | Goto previous test |
| `goto_next_entry` | Goto next pairing |
| `goto_prev_entry` | Goto previous pairing |
| `goto_next_paragraph` | Goto next paragraph |
| `goto_prev_paragraph` | Goto previous paragraph |
| `dap_launch` | Launch debug target |
| `dap_restart` | Restart debugging session |
| `dap_toggle_breakpoint` | Toggle breakpoint |
| `dap_continue` | Continue program execution |
| `dap_pause` | Pause program execution |
| `dap_step_in` | Step in |
| `dap_step_out` | Step out |
| `dap_next` | Step to next |
| `dap_variables` | List variables |
| `dap_terminate` | End debug session |
| `dap_edit_condition` | Edit breakpoint condition on current line |
| `dap_edit_log` | Edit breakpoint log message on current line |
| `dap_switch_thread` | Switch current thread |
| `dap_switch_stack_frame` | Switch stack frame |
| `dap_enable_exceptions` | Enable exception breakpoints |
| `dap_disable_exceptions` | Disable exception breakpoints |
| `shell_pipe` | Pipe selections through shell command |
| `shell_pipe_to` | Pipe selections into shell command ignoring output |
| `shell_insert_output` | Insert shell command output before selections |
| `shell_append_output` | Append shell command output after selections |
| `shell_keep_pipe` | Filter selections with shell predicate |
| `suspend` | Suspend and return to shell |
| `rename_symbol` | Rename symbol |
| `increment` | Increment item under cursor |
| `decrement` | Decrement item under cursor |
| `record_macro` | Record macro |
| `replay_macro` | Replay macro |
| `command_palette` | Open command palette |
| `goto_word` | Jump to a two-character label |
| `extend_to_word` | Extend to a two-character label |
