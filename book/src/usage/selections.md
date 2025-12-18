# Selections

## Select / Extend Mode

As a recap from the discussion in [Editor Basics](./modal-editing-basics.md#selection-first-approach):

- Type `v` to enter `SEL` mode.
- Type `v` or press `Esc` to return to `NOR` mode.

In `SEL` mode, every movement will extend the selection, as opposed to replacing
it.

## Selecting Lines

Type `x` to select the entire line, and move the cursor to the end of it.
Pressing `x` again will extend the selection with the next line below the cursor.

To select everything in a buffer, press `%`.

## Collapsing & Flipping Selections

To collapse a selection, press `;`.

To flip the direction of the selection, press `Alt` + `;`. This will keep the
selected content, but place the cursor at the other side of it.

## Select Command

The select command allows you to narrow down a selection to matches within the
current selection. To enter the select command, press `s` while some content in
the buffer is selected. A prompt will appear below the statusline that allows
you to filter by whatever text is entered in the prompt. This text is
regex-compatible.

## Joining Lines

Type `J` to join together lines in a selection.
