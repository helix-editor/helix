| Name | Description |
| --- | --- |
| `:quit`, `:q` | Close the current view. |
| `:quit!`, `:q!` | Force close the current view, ignoring unsaved changes. |
| `:open`, `:o` | Open a file from disk into the current view. |
| `:buffer-close`, `:bc`, `:bclose` | Close the current buffer. |
| `:buffer-close!`, `:bc!`, `:bclose!` | Close the current buffer forcefully, ignoring unsaved changes. |
| `:buffer-close-others`, `:bco`, `:bcloseother` | Close all buffers but the currently focused one. |
| `:buffer-close-others!`, `:bco!`, `:bcloseother!` | Force close all buffers but the currently focused one. |
| `:buffer-close-all`, `:bca`, `:bcloseall` | Close all buffers without quitting. |
| `:buffer-close-all!`, `:bca!`, `:bcloseall!` | Force close all buffers ignoring unsaved changes without quitting. |
| `:buffer-next`, `:bn`, `:bnext` | Goto next buffer. |
| `:buffer-previous`, `:bp`, `:bprev` | Goto previous buffer. |
| `:write`, `:w` | Write changes to disk. Accepts an optional path (:write some/path.txt) |
| `:write!`, `:w!` | Force write changes to disk creating necessary subdirectories. Accepts an optional path (:write! some/path.txt) |
| `:write-buffer-close`, `:wbc` | Write changes to disk and closes the buffer. Accepts an optional path (:write-buffer-close some/path.txt) |
| `:write-buffer-close!`, `:wbc!` | Force write changes to disk creating necessary subdirectories and closes the buffer. Accepts an optional path (:write-buffer-close! some/path.txt) |
| `:new`, `:n` | Create a new scratch buffer. |
| `:format`, `:fmt` | Format the file using the LSP formatter. |
| `:indent-style` | Set the indentation style for editing. ('t' for tabs or 1-8 for number of spaces.) |
| `:line-ending` | Set the document's default line ending. Options: crlf, lf. |
| `:earlier`, `:ear` | Jump back to an earlier point in edit history. Accepts a number of steps or a time span. |
| `:later`, `:lat` | Jump to a later point in edit history. Accepts a number of steps or a time span. |
| `:write-quit`, `:wq`, `:x` | Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt) |
| `:write-quit!`, `:wq!`, `:x!` | Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt) |
| `:write-all`, `:wa` | Write changes from all buffers to disk. |
| `:write-quit-all`, `:wqa`, `:xa` | Write changes from all buffers to disk and close all views. |
| `:write-quit-all!`, `:wqa!`, `:xa!` | Write changes from all buffers to disk and close all views forcefully (ignoring unsaved changes). |
| `:quit-all`, `:qa` | Close all views. |
| `:quit-all!`, `:qa!` | Force close all views ignoring unsaved changes. |
| `:cquit`, `:cq` | Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2). |
| `:cquit!`, `:cq!` | Force quit with exit code (default 1) ignoring unsaved changes. Accepts an optional integer exit code (:cq! 2). |
| `:theme` | Change the editor theme (show current theme if no name specified). |
| `:clipboard-yank` | Yank main selection into system clipboard. |
| `:clipboard-yank-join` | Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline. |
| `:primary-clipboard-yank` | Yank main selection into system primary clipboard. |
| `:primary-clipboard-yank-join` | Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline. |
| `:clipboard-paste-after` | Paste system clipboard after selections. |
| `:clipboard-paste-before` | Paste system clipboard before selections. |
| `:clipboard-paste-replace` | Replace selections with content of system clipboard. |
| `:primary-clipboard-paste-after` | Paste primary clipboard after selections. |
| `:primary-clipboard-paste-before` | Paste primary clipboard before selections. |
| `:primary-clipboard-paste-replace` | Replace selections with content of system primary clipboard. |
| `:show-clipboard-provider` | Show clipboard provider name in status bar. |
| `:change-current-directory`, `:cd` | Change the current working directory. |
| `:show-directory`, `:pwd` | Show the current working directory. |
| `:encoding` | Set encoding. Based on `https://encoding.spec.whatwg.org`. |
| `:character-info`, `:char` | Get info about the character under the primary cursor. |
| `:reload` | Discard changes and reload from the source file. |
| `:reload-all` | Discard changes and reload all documents from the source files. |
| `:update`, `:u` | Write changes only if the file has been modified. |
| `:lsp-workspace-command` | Open workspace command picker |
| `:lsp-restart` | Restarts the Language Server that is in use by the current doc |
| `:lsp-stop` | Stops the Language Server that is in use by the current doc |
| `:tree-sitter-scopes` | Display tree sitter scopes, primarily for theming and development. |
| `:debug-start`, `:dbg` | Start a debug session from a given template with given parameters. |
| `:debug-remote`, `:dbg-tcp` | Connect to a debug adapter by TCP address and start a debugging session from a given template with given parameters. |
| `:debug-eval` | Evaluate expression in current debug context. |
| `:vsplit`, `:vs` | Open the file in a vertical split. |
| `:vsplit-new`, `:vnew` | Open a scratch buffer in a vertical split. |
| `:hsplit`, `:hs`, `:sp` | Open the file in a horizontal split. |
| `:hsplit-new`, `:hnew` | Open a scratch buffer in a horizontal split. |
| `:tutor` | Open the tutorial. |
| `:goto`, `:g` | Goto line number. |
| `:set-language`, `:lang` | Set the language of current buffer (show current language if no value specified). |
| `:set-option`, `:set` | Set a config option at runtime.<br>For example to disable smart case search, use `:set search.smart-case false`. |
| `:toggle-option`, `:toggle` | Toggle a boolean config option at runtime.<br>For example to toggle smart case search, use `:toggle search.smart-case`. |
| `:get-option`, `:get` | Get the current value of a config option. |
| `:sort` | Sort ranges in selection. |
| `:rsort` | Sort ranges in selection in reverse order. |
| `:reflow` | Hard-wrap the current selection of lines to a given width. |
| `:tree-sitter-subtree`, `:ts-subtree` | Display tree sitter subtree under cursor, primarily for debugging queries. |
| `:config-reload` | Refresh user config. |
| `:config-open` | Open the user config.toml file. |
| `:config-open-workspace` | Open the workspace config.toml file. |
| `:log-open` | Open the helix log file. |
| `:insert-output` | Run shell command, inserting output before each selection. |
| `:append-output` | Run shell command, appending output after each selection. |
| `:pipe` | Pipe each selection to the shell command. |
| `:pipe-to` | Pipe each selection to the shell command, ignoring output. |
| `:run-shell-command`, `:sh` | Run a shell command |
| `:reset-diff-change`, `:diffget`, `:diffg` | Reset the diff change at the cursor position. |
| `:clear-register` | Clear given register. If no argument is provided, clear all registers. |
