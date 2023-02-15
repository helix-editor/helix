# Usage

(Currently not fully documented, see the [keymappings](./keymap.md) list for more.)

See [tutor](https://github.com/helix-editor/helix/blob/master/runtime/tutor) (accessible via `hx --tutor` or `:tutor`) for a vimtutor-like introduction.

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
| `_`                | Black hole            |

> There is no special register for copying to system clipboard, instead special commands and keybindings are provided. See the [keymap](keymap.md#space-mode) for the specifics.
> The black hole register works as a no-op register, meaning no data will be written to / read from it.

## Surround

Functionality similar to [vim-surround](https://github.com/tpope/vim-surround) is built into
helix. The keymappings have been inspired from [vim-sandwich](https://github.com/machakann/vim-sandwich):

![surround demo](https://user-images.githubusercontent.com/23398472/122865801-97073180-d344-11eb-8142-8f43809982c6.gif)

- `ms` - Add surround characters
- `mr` - Replace surround characters
- `md` - Delete surround characters

`ms` acts on a selection, so select the text first and use `ms<char>`. `mr` and `md` work
on the closest pairs found and selections are not required; use counts to act in outer pairs.

It can also act on multiple selections (yay!). For example, to change every occurrence of `(use)` to `[use]`:

- `%` to select the whole file
- `s` to split the selections on a search term
- Input `use` and hit Enter
- `mr([` to replace the parens with square brackets

Multiple characters are currently not supported, but planned.

## Syntax-tree Motions

`Alt-p`, `Alt-o`, `Alt-i`, and `Alt-n` (or `Alt` and arrow keys) move the primary
selection according to the selection's place in the syntax tree. Let's walk
through an example to get familiar with them. Many languages have a syntax like
so for function calls:

```
func(arg1, arg2, arg3)
```

A function call might be parsed by tree-sitter into a tree like the following.

```tsq
(call
  function: (identifier) ; func
  arguments:
    (arguments           ; (arg1, arg2, arg3)
      (identifier)       ; arg1
      (identifier)       ; arg2
      (identifier)))     ; arg3
```

Use `:tree-sitter-subtree` to view the syntax tree of the primary selection. In
a more intuitive tree format:

```
            ┌────┐
            │call│
      ┌─────┴────┴─────┐
      │                │
┌─────▼────┐      ┌────▼────┐
│identifier│      │arguments│
│  "func"  │ ┌────┴───┬─────┴───┐
└──────────┘ │        │         │
             │        │         │
   ┌─────────▼┐  ┌────▼─────┐  ┌▼─────────┐
   │identifier│  │identifier│  │identifier│
   │  "arg1"  │  │  "arg2"  │  │  "arg3"  │
   └──────────┘  └──────────┘  └──────────┘
```

Say we have a selection that wraps `arg1`. The selection is on the `arg1` leaf
in the tree above.

```
func([arg1], arg2, arg3)
```

Using `Alt-n` would select the next sibling in the syntax tree: `arg2`.

```
func(arg1, [arg2], arg3)
```

While `Alt-o` would expand the selection to the parent node. In the tree above we
can see that we would select the `arguments` node.

```
func[(arg1, arg2, arg3)]
```

There is also some nuanced behavior that prevents you from getting stuck on a
node with no sibling. If we have a selection on `arg1`, `Alt-p` would bring us
to the previous child node. Since `arg1` doesn't have a sibling to its left,
though, we climb the syntax tree and then take the previous selection. So
`Alt-p` will move the selection over to the "func" `identifier`.

```
[func](arg1, arg2, arg3)
```

## Textobjects

![textobject-demo](https://user-images.githubusercontent.com/23398472/124231131-81a4bb00-db2d-11eb-9d10-8e577ca7b177.gif)
![textobject-treesitter-demo](https://user-images.githubusercontent.com/23398472/132537398-2a2e0a54-582b-44ab-a77f-eb818942203d.gif)

- `ma` - Select around the object (`va` in Vim, `<alt-a>` in Kakoune)
- `mi` - Select inside the object (`vi` in Vim, `<alt-i>` in Kakoune)

| Key after `mi` or `ma` | Textobject selected      |
| ---                    | ---                      |
| `w`                    | Word                     |
| `W`                    | WORD                     |
| `p`                    | Paragraph                |
| `(`, `[`, `'`, etc     | Specified surround pairs |
| `m`                    | Closest surround pair    |
| `f`                    | Function                 |
| `c`                    | Class                    |
| `a`                    | Argument/parameter       |
| `o`                    | Comment                  |
| `t`                    | Test                     |
| `g`                    | Change                   |

> NOTE: `f`, `c`, etc need a tree-sitter grammar active for the current
document and a special tree-sitter query file to work properly. [Only
some grammars][lang-support] currently have the query file implemented.
Contributions are welcome!

## Tree-sitter Textobject Based Navigation

Navigating between functions, classes, parameters, etc is made
possible by leveraging tree-sitter and textobjects queries. For
example to move to the next function use `]f`, to move to previous
class use `[c`, and so on.

![tree-sitter-nav-demo][tree-sitter-nav-demo]

See the [unimpaired][unimpaired-keybinds] section of the keybind
documentation for the full reference.

> NOTE: This feature is dependent on tree-sitter based textobjects
and therefore requires the corresponding query file to work properly.

[lang-support]: ./lang-support.md
[unimpaired-keybinds]: ./keymap.md#unimpaired
[tree-sitter-nav-demo]: https://user-images.githubusercontent.com/23398472/152332550-7dfff043-36a2-4aec-b8f2-77c13eb56d6f.gif
