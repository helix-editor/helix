# Helix

| Crate        | Description                                            |
| -----------  | -----------                                            |
| helix-core   | Core editing primitives, functional.                   |
| helix-syntax | Tree-sitter grammars                                   |
| helix-view   | UI abstractions for use in backends, imperative shell. |
| helix-term   | Terminal UI                                            |

- server-client architecture via gRPC, UI separate from core
- multi cursor based editing and slicing
- WASM based plugins (builtin LSP & fuzzy file finder)
- piece table-based tree structure for changes


Structure similar to codemirror:

text (ropes)
- column utils, stuff like tab aware (row, col) -> char pos translation
- word/grapheme/code point utils and iterators
state
- transactions
  - changes
  - annotations (time changed etc)
  - state effects
  - additional editor state as facets
- snapshots as an async view into current state
- selections { anchor (nonmoving), head (moving) from/to } -> SelectionSet with a primary
  - cursor is just a single range selection
- markers
  track a position inside text that synchronizes with edits
{ doc, selection, update(), splice, changes(), facets, tabSize, identUnit, lineSeparator, changeFilter/transactionFilter to modify stuff before }
view (actual UI)
- renders via termwiz
- viewport(Lines) -> what's actually visible
- extend the view via Decorations (inline styling) or Components (UI)
    - mark / wieget / line / replace decoration
commands (transform state)
- movement
- selection extension
- deletion
- indentation
keymap (maps keys to commands)
history (undo tree via immutable ropes)
- undoes transactions
- invert changes (generates a revert)
(collab mode)
gutter (line numbers, diagnostic marker, etc) -> ties into UI components
rangeset/span -> mappable over changes (can be a marker primitive?)
syntax (treesitter)
- indentation strategies
fold
selections (select mode/multiselect)
matchbrackets
closebrackets
special-chars (shows dots etc for specials)
panel (for UI: file pickers, search dialogs, etc)
tooltip (for UI)
search (regex? pcre)
lint (async linters)
lsp
highlight (?)
stream-syntax
autocomplete
comment (gc, etc for auto commenting)
snippets

terminal mode?

plugins can contain more commands/ui abstractions to use elsewhere

languageData as presets for each language (syntax, indent, comment, etc)

TODO: determine rust vs script portions

vim stuff:
motions/operators/text objects
full visual mode
macros
jump lists
marks
yank/paste
conceal for markdown markers, etc


---

codemirror uses offsets exclusively with Line being computed when necessary
(with start/end extents)
lines are temporarily cached in a lineCache
