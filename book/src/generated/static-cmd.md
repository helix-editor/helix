| Name | Description |
| --- | --- |
| no_op | Do nothing |
| move_char_left | Move left |
| move_char_right | Move right |
| move_line_up | Move up |
| move_line_down | Move down |
| extend_char_left | Extend left |
| extend_char_right | Extend right |
| extend_line_up | Extend up |
| extend_line_down | Extend down |
| copy_selection_on_next_line | Copy selection on next line |
| copy_selection_on_prev_line | Copy selection on previous line |
| move_next_word_start | Move to start of next word |
| move_prev_word_start | Move to start of previous word |
| move_prev_word_end | Move to end of previous word |
| move_next_word_end | Move to end of next word |
| move_next_long_word_start | Move to start of next long word |
| move_prev_long_word_start | Move to start of previous long word |
| move_next_long_word_end | Move to end of next long word |
| extend_next_word_start | Extend to start of next word |
| extend_prev_word_start | Extend to start of previous word |
| extend_next_long_word_start | Extend to start of next long word |
| extend_prev_long_word_start | Extend to start of previous long word |
| extend_next_long_word_end | Extend to end of next long word |
| extend_next_word_end | Extend to end of next word |
| find_till_char | Move till next occurrence of char |
| find_next_char | Move to next occurrence of char |
| extend_till_char | Extend till next occurrence of char |
| extend_next_char | Extend to next occurrence of char |
| till_prev_char | Move till previous occurrence of char |
| find_prev_char | Move to previous occurrence of char |
| extend_till_prev_char | Extend till previous occurrence of char |
| extend_prev_char | Extend to previous occurrence of char |
| repeat_last_motion | Repeat last motion |
| replace | Replace with new char |
| switch_case | Switch (toggle) case |
| switch_to_uppercase | Switch to uppercase |
| switch_to_lowercase | Switch to lowercase |
| page_up | Move page up |
| page_down | Move page down |
| half_page_up | Move half page up |
| half_page_down | Move half page down |
| select_all | Select whole document |
| select_regex | Select all regex matches inside selections |
| split_selection | Split selections on regex matches |
| split_selection_on_newline | Split selection on newlines |
| search | Search for regex pattern |
| rsearch | Reverse search for regex pattern |
| search_next | Select next search match |
| search_prev | Select previous search match |
| extend_search_next | Add next search match to selection |
| extend_search_prev | Add previous search match to selection |
| search_selection | Use current selection as search pattern |
| global_search | Global search in workspace folder |
| extend_line | Select current line, if already selected, extend to another line based on the anchor |
| extend_line_below | Select current line, if already selected, extend to next line |
| extend_line_above | Select current line, if already selected, extend to previous line |
| extend_to_line_bounds | Extend selection to line bounds |
| shrink_to_line_bounds | Shrink selection to line bounds |
| delete_selection | Delete selection |
| delete_selection_noyank | Delete selection without yanking |
| change_selection | Change selection |
| change_selection_noyank | Change selection without yanking |
| collapse_selection | Collapse selection into single cursor |
| flip_selections | Flip selection cursor and anchor |
| ensure_selections_forward | Ensure all selections face forward |
| insert_mode | Insert before selection |
| append_mode | Append after selection |
| command_mode | Enter command mode |
| file_picker | Open file picker |
| file_picker_in_current_directory | Open file picker at current working directory |
| code_action | Perform code action |
| buffer_picker | Open buffer picker |
| jumplist_picker | Open jumplist picker |
| symbol_picker | Open symbol picker |
| select_references_to_symbol_under_cursor | Select symbol references |
| workspace_symbol_picker | Open workspace symbol picker |
| diagnostics_picker | Open diagnostic picker |
| workspace_diagnostics_picker | Open workspace diagnostic picker |
| last_picker | Open last picker |
| insert_at_line_start | Insert at start of line |
| insert_at_line_end | Insert at end of line |
| open_below | Open new line below selection |
| open_above | Open new line above selection |
| normal_mode | Enter normal mode |
| select_mode | Enter selection extend mode |
| exit_select_mode | Exit selection mode |
| goto_definition | Goto definition |
| add_newline_above | Add newline above |
| add_newline_below | Add newline below |
| goto_type_definition | Goto type definition |
| goto_implementation | Goto implementation |
| goto_file_start | Goto line number <n> else file start |
| goto_file_end | Goto file end |
| goto_file | Goto files in selection |
| goto_file_hsplit | Goto files in selection (hsplit) |
| goto_file_vsplit | Goto files in selection (vsplit) |
| goto_reference | Goto references |
| goto_window_top | Goto window top |
| goto_window_center | Goto window center |
| goto_window_bottom | Goto window bottom |
| goto_last_accessed_file | Goto last accessed file |
| goto_last_modified_file | Goto last modified file |
| goto_last_modification | Goto last modification |
| goto_line | Goto line |
| goto_last_line | Goto last line |
| goto_first_diag | Goto first diagnostic |
| goto_last_diag | Goto last diagnostic |
| goto_next_diag | Goto next diagnostic |
| goto_prev_diag | Goto previous diagnostic |
| goto_line_start | Goto line start |
| goto_line_end | Goto line end |
| goto_next_buffer | Goto next buffer |
| goto_previous_buffer | Goto previous buffer |
| goto_line_end_newline | Goto line end |
| goto_first_nonwhitespace | Goto first non-blank in line |
| trim_selections | Trim whitespace from selections |
| extend_to_line_start | Extend to line start |
| extend_to_line_end | Extend to line end |
| extend_to_line_end_newline | Extend to line end |
| signature_help | Show signature help |
| insert_tab | Insert tab char |
| insert_newline | Insert newline char |
| delete_char_backward | Delete previous char |
| delete_char_forward | Delete next char |
| delete_word_backward | Delete previous word |
| delete_word_forward | Delete next word |
| kill_to_line_start | Delete till start of line |
| kill_to_line_end | Delete till end of line |
| undo | Undo change |
| redo | Redo change |
| earlier | Move backward in history |
| later | Move forward in history |
| commit_undo_checkpoint | Commit changes to new checkpoint |
| yank | Yank selection |
| yank_joined_to_clipboard | Join and yank selections to clipboard |
| yank_main_selection_to_clipboard | Yank main selection to clipboard |
| yank_joined_to_primary_clipboard | Join and yank selections to primary clipboard |
| yank_main_selection_to_primary_clipboard | Yank main selection to primary clipboard |
| replace_with_yanked | Replace with yanked text |
| replace_selections_with_clipboard | Replace selections by clipboard content |
| replace_selections_with_primary_clipboard | Replace selections by primary clipboard |
| paste_after | Paste after selection |
| paste_before | Paste before selection |
| paste_clipboard_after | Paste clipboard after selections |
| paste_clipboard_before | Paste clipboard before selections |
| paste_primary_clipboard_after | Paste primary clipboard after selections |
| paste_primary_clipboard_before | Paste primary clipboard before selections |
| indent | Indent selection |
| unindent | Unindent selection |
| format_selections | Format selection |
| join_selections | Join lines inside selection |
| join_selections_space | Join lines inside selection and select spaces |
| keep_selections | Keep selections matching regex |
| remove_selections | Remove selections matching regex |
| align_selections | Align selections in column |
| keep_primary_selection | Keep primary selection |
| remove_primary_selection | Remove primary selection |
| completion | Invoke completion popup |
| hover | Show docs for item under cursor |
| toggle_comments | Comment/uncomment selections |
| rotate_selections_forward | Rotate selections forward |
| rotate_selections_backward | Rotate selections backward |
| rotate_selection_contents_forward | Rotate selection contents forward |
| rotate_selection_contents_backward | Rotate selections contents backward |
| expand_selection | Expand selection to parent syntax node |
| shrink_selection | Shrink selection to previously expanded syntax node |
| select_next_sibling | Select next sibling in syntax tree |
| select_prev_sibling | Select previous sibling in syntax tree |
| jump_forward | Jump forward on jumplist |
| jump_backward | Jump backward on jumplist |
| save_selection | Save current selection to jumplist |
| jump_view_right | Jump to right split |
| jump_view_left | Jump to left split |
| jump_view_up | Jump to split above |
| jump_view_down | Jump to split below |
| swap_view_right | Swap with right split |
| swap_view_left | Swap with left split |
| swap_view_up | Swap with split above |
| swap_view_down | Swap with split below |
| transpose_view | Transpose splits |
| rotate_view | Goto next window |
| hsplit | Horizontal bottom split |
| hsplit_new | Horizontal bottom split scratch buffer |
| vsplit | Vertical right split |
| vsplit_new | Vertical right split scratch buffer |
| wclose | Close window |
| wonly | Close windows except current |
| select_register | Select register |
| insert_register | Insert register |
| align_view_middle | Align view middle |
| align_view_top | Align view top |
| align_view_center | Align view center |
| align_view_bottom | Align view bottom |
| scroll_up | Scroll view up |
| scroll_down | Scroll view down |
| match_brackets | Goto matching bracket |
| surround_add | Surround add |
| surround_replace | Surround replace |
| surround_delete | Surround delete |
| select_textobject_around | Select around object |
| select_textobject_inner | Select inside object |
| goto_next_function | Goto next function |
| goto_prev_function | Goto previous function |
| goto_next_class | Goto next class |
| goto_prev_class | Goto previous class |
| goto_next_parameter | Goto next parameter |
| goto_prev_parameter | Goto previous parameter |
| goto_next_comment | Goto next comment |
| goto_prev_comment | Goto previous comment |
| goto_next_test | Goto next test |
| goto_prev_test | Goto previous test |
| goto_next_paragraph | Goto next paragraph |
| goto_prev_paragraph | Goto previous paragraph |
| dap_launch | Launch debug target |
| dap_toggle_breakpoint | Toggle breakpoint |
| dap_continue | Continue program execution |
| dap_pause | Pause program execution |
| dap_step_in | Step in |
| dap_step_out | Step out |
| dap_next | Step to next |
| dap_variables | List variables |
| dap_terminate | End debug session |
| dap_edit_condition | Edit breakpoint condition on current line |
| dap_edit_log | Edit breakpoint log message on current line |
| dap_switch_thread | Switch current thread |
| dap_switch_stack_frame | Switch stack frame |
| dap_enable_exceptions | Enable exception breakpoints |
| dap_disable_exceptions | Disable exception breakpoints |
| shell_pipe | Pipe selections through shell command |
| shell_pipe_to | Pipe selections into shell command ignoring output |
| shell_insert_output | Insert shell command output before selections |
| shell_append_output | Append shell command output after selections |
| shell_keep_pipe | Filter selections with shell predicate |
| suspend | Suspend and return to shell |
| rename_symbol | Rename symbol |
| increment | Increment item under cursor |
| decrement | Decrement item under cursor |
| record_macro | Record macro |
| replay_macro | Replay macro |
| command_palette | Open command palette |
