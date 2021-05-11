
| Crate        | Description                                            |
| -----------  | -----------                                            |
| helix-core   | Core editing primitives, functional.                   |
| helix-syntax | Tree-sitter grammars                                   |
| helix-lsp    | Language server client                                 |
| helix-view   | UI abstractions for use in backends, imperative shell. |
| helix-term   | Terminal UI                                            |
| helix-tui    | TUI primitives, forked from tui-rs, inspired by Cursive |

# Notes

- server-client architecture via gRPC, UI separate from core
- multi cursor based editing and slicing
- WASM based plugins (builtin LSP & fuzzy file finder)

Structure similar to codemirror:

- text (ropes)
- transactions
  - changes
  - invert changes (generates a revert)
  - annotations (time changed etc)
  - state effects
  - additional editor state as facets
- snapshots as an async view into current state
- selections { anchor (nonmoving), head (moving) from/to } -> SelectionSet with a primary
  - cursor is just a single range selection
- markers
  track a position inside text that synchronizes with edits
- { doc, selection, update(), splice, changes(), facets, tabSize, identUnit, lineSeparator, changeFilter/transactionFilter to modify stuff before }
- view (actual UI)
- viewport(Lines) -> what's actually visible
- extend the view via Decorations (inline styling) or Components (UI)
    - mark / wieget / line / replace decoration
- commands (transform state)
- movement
- selection extension
- deletion
- indentation
- keymap (maps keys to commands)
- history (undo tree via immutable ropes)
  - undoes transactions via reverts
- (collab mode)
- gutter (line numbers, diagnostic marker, etc) -> ties into UI components
- rangeset/span -> mappable over changes (can be a marker primitive?)
- syntax (treesitter)
- fold
- selections (select mode/multiselect)
- matchbrackets
- closebrackets
- special-chars (shows dots etc for specials)
- panel (for UI: file pickers, search dialogs, etc)
- tooltip (for UI)
- search (regex)
- lint (async linters)
- lsp
- highlight
- stream-syntax
- autocomplete
- comment (gc, etc for auto commenting)
- snippets
- terminal mode?

- plugins can contain more commands/ui abstractions to use elsewhere
- languageData as presets for each language (syntax, indent, comment, etc)

Vim stuff:
- motions/operators/text objects
- full visual mode
- macros
- jump lists
- marks
- yank/paste
- conceal for markdown markers, etc
