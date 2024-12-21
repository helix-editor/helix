| Name | Description |
| --- | --- |
| `:quit`, `:q` | close the current view. |
| `:quit!`, `:q!` | force close the current view, ignoring unsaved changes. |
| `:open`, `:o`, `:edit`, `:e` | open a file from disk into the current view. |
| `:buffer-close`, `:bc`, `:bclose` | close the current buffer. |
| `:buffer-close!`, `:bc!`, `:bclose!` | close the current buffer forcefully, ignoring unsaved changes. |
| `:buffer-close-others`, `:bco`, `:bcloseother` | close all buffers but the currently focused one. |
| `:buffer-close-others!`, `:bco!`, `:bcloseother!` | force close all buffers but the currently focused one. |
| `:buffer-close-all`, `:bca`, `:bcloseall` | close all buffers without quitting. |
| `:buffer-close-all!`, `:bca!`, `:bcloseall!` | force close all buffers ignoring unsaved changes without quitting. |
| `:buffer-next`, `:bn`, `:bnext` | goto next buffer. |
| `:buffer-previous`, `:bp`, `:bprev` | goto previous buffer. |
| `:write`, `:w`, `:u`, `:x`, `:wq`, `:x!`, `:wq!`, `:w!`, `:wa`, `:wa!`, `:waq`, `:wqa`, `:xa`, `:waq!`, `:wqa!`, `:xa!`, `:wbc`, `:wbc!` | write changes to disk |
| `:new`, `:n` | create a new scratch buffer. |
| `:format`, `:fmt` | format the file using an external formatter or language server. |
| `:indent-style` | set the indentation style for editing. ('t' for tabs or 1-16 for number of spaces.) |
| `:line-ending` | Set the document's default line ending. Options: crlf, lf. |
| `:earlier`, `:ear` | jump back to an earlier point in edit history. |
| `:later`, `:lat` | jump to a later point in edit history. |
| `:quit-all`, `:qa` | close all views. |
| `:quit-all!`, `:qa!` | force close all views ignoring unsaved changes. |
| `:cquit`, `:cq` | quit with exit code (default 1) |
| `:cquit!`, `:cq!` | force quit with exit code (default 1) ignoring unsaved changes. |
| `:theme` | change the editor theme (show current theme if no name specified). |
| `:yank`, `:y`, `:yj`, `:yd` | yank selection to clipboard. |
| `:clipboard-paste-after` | paste system clipboard after selections. |
| `:clipboard-paste-before` | paste system clipboard before selections. |
| `:clipboard-paste-replace` | replace selections with content of system clipboard. |
| `:primary-clipboard-paste-after` | paste primary clipboard after selections. |
| `:primary-clipboard-paste-before` | paste primary clipboard before selections. |
| `:primary-clipboard-paste-replace` | replace selections with content of system primary clipboard. |
| `:show-clipboard-provider` | show clipboard provider name in status bar. |
| `:change-current-directory`, `:cd` | change the current working directory. |
| `:show-directory`, `:pwd` | show the current working directory. |
| `:encoding` | set encoding. based on `https://encoding.spec.whatwg.org`. |
| `:character-info`, `:char` | get info about the character under the primary cursor. |
| `:reload`, `:rl` | discard changes and reload from the source file. |
| `:reload-all`, `:rla` | discard changes and reload all documents from the source files. |
| `:lsp-workspace-command` | open workspace command picker |
| `:lsp-restart` | restarts the language servers used by the current doc |
| `:lsp-stop` | stops the language servers that are used by the current doc |
| `:tree-sitter-scopes` | display tree sitter scopes, primarily for theming and development. |
| `:tree-sitter-highlight-name` | display name of tree-sitter highlight scope under the cursor. |
| `:debug-start`, `:dbg` | start a debug session from a given template with given parameters. |
| `:debug-remote`, `:dbg-tcp` | connect to a debug adapter by TCP address and start a debugging session from a given template with given parameters. |
| `:debug-eval` | evaluate expression in current debug context. |
| `:vsplit`, `:vs` | open the file in a vertical split. |
| `:vsplit-new`, `:vnew` | open a scratch buffer in a vertical split. |
| `:hsplit`, `:hs`, `:sp` | open the file in a horizontal split. |
| `:hsplit-new`, `:hnew` | open a scratch buffer in a horizontal split. |
| `:tutor` | open the tutorial. |
| `:goto`, `:g` | goto line number. |
| `:set-language`, `:lang` | set the language of current buffer (show current language if no value specified). |
| `:set-option`, `:set` | set a config option at runtime. for example to disable smart case search, use `:set search.smart-case false`. |
| `:toggle-option`, `:toggle` | toggle a boolean config option at runtime. for example to toggle smart case search, use `:toggle search.smart-case`. |
| `:get-option`, `:get` | get the current value of a config option. |
| `:sort`, `:rsort` | sort ranges in selection. |
| `:reflow` | hard-wrap the current selection of lines to a given width. |
| `:tree-sitter-subtree`, `:ts-subtree` | display the smallest tree-sitter subtree that spans the primary selection, primarily for debugging queries. |
| `:config-reload` | refresh user config. |
| `:config-open` | open the user config.toml file. |
| `:config-open-workspace` | open the workspace config.toml file. |
| `:log-open` | open the helix log file. |
| `:insert-output` | run shell command, inserting output before each selection. |
| `:append-output` | run shell command, appending output after each selection. |
| `:pipe` | pipe each selection to the shell command. |
| `:pipe-to` | pipe each selection to the shell command, ignoring output. |
| `:run-shell-command`, `:sh` | run a shell command |
| `:reset-diff-change`, `:diffget`, `:diffg` | reset the diff change at the cursor position. |
| `:clear-register` | clear given register. |
| `:redraw` | clear and re-render the whole UI |
| `:move`, `:mv` | move the current buffer and its corresponding file to a different path |
| `:read`, `:r` | load a file into buffer |
