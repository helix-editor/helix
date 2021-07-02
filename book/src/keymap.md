# Keymap

## Normal mode

### Movement

> NOTE: `f`, `F`, `t` and `T` are not confined to the current line.

| Key          | Description                                                                |
| -----        | -----------                                                                |
| `h`, `Left`  | Move left                                                                  |
| `j`, `Down`  | Move down                                                                  |
| `k`, `Up`    | Move up                                                                    |
| `l`, `Right` | Move right                                                                 |
| `w`          | Move next word start                                                       |
| `b`          | Move previous word start                                                   |
| `e`          | Move next word end                                                         |
| `W`          | Move next WORD start                                                       |
| `B`          | Move previous WORD start                                                   |
| `E`          | Move next WORD end                                                         |
| `t`          | Find 'till next char                                                       |
| `f`          | Find next char                                                             |
| `T`          | Find 'till previous char                                                   |
| `F`          | Find previous char                                                         |
| `Home`       | Move to the start of the line                                              |
| `End`        | Move to the end of the line                                                |
| `PageUp`     | Move page up                                                               |
| `PageDown`   | Move page down                                                             |
| `Ctrl-u`     | Move half page up                                                          |
| `Ctrl-d`     | Move half page down                                                        |
| `Ctrl-i`     | Jump forward on the jumplist TODO: conflicts tab                           |
| `Ctrl-o`     | Jump backward on the jumplist                                              |
| `v`          | Enter [select (extend) mode](#select--extend-mode)                         |
| `g`          | Enter [goto mode](#goto-mode)                                              |
| `m`          | Enter [match mode](#match-mode)                                            |
| `:`          | Enter command mode                                                         |
| `z`          | Enter [view mode](#view-mode)                                              |
| `Ctrl-w`     | Enter [window mode](#window-mode) (maybe will be remove for spc w w later) |
| `Space`      | Enter [space mode](#space-mode)                                            |
| `K`          | Show documentation for the item under the cursor                           |

### Changes

| Key   | Description                                     |
| ----- | -----------                                     |
| `r`   | Replace with a character                        |
| `R`   | Replace with yanked text                        |
| `i`   | Insert before selection                         |
| `a`   | Insert after selection (append)                 |
| `I`   | Insert at the start of the line                 |
| `A`   | Insert at the end of the line                   |
| `o`   | Open new line below selection                   |
| `o`   | Open new line above selection                   |
| `u`   | Undo change                                     |
| `U`   | Redo change                                     |
| `y`   | Yank selection                                  |
| `p`   | Paste after selection                           |
| `P`   | Paste before selection                          |
| `>`   | Indent selection                                |
| `<`   | Unindent selection                              |
| `=`   | Format selection                                |
| `d`   | Delete selection                                |
| `c`   | Change selection (delete and enter insert mode) |

### Selection manipulation

| Key      | Description                                                       |
| -----    | -----------                                                       |
| `s`      | Select all regex matches inside selections                        |
| `S`      | Split selection into subselections on regex matches               |
| `Alt-s`  | Split selection on newlines                                       |
| `;`      | Collapse selection onto a single cursor                           |
| `Alt-;`  | Flip selection cursor and anchor                                  |
| `%`      | Select entire file                                                |
| `x`      | Select current line, if already selected, extend to next line     |
|          | Expand selection to parent syntax node TODO: pick a key           |
| `J`      | join lines inside selection                                       |
| `K`      | keep selections matching the regex TODO: overlapped by hover help |
| `Space`  | keep only the primary selection TODO: overlapped by space mode    |
| `Ctrl-c` | Comment/uncomment the selections                                  |

### Search

> TODO: The search implementation isn't ideal yet -- we don't support searching
in reverse, or searching via smartcase.

| Key   | Description                                 |
| ----- | -----------                                 |
| `/`   | Search for regex pattern                    |
| `n`   | Select next search match                    |
| `N`   | Add next search match to selection          |
| `*`   | Use current selection as the search pattern |

### Diagnostics

> NOTE: `[` and `]` will likely contain more pair mappings in the style of
> [vim-unimpaired](https://github.com/tpope/vim-unimpaired)

| Key   | Description                        |
| ----- | -----------                        |
| `[d`  | Go to previous diagnostic          |
| `]d`  | Go to next diagnostic              |
| `[D`  | Go to first diagnostic in document |
| `]D`  | Go to last diagnostic in document  |

## Select / extend mode

I'm still pondering whether to keep this mode or not. It changes movement
commands to extend the existing selection instead of replacing it.

> NOTE: It's a bit confusing at the moment because extend hasn't been
> implemented for all movement commands yet.

## View mode

View mode is intended for scrolling and manipulating the view without changing
the selection.

| Key       | Description                                               |
| -----     | -----------                                               |
| `z` , `c` | Vertically center the line                                |
| `t`       | Align the line to the top of the screen                   |
| `b`       | Align the line to the bottom of the screen                |
| `m`       | Align the line to the middle of the screen (horizontally) |
| `j`       | Scroll the view downwards                                 |
| `k`       | Scroll the view upwards                                   |

## Goto mode

Jumps to various locations.

> NOTE: Some of these features are only available with the LSP present.

| Key   | Description                                      |
| ----- | -----------                                      |
| `g`   | Go to the start of the file                      |
| `e`   | Go to the end of the file                        |
| `h`   | Go to the start of the line                      |
| `l`   | Go to the end of the line                        |
| `s`   | Go to first non-whitespace character of the line |
| `t`   | Go to the top of the screen                      |
| `m`   | Go to the middle of the screen                   |
| `b`   | Go to the bottom of the screen                   |
| `d`   | Go to definition                                 |
| `y`   | Go to type definition                            |
| `r`   | Go to references                                 |
| `i`   | Go to implementation                             |
| `a`   | Go to the last accessed/alternate file           |

## Match mode

Enter this mode using `m` from normal mode. See the relavant section
in [Usage](./usage.md) for an explanation about [surround](./usage.md#surround)
and [textobject](./usage.md#textobject) usage.

| Key              | Description                                     |
| -----            | -----------                                     |
| `m`              | Goto matching bracket                           |
| `s` `<char>`     | Surround current selection with `<char>`        |
| `r` `<from><to>` | Replace surround character `<from>` with `<to>` |
| `d` `<char>`     | Delete surround character `<char>`              |
| `a` `<object>`   | Select around textobject                        |
| `i` `<object>`   | Select inside textobject                        |

## Object mode

TODO: Mappings for selecting syntax nodes (a superset of `[`).

## Window mode

This layer is similar to vim keybindings as kakoune does not support window.

| Key           | Description             |
| -----         | -------------           |
| `w`, `Ctrl-w` | Switch to next window   |
| `v`, `Ctrl-v` | Vertical right split    |
| `h`, `Ctrl-h` | Horizontal bottom split |
| `q`, `Ctrl-q` | Close current window    |

## Space mode

This layer is a kludge of mappings I had under leader key in neovim.

| Key     | Description                                                           |
| -----   | -----------                                                           |
| `f`     | Open file picker                                                      |
| `b`     | Open buffer picker                                                    |
| `s`     | Open symbol picker (current document)                                 |
| `w`     | Enter [window mode](#window-mode)                                     |
| `space` | Keep primary selection TODO: it's here because space mode replaced it |
| `p`     | paste system clipboard after selections                               |
| `P`     | paste system clipboard before selections                              |
| `y`     | join and yank selections to clipboard                                 |
| `Y`     | yank main selection to clipboard                                      |
| `R`     | replace selections by clipboard contents                              |

# Picker

Keys to use within picker.

| Key                | Description       |
| -----              | -------------     |
| `Up`, `Ctrl-p`     | Previous entry    |
| `Down`, `Ctrl-n`   | Next entry        |
| `Ctrl-space`       | Filter options    |
| `Enter`            | Open selected     |
| `Ctrl-h`           | Open horizontally |
| `Ctrl-v`           | Open vertically   |
| `Escape`, `Ctrl-c` | Close picker      |
