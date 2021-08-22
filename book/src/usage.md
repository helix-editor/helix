# Usage

(Currently not fully documented, see the [keymappings](./keymap.md) list for more.)

## Registers

Vim-like registers can be used to yank and store text to be pasted later. Usage is similar, with `"` being used to select a register:

- `"ay` - Yank the current selection to register `a`.
- `"op` - Paste the text in register `o` after the selection.

If there is a selected register before invoking a change or delete command, the selection will be stored in the register and the action will be carried out:

- `"hc` - Store the selection in register `h` and then change it (delete and enter insert mode).
- `"md` - Store the selection in register `m` and delete it.

### Special Registers

| Register character | Contains              |
| ---                | ---                   |
| `/`                | Last search           |
| `:`                | Last executed command |
| `"`                | Last yanked text      |

> There is no special register for copying to system clipboard, instead special commands and keybindings are provided. See the [keymap](keymap.md#space-mode) for the specifics.

## Surround

Functionality similar to [vim-surround](https://github.com/tpope/vim-surround) is built into
helix. The keymappings have been inspired from [vim-sandwich](https://github.com/machakann/vim-sandwich):

![surround demo](https://user-images.githubusercontent.com/23398472/122865801-97073180-d344-11eb-8142-8f43809982c6.gif)

- `ms` - Add surround characters
- `mr` - Replace surround characters
- `md` - Delete surround characters

`ms` acts on a selection, so select the text first and use `ms<char>`. `mr` and `md` work
on the closest pairs found and selections are not required; use counts to act in outer pairs.

It can also act on multiple seletions (yay!). For example, to change every occurance of `(use)` to `[use]`:

- `%` to select the whole file
- `s` to split the selections on a search term
- Input `use` and hit Enter
- `mr([` to replace the parens with square brackets

Multiple characters are currently not supported, but planned.

## Textobjects

Currently supported: `word`, `surround`.

![textobject-demo](https://user-images.githubusercontent.com/23398472/124231131-81a4bb00-db2d-11eb-9d10-8e577ca7b177.gif)

- `ma` - Select around the object (`va` in vim, `<alt-a>` in kakoune)
- `mi` - Select inside the object (`vi` in vim, `<alt-i>` in kakoune)

| Key after `mi` or `ma` | Textobject selected      |
| ---                    | ---                      |
| `w`                    | Word                     |
| `(`, `[`, `'`, etc     | Specified surround pairs |

Textobjects based on treesitter, like `function`, `class`, etc are planned.
