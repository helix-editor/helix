# Text Objects

## Selecting and Manipulating Text With Text Objects

In Helix, Text Objects are a way to select, manipulate and operate on a piece of
text in a structured way. They allow you to refer to blocks of text based on
their structure or purpose. For example, they allow you to:

- Select inside of a function,
- Select the next argument,
- Select the next changed text tracked by git; and,
- Select the previous type.

The two selection modes available are:

- `ma` - Select around the object (`va` in Vim, `<alt-a>` in Kakoune)
- `mi` - Select inside the object (`vi` in Vim, `<alt-i>` in Kakoune)


![Textobject demo](https://user-images.githubusercontent.com/23398472/124231131-81a4bb00-db2d-11eb-9d10-8e577ca7b177.gif)
![Textobject tree-sitter demo](https://user-images.githubusercontent.com/23398472/132537398-2a2e0a54-582b-44ab-a77f-eb818942203d.gif)


## Navigation Using Text Objects

Navigating between functions, classes, parameters, and other elements is
possible using tree-sitter and textobject queries. For
example to move to the next function use `]f`, to move to previous
type use `[t`, and so on.

![Tree-sitter-nav-demo](https://user-images.githubusercontent.com/23398472/152332550-7dfff043-36a2-4aec-b8f2-77c13eb56d6f.gif)

For the full reference see the [unimpaired](./keymap.html#unimpaired) section of the key bind
documentation.

> 💡 This feature relies on tree-sitter textobjects
> and requires the corresponding query file to work properly.

## Keybind Reference

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
| `x`                    | (X)HTML element          |

> 💡 `f`, `t`, etc. need a tree-sitter grammar active for the current
> document and a special tree-sitter query file to work properly. [Only
> some grammars](./lang-support.md) currently have the query file implemented.
> Contributions are welcome!

## Syntax-aware Motions

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

