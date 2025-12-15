# Search

## Searching In a File

To search in a file, press `/`. This prompts for a regex-compatible search
below the statusline. If the search has a match in the file, it will immediately
highlight the matching string and jump to it. To *confirm* the search and stay
on the result, press `Enter`. To *cancel* the search and return to your original
position in the buffer, press `Esc`.

To cycle between search results, press `n` to move forward one result and `N` to
move backwards one result.

To search backwards, press `?` (`Shift`-`/`). In this mode, `n` will still cycle
forwards, and `N` will still cycle backwards through results in the same order.

## Searching for Selections

The most recent search is stored in the `/` register. Registers will be discussed
in more detail later, however for now it is enough to understand that when
you press `Enter` to confirm a search, the search query is loaded into the `/`
register, and that the content of this register is then referred to by `n` and
`N`.

Pressing `*` will load the current selection in the buffer into the `/` register.
This sets the search term to the current selection, which can then be jumped to
in the open buffer with `n` and `N`.

## Adding Next Search Match to Selection

TODO
