# Using Helix

<!--toc:start-->
- [Registers](#registers)
  - [User-defined registers](#user-defined-registers)
  - [Special registers](#special-registers)
- [Surround](#surround)
- [Selecting and manipulating text with textobjects](#selecting-and-manipulating-text-with-textobjects)
- [Navigating using tree-sitter textobjects](#navigating-using-tree-sitter-textobjects)
- [Moving the selection with syntax-aware motions](#moving-the-selection-with-syntax-aware-motions)
<!--toc:end-->


> 💡 Currently, not all functionality is fully documented, please refer to the
> [key mappings](./keymap.md) list.

## Opening a file

When you start Helix (`hx`), it automatically opens a file-picker. You can return to file-picker anytime
with `<space>f`. To change config of the file you will need to enter insert mode with `i` to perform a
change and leave it with `<esc>`. To add a newline below and enter insert mode `o`. For a newline above
your cursor it's `O`.

> 💡 If you are a new user, cursor navigation, backspace and delete are fully supported, however
> it's recommended to learn j,k,h,l for navigation. This allows you to keep your fingers on your keyboard
> ergonomically and gradually you will start using more efficient ways to navigate and change your text,
> rather repeat-pressing arrow keys. Vim/Helix users call this "modal editing" paradigm

If you perform a change by mistake use `u` to undo and `U` to re-do.

## `:` Menu

Opening a menu provides access to the most essential functions of Helix such as `:w` (write) `:q` (quit).
To quickly open helix config file `:config-open`, although you can also type `:co` and use `<tab>` to
autocomplete.

After you modify your config file, save it (`:w`) and use `:config-reload` to activate your changes
without leaving Helix.

[Tutor](https://github.com/helix-editor/helix/blob/master/runtime/tutor) is an interractive mode you can
start with `:tutor` command that can teach you about proper / idiomatic way to use Helix through a series
of drills. You can also launch it wiht `hx --tutor`.

## Navigating between buffers

By this point you have multiple buffers open, which should be reflected by the tabs at the top of your screen.
When Helix opens a new file (called buffer, while in memory), your cursor location in a previous file
is marked. To return "back" use `^o` or to go "forward" use `^i`. You can store your current position
in this jumplist stack manually with `^s`.

You can also cycle through buffers by going into next `gn` or previous `gp` one. Close selected buffer
with `:buffer-close` or `:bc`. Searching through open buffers is with `<space>b`.

## Code / Language support

Helix integrates with LSP (Language Service Protocol) if you have a [necessary language service installed](./lang-support.md).
To verify support, run Helix with `--health` argument: `hx --health dart`. 

Use `<space>k` to bring up documentation of a currently selected item. To scroll this pop-up window
use `^D` and `^U`. To go to the definition of selection use `gd` and `^o` to come back. To list
references - `gr`. 

## `Goto` mode

Additional navigation options can be found in the `Goto` panel, which appears after you hit `g`.
Remember that if you navigated yourself into a weird location, you can jump back to previous
location with `^o`.

Absolutely essentials movements are to go to top of the file `gg`, bottom, `ge` beginning of
the current line `gh` and end of the line `gl`. More often you'll find it more useful if you
go to first non-space character with `gs`

## `Left/Right bracket`

Using `[` and `]` for additional navigation options such as `]d` would take you to next
diagnostic (syntax error). To see all diagnostics you can use `<space>d` for current or `<space>D`
to list all diagnostics in your workspace. 
 
## Searching

Another way to navigate around is by searching for text, which you can do with `/` (below) or 
`?` (above) your current position. If there are multiple search matches in a current file you
can go to next match with `n` or previous with `p`.

To search in multiple files use `<space>/` and in the file-picker you can navigate with `^N` and `^P`.

## Selecting

Helix have a great ways to select objects with "match inside" `mi` and "match outside" `ma`. Navigate
to some text surrounded by brackets, quotes or anything else really and use `mim` to select
surrounded text. `mam` is similar, but will also select brackets/quotes that surround your text.

To select current paragraph `mip` or current function `mif` or even a word you are currently
hovering over `miw`. 

A more handy way to select current node (such as function or paragraph) could be `Alt+o` to expand
selection, `Alt+i` to collapse, `Alt+p` to select previous node and `Alt+n` for next node. (This can
also be used with arrow keys - `Alt-up`, `Alt-down`, `Alt-left`, `Alt-right`)
 
To select current line use `x` (will select next line if you do it several times) and `Alt+x` will
shrink your selection.

Most of commands in Helix can be prefixed with a number. `3mip` will select current and 2 following
paragraphs and `2mi(` will select 2nd surrounding pair of brackets. You can also use `10x` to select
10 lines

You can extend your selection with `v` and then moving your cursor around.

To remove your selection it's `;`

## Multiple cursors

Once you have a large selection you can now search inside your selection with `s`. This search
is different to `/` - if it finds multiple occurences of your search text, it will place cursor
on top of each match. You can even repeat the `mi` command to expact selection around each cursor
to refine your selection.

You can also spawn new cursor on a line below with `C` or above with `alt+C`. Any editing
operatior will then be performed below every one of your cursors.

To get rid of extra cursors `,`

If you keep forgetting `;` and `,` and find yourself mashing `<esc>` key to make them go away,
you might as well bind your `<esc>` key to clear selection and remove cursors by placing this
in your config:

```
[keys.normal]
esc = ["collapse_selection", "keep_primary_selection"]
```

## Registers

In Helix, a register is an internal clipboard. After selecting some text, `y` will copy (yank)
your text and `p` can place that text after selection or 'P' to place it before selection. To yank and
delete selected text use `d` and to replace selected text with register content use `R`. You can also
yank text into register, delete it and go into insert mode with a single press of `c`.

To copy selected text into a system clipboard use `<space>y` and to paste from a system clipboard use
`<space>p` or `<space>P`

Any of the mentoned commands can be modified to use a named register by pressing `"` followed by a symbol.
By default `"` register is used. To avoid overwriting contents of register `"`, yank into register `a` with
`"ay` then paste it with `"ap`. 

## Advanced Usage

The above usage guide is aimed to give a new user only the most useful tools to start using Helix efficiently.
Some additional details are provided below, if you wish to go more in-depth.

### Special registers

| Register character | Contains              |
| ---                | ---                   |
| `/`                | Last search           |
| `:`                | Last executed command |
| `"`                | Last yanked text      |
| `_`                | Black hole            |

The system clipboard is not directly supported by a special register. Instead, special commands and keybindings are provided. Refer to the
[key map](keymap.md#space-mode) for more details.

The black hole register is a no-op register, meaning that no data will be read or written to it.

### Surround

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

### Selecting and manipulating text with textobjects

In Helix, textobjects are a way to select, manipulate and operate on a piece of
text in a structured way. They allow you to refer to blocks of text based on
their structure or purpose, such as a word, sentence, paragraph, or even a
function or block of code.

![Textobject demo](https://user-images.githubusercontent.com/23398472/124231131-81a4bb00-db2d-11eb-9d10-8e577ca7b177.gif)
![Textobject tree-sitter demo](https://user-images.githubusercontent.com/23398472/132537398-2a2e0a54-582b-44ab-a77f-eb818942203d.gif)

- `ma` - Select around the object (`va` in Vim, `<alt-a>` in Kakoune)
- `mi` - Select inside the object (`vi` in Vim, `<alt-i>` in Kakoune)

| Key after `mi` or `ma` | Textobject selected      |
| ---                    | ---                      |
| `w`                    | Word                     |
| `W`                    | WORD                     |
| `p`                    | Paragraph                |
| `(`, `[`, `'`, etc.    | Specified surround pairs |
| `m`                    | The closest surround pair    |
| `f`                    | Function                 |
| `t`                    | Type (or Class)          |
| `a`                    | Argument/parameter       |
| `c`                    | Comment                  |
| `T`                    | Test                     |
| `g`                    | Change                   |

> 💡 `f`, `t`, etc. need a tree-sitter grammar active for the current
document and a special tree-sitter query file to work properly. [Only
some grammars][lang-support] currently have the query file implemented.
Contributions are welcome!

### Navigating using tree-sitter textobjects

Navigating between functions, classes, parameters, and other elements is
possible using tree-sitter and textobject queries. For
example to move to the next function use `]f`, to move to previous
type use `[t`, and so on.

![Tree-sitter-nav-demo][tree-sitter-nav-demo]

For the full reference see the [unimpaired][unimpaired-keybinds] section of the key bind
documentation.

> 💡 This feature relies on tree-sitter textobjects
> and requires the corresponding query file to work properly.

### Moving the selection with syntax-aware motions

`Alt-p`, `Alt-o`, `Alt-i`, and `Alt-n` (or `Alt` and arrow keys) allow you to move the 
selection according to its location in the syntax tree. For example, many languages have the
following syntax for function calls:

```js
func(arg1, arg2, arg3);
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

If you have a selection that wraps `arg1` (see the tree above), and you use
`Alt-n`, it will select the next sibling in the syntax tree: `arg2`.

```js
// before
func([arg1], arg2, arg3)
// after
func(arg1, [arg2], arg3);
```

Similarly, `Alt-o` will expand the selection to the parent node, in this case, the
arguments node.

```js
func[(arg1, arg2, arg3)];
```

There is also some nuanced behavior that prevents you from getting stuck on a
node with no sibling. When using `Alt-p` with a selection on `arg1`, the previous
child node will be selected. In the event that `arg1` does not have a previous
sibling, the selection will move up the syntax tree and select the previous
element. As a result, using `Alt-p` with a selection on `arg1` will move the
selection to the "func" `identifier`.

[lang-support]: ./lang-support.md
[unimpaired-keybinds]: ./keymap.md#unimpaired
[tree-sitter-nav-demo]: https://user-images.githubusercontent.com/23398472/152332550-7dfff043-36a2-4aec-b8f2-77c13eb56d6f.gif
