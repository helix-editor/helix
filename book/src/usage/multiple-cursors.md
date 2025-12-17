# Multiple cursors

## Creating Multiple Cursors

Multiple cursors allow you to perform complex refactors which can be broken
down to a series of steps, as well as search-and-replace.

To create another cursor below the current cursor, press `C`. If the current
primary cursor is a visual selection, the visual selection will be transferred
over to the new cursor too.

Of all cursors created, one cursor is the "primary" cursor. This is highlighted
a slightly different colour in most color themes. It is also represented
in the bottom right of the default status bar, and looks like `[num1]/[num2]`.
`num2` represents the total amount of cursors in the buffer. `num1` represents
which of these cursors is currently the "primary" cursor.

## Aligning Multiple Selections

Where there are multiple cursors across multiple lines, sometimes the cursors will
not be in a perfect vertical line. Where this is the case, hitting `&` will align
all the content under the cursors.

This alignment only cares about the "head" of the selections (the end that moves).
The other end is called the "anchor."

## Split Multi-line Selection Into Multiple Cursors

If multiple lines of text have been selected (whether with `x` or simply
extending a selection far enough), you can split each line of the selection into
its own cursor by pressing `Alt`-`s`.

You can also split a selection on a specific regex pattern, instead of newlines.
To do this, press `S`.

## Filtering Multiple Cursors

You can narrow down the number of cursors by using a regex-based text query,
similarly to using the Select command to narrow selected text. To activate
the prompt, press `K`.

## Cycling Cursors/Selectors

You can cycle between which cursor is the "primary" cursor by using `)` and `(`.
To remove the current primary cursor and selection, press `Alt` + `,`.

You can also cycle the *content* of selections, by using `Alt`-`(` and `Alt`-`)`.

