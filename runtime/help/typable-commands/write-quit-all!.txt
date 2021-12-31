`:write-quit-all!`

Aliases: `:wqa!`, `:xa!`

Writes (saves) all unsaved buffers to disk, then
closes all views, exiting the editor.
Scratch buffers (without associated paths) will
not be saved, and will be discarded when the
editor exits.
Identical to `:write-all` followed by `:quit-all!`.
