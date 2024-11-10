## Registers

- [User-defined registers](#user-defined-registers)
- [Default registers](#default-registers)
- [Special registers](#special-registers)

In Helix, registers are storage locations for text and other data, such as the
result of a search. Registers can be used to cut, copy, and paste text, similar
to the clipboard in other text editors. Usage is similar to Vim, with `"` being
used to select a register.

### User-defined registers

Helix allows you to create your own named registers for storing text, for
example:

- `"ay` - Yank the current selection to register `a`.
- `"op` - Paste the text in register `o` after the selection.

If a register is selected before invoking a change or delete command, the selection will be stored in the register and the action will be carried out:

- `"hc` - Store the selection in register `h` and then change it (delete and enter insert mode).
- `"md` - Store the selection in register `m` and delete it.

### Default registers

Commands that use registers, like yank (`y`), use a default register if none is specified.
These registers are used as defaults:

| Register character | Contains              |
| ---                | ---                   |
| `/`                | Last search           |
| `:`                | Last executed command |
| `"`                | Last yanked text      |
| `@`                | Last recorded macro   |

### Special registers

Some registers have special behavior when read from and written to.

| Register character | When read              | When written             |
| ---                | ---                    | ---                      |
| `_`                | No values are returned | All values are discarded |
| `#`                | Selection indices (first selection is `1`, second is `2`, etc.) | This register is not writable |
| `.`                | Contents of the current selections | This register is not writable |
| `%`                | Name of the current file | This register is not writable |
| `+`                | Reads from the system clipboard | Joins and yanks to the system clipboard |
| `*`                | Reads from the primary clipboard | Joins and yanks to the primary clipboard |

When yanking multiple selections to the clipboard registers, the selections
are joined with newlines. Pasting from these registers will paste multiple
selections if the clipboard was last yanked to by the Helix session. Otherwise
the clipboard contents are pasted as one selection.

