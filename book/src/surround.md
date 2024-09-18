## Surround

Helix includes built-in functionality similar to [vim-surround](https://github.com/tpope/vim-surround).
The keymappings have been inspired from [vim-sandwich](https://github.com/machakann/vim-sandwich):

![Surround demo](https://user-images.githubusercontent.com/23398472/122865801-97073180-d344-11eb-8142-8f43809982c6.gif)

| Key Sequence                      | Action                                  |
| --------------------------------- | --------------------------------------- |
| `ms<char>` (after selecting text) | Add surround characters to selection    |
| `mr<char_to_replace><new_char>`   | Replace the closest surround characters |
| `md<char_to_delete>`              | Delete the closest surround characters  |

You can use counts to act on outer pairs.

Surround can also act on multiple selections. For example, to change every occurrence of `(use)` to `[use]`:

1. `%` to select the whole file
2. `s` to split the selections on a search term
3. Input `use` and hit Enter
4. `mr([` to replace the parentheses with square brackets

Multiple characters are currently not supported, but planned for future release.

