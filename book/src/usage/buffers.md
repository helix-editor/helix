# Buffers

## Buffer Overview

A buffer is the **in-memory** representation of a document. If a file
was opened directly (e.g., `hx README.md`), then the buffer will hold
the in-memory text of `README.md`. If Helix was opened without a
document (`hx`), then Helix will open an empty *scratch* buffer.
As it is in-memory, data in any buffer that is closed without first
being saved to the disk will be lost.

Helix can manage many buffers at once. Each time you open a new file
(with `:e <filename>`, for example), or each time you create a new empty
scratch buffer (with `:n`), Helix will open that in a new buffer,
but still hold the other buffers in memory.

Additionally, you can open multiple buffers directly from the command line.
For example, you can type `hx file1 file2 file3`. Only one file will
display in the viewport. However, a message below the statusline will
indicate `Loaded 3 files.` This tells us that each is currently
held in a buffer.

## Navigating Buffers

To go to the next buffer, use `gn` for ***g****oto* ***n****ext*.

To go to the previous buffer, use `gp` for ***g****oto* ***p****revious*.

## Bufferline

*When enabled*, the bufferline will appear at the top of the screen. It will
show all buffers Helix is currently managing.

If the bufferline is not enabled in your configuration, you can enable it for
a single session of Helix by typing `:set bufferline always`. You can also
make it only appear when more than 1 buffer is open by typing `:set bufferline multiple`.

## Opening Buffers With Globs and Filters

You can use shell globs to automatically open a range of files into multiple
buffers. For example, if we wanted to open every file in the current
directory beginning with the text "file", we could type: `hx file*`.

We can also use more complex filters to open many buffers. For example, to
open every single `.rs` file in the current directory, recursively, we
can type `hx **/*.rs`.

As another example, we can open a Helix buffer for every file that contains
the string `export default async function` using the GNU utilities `find`
and `grep`:

```sh
hx $(find . -type f -exec grep -l \
 "export default async function" {} +)
```

## Opening Buffers At Specific Locations

Helix can open lines at specific line and column numbers. It uses
the format:

```sh
hx <filname>:<line_number>:<column_number>
```

This can be combined with a variety of other CLI tooling to make some
very powerful workflows.
