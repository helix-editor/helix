# Usage

(Currently not fully documented, see the [keymappings](./keymap.md) list for more.)

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
