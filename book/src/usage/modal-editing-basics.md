# Editor Basics

## Entering Helix

Start Helix by running `hx`. This will open the editor with an empty buffer.
A file can be opened directly in Helix from the command line with `hx {filename}`.

## Modal Editing

Helix is a modal editor, meaning it has different modes for different tasks.
The main modes are:

- **Normal** (`NOR`) mode: For navigation and editing commands. This is the default mode.
- **Insert** (`INS`) mode: For typing text directly into the document. Access by typing `i`
  in normal mode.
- **Select**/extend (`SEL`) mode: For making selections and performing operations on them.
  Access by typing `v` in normal mode.

The bottom row of the editor is called the `statusline`. It can contain a
variety of information relevant to the current editor instance. In the bottom
left of the statusline, there will be a segment saying `NOR`. This indicates
that the current mode of the editor is *Normal mode*.

In Normal mode, typing letters will not insert them as text, but rather will
execute specific *commands*. These commands will be explored more later. To
insert text, press `i`, which will switch to insert mode. Now, text can be
written directly into the buffer.

To get back into `NOR` mode, press `esc`.

> 💡 *TIP*: The `Esc` key is quite far away from the home row on most keyboards;
> many people remap `Caps Lock` to `Esc`

## Basic movement

In `NOR` mode and `h`, `j`, `k` and `l` to move your cursor:

- `h`: moves cursor 1 character to the left.
- `j`: moves cursor 1 line below.
- `k`: moves cursor 1 line above.
- `l`: moves cursor 1 character to the right.

The arrow keys can also be used to move, both in `NOR` mode and `INS` mode.
This is *not* encouraged, however, as that involves a lot of movement between
the arrow keys and the home row of the keyboard.

This is the most basic way to navigate files and buffers in Helix, however it is
not the most efficient or only way to do so. More movement methods are discussed
in the following chapter.

## Command mode & Exiting Helix

Command mode lets you send commands directly to the editor without relying
on keybinds. To enter Command mode, press `:` while in `NOR` mode. To exit
command mode without entering a command, press `Esc`.

To exit Helix, type `q` or `quit` while in Command mode, and then press enter
to confirm the command. This quite command will fail if there are unsaved changes
to the file. To force quit and discard these changes, type `q!` or `quit!`.

To open a file, type `e FILENAME`, `edit FILENAME`, `o FILENAME` or `open FILENAME`.

To save a file, use the `w` or `write` command. This command can be executed
on its own, or have a file path optionally provided as a parameter in order
to save the file to that path.

## Selection-first Approach

Helix's philosophy is that each **action** (e.g., delete) will act on a **selection**.
This means whatever you are going to act on (a word, paragraph, line, etc.) is
selected first, and the action itself (delete, change, yank, etc) comes second.
A *cursor* is just a single-width selection.

Every time text is modified (an action), you will fully anticipate the result
because you can clearly see the area of text which is highlighted, and thus
will be modified.

To enter `SEL` mode manually, you can press `v`. Motions done in this state
will extend the current selection, instead of just moving the cursor.
`Esc` will exit back to `NOR` mode, but will keep the selection highlighted.

To remove the current selection, press `;`.

Certain movement commands will also highlight the content contained within the
motion. This will be discussed more in TODO.

## Deletion

To delete the characters under the cursor, press the `d` key. If the current
selection is greater than a single character, all characters in the selection
will be deleted.

The deleted section will be saved to Helix's default clipboard. This is more commonly
known as the default *register*. These are discussed in more detail at TODO.

## Pasting and Copying Text

As deleted text is stored inside the default register, this means we can paste
the content back into the buffer. To do this, in `NOR` mode press `p`, which
will paste the contents of the selection *after* the cursor. To paste text
*before* the cursor, press `P`.

To copy text into the register without deleting it, you can press `y`. This will
**y**ank the contents of the current selection into the default register.

To replace the current selection with the contents of the default register,
you can press `R`.

## Undo and Redo

The `u` command will undo our most recent action.

The `U` command will redo our most recent undo.

## Insertion Commands

There are a number of options available to enter `INS` mode, beyond just `i`.
The most common of these are:

- `i`: insert before the selection.
- `a`: append after the selection.
- `I`: insert at the start of the line.
- `A`: append to the end of the line.

You can also directly start inserting content on a newline:

- `o`: adds a newline below the cursor and enters insert mode on it.
- `O`: adds a newline above the cursor and enters insert mode on it.

You can also delete the current selection and immediately enter into insert
mode to replace it with `c` (change).

## Replace

Type `r<character>` to replace all selected characters with `<ch>`.

## Repetition

Type `.` to repeat the last *insert* command.

Type `Alt` + `.` to repeat the last `f` / `t` movement.
