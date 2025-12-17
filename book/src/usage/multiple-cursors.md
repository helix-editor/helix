# Multiple cursors

## Introduction

Multiple cursors allows you to perform complex refactors which can be broken down
into a series of steps, as well as a search-and-replace.

To duplicate the current cursor to the next suitable line, press `C`.

- If there is currently a selection that can also be extended down, this
  visual selection will also get transferred over to the new cursor.

When multiple cursors exist, one cursor will be the *primary* cursor/selection.
This cursor will be a slightly different color to the others. It is also
identifiable by the text that appears in the bottom right of the default
statusline, which will look like `[num1]/[num2]`. `num2` shows how many cursors
are currently in the buffer. `num1` shows which of these is the current primary
cursor.

## Keep Commands

There are a number of commands available to change the currently created multiple
cursors:

