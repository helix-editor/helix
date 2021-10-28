# Keymap

## Normal mode

### Movement

> NOTE: Unlike vim, `f`, `F`, `t` and `T` are not confined to the current line.

| Key         | Description                                        | Command                     |
| -----       | -----------                                        | -------                     |
| `h`/`Left`  | Move left                                          | `move_char_left`            |
| `j`/`Down`  | Move down                                          | `move_line_down`            |
| `k`/`Up`    | Move up                                            | `move_line_up`              |
| `l`/`Right` | Move right                                         | `move_char_right`           |
| `w`         | Move next word start                               | `move_next_word_start`      |
| `b`         | Move previous word start                           | `move_prev_word_start`      |
| `e`         | Move next word end                                 | `move_next_word_end`        |
| `W`         | Move next WORD start                               | `move_next_long_word_start` |
| `B`         | Move previous WORD start                           | `move_prev_long_word_start` |
| `E`         | Move next WORD end                                 | `move_next_long_word_end`   |
| `t`         | Find 'till next char                               | `find_till_char`            |
| `f`         | Find next char                                     | `find_next_char`            |
| `T`         | Find 'till previous char                           | `till_prev_char`            |
| `F`         | Find previous char                                 | `find_prev_char`            |
| `Alt-.`     | Repeat last motion (`f`, `t` or `m`)               | `repeat_last_motion`        |
| `Home`      | Move to the start of the line                      | `goto_line_start`           |
| `End`       | Move to the end of the line                        | `goto_line_end`             |
| `PageUp`    | Move page up                                       | `page_up`                   |
| `PageDown`  | Move page down                                     | `page_down`                 |
| `Ctrl-u`    | Move half page up                                  | `half_page_up`              |
| `Ctrl-d`    | Move half page down                                | `half_page_down`            |
| `Ctrl-i`    | Jump forward on the jumplist                       | `jump_forward`              |
| `Ctrl-o`    | Jump backward on the jumplist                      | `jump_backward`             |
| `v`         | Enter [select (extend) mode](#select--extend-mode) | `select_mode`               |
| `g`         | Enter [goto mode](#goto-mode)                      | N/A                         |
| `m`         | Enter [match mode](#match-mode)                    | N/A                         |
| `:`         | Enter command mode                                 | `command_mode`              |
| `z`         | Enter [view mode](#view-mode)                      | N/A                         |
| `Z`         | Enter sticky [view mode](#view-mode)               | N/A                         |
| `Ctrl-w`    | Enter [window mode](#window-mode)                  | N/A                         |
| `Space`     | Enter [space mode](#space-mode)                    | N/A                         |

### Changes

| Key         | Description                                     | Command               |
| -----       | -----------                                     | -------               |
| `r`         | Replace with a character                        | `replace`             |
| `R`         | Replace with yanked text                        | `replace_with_yanked` |
| `~`         | Switch case of the selected text                | `switch_case`         |
| `` ` ``     | Set the selected text to lower case             | `switch_to_lowercase` |
| `` Alt-` `` | Set the selected text to upper case             | `switch_to_uppercase` |
| `i`         | Insert before selection                         | `insert_mode`         |
| `a`         | Insert after selection (append)                 | `append_mode`         |
| `I`         | Insert at the start of the line                 | `prepend_to_line`     |
| `A`         | Insert at the end of the line                   | `append_to_line`      |
| `o`         | Open new line below selection                   | `open_below`          |
| `O`         | Open new line above selection                   | `open_above`          |
| `.`         | Repeat last change                              | N/A                   |
| `u`         | Undo change                                     | `undo`                |
| `U`         | Redo change                                     | `redo`                |
| `y`         | Yank selection                                  | `yank`                |
| `p`         | Paste after selection                           | `paste_after`         |
| `P`         | Paste before selection                          | `paste_before`        |
| `"` `<reg>` | Select a register to yank to or paste from      | `select_register`     |
| `>`         | Indent selection                                | `indent`              |
| `<`         | Unindent selection                              | `unindent`            |
| `=`         | Format selection                                | `format_selections`   |
| `d`         | Delete selection                                | `delete_selection`    |
| `c`         | Change selection (delete and enter insert mode) | `change_selection`    |

#### Shell

| Key                   | Description                                                                      | Command               |
| ------                | -----------                                                                      | -------               |
| <code>&#124;</code>   | Pipe each selection through shell command, replacing with output                 | `shell_pipe`          |
| <code>A-&#124;</code> | Pipe each selection into shell command, ignoring output                          | `shell_pipe_to`       |
| `!`                   | Run shell command, inserting output before each selection                        | `shell_insert_output` |
| `A-!`                 | Run shell command, appending output after each selection                         | `shell_append_output` |


### Selection manipulation

| Key      | Description                                                       | Command                              |
| -----    | -----------                                                       | -------                              |
| `s`      | Select all regex matches inside selections                        | `select_regex`                       |
| `S`      | Split selection into subselections on regex matches               | `split_selection`                    |
| `Alt-s`  | Split selection on newlines                                       | `split_selection_on_newline`         |
| `;`      | Collapse selection onto a single cursor                           | `collapse_selection`                 |
| `Alt-;`  | Flip selection cursor and anchor                                  | `flip_selections`                    |
| `,`      | Keep only the primary selection                                   | `keep_primary_selection`             |
| `Alt-,`  | Remove the primary selection                                      | `remove_primary_selection`           |
| `C`      | Copy selection onto the next line                                 | `copy_selection_on_next_line`        |
| `Alt-C`  | Copy selection onto the previous line                             | `copy_selection_on_prev_line`        |
| `(`      | Rotate main selection backward                                    | `rotate_selections_backward`         |
| `)`      | Rotate main selection forward                                     | `rotate_selections_forward`          |
| `Alt-(`  | Rotate selection contents backward                                | `rotate_selection_contents_backward` |
| `Alt-)`  | Rotate selection contents forward                                 | `rotate_selection_contents_forward`  |
| `%`      | Select entire file                                                | `select_all`                         |
| `x`      | Select current line, if already selected, extend to next line     | `extend_line`                        |
| `X`      | Extend selection to line bounds (line-wise selection)             | `extend_to_line_bounds`              |
|          | Expand selection to parent syntax node TODO: pick a key           | `expand_selection`                   |
| `J`      | Join lines inside selection                                       | `join_selections`                    |
| `K`      | Keep selections matching the regex                                | `keep_selections`                    |
| `$`      | Pipe each selection into shell command, keep selections where command returned 0 | `shell_keep_pipe`     |
| `Ctrl-c` | Comment/uncomment the selections                                  | `toggle_comments`                    |

### Search

> TODO: The search implementation isn't ideal yet -- we don't support searching in reverse.

| Key   | Description                                 | Command              |
| ----- | -----------                                 | -------              |
| `/`   | Search for regex pattern                    | `search`             |
| `n`   | Select next search match                    | `search_next`        |
| `N`   | Add next search match to selection          | `extend_search_next` |
| `*`   | Use current selection as the search pattern | `search_selection`   |

### Minor modes

These sub-modes are accessible from normal mode and typically switch back to normal mode after a command.

#### View mode

View mode is intended for scrolling and manipulating the view without changing
the selection. The "sticky" variant of this mode is persistent; use the Escape
key to return to normal mode after usage (useful when you're simply looking
over text and not actively editing it).


| Key       | Description                                               | Command             |
| -----     | -----------                                               | -------             |
| `z` , `c` | Vertically center the line                                | `align_view_center` |
| `t`       | Align the line to the top of the screen                   | `align_view_top`    |
| `b`       | Align the line to the bottom of the screen                | `align_view_bottom` |
| `m`       | Align the line to the middle of the screen (horizontally) | `align_view_middle` |
| `j`       | Scroll the view downwards                                 | `scroll_down`       |
| `k`       | Scroll the view upwards                                   | `scroll_up`         |
| `f`       | Move page down                                            | `page_down`         |
| `b`       | Move page up                                              | `page_up`           |
| `d`       | Move half page down                                       | `half_page_down`    |
| `u`       | Move half page up                                         | `half_page_up`      |

#### Goto mode

Jumps to various locations.

> NOTE: Some of these features are only available with the LSP present.

| Key   | Description                                      | Command                    |
| ----- | -----------                                      | -------                    |
| `g`   | Go to the start of the file                      | `goto_file_start`          |
| `e`   | Go to the end of the file                        | `goto_last_line`           |
| `h`   | Go to the start of the line                      | `goto_line_start`          |
| `l`   | Go to the end of the line                        | `goto_line_end`            |
| `s`   | Go to first non-whitespace character of the line | `goto_first_nonwhitespace` |
| `t`   | Go to the top of the screen                      | `goto_window_top`          |
| `m`   | Go to the middle of the screen                   | `goto_window_middle`       |
| `b`   | Go to the bottom of the screen                   | `goto_window_bottom`       |
| `d`   | Go to definition                                 | `goto_definition`          |
| `y`   | Go to type definition                            | `goto_type_definition`     |
| `r`   | Go to references                                 | `goto_reference`           |
| `i`   | Go to implementation                             | `goto_implementation`      |
| `a`   | Go to the last accessed/alternate file           | `goto_last_accessed_file`  |

#### Match mode

Enter this mode using `m` from normal mode. See the relavant section
in [Usage](./usage.md) for an explanation about [surround](./usage.md#surround)
and [textobject](./usage.md#textobject) usage.

| Key              | Description                                     | Command                    |
| -----            | -----------                                     | -------                    |
| `m`              | Goto matching bracket                           | `match_brackets`           |
| `s` `<char>`     | Surround current selection with `<char>`        | `surround_add`             |
| `r` `<from><to>` | Replace surround character `<from>` with `<to>` | `surround_replace`         |
| `d` `<char>`     | Delete surround character `<char>`              | `surround_delete`          |
| `a` `<object>`   | Select around textobject                        | `select_textobject_around` |
| `i` `<object>`   | Select inside textobject                        | `select_textobject_inner`  |

TODO: Mappings for selecting syntax nodes (a superset of `[`).

#### Window mode

This layer is similar to vim keybindings as kakoune does not support window.

| Key           | Description             | Command           |
| -----         | -------------           | -------           |
| `w`, `Ctrl-w` | Switch to next window   | `rotate_view`     |
| `v`, `Ctrl-v` | Vertical right split    | `vsplit`          |
| `s`, `Ctrl-s` | Horizontal bottom split | `hsplit`          |
| `h`, `Ctrl-h` | Move to left split      | `jump_view_left`  |
| `j`, `Ctrl-j` | Move to split below     | `jump_view_down`  |
| `k`, `Ctrl-k` | Move to split above     | `jump_view_up`    |
| `l`, `Ctrl-l` | Move to right split     | `jump_view_right` |
| `q`, `Ctrl-q` | Close current window    | `wclose`          |

#### Space mode

This layer is a kludge of mappings, mostly pickers.

| Key     | Description                                                           | Command                             |
| -----   | -----------                                                           | -------                             |
| `k`     | Show documentation for the item under the cursor                      | `hover`                             |
| `f`     | Open file picker                                                      | `file_picker`                       |
| `b`     | Open buffer picker                                                    | `buffer_picker`                     |
| `s`     | Open symbol picker (current document)                                 | `symbol_picker`                     |
| `a`     | Apply code action                                                     | `code_action`                       |
| `'`     | Open last fuzzy picker                                                | `last_picker`                       |
| `w`     | Enter [window mode](#window-mode)                                     | N/A                                 |
| `p`     | Paste system clipboard after selections                               | `paste_clipboard_after`             |
| `P`     | Paste system clipboard before selections                              | `paste_clipboard_before`            |
| `y`     | Join and yank selections to clipboard                                 | `yank_joined_to_clipboard`          |
| `Y`     | Yank main selection to clipboard                                      | `yank_main_selection_to_clipboard`  |
| `R`     | Replace selections by clipboard contents                              | `replace_selections_with_clipboard` |
| `/`     | Global search in workspace folder                                     | `global_search`                     |

> NOTE: Global search display results in a fuzzy picker, use `space + '` to bring it back up after opening a file.
 
#### Unimpaired

Mappings in the style of [vim-unimpaired](https://github.com/tpope/vim-unimpaired).

| Key       | Description                        | Command           |
| -----     | -----------                        | -------           |
| `[d`      | Go to previous diagnostic          | `goto_prev_diag`  |
| `]d`      | Go to next diagnostic              | `goto_next_diag`  |
| `[D`      | Go to first diagnostic in document | `goto_first_diag` |
| `]D`      | Go to last diagnostic in document  | `goto_last_diag`  |
| `[space`  | Add newline above                  | `add_newline_above` |
| `]space`  | Add newline below                  | `add_newline_below` |

## Insert Mode

| Key      | Description           | Command                |
| -----    | -----------           | -------                |
| `Escape` | Switch to normal mode | `normal_mode`          |
| `Ctrl-x` | Autocomplete          | `completion`           |
| `Ctrl-w` | Delete previous word  | `delete_word_backward` |

## Select / extend mode

I'm still pondering whether to keep this mode or not. It changes movement
commands (including goto) to extend the existing selection instead of replacing it.

> NOTE: It's a bit confusing at the moment because extend hasn't been
> implemented for all movement commands yet.

# Picker

Keys to use within picker. Remapping currently not supported.

| Key                          | Description       |
| -----                        | -------------     |
| `Up`, `Ctrl-k`, `Ctrl-p`     | Previous entry    |
| `Down`, `Ctrl-j`, `Ctrl-n`   | Next entry        |
| `Ctrl-space`                 | Filter options    |
| `Enter`                      | Open selected     |
| `Ctrl-s`                     | Open horizontally |
| `Ctrl-v`                     | Open vertically   |
| `Escape`, `Ctrl-c`           | Close picker      |
