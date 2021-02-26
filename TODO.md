- Implement style configs, tab settings
- Refactor tree-sitter-highlight to work like the atom one, recomputing partial tree updates.
- syntax errors highlight query

- UI work: completion popup


------

1
- [ ] selection mode
- [x] % for whole doc selection
- [x] vertical splits
- [x] input counts (30j)
  - [x] input counts for b, w, e
- [ ] respect view fullscreen flag
- [x] retain horiz when moving vertically
- [w] retain horiz when moving via ctrl-u/d
- [x] deindent
- [x] update lsp on redo/undo
- [ ] Implement marks (superset of Selection/Range)
- [ ] ctrl-v/ctrl-x on file picker
- [ ] linewise selection work
- [ ] nixos packaging
- [ ] CI binary builds

- [ ] regex search / select next
- [ ] f / t mappings
- [ ] open_above (O) command
- [ ] = for auto indent line/selection
- [x] q should only close the view, if all are closed, close the editor
- [ ] buffers should sit on editor.buffers, view simply refs them


- [ ] lsp: signature help
- [ ] lsp: hover
- [ ] lsp: document symbols (nested/vec)
- [ ] lsp: code actions
- [ ] lsp: formatting
- [ ] lsp: goto

2
- [ ] tab completion for paths on the prompt
- [ ] extend selection (treesitter select parent node) (replaces viw, vi(, va( etc )
- [ ] bracket pairs
- [x] comment block (gcc)
- [ ] completion signature popups/docs
- [ ] multiple views into the same file
- [ ] selection align
- [ ] store some state between restarts: file positions, prompt history

3
- [ ] diagnostics popups
- [ ] diff mode with highlighting?
- [ ] snippet support (tab to jump between marks)
- [ ] gamelisp/wasm scripting

X
- [ ] rendering via skulpin/skia or raw wgpu
