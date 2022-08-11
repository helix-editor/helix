# Keymap

- [Normal mode](#normal-mode)
  - [Movement](#movement)
  - [Changes](#changes)
    - [Shell](#shell)
  - [Selection manipulation](#selection-manipulation)
  - [Search](#search)
  - [Minor modes](#minor-modes)
    - [View mode](#view-mode)
    - [Goto mode](#goto-mode)
    - [Match mode](#match-mode)
    - [Window mode](#window-mode)
    - [Space mode](#space-mode)
      - [Popup](#popup)
    - [Unimpaired](#unimpaired)
- [Insert Mode](#insert-mode)
- [Select / extend mode](#select--extend-mode)
- [Picker](#picker)
- [Prompt](#prompt)

> 💡 Mappings marked (**LSP**) require an active language server for the file.

> 💡 Mappings marked (**TS**) require a tree-sitter grammar for the filetype.

## Normal mode

### Movement

> NOTE: Unlike vim, `f`, `F`, `t` and `T` are not confined to the current line.

| Key                   | Description                                        | Command                     |
| -----                 | -----------                                        | -------                     |
| `h`, `Left`           | Move left                                          | `move_char_left`            |
| `j`, `Down`           | Move down                                          | `move_line_down`            |
| `k`, `Up`             | Move up                                            | `move_line_up`              |
| `l`, `Right`          | Move right                                         | `move_char_right`           |
| `w`                   | Move next word start                               | `move_next_word_start`      |
| `b`                   | Move previous word start                           | `move_prev_word_start`      |
| `e`                   | Move next word end                                 | `move_next_word_end`        |
| `W`                   | Move next WORD start                               | `move_next_long_word_start` |
| `B`                   | Move previous WORD start                           | `move_prev_long_word_start` |
| `E`                   | Move next WORD end                                 | `move_next_long_word_end`   |
| `t`                   | Find 'till next char                               | `find_till_char`            |
| `f`                   | Find next char                                     | `find_next_char`            |
| `T`                   | Find 'till previous char                           | `till_prev_char`            |
| `F`                   | Find previous char                                 | `find_prev_char`            |
| `G`                   | Go to line number `<n>`                            | `goto_line`                 |
| `Alt-.`               | Repeat last motion (`f`, `t` or `m`)               | `repeat_last_motion`        |
| `Home`                | Move to the start of the line                      | `goto_line_start`           |
| `End`                 | Move to the end of the line                        | `goto_line_end`             |
| `Ctrl-b`, `PageUp`    | Move page up                                       | `page_up`                   |
| `Ctrl-f`, `PageDown`  | Move page down                                     | `page_down`                 |
| `Ctrl-u`              | Move half page up                                  | `half_page_up`              |
| `Ctrl-d`              | Move half page down                                | `half_page_down`            |
| `Ctrl-i`              | Jump forward on the jumplist                       | `jump_forward`              |
| `Ctrl-o`              | Jump backward on the jumplist                      | `jump_backward`             |
| `Ctrl-s`              | Save the current selection to the jumplist         | `save_selection`            |

### Changes

| Key         | Description                                                          | Command                   |
| -----       | -----------                                                          | -------                   |
| `r`         | Replace with a character                                             | `replace`                 |
| `R`         | Replace with yanked text                                             | `replace_with_yanked`     |
| `~`         | Switch case of the selected text                                     | `switch_case`             |
| `` ` ``     | Set the selected text to lower case                                  | `switch_to_lowercase`     |
| `` Alt-` `` | Set the selected text to upper case                                  | `switch_to_uppercase`     |
| `i`         | Insert before selection                                              | `insert_mode`             |
| `a`         | Insert after selection (append)                                      | `append_mode`             |
| `I`         | Insert at the start of the line                                      | `prepend_to_line`         |
| `A`         | Insert at the end of the line                                        | `append_to_line`          |
| `o`         | Open new line below selection                                        | `open_below`              |
| `O`         | Open new line above selection                                        | `open_above`              |
| `.`         | Repeat last insert                                                   | N/A                       |
| `u`         | Undo change                                                          | `undo`                    |
| `U`         | Redo change                                                          | `redo`                    |
| `Alt-u`     | Move backward in history                                             | `earlier`                 |
| `Alt-U`     | Move forward in history                                              | `later`                   |
| `y`         | Yank selection                                                       | `yank`                    |
| `p`         | Paste after selection                                                | `paste_after`             |
| `P`         | Paste before selection                                               | `paste_before`            |
| `"` `<reg>` | Select a register to yank to or paste from                           | `select_register`         |
| `>`         | Indent selection                                                     | `indent`                  |
| `<`         | Unindent selection                                                   | `unindent`                |
| `=`         | Format selection (currently nonfunctional/disabled) (**LSP**)        | `format_selections`       |
| `d`         | Delete selection                                                     | `delete_selection`        |
| `Alt-d`     | Delete selection, without yanking                                    | `delete_selection_noyank` |
| `c`         | Change selection (delete and enter insert mode)                      | `change_selection`        |
| `Alt-c`     | Change selection (delete and enter insert mode, without yanking)     | `change_selection_noyank` |
| `Ctrl-a`    | Increment object (number) under cursor                               | `increment`               |
| `Ctrl-x`    | Decrement object (number) under cursor                               | `decrement`               |
| `Q`         | Start/stop macro recording to the selected register (experimental)   | `record_macro`            |
| `q`         | Play back a recorded macro from the selected register (experimental) | `replay_macro`            |

#### Shell

| Key     | Description                                                                      | Command               |
| ------  | -----------                                                                      | -------               |
| <code>&#124;</code>     | Pipe each selection through shell command, replacing with output                 | `shell_pipe`          |
| <code>Alt-&#124;</code> | Pipe each selection into shell command, ignoring output                          | `shell_pipe_to`       |
| `!`     | Run shell command, inserting output before each selection                        | `shell_insert_output` |
| `Alt-!` | Run shell command, appending output after each selection                         | `shell_append_output` |
| `$`     | Pipe each selection into shell command, keep selections where command returned 0 | `shell_keep_pipe`     |


### Selection manipulation

| Key                   | Description                                                       | Command                              |
| -----                 | -----------                                                       | -------                              |
| `s`                   | Select all regex matches inside selections                        | `select_regex`                       |
| `S`                   | Split selection into subselections on regex matches               | `split_selection`                    |
| `Alt-s`               | Split selection on newlines                                       | `split_selection_on_newline`         |
| `&`                   | Align selection in columns                                        | `align_selections`                   |
| `_`                   | Trim whitespace from the selection                                | `trim_selections`                    |
| `;`                   | Collapse selection onto a single cursor                           | `collapse_selection`                 |
| `Alt-;`               | Flip selection cursor and anchor                                  | `flip_selections`                    |
| `Alt-:`               | Ensures the selection is in forward direction                     | `ensure_selections_forward`          |
| `,`                   | Keep only the primary selection                                   | `keep_primary_selection`             |
| `Alt-,`               | Remove the primary selection                                      | `remove_primary_selection`           |
| `C`                   | Copy selection onto the next line (Add cursor below)              | `copy_selection_on_next_line`        |
| `Alt-C`               | Copy selection onto the previous line (Add cursor above)          | `copy_selection_on_prev_line`        |
| `(`                   | Rotate main selection backward                                    | `rotate_selections_backward`         |
| `)`                   | Rotate main selection forward                                     | `rotate_selections_forward`          |
| `Alt-(`               | Rotate selection contents backward                                | `rotate_selection_contents_backward` |
| `Alt-)`               | Rotate selection contents forward                                 | `rotate_selection_contents_forward`  |
| `%`                   | Select entire file                                                | `select_all`                         |
| `x`                   | Select current line, if already selected, extend to next line     | `extend_line`                        |
| `X`                   | Extend selection to line bounds (line-wise selection)             | `extend_to_line_bounds`              |
| `Alt-x`               | Shrink selection to line bounds (line-wise selection)             | `shrink_to_line_bounds`              |
| `J`                   | Join lines inside selection                                       | `join_selections`                    |
| `K`                   | Keep selections matching the regex                                | `keep_selections`                    |
| `Alt-K`               | Remove selections matching the regex                              | `remove_selections`                  |
| `Ctrl-c`              | Comment/uncomment the selections                                  | `toggle_comments`                    |
| `Alt-o`, `Alt-up`     | Expand selection to parent syntax node (**TS**)                   | `expand_selection`                   |
| `Alt-i`, `Alt-down`   | Shrink syntax tree object selection (**TS**)                      | `shrink_selection`                   |
| `Alt-p`, `Alt-left`   | Select previous sibling node in syntax tree (**TS**)              | `select_prev_sibling`                |
| `Alt-n`, `Alt-right`  | Select next sibling node in syntax tree (**TS**)                  | `select_next_sibling`                |

### Search

Search commands all operate on the `/` register by default. Use `"<char>` to operate on a different one.

| Key   | Description                                 | Command              |
| ----- | -----------                                 | -------              |
| `/`   | Search for regex pattern                    | `search`             |
| `?`   | Search for previous pattern                 | `rsearch`            |
| `n`   | Select next search match                    | `search_next`        |
| `N`   | Select previous search match                | `search_prev`        |
| `*`   | Use current selection as the search pattern | `search_selection`   |

### Minor modes

These sub-modes are accessible from normal mode and typically switch back to normal mode after a command.

| Key      | Description                                        | Command        |
| -----    | -----------                                        | -------        |
| `v`      | Enter [select (extend) mode](#select--extend-mode) | `select_mode`  |
| `g`      | Enter [goto mode](#goto-mode)                      | N/A            |
| `m`      | Enter [match mode](#match-mode)                    | N/A            |
| `:`      | Enter command mode                                 | `command_mode` |
| `z`      | Enter [view mode](#view-mode)                      | N/A            |
| `Z`      | Enter sticky [view mode](#view-mode)               | N/A            |
| `Ctrl-w` | Enter [window mode](#window-mode)                  | N/A            |
| `Space`  | Enter [space mode](#space-mode)                    | N/A            |

#### View mode

View mode is intended for scrolling and manipulating the view without changing
the selection. The "sticky" variant of this mode is persistent; use the Escape
key to return to normal mode after usage (useful when you're simply looking
over text and not actively editing it).


| Key                  | Description                                               | Command             |
| -----                | -----------                                               | -------             |
| `z`, `c`             | Vertically center the line                                | `align_view_center` |
| `t`                  | Align the line to the top of the screen                   | `align_view_top`    |
| `b`                  | Align the line to the bottom of the screen                | `align_view_bottom` |
| `m`                  | Align the line to the middle of the screen (horizontally) | `align_view_middle` |
| `j`, `down`          | Scroll the view downwards                                 | `scroll_down`       |
| `k`, `up`            | Scroll the view upwards                                   | `scroll_up`         |
| `Ctrl-f`, `PageDown` | Move page down                                            | `page_down`         |
| `Ctrl-b`, `PageUp`   | Move page up                                              | `page_up`           |
| `Ctrl-d`             | Move half page down                                       | `half_page_down`    |
| `Ctrl-u`             | Move half page up                                         | `half_page_up`      |

#### Goto mode

Jumps to various locations.

| Key   | Description                                      | Command                    |
| ----- | -----------                                      | -------                    |
| `g`   | Go to line number `<n>` else start of file       | `goto_file_start`          |
| `e`   | Go to the end of the file                        | `goto_last_line`           |
| `f`   | Go to files in the selection                     | `goto_file`                |
| `h`   | Go to the start of the line                      | `goto_line_start`          |
| `l`   | Go to the end of the line                        | `goto_line_end`            |
| `s`   | Go to first non-whitespace character of the line | `goto_first_nonwhitespace` |
| `t`   | Go to the top of the screen                      | `goto_window_top`          |
| `c`   | Go to the middle of the screen                   | `goto_window_center`       |
| `b`   | Go to the bottom of the screen                   | `goto_window_bottom`       |
| `d`   | Go to definition (**LSP**)                       | `goto_definition`          |
| `y`   | Go to type definition (**LSP**)                  | `goto_type_definition`     |
| `r`   | Go to references (**LSP**)                       | `goto_reference`           |
| `i`   | Go to implementation (**LSP**)                   | `goto_implementation`      |
| `a`   | Go to the last accessed/alternate file           | `goto_last_accessed_file`  |
| `m`   | Go to the last modified/alternate file           | `goto_last_modified_file`  |
| `n`   | Go to next buffer                                | `goto_next_buffer`         |
| `p`   | Go to previous buffer                            | `goto_previous_buffer`     |
| `.`   | Go to last modification in current file          | `goto_last_modification`   |

#### Match mode

Enter this mode using `m` from normal mode. See the relevant section
in [Usage](./usage.md) for an explanation about [surround](./usage.md#surround)
and [textobject](./usage.md#textobject) usage.

| Key              | Description                                     | Command                    |
| -----            | -----------                                     | -------                    |
| `m`              | Goto matching bracket (**TS**)                  | `match_brackets`           |
| `s` `<char>`     | Surround current selection with `<char>`        | `surround_add`             |
| `r` `<from><to>` | Replace surround character `<from>` with `<to>` | `surround_replace`         |
| `d` `<char>`     | Delete surround character `<char>`              | `surround_delete`          |
| `a` `<object>`   | Select around textobject                        | `select_textobject_around` |
| `i` `<object>`   | Select inside textobject                        | `select_textobject_inner`  |

TODO: Mappings for selecting syntax nodes (a superset of `[`).

#### Window mode

This layer is similar to vim keybindings as kakoune does not support window.

| Key                    | Description                                          | Command           |
| -----                  | -------------                                        | -------           |
| `w`, `Ctrl-w`          | Switch to next window                                | `rotate_view`     |
| `v`, `Ctrl-v`          | Vertical right split                                 | `vsplit`          |
| `s`, `Ctrl-s`          | Horizontal bottom split                              | `hsplit`          |
| `f`                    | Go to files in the selection in horizontal splits    | `goto_file`       |
| `F`                    | Go to files in the selection in vertical splits      | `goto_file`       |
| `h`, `Ctrl-h`, `Left`  | Move to left split                                   | `jump_view_left`  |
| `j`, `Ctrl-j`, `Down`  | Move to split below                                  | `jump_view_down`  |
| `k`, `Ctrl-k`, `Up`    | Move to split above                                  | `jump_view_up`    |
| `l`, `Ctrl-l`, `Right` | Move to right split                                  | `jump_view_right` |
| `q`, `Ctrl-q`          | Close current window                                 | `wclose`          |
| `o`, `Ctrl-o`          | Only keep the current window, closing all the others | `wonly`           |
| `H`                    | Swap window to the left                              | `swap_view_left`  |
| `J`                    | Swap window downwards                                | `swap_view_down`  |
| `K`                    | Swap window upwards                                  | `swap_view_up`    |
| `L`                    | Swap window to the right                             | `swap_view_right` |

#### Space mode

This layer is a kludge of mappings, mostly pickers.


| Key     | Description                                                             | Command                             |
| -----   | -----------                                                             | -------                             |
| `f`     | Open file picker                                                        | `file_picker`                       |
| `b`     | Open buffer picker                                                      | `buffer_picker`                     |
| `j`     | Open jumplist picker                                                    | `jumplist_picker`                   |
| `k`     | Show documentation for item under cursor in a [popup](#popup) (**LSP**) | `hover`                             |
| `s`     | Open document symbol picker (**LSP**)                                   | `symbol_picker`                     |
| `S`     | Open workspace symbol picker (**LSP**)                                  | `workspace_symbol_picker`           |
| `g`     | Open document diagnostics picker (**LSP**)                              | `diagnostics_picker`                |
| `G`     | Open workspace diagnostics picker (**LSP**)                             | `workspace_diagnostics_picker`
| `r`     | Rename symbol (**LSP**)                                                 | `rename_symbol`                     |
| `a`     | Apply code action  (**LSP**)                                            | `code_action`                       |
| `'`     | Open last fuzzy picker                                                  | `last_picker`                       |
| `w`     | Enter [window mode](#window-mode)                                       | N/A                                 |
| `p`     | Paste system clipboard after selections                                 | `paste_clipboard_after`             |
| `P`     | Paste system clipboard before selections                                | `paste_clipboard_before`            |
| `y`     | Join and yank selections to clipboard                                   | `yank_joined_to_clipboard`          |
| `Y`     | Yank main selection to clipboard                                        | `yank_main_selection_to_clipboard`  |
| `R`     | Replace selections by clipboard contents                                | `replace_selections_with_clipboard` |
| `/`     | Global search in workspace folder                                       | `global_search`                     |
| `?`     | Open command palette                                                    | `command_palette`                   |

> TIP: Global search displays results in a fuzzy picker, use `space + '` to bring it back up after opening a file.

##### Popup

Displays documentation for item under cursor.

| Key      | Description |
| ----     | ----------- |
| `Ctrl-u` | Scroll up   |
| `Ctrl-d` | Scroll down |
 
#### Unimpaired

Mappings in the style of [vim-unimpaired](https://github.com/tpope/vim-unimpaired).

| Key      | Description                                  | Command               |
| -----    | -----------                                  | -------               |
| `[d`     | Go to previous diagnostic (**LSP**)          | `goto_prev_diag`      |
| `]d`     | Go to next diagnostic (**LSP**)              | `goto_next_diag`      |
| `[D`     | Go to first diagnostic in document (**LSP**) | `goto_first_diag`     |
| `]D`     | Go to last diagnostic in document (**LSP**)  | `goto_last_diag`      |
| `]f`     | Go to next function (**TS**)                 | `goto_next_function`  |
| `[f`     | Go to previous function (**TS**)             | `goto_prev_function`  |
| `]c`     | Go to next class (**TS**)                    | `goto_next_class`     |
| `[c`     | Go to previous class (**TS**)                | `goto_prev_class`     |
| `]a`     | Go to next argument/parameter (**TS**)       | `goto_next_parameter` |
| `[a`     | Go to previous argument/parameter (**TS**)   | `goto_prev_parameter` |
| `]o`     | Go to next comment (**TS**)                  | `goto_next_comment`   |
| `[o`     | Go to previous comment (**TS**)              | `goto_prev_comment`   |
| `]t`     | Go to next test (**TS**)                     | `goto_next_test`      |
| `]t`     | Go to previous test (**TS**)                 | `goto_prev_test`      |
| `]p`     | Go to next paragraph                         | `goto_next_paragraph` |
| `[p`     | Go to previous paragraph                     | `goto_prev_paragraph` |
| `[space` | Add newline above                            | `add_newline_above`   |
| `]space` | Add newline below                            | `add_newline_below`   |

## Insert Mode

We support many readline/emacs style bindings in insert mode for
convenience. These can be helpful for making simple modifications
without escaping to normal mode, but beware that you will not have an
undo-able "save point" until you return to normal mode.

| Key                                         | Description                 | Command                 |
| -----                                       | -----------                 | -------                 |
| `Escape`                                    | Switch to normal mode       | `normal_mode`           |
| `Ctrl-x`                                    | Autocomplete                | `completion`            |
| `Ctrl-r`                                    | Insert a register content   | `insert_register`       |
| `Ctrl-w`, `Alt-Backspace`, `Ctrl-Backspace` | Delete previous word        | `delete_word_backward`  |
| `Alt-d`, `Alt-Delete`, `Ctrl-Delete`        | Delete next word            | `delete_word_forward`   |
| `Alt-b`, `Ctrl-Left`                        | Backward a word             | `move_prev_word_end`    |
| `Ctrl-b`, `Left`                            | Backward a char             | `move_char_left`        |
| `Alt-f`, `Ctrl-Right`                       | Forward a word              | `move_next_word_start`  |
| `Ctrl-f`, `Right`                           | Forward a char              | `move_char_right`       |
| `Ctrl-e`, `End`                             | Move to line end            | `goto_line_end_newline` |
| `Ctrl-a`, `Home`                            | Move to line start          | `goto_line_start`       |
| `Ctrl-u`                                    | Delete to start of line     | `kill_to_line_start`    |
| `Ctrl-k`                                    | Delete to end of line       | `kill_to_line_end`      |
| `Ctrl-j`, `Enter`                           | Insert new line             | `insert_newline`        |
| `Backspace`, `Ctrl-h`                       | Delete previous char        | `delete_char_backward`  |
| `Delete`, `Ctrl-d`                          | Delete next char            | `delete_char_forward`   |
| `Ctrl-p`, `Up`                              | Move to previous line       | `move_line_up`          |
| `Ctrl-n`, `Down`                            | Move to next line           | `move_line_down`        |
| `PageUp`                                    | Move one page up            | `page_up`               |
| `PageDown`                                  | Move one page down          | `page_down`             |
| `Alt->`                                     | Go to end of buffer         | `goto_file_end`         |
| `Alt-<`                                     | Go to start of buffer       | `goto_file_start`       |

## Select / extend mode

This mode echoes Normal mode, but changes any movements to extend
selections rather than replace them. Goto motions are also changed to
extend, so that `vgl` for example extends the selection to the end of
the line.

Search is also affected. By default, `n` and `N` will remove the current
selection and select the next instance of the search term. Toggling this
mode before pressing `n` or `N` makes it possible to keep the current
selection. Toggling it on and off during your iterative searching allows
you to selectively add search terms to your selections.

## Picker

Keys to use within picker. Remapping currently not supported.

| Key                          | Description       |
| -----                        | -------------     |
| `Up`, `Ctrl-p`               | Previous entry    |
| `PageUp`, `Ctrl-u`           | Page up           |
| `Down`, `Ctrl-n`             | Next entry        |
| `PageDown`, `Ctrl-d`         | Page down         |
| `Home`                       | Go to first entry |
| `End`                        | Go to last entry  |
| `Ctrl-space`                 | Filter options    |
| `Enter`                      | Open selected     |
| `Ctrl-s`                     | Open horizontally |
| `Ctrl-v`                     | Open vertically   |
| `Ctrl-t`                     | Toggle preview    |
| `Escape`, `Ctrl-c`           | Close picker      |

## Prompt

Keys to use within prompt, Remapping currently not supported.

| Key                                         | Description                                                             |
| -----                                       | -------------                                                           |
| `Escape`, `Ctrl-c`                          | Close prompt                                                            |
| `Alt-b`, `Ctrl-Left`                        | Backward a word                                                         |
| `Ctrl-b`, `Left`                            | Backward a char                                                         |
| `Alt-f`, `Ctrl-Right`                       | Forward a word                                                          |
| `Ctrl-f`, `Right`                           | Forward a char                                                          |
| `Ctrl-e`, `End`                             | Move prompt end                                                         |
| `Ctrl-a`, `Home`                            | Move prompt start                                                       |
| `Ctrl-w`, `Alt-Backspace`, `Ctrl-Backspace` | Delete previous word                                                    |
| `Alt-d`, `Alt-Delete`, `Ctrl-Delete`        | Delete next word                                                        |
| `Ctrl-u`                                    | Delete to start of line                                                 |
| `Ctrl-k`                                    | Delete to end of line                                                   |
| `backspace`, `Ctrl-h`                       | Delete previous char                                                    |
| `delete`, `Ctrl-d`                          | Delete next char                                                        |
| `Ctrl-s`                                    | Insert a word under doc cursor, may be changed to Ctrl-r Ctrl-w later   |
| `Ctrl-p`, `Up`                              | Select previous history                                                 |
| `Ctrl-n`, `Down`                            | Select next history                                                     |
| `Ctrl-r`                                    | Insert the content of the register selected by following input char     |
| `Tab`                                       | Select next completion item                                             |
| `BackTab`                                   | Select previous completion item                                         |
| `Enter`                                     | Open selected                                                           |
