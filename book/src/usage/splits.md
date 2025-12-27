# Splits

## Multiple Windows

A *window* refers to the actual viewport in Helix that lets us see
a particular open buffer. By default, Helix uses a single viewport
displaying a single buffer. However, you can split the default
single viewport into multiple viewports, which can point at
either the same buffer, or different buffers.

## Creating Multiple Windows

`Ctrl`-`w` will open the Window menu, which displays a list
of available commands related to window management. You can
also use `<space>`-`w` instead for any commands described
in this section.

To create a horizontal "split", we can use `Ctrl`+`w`+`s`.
To create a vertical "split", we can use `Ctrl`+`w`+`v`.

Both commands will create a split of the *current buffer*,
and will move your cursor to the new window.

To create a horizontal or vertical split with a *new buffer*
in the new split, you can use `Ctrl`+`w`+`ns` or Ctrl`+`w`+`ns`
respectively.

## Navigating Splits

You can navigate splits using `Ctrl`+`w` followed by the usual
directional controls (`h`, `j`, `k`, `l`). You can also use
`Ctrl`+`w`-`w`. To automatically switch to the next window.

## Alternative Ways To Open Splits

The `--vsplit` and `--hsplit` flags can be used with Helix to open all
of the files as either vertical or horizontal splits. For example,

```sh
hx --vsplit file1 file2
```

Helix commands can also be used: the `:vsplit` and `:hsplit` commands will
vertically or horizontally split the current buffer viewport.

## Modifying Splits

A split can be transposed with `Ctrl`-`w` + `t`. This will turn a vertical
split into a horizontal split, and vice-versa.

A window's position can also be swapped with its neighbors by using
`Ctrl`-`w` + `h`, `j`, `k`, or `l`. This will switch it with its
left, bottom, top or right neighbor respectively.