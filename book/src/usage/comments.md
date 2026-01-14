# Commenting

## Commenting a line

`Ctrl`-`c` will comment the line under the cursor. `Ctrl`-`c`
will uncomment the line if it is already commented. You can also
use `<space>`-`c`.

If the current selection spans multiple lines, each line in
the selection will be commented. If there are multiple cursors,
the lines under each cursor will be selected.

## Block comments

Some languages have support for *block comments*. For example, in Rust,
you might use `/* ... */` to comment a range of lines, without needing
to place a `//` at the start of each.

Helix supports block commenting. Press `<space>`-`C` to place a block
comment around the current selections. `<space>-`-`C` will remove the
block comment if the selection surrounds it.
