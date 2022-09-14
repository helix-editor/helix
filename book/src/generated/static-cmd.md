ANCHOR: all
## Normal
| Key | Description | Command |
| --- | --- | --- |
| `h`, `left` | Move left | `move_char_left` |
| `j`, `down` | Move down | `move_line_down` |
| `k`, `up` | Move up | `move_line_up` |
| `l`, `right` | Move right | `move_char_right` |
| `t` | Move till next occurrence of char | `find_till_char` |
| `f` | Move to next occurrence of char | `find_next_char` |
| `T` | Move till previous occurrence of char | `till_prev_char` |
| `F` | Move to previous occurrence of char | `find_prev_char` |
| `r` | Replace with new char | `replace` |
| `R` | Replace with yanked text | `replace_with_yanked` |
| `A-.` | Repeat last motion | `repeat_last_motion` |
| `~` | Switch (toggle) case | `switch_case` |
| `` ` `` | Switch to lowercase | `switch_to_lowercase` |
| `` A-` `` | Switch to uppercase | `switch_to_uppercase` |
| `home` | Goto line start | `goto_line_start` |
| `end` | Goto line end | `goto_line_end` |
| `w` | Move to start of next word | `move_next_word_start` |
| `b` | Move to start of previous word | `move_prev_word_start` |
| `e` | Move to end of next word | `move_next_word_end` |
| `W` | Move to start of next long word | `move_next_long_word_start` |
| `B` | Move to start of previous long word | `move_prev_long_word_start` |
| `E` | Move to end of next long word | `move_next_long_word_end` |
| `v` | Enter selection extend mode | `select_mode` |
| `G` | Goto line | `goto_line` |
| `g` | Enter [goto mode](#goto) |  |
| `:` | Enter command mode | `command_mode` |
| `i` | Insert before selection | `insert_mode` |
| `I` | Insert at start of line | `prepend_to_line` |
| `a` | Append after selection | `append_mode` |
| `A` | Append to end of line | `append_to_line` |
| `o` | Open new line below selection | `open_below` |
| `O` | Open new line above selection | `open_above` |
| `d` | Delete selection | `delete_selection` |
| `A-d` | Delete selection without yanking | `delete_selection_noyank` |
| `c` | Change selection | `change_selection` |
| `A-c` | Change selection without yanking | `change_selection_noyank` |
| `C` | Copy selection on next line | `copy_selection_on_next_line` |
| `A-C` | Copy selection on previous line | `copy_selection_on_prev_line` |
| `s` | Select all regex matches inside selections | `select_regex` |
| `A-s` | Split selection on newlines | `split_selection_on_newline` |
| `S` | Split selections on regex matches | `split_selection` |
| `;` | Collapse selection into single cursor | `collapse_selection` |
| `A-;` | Flip selection cursor and anchor | `flip_selections` |
| `A-o`, `A-up` | Expand selection to parent syntax node (**TS**) | `expand_selection` |
| `A-i`, `A-down` | Shrink selection to previously expanded syntax node (**TS**) | `shrink_selection` |
| `A-p`, `A-left` | Select previous sibling in syntax tree (**TS**) | `select_prev_sibling` |
| `A-n`, `A-right` | Select next sibling in syntax tree (**TS**) | `select_next_sibling` |
| `%` | Select whole document | `select_all` |
| `x` | Select current line, if already selected, extend to next line | `extend_line_below` |
| `X` | Extend selection to line bounds | `extend_to_line_bounds` |
| `A-x` | Shrink selection to line bounds | `shrink_to_line_bounds` |
| `m` | Enter [match mode](#match) |  |
| `[` | Enter [left bracket mode](#left-bracket) |  |
| `]` | Enter [right bracket mode](#right-bracket) |  |
| `/` | Search for regex pattern | `search` |
| `?` | Reverse search for regex pattern | `rsearch` |
| `n` | Select next search match | `search_next` |
| `N` | Select previous search match | `search_prev` |
| `*` | Use current selection as search pattern | `search_selection` |
| `u` | Undo change | `undo` |
| `U` | Redo change | `redo` |
| `A-u` | Move backward in history | `earlier` |
| `A-U` | Move forward in history | `later` |
| `y` | Yank selection | `yank` |
| `p` | Paste after selection | `paste_after` |
| `P` | Paste before selection | `paste_before` |
| `Q` | Record macro | `record_macro` |
| `q` | Replay macro | `replay_macro` |
| `gt` | Indent selection | `indent` |
| `lt` | Unindent selection | `unindent` |
| `=` | Format selection (**LSP**) | `format_selections` |
| `J` | Join lines inside selection | `join_selections` |
| `K` | Keep selections matching regex | `keep_selections` |
| `A-K` | Remove selections matching regex | `remove_selections` |
| `,` | Keep primary selection | `keep_primary_selection` |
| `A-,` | Remove primary selection | `remove_primary_selection` |
| `&` | Align selections in column | `align_selections` |
| `_` | Trim whitespace from selections | `trim_selections` |
| `(` | Rotate selections backward | `rotate_selections_backward` |
| `)` | Rotate selections forward | `rotate_selections_forward` |
| `A-(` | Rotate selections contents backward | `rotate_selection_contents_backward` |
| `A-)` | Rotate selection contents forward | `rotate_selection_contents_forward` |
| `A-:` | Ensure all selections face forward | `ensure_selections_forward` |
| `esc` | Enter normal mode | `normal_mode` |
| `C-b`, `pageup` | Move page up | `page_up` |
| `C-f`, `pagedown` | Move page down | `page_down` |
| `C-u` | Move half page up | `half_page_up` |
| `C-d` | Move half page down | `half_page_down` |
| `C-w` | Enter [window mode](#window) |  |
| `C-c` | Comment/uncomment selections | `toggle_comments` |
| `tab` | Jump forward on jumplist | `jump_forward` |
| `C-o` | Jump backward on jumplist | `jump_backward` |
| `C-s` | Save current selection to jumplist | `save_selection` |
| `space` | Enter [space mode](#space) |  |
| `z` | Enter [view mode](#view) |  |
| `Z` | Enter sticky [view mode](#view) |  |
| `"` | Select register | `select_register` |
| <code>&#124;</code> | Pipe selections through shell command | `shell_pipe` |
| <code>A-&#124;</code> | Pipe selections into shell command ignoring output | `shell_pipe_to` |
| `!` | Insert shell command output before selections | `shell_insert_output` |
| `A-!` | Append shell command output after selections | `shell_append_output` |
| `$` | Filter selections with shell predicate | `shell_keep_pipe` |
| `C-z` | Suspend and return to shell | `suspend` |
| `C-a` | Increment item under cursor | `increment` |
| `C-x` | Decrement item under cursor | `decrement` |
### Goto
Jumps to various locations.

| Key | Description | Command |
| --- | --- | --- |
| `g` | Goto line number <n> else file start | `goto_file_start` |
| `e` | Goto last line | `goto_last_line` |
| `f` | Goto files in selection | `goto_file` |
| `h` | Goto line start | `goto_line_start` |
| `l` | Goto line end | `goto_line_end` |
| `s` | Goto first non-blank in line | `goto_first_nonwhitespace` |
| `d` | Goto definition (**LSP**) | `goto_definition` |
| `y` | Goto type definition (**LSP**) | `goto_type_definition` |
| `r` | Goto references (**LSP**) | `goto_reference` |
| `i` | Goto implementation (**LSP**) | `goto_implementation` |
| `t` | Goto window top | `goto_window_top` |
| `c` | Goto window center | `goto_window_center` |
| `b` | Goto window bottom | `goto_window_bottom` |
| `a` | Goto last accessed file | `goto_last_accessed_file` |
| `m` | Goto last modified file | `goto_last_modified_file` |
| `n` | Goto next buffer | `goto_next_buffer` |
| `p` | Goto previous buffer | `goto_previous_buffer` |
| `.` | Goto last modification | `goto_last_modification` |
### Match
Enter this mode using `m` from normal mode. See the relevant section
in [Usage](./usage.md) for an explanation about [surround](./usage.md#surround)
and [textobject](./usage.md#textobject) usage.

| Key | Description | Command |
| --- | --- | --- |
| `m` | Goto matching bracket (**TS**) | `match_brackets` |
| `s` | Surround add | `surround_add` |
| `r` | Surround replace | `surround_replace` |
| `d` | Surround delete | `surround_delete` |
| `a` | Select around object | `select_textobject_around` |
| `i` | Select inside object | `select_textobject_inner` |

TODO: Mappings for selecting syntax nodes (a superset of `[`).

### Left bracket
| Key | Description | Command |
| --- | --- | --- |
| `d` | Goto previous diagnostic (**LSP**) | `goto_prev_diag` |
| `D` | Goto first diagnostic (**LSP**) | `goto_first_diag` |
| `f` | Goto previous function (**TS**) | `goto_prev_function` |
| `c` | Goto previous class (**TS**) | `goto_prev_class` |
| `a` | Goto previous parameter (**TS**) | `goto_prev_parameter` |
| `o` | Goto previous comment (**TS**) | `goto_prev_comment` |
| `t` | Goto previous test (**TS**) | `goto_prev_test` |
| `p` | Goto previous paragraph (**TS**) | `goto_prev_paragraph` |
| `space` | Add newline above | `add_newline_above` |
### Right bracket
| Key | Description | Command |
| --- | --- | --- |
| `d` | Goto next diagnostic (**LSP**) | `goto_next_diag` |
| `D` | Goto last diagnostic (**LSP**) | `goto_last_diag` |
| `f` | Goto next function (**TS**) | `goto_next_function` |
| `c` | Goto next class (**TS**) | `goto_next_class` |
| `a` | Goto next parameter (**TS**) | `goto_next_parameter` |
| `o` | Goto next comment (**TS**) | `goto_next_comment` |
| `t` | Goto next test (**TS**) | `goto_next_test` |
| `p` | Goto next paragraph (**TS**) | `goto_next_paragraph` |
| `space` | Add newline below | `add_newline_below` |
### Window
This layer is similar to vim keybindings as kakoune does not support window.

| Key | Description | Command |
| --- | --- | --- |
| `C-w`, `w` | Goto next window | `rotate_view` |
| `C-s`, `s` | Horizontal bottom split | `hsplit` |
| `C-v`, `v` | Vertical right split | `vsplit` |
| `C-t`, `t` | Transpose splits | `transpose_view` |
| `f` | Goto files in selection (hsplit) | `goto_file_hsplit` |
| `F` | Goto files in selection (vsplit) | `goto_file_vsplit` |
| `C-q`, `q` | Close window | `wclose` |
| `C-o`, `o` | Close windows except current | `wonly` |
| `C-h`, `h`, `left` | Jump to left split | `jump_view_left` |
| `C-j`, `j`, `down` | Jump to split below | `jump_view_down` |
| `C-k`, `k`, `up` | Jump to split above | `jump_view_up` |
| `C-l`, `l`, `right` | Jump to right split | `jump_view_right` |
| `L` | Swap with right split | `swap_view_right` |
| `K` | Swap with split above | `swap_view_up` |
| `H` | Swap with left split | `swap_view_left` |
| `J` | Swap with split below | `swap_view_down` |
| `n` | Enter [new split scratch buffer mode](#new-split-scratch-buffer) |  |
#### New split scratch buffer
| Key | Description | Command |
| --- | --- | --- |
| `C-s`, `s` | Horizontal bottom split scratch buffer | `hsplit_new` |
| `C-v`, `v` | Vertical right split scratch buffer | `vsplit_new` |
### Space
This layer is a kludge of mappings, mostly pickers.

| Key | Description | Command |
| --- | --- | --- |
| `f` | Open file picker | `file_picker` |
| `F` | Open file picker at current working directory | `file_picker_in_current_directory` |
| `b` | Open buffer picker | `buffer_picker` |
| `j` | Open jumplist picker | `jumplist_picker` |
| `s` | Open symbol picker (**LSP**) | `symbol_picker` |
| `S` | Open workspace symbol picker (**LSP**) | `workspace_symbol_picker` |
| `g` | Open diagnostic picker (**LSP**) | `diagnostics_picker` |
| `G` | Open workspace diagnostic picker (**LSP**) | `workspace_diagnostics_picker` |
| `a` | Perform code action (**LSP**) | `code_action` |
| `'` | Open last picker | `last_picker` |
| `d` | Enter sticky [debug (experimental) mode](#debug-experimental) |  |
| `w` | Enter [window mode](#window) |  |
| `y` | Join and yank selections to clipboard | `yank_joined_to_clipboard` |
| `Y` | Yank main selection to clipboard | `yank_main_selection_to_clipboard` |
| `p` | Paste clipboard after selections | `paste_clipboard_after` |
| `P` | Paste clipboard before selections | `paste_clipboard_before` |
| `R` | Replace selections by clipboard content | `replace_selections_with_clipboard` |
| `/` | Global search in workspace folder | `global_search` |
| `k` | Show docs for item under cursor (**LSP**) | `hover` |
| `r` | Rename symbol (**LSP**) | `rename_symbol` |
| `h` | Select symbol references (**LSP**) | `select_references_to_symbol_under_cursor` |
| `?` | Open command palette | `command_palette` |

> TIP: Global search displays results in a fuzzy picker, use `space + '` to bring it back up after opening a file.

#### Debug (experimental)
| Key | Description | Command |
| --- | --- | --- |
| `l` | Launch debug target (**DAP**) | `dap_launch` |
| `b` | Toggle breakpoint (**DAP**) | `dap_toggle_breakpoint` |
| `c` | Continue program execution (**DAP**) | `dap_continue` |
| `h` | Pause program execution (**DAP**) | `dap_pause` |
| `i` | Step in (**DAP**) | `dap_step_in` |
| `o` | Step out (**DAP**) | `dap_step_out` |
| `n` | Step to next (**DAP**) | `dap_next` |
| `v` | List variables (**DAP**) | `dap_variables` |
| `t` | End debug session (**DAP**) | `dap_terminate` |
| `C-c` | Edit breakpoint condition on current line (**DAP**) | `dap_edit_condition` |
| `C-l` | Edit breakpoint log message on current line (**DAP**) | `dap_edit_log` |
| `s` | Enter [switch mode](#switch) |  |
| `e` | Enable exception breakpoints (**DAP**) | `dap_enable_exceptions` |
| `E` | Disable exception breakpoints (**DAP**) | `dap_disable_exceptions` |
##### Switch
| Key | Description | Command |
| --- | --- | --- |
| `t` | Switch current thread (**DAP**) | `dap_switch_thread` |
| `f` | Switch stack frame (**DAP**) | `dap_switch_stack_frame` |
### View
View mode is intended for scrolling and manipulating the view without changing
the selection. The "sticky" variant of this mode is persistent; use the
`Escape` key to return to normal mode after usage (useful when you're simply
looking over text and not actively editing it).

| Key | Description | Command |
| --- | --- | --- |
| `z`, `c` | Align view center | `align_view_center` |
| `t` | Align view top | `align_view_top` |
| `b` | Align view bottom | `align_view_bottom` |
| `m` | Align view middle | `align_view_middle` |
| `k`, `up` | Scroll view up | `scroll_up` |
| `j`, `down` | Scroll view down | `scroll_down` |
| `C-b`, `pageup` | Move page up | `page_up` |
| `C-f`, `pagedown` | Move page down | `page_down` |
| `C-u`, `backspace` | Move half page up | `half_page_up` |
| `C-d`, `space` | Move half page down | `half_page_down` |
| `/` | Search for regex pattern | `search` |
| `?` | Reverse search for regex pattern | `rsearch` |
| `n` | Select next search match | `search_next` |
| `N` | Select previous search match | `search_prev` |
## Insert
We support many readline/emacs style bindings in insert mode for convenience.
These can be helpful for making simple modifications without escaping to normal
mode, but beware that you will not have an undo-able "save point" until you
return to normal mode.

| Key | Description | Command |
| --- | --- | --- |
| `esc` | Enter normal mode | `normal_mode` |
| `backspace`, `C-h` | Delete previous char | `delete_char_backward` |
| `del`, `C-d` | Delete next char | `delete_char_forward` |
| `ret`, `C-j` | Insert newline char | `insert_newline` |
| `tab` | Insert tab char | `insert_tab` |
| `C-w`, `A-backspace` | Delete previous word | `delete_word_backward` |
| `A-d`, `A-del` | Delete next word | `delete_word_forward` |
| `C-s` | Commit changes to new checkpoint | `commit_undo_checkpoint` |
| `C-k` | Delete till end of line | `kill_to_line_end` |
| `C-u` | Delete till start of line | `kill_to_line_start` |
| `C-x` | Invoke completion popup | `completion` |
| `C-r` | Insert register | `insert_register` |
## Select
This mode echoes Normal mode, but changes any movements to extend
selections rather than replace them. Goto motions are also changed to
extend, so that `vgl` for example extends the selection to the end of
the line.

Search is also affected. By default, `n` and `N` will remove the current
selection and select the next instance of the search term. Toggling this
mode before pressing `n` or `N` makes it possible to keep the current
selection. Toggling it on and off during your iterative searching allows
you to selectively add search terms to your selections.

| Key | Description | Command |
| --- | --- | --- |
| `h`, `left` | Extend left | `extend_char_left` |
| `j`, `down` | Extend down | `extend_line_down` |
| `k`, `up` | Extend up | `extend_line_up` |
| `l`, `right` | Extend right | `extend_char_right` |
| `w` | Extend to start of next word | `extend_next_word_start` |
| `b` | Extend to start of previous word | `extend_prev_word_start` |
| `e` | Extend to end of next word | `extend_next_word_end` |
| `W` | Extend to start of next long word | `extend_next_long_word_start` |
| `B` | Extend to start of previous long word | `extend_prev_long_word_start` |
| `E` | Extend to end of next long word | `extend_next_long_word_end` |
| `n` | Add next search match to selection | `extend_search_next` |
| `N` | Add previous search match to selection | `extend_search_prev` |
| `t` | Extend till next occurrence of char | `extend_till_char` |
| `f` | Extend to next occurrence of char | `extend_next_char` |
| `T` | Extend till previous occurrence of char | `extend_till_prev_char` |
| `F` | Extend to previous occurrence of char | `extend_prev_char` |
| `home` | Extend to line start | `extend_to_line_start` |
| `end` | Extend to line end | `extend_to_line_end` |
| `esc` | Exit selection mode | `exit_select_mode` |
| `v` | Enter normal mode | `normal_mode` |
## Unmapped Commands
Some commands exist but do not have a default keybinding. These commands
may be quite niche, or simply alternatives to the standard commands. If you
want to use them, simply map them in your configuration (see [remapping]).

[remapping]: ./remapping.md

| Command | Description |
| --- | --- |
| `no_op` | Do nothing |
| `move_prev_word_end` | Move to end of previous word |
| `extend_line` | Select current line, if already selected, extend to another line based on the anchor |
| `extend_line_above` | Select current line, if already selected, extend to previous line |
| `goto_file_end` | Goto file end |
| `goto_line_end_newline` | Goto line end |
| `extend_to_line_end_newline` | Extend to line end |
| `signature_help` | Show signature help |
| `yank_joined_to_primary_clipboard` | Join and yank selections to primary clipboard |
| `yank_main_selection_to_primary_clipboard` | Yank main selection to primary clipboard |
| `replace_selections_with_primary_clipboard` | Replace selections by primary clipboard |
| `paste_primary_clipboard_after` | Paste primary clipboard after selections |
| `paste_primary_clipboard_before` | Paste primary clipboard before selections |

ANCHOR_END: all
ANCHOR: toc
- [Normal](#normal)
  - [Goto](#goto)
  - [Match](#match)
  - [Left bracket](#left-bracket)
  - [Right bracket](#right-bracket)
  - [Window](#window)
    - [New split scratch buffer](#new-split-scratch-buffer)
  - [Space](#space)
    - [Debug (experimental)](#debug-experimental)
      - [Switch](#switch)
  - [View](#view)
- [Insert](#insert)
- [Select](#select)
- [Unmapped Commands](#unmapped-commands)

ANCHOR_END: toc
