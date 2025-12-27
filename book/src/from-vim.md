# Migrating from Vim

- [Delete/Change Commands](#Delete/Change%20Commands)
- [Navigation](#Navigation)
- [Line Deletes](#Line%20Deletes)
- [Comment lines, Completion, Search](#Comment%20lines,%20Completion,%20Search)
- [File actions](#File%20actions)

Helix's editing model is strongly inspired from Vim and Kakoune, and a notable
difference from Vim (and the most striking similarity to Kakoune) is that Helix
follows the `selection → action` model. This means that whatever you are
going to act on (a word, a paragraph, a line, etc.) is selected first and the
action itself (delete, change, yank, etc.) comes second. A cursor is simply a
single width selection.

*Note:* As Helix is inspired by Vim and [Kakoune](https://github.com/mawww/kakoune), the keybindings are similar but also have some differences. The content of this page is inspired by [Kakoune Wiki](https://github.com/mawww/kakoune/wiki/Migrating-from-Vim).

NOTE: Unlike vim, `f`, `F`, `t` and `T` are not confined to the current line.

## Delete/Change Commands

delete a word:
* vim: `dw`
* helix: `wd`

change a word:
* vim: `cw`
* helix: `ec` or `wc` (includes the whitespace after the word)

delete a character:
* vim: `x`
* helix: `d` or `;d` (`;` reduces the selection to a single char)

copy a line:
* vim: `yy`
* helix: `Xy` (`X` extends all selections to whole lines)

global replace:
* vim: `:%s/word/replacement/g<ret>`
* helix: `%sword<ret>creplacement<esc>`

Explanation: `%` selects the entire buffer, `s` opens a prompt for a regex, `<ret>` validates the regex and reduces the selection to each match (hence, all occurrences of word are selected). `c` deletes the selection contents and enter insert mode, replacement is typed and then `<esc>` goes back to normal mode.

## Navigation

go to first line:
* vim: `gg`
* helix: `gg`

go to last line:
* vim: `G`
* helix: `ge`

go to line start:
* vim: `0`
* helix: `gh`

go to line first non-blank character:
* vim: `^`
* helix: `gs`

go to line end:
* vim: `$`
* helix: `gl`

jump to matching bracket:
* vim: `%`
* helix: `mm`

## Line Deletes

delete to line end:
* vim: `D`
* helix: `vgld` or `t<ret>d`

Note: `v` is used along with `gl` (go to line end), because [`gl` does not select text](https://github.com/helix-editor/helix/issues/1630).
`t<ret>` selects "'til" the newline represented by `<ret>`.

delete entire line:
* vim: `dd`
* helix: `xd`

Note: `x` selects the entire line under the cursor

## Comment lines, Completion, Search

auto complete:
* vim: `C-p`
* helix: `C-x`

comment lines:
* vim: `gc`
* helix: `Space-c`

search for the word under the cursor:
* vim: `*`
* helix: `A-o*n` (if there's a tree-sitter grammar or LSP) or `be*n`

Explanation: if there's a grammar or LSP, `A-o` expands selection to the parent syntax node (which would be the word in our case). Then `*` uses the current selection as the search pattern, and `n` goes to the next occurrence. `b` selects to the beginning of the word, and `e` selects to the end of the word, effectively selecting the whole word.

block selection:
* vim: `C-v`, then expand your selection vertically and horizontally
* helix: There's no "block selection" mode, so instead you'd use multiple cursors. Expand your block selection vertically by adding new cursors on the line below with `C`, and horizontally using standard movements

search "foo" and replace with "bar" in the current selection:
* vim: `:s/foo/bar/g<ret>`
* helix: `sfoo<ret>cbar<esc>,`

Explanation: `s` will open a prompt in the command line for a regex, and select all matches inside the selection (effectively adding a new cursor on each match). Pressing enter will then finalise this step, and allow the `c` to change the selections to "bar". When done, go back to normal mode with `<esc>`, and keep only the primary selection with `,` (remove all the additional cursors).

## File actions

select the whole file:
* vim: `ggVG`
* helix: `%`

reload a file from disk:
* vim: `:e<ret>`
* helix: `:reload<ret>` (or `:reload-all<ret>` to reload all the buffers)

run shell command:
* vim: `:!command`
* helix: `:sh command` (or `!command` to insert its output into the buffer)

setting a bookmark (bookmarking a location):
* vim: `ma` to set bookmark with name a. Use `` `a `` to go back to this bookmarked location.
* helix: there are no named bookmarks, but you can save a location in the jumplist with `C-s`, then jump back to that location by opening the jumplist picker with `<space>-j`, or back in the jumplist with `C-o` and forward with `C-i`

Helix allows [some limited movement in `insert` mode](https://docs.helix-editor.com/keymap.html#insert-mode) without switching to `normal` mode.

Unlike Vim, under Helix, the cursor shape is the same (block) in insert mode and normal mode by default.
This can be adjusted in configuration:

```toml
[editor.cursor-shape]
insert = "bar"
```

> TODO: Mention textobjects, surround, registers

