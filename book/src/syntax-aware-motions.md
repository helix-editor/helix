## Moving the selection with syntax-aware motions

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
