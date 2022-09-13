| Normal | Insert | Select | Name | Description |
| --- | --- | --- | --- | --- |
| - | - | - | `no_op` | Do nothing |
| `left`, `h` | - | - | `move_char_left` | Move left |
| `right`, `l` | - | - | `move_char_right` | Move right |
| `up`, `k` | - | - | `move_line_up` | Move up |
| `down`, `j` | - | - | `move_line_down` | Move down |
| - | - | `left`, `h` | `extend_char_left` | Extend left |
| - | - | `right`, `l` | `extend_char_right` | Extend right |
| - | - | `up`, `k` | `extend_line_up` | Extend up |
| - | - | `down`, `j` | `extend_line_down` | Extend down |
| `C` | - | `C` | `copy_selection_on_next_line` | Copy selection on next line |
| `A-C` | - | `A-C` | `copy_selection_on_prev_line` | Copy selection on previous line |
| `w` | - | - | `move_next_word_start` | Move to start of next word |
| `b` | - | - | `move_prev_word_start` | Move to start of previous word |
| - | - | - | `move_prev_word_end` | Move to end of previous word |
| `e` | - | - | `move_next_word_end` | Move to end of next word |
| `W` | - | - | `move_next_long_word_start` | Move to start of next long word |
| `B` | - | - | `move_prev_long_word_start` | Move to start of previous long word |
| `E` | - | - | `move_next_long_word_end` | Move to end of next long word |
| - | - | `w` | `extend_next_word_start` | Extend to start of next word |
| - | - | `b` | `extend_prev_word_start` | Extend to start of previous word |
| - | - | `W` | `extend_next_long_word_start` | Extend to start of next long word |
| - | - | `B` | `extend_prev_long_word_start` | Extend to start of previous long word |
| - | - | `E` | `extend_next_long_word_end` | Extend to end of next long word |
| - | - | `e` | `extend_next_word_end` | Extend to end of next word |
| `t` | - | - | `find_till_char` | Move till next occurrence of char |
| `f` | - | - | `find_next_char` | Move to next occurrence of char |
| - | - | `t` | `extend_till_char` | Extend till next occurrence of char |
| - | - | `f` | `extend_next_char` | Extend to next occurrence of char |
| `T` | - | - | `till_prev_char` | Move till previous occurrence of char |
| `F` | - | - | `find_prev_char` | Move to previous occurrence of char |
| - | - | `T` | `extend_till_prev_char` | Extend till previous occurrence of char |
| - | - | `F` | `extend_prev_char` | Extend to previous occurrence of char |
| `A-.` | - | `A-.` | `repeat_last_motion` | Repeat last motion |
| `r` | - | `r` | `replace` | Replace with new char |
| `~` | - | `~` | `switch_case` | Switch (toggle) case |
| `` A-` `` | - | `` A-` `` | `switch_to_uppercase` | Switch to uppercase |
| `` ` `` | - | `` ` `` | `switch_to_lowercase` | Switch to lowercase |
| `pageup`, `Z pageup`, `Z C-b`, `C-b`, `z pageup`, `z C-b` | - | `pageup`, `Z pageup`, `Z C-b`, `C-b`, `z pageup`, `z C-b` | `page_up` | Move page up |
| `pagedown`, `Z pagedown`, `Z C-f`, `C-f`, `z pagedown`, `z C-f` | - | `pagedown`, `Z pagedown`, `Z C-f`, `C-f`, `z pagedown`, `z C-f` | `page_down` | Move page down |
| `Z backspace`, `Z C-u`, `C-u`, `z backspace`, `z C-u` | - | `Z backspace`, `Z C-u`, `C-u`, `z backspace`, `z C-u` | `half_page_up` | Move half page up |
| `Z space`, `Z C-d`, `C-d`, `z space`, `z C-d` | - | `Z space`, `Z C-d`, `C-d`, `z space`, `z C-d` | `half_page_down` | Move half page down |
| `%` | - | `%` | `select_all` | Select whole document |
| `s` | - | `s` | `select_regex` | Select all regex matches inside selections |
| `S` | - | `S` | `split_selection` | Split selections on regex matches |
| `A-s` | - | `A-s` | `split_selection_on_newline` | Split selection on newlines |
| `/`, `Z /`, `z /` | - | `/`, `Z /`, `z /` | `search` | Search for regex pattern |
| `?`, `Z ?`, `z ?` | - | `?`, `Z ?`, `z ?` | `rsearch` | Reverse search for regex pattern |
| `Z n`, `n`, `z n` | - | `Z n`, `z n` | `search_next` | Select next search match |
| `N`, `Z N`, `z N` | - | `Z N`, `z N` | `search_prev` | Select previous search match |
| - | - | `n` | `extend_search_next` | Add next search match to selection |
| - | - | `N` | `extend_search_prev` | Add previous search match to selection |
| `*` | - | `*` | `search_selection` | Use current selection as search pattern |
| `space /` | - | `space /` | `global_search` | Global search in workspace folder |
| - | - | - | `extend_line` | Select current line, if already selected, extend to another line based on the anchor |
| `x` | - | `x` | `extend_line_below` | Select current line, if already selected, extend to next line |
| - | - | - | `extend_line_above` | Select current line, if already selected, extend to previous line |
| `X` | - | `X` | `extend_to_line_bounds` | Extend selection to line bounds |
| `A-x` | - | `A-x` | `shrink_to_line_bounds` | Shrink selection to line bounds |
| `d` | - | `d` | `delete_selection` | Delete selection |
| `A-d` | - | `A-d` | `delete_selection_noyank` | Delete selection without yanking |
| `c` | - | `c` | `change_selection` | Change selection |
| `A-c` | - | `A-c` | `change_selection_noyank` | Change selection without yanking |
| `;` | - | `;` | `collapse_selection` | Collapse selection into single cursor |
| `A-;` | - | `A-;` | `flip_selections` | Flip selection cursor and anchor |
| `A-:` | - | `A-:` | `ensure_selections_forward` | Ensure all selections face forward |
| `i` | - | `i` | `insert_mode` | Insert before selection |
| `a` | - | `a` | `append_mode` | Append after selection |
| `:` | - | `:` | `command_mode` | Enter command mode |
| `space f` | - | `space f` | `file_picker` | Open file picker |
| `space F` | - | `space F` | `file_picker_in_current_directory` | Open file picker at current working directory |
| `space a` | - | `space a` | `code_action` | Perform code action (**LSP**) |
| `space b` | - | `space b` | `buffer_picker` | Open buffer picker |
| `space j` | - | `space j` | `jumplist_picker` | Open jumplist picker |
| `space s` | - | `space s` | `symbol_picker` | Open symbol picker (**LSP**) |
| `space h` | - | `space h` | `select_references_to_symbol_under_cursor` | Select symbol references (**LSP**) |
| `space S` | - | `space S` | `workspace_symbol_picker` | Open workspace symbol picker (**LSP**) |
| `space g` | - | `space g` | `diagnostics_picker` | Open diagnostic picker (**LSP**) |
| `space G` | - | `space G` | `workspace_diagnostics_picker` | Open workspace diagnostic picker (**LSP**) |
| `space '` | - | `space '` | `last_picker` | Open last picker |
| `I` | - | `I` | `prepend_to_line` | Insert at start of line |
| `A` | - | `A` | `append_to_line` | Append to end of line |
| `o` | - | `o` | `open_below` | Open new line below selection |
| `O` | - | `O` | `open_above` | Open new line above selection |
| `esc` | `esc` | `v` | `normal_mode` | Enter normal mode |
| `v` | - | - | `select_mode` | Enter selection extend mode |
| - | - | `esc` | `exit_select_mode` | Exit selection mode |
| `[ space` | - | `[ space` | `add_newline_above` | Add newline above |
| `] space` | - | `] space` | `add_newline_below` | Add newline below |
| `g d` | - | `g d` | `goto_definition` | Goto definition (**LSP**) |
| `g y` | - | `g y` | `goto_type_definition` | Goto type definition (**LSP**) |
| `g r` | - | `g r` | `goto_reference` | Goto references (**LSP**) |
| `g i` | - | `g i` | `goto_implementation` | Goto implementation (**LSP**) |
| `g g` | - | `g g` | `goto_file_start` | Goto line number <n> else file start |
| - | - | - | `goto_file_end` | Goto file end |
| `g f` | - | `g f` | `goto_file` | Goto files in selection |
| `space w f`, `C-w f` | - | `space w f`, `C-w f` | `goto_file_hsplit` | Goto files in selection (hsplit) |
| `space w F`, `C-w F` | - | `space w F`, `C-w F` | `goto_file_vsplit` | Goto files in selection (vsplit) |
| `g t` | - | `g t` | `goto_window_top` | Goto window top |
| `g c` | - | `g c` | `goto_window_center` | Goto window center |
| `g b` | - | `g b` | `goto_window_bottom` | Goto window bottom |
| `g a` | - | `g a` | `goto_last_accessed_file` | Goto last accessed file |
| `g m` | - | `g m` | `goto_last_modified_file` | Goto last modified file |
| `g .` | - | `g .` | `goto_last_modification` | Goto last modification |
| `G` | - | `G` | `goto_line` | Goto line |
| `g e` | - | `g e` | `goto_last_line` | Goto last line |
| `[ D` | - | `[ D` | `goto_first_diag` | Goto first diagnostic (**LSP**) |
| `] D` | - | `] D` | `goto_last_diag` | Goto last diagnostic (**LSP**) |
| `] d` | - | `] d` | `goto_next_diag` | Goto next diagnostic (**LSP**) |
| `[ d` | - | `[ d` | `goto_prev_diag` | Goto previous diagnostic (**LSP**) |
| `home`, `g h` | - | `g h` | `goto_line_start` | Goto line start |
| `end`, `g l` | - | `g l` | `goto_line_end` | Goto line end |
| `g n` | - | `g n` | `goto_next_buffer` | Goto next buffer |
| `g p` | - | `g p` | `goto_previous_buffer` | Goto previous buffer |
| - | - | - | `goto_line_end_newline` | Goto line end |
| `g s` | - | `g s` | `goto_first_nonwhitespace` | Goto first non-blank in line |
| `_` | - | `_` | `trim_selections` | Trim whitespace from selections |
| - | - | `home` | `extend_to_line_start` | Extend to line start |
| - | - | `end` | `extend_to_line_end` | Extend to line end |
| - | - | - | `extend_to_line_end_newline` | Extend to line end |
| - | - | - | `signature_help` | Show signature help |
| - | `tab` | - | `insert_tab` | Insert tab char |
| - | `ret`, `C-j` | - | `insert_newline` | Insert newline char |
| - | `backspace`, `C-h` | - | `delete_char_backward` | Delete previous char |
| - | `del`, `C-d` | - | `delete_char_forward` | Delete next char |
| - | `A-backspace`, `C-w` | - | `delete_word_backward` | Delete previous word |
| - | `A-del`, `A-d` | - | `delete_word_forward` | Delete next word |
| - | `C-u` | - | `kill_to_line_start` | Delete till start of line |
| - | `C-k` | - | `kill_to_line_end` | Delete till end of line |
| `u` | - | `u` | `undo` | Undo change |
| `U` | - | `U` | `redo` | Redo change |
| `A-u` | - | `A-u` | `earlier` | Move backward in history |
| `A-U` | - | `A-U` | `later` | Move forward in history |
| - | `C-s` | - | `commit_undo_checkpoint` | Commit changes to new checkpoint |
| `y` | - | `y` | `yank` | Yank selection |
| `space y` | - | `space y` | `yank_joined_to_clipboard` | Join and yank selections to clipboard |
| `space Y` | - | `space Y` | `yank_main_selection_to_clipboard` | Yank main selection to clipboard |
| - | - | - | `yank_joined_to_primary_clipboard` | Join and yank selections to primary clipboard |
| - | - | - | `yank_main_selection_to_primary_clipboard` | Yank main selection to primary clipboard |
| `R` | - | `R` | `replace_with_yanked` | Replace with yanked text |
| `space R` | - | `space R` | `replace_selections_with_clipboard` | Replace selections by clipboard content |
| - | - | - | `replace_selections_with_primary_clipboard` | Replace selections by primary clipboard |
| `p` | - | `p` | `paste_after` | Paste after selection |
| `P` | - | `P` | `paste_before` | Paste before selection |
| `space p` | - | `space p` | `paste_clipboard_after` | Paste clipboard after selections |
| `space P` | - | `space P` | `paste_clipboard_before` | Paste clipboard before selections |
| - | - | - | `paste_primary_clipboard_after` | Paste primary clipboard after selections |
| - | - | - | `paste_primary_clipboard_before` | Paste primary clipboard before selections |
| `gt` | - | `gt` | `indent` | Indent selection |
| `lt` | - | `lt` | `unindent` | Unindent selection |
| `=` | - | `=` | `format_selections` | Format selection (**LSP**) |
| `J` | - | `J` | `join_selections` | Join lines inside selection |
| `K` | - | `K` | `keep_selections` | Keep selections matching regex |
| `A-K` | - | `A-K` | `remove_selections` | Remove selections matching regex |
| `&` | - | `&` | `align_selections` | Align selections in column |
| `,` | - | `,` | `keep_primary_selection` | Keep primary selection |
| `A-,` | - | `A-,` | `remove_primary_selection` | Remove primary selection |
| - | `C-x` | - | `completion` | Invoke completion popup |
| `space k` | - | `space k` | `hover` | Show docs for item under cursor (**LSP**) |
| `C-c` | - | `C-c` | `toggle_comments` | Comment/uncomment selections |
| `)` | - | `)` | `rotate_selections_forward` | Rotate selections forward |
| `(` | - | `(` | `rotate_selections_backward` | Rotate selections backward |
| `A-)` | - | `A-)` | `rotate_selection_contents_forward` | Rotate selection contents forward |
| `A-(` | - | `A-(` | `rotate_selection_contents_backward` | Rotate selections contents backward |
| `A-up`, `A-o` | - | `A-up`, `A-o` | `expand_selection` | Expand selection to parent syntax node (**TS**) |
| `A-down`, `A-i` | - | `A-down`, `A-i` | `shrink_selection` | Shrink selection to previously expanded syntax node (**TS**) |
| `A-right`, `A-n` | - | `A-right`, `A-n` | `select_next_sibling` | Select next sibling in syntax tree (**TS**) |
| `A-left`, `A-p` | - | `A-left`, `A-p` | `select_prev_sibling` | Select previous sibling in syntax tree (**TS**) |
| `tab` | - | `tab` | `jump_forward` | Jump forward on jumplist |
| `C-o` | - | `C-o` | `jump_backward` | Jump backward on jumplist |
| `C-s` | - | `C-s` | `save_selection` | Save current selection to jumplist |
| `space w right`, `space w l`, `space w C-l`, `C-w right`, `C-w l`, `C-w C-l` | - | `space w right`, `space w l`, `space w C-l`, `C-w right`, `C-w l`, `C-w C-l` | `jump_view_right` | Jump to right split |
| `space w left`, `space w h`, `space w C-h`, `C-w left`, `C-w h`, `C-w C-h` | - | `space w left`, `space w h`, `space w C-h`, `C-w left`, `C-w h`, `C-w C-h` | `jump_view_left` | Jump to left split |
| `space w up`, `space w k`, `space w C-k`, `C-w up`, `C-w k`, `C-w C-k` | - | `space w up`, `space w k`, `space w C-k`, `C-w up`, `C-w k`, `C-w C-k` | `jump_view_up` | Jump to split above |
| `space w down`, `space w j`, `space w C-j`, `C-w down`, `C-w j`, `C-w C-j` | - | `space w down`, `space w j`, `space w C-j`, `C-w down`, `C-w j`, `C-w C-j` | `jump_view_down` | Jump to split below |
| `space w L`, `C-w L` | - | `space w L`, `C-w L` | `swap_view_right` | Swap with right split |
| `space w H`, `C-w H` | - | `space w H`, `C-w H` | `swap_view_left` | Swap with left split |
| `space w K`, `C-w K` | - | `space w K`, `C-w K` | `swap_view_up` | Swap with split above |
| `space w J`, `C-w J` | - | `space w J`, `C-w J` | `swap_view_down` | Swap with split below |
| `space w t`, `space w C-t`, `C-w t`, `C-w C-t` | - | `space w t`, `space w C-t`, `C-w t`, `C-w C-t` | `transpose_view` | Transpose splits |
| `space w w`, `space w C-w`, `C-w w`, `C-w C-w` | - | `space w w`, `space w C-w`, `C-w w`, `C-w C-w` | `rotate_view` | Goto next window |
| `space w s`, `space w C-s`, `C-w s`, `C-w C-s` | - | `space w s`, `space w C-s`, `C-w s`, `C-w C-s` | `hsplit` | Horizontal bottom split |
| `space w n s`, `space w n C-s`, `C-w n s`, `C-w n C-s` | - | `space w n s`, `space w n C-s`, `C-w n s`, `C-w n C-s` | `hsplit_new` | Horizontal bottom split scratch buffer |
| `space w v`, `space w C-v`, `C-w v`, `C-w C-v` | - | `space w v`, `space w C-v`, `C-w v`, `C-w C-v` | `vsplit` | Vertical right split |
| `space w n v`, `space w n C-v`, `C-w n v`, `C-w n C-v` | - | `space w n v`, `space w n C-v`, `C-w n v`, `C-w n C-v` | `vsplit_new` | Vertical right split scratch buffer |
| `space w q`, `space w C-q`, `C-w q`, `C-w C-q` | - | `space w q`, `space w C-q`, `C-w q`, `C-w C-q` | `wclose` | Close window |
| `space w o`, `space w C-o`, `C-w o`, `C-w C-o` | - | `space w o`, `space w C-o`, `C-w o`, `C-w C-o` | `wonly` | Close windows except current |
| `"` | - | `"` | `select_register` | Select register |
| - | `C-r` | - | `insert_register` | Insert register |
| `Z m`, `z m` | - | `Z m`, `z m` | `align_view_middle` | Align view middle |
| `Z t`, `z t` | - | `Z t`, `z t` | `align_view_top` | Align view top |
| `Z c`, `Z z`, `z c`, `z z` | - | `Z c`, `Z z`, `z c`, `z z` | `align_view_center` | Align view center |
| `Z b`, `z b` | - | `Z b`, `z b` | `align_view_bottom` | Align view bottom |
| `Z up`, `Z k`, `z up`, `z k` | - | `Z up`, `Z k`, `z up`, `z k` | `scroll_up` | Scroll view up |
| `Z down`, `Z j`, `z down`, `z j` | - | `Z down`, `Z j`, `z down`, `z j` | `scroll_down` | Scroll view down |
| `m m` | - | `m m` | `match_brackets` | Goto matching bracket (**TS**) |
| `m s` | - | `m s` | `surround_add` | Surround add |
| `m r` | - | `m r` | `surround_replace` | Surround replace |
| `m d` | - | `m d` | `surround_delete` | Surround delete |
| `m a` | - | `m a` | `select_textobject_around` | Select around object |
| `m i` | - | `m i` | `select_textobject_inner` | Select inside object |
| `] f` | - | `] f` | `goto_next_function` | Goto next function (**TS**) |
| `[ f` | - | `[ f` | `goto_prev_function` | Goto previous function (**TS**) |
| `] c` | - | `] c` | `goto_next_class` | Goto next class (**TS**) |
| `[ c` | - | `[ c` | `goto_prev_class` | Goto previous class (**TS**) |
| `] a` | - | `] a` | `goto_next_parameter` | Goto next parameter (**TS**) |
| `[ a` | - | `[ a` | `goto_prev_parameter` | Goto previous parameter (**TS**) |
| `] o` | - | `] o` | `goto_next_comment` | Goto next comment (**TS**) |
| `[ o` | - | `[ o` | `goto_prev_comment` | Goto previous comment (**TS**) |
| `] t` | - | `] t` | `goto_next_test` | Goto next test (**TS**) |
| `[ t` | - | `[ t` | `goto_prev_test` | Goto previous test (**TS**) |
| `] p` | - | `] p` | `goto_next_paragraph` | Goto next paragraph (**TS**) |
| `[ p` | - | `[ p` | `goto_prev_paragraph` | Goto previous paragraph (**TS**) |
| `space d l` | - | `space d l` | `dap_launch` | Launch debug target (**DAP**) |
| `space d b` | - | `space d b` | `dap_toggle_breakpoint` | Toggle breakpoint (**DAP**) |
| `space d c` | - | `space d c` | `dap_continue` | Continue program execution (**DAP**) |
| `space d h` | - | `space d h` | `dap_pause` | Pause program execution (**DAP**) |
| `space d i` | - | `space d i` | `dap_step_in` | Step in (**DAP**) |
| `space d o` | - | `space d o` | `dap_step_out` | Step out (**DAP**) |
| `space d n` | - | `space d n` | `dap_next` | Step to next (**DAP**) |
| `space d v` | - | `space d v` | `dap_variables` | List variables (**DAP**) |
| `space d t` | - | `space d t` | `dap_terminate` | End debug session (**DAP**) |
| `space d C-c` | - | `space d C-c` | `dap_edit_condition` | Edit breakpoint condition on current line (**DAP**) |
| `space d C-l` | - | `space d C-l` | `dap_edit_log` | Edit breakpoint log message on current line (**DAP**) |
| `space d s t` | - | `space d s t` | `dap_switch_thread` | Switch current thread (**DAP**) |
| `space d s f` | - | `space d s f` | `dap_switch_stack_frame` | Switch stack frame (**DAP**) |
| `space d e` | - | `space d e` | `dap_enable_exceptions` | Enable exception breakpoints (**DAP**) |
| `space d E` | - | `space d E` | `dap_disable_exceptions` | Disable exception breakpoints (**DAP**) |
| <code>&#124;</code> | - | <code>&#124;</code> | `shell_pipe` | Pipe selections through shell command |
| <code>A-&#124;</code> | - | <code>A-&#124;</code> | `shell_pipe_to` | Pipe selections into shell command ignoring output |
| `!` | - | `!` | `shell_insert_output` | Insert shell command output before selections |
| `A-!` | - | `A-!` | `shell_append_output` | Append shell command output after selections |
| `$` | - | `$` | `shell_keep_pipe` | Filter selections with shell predicate |
| `C-z` | - | `C-z` | `suspend` | Suspend and return to shell |
| `space r` | - | `space r` | `rename_symbol` | Rename symbol (**LSP**) |
| `C-a` | - | `C-a` | `increment` | Increment item under cursor |
| `C-x` | - | `C-x` | `decrement` | Decrement item under cursor |
| `Q` | - | `Q` | `record_macro` | Record macro |
| `q` | - | `q` | `replay_macro` | Replay macro |
| `space ?` | - | `space ?` | `command_palette` | Open command palette |
