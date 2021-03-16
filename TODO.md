- Implement style configs, tab settings
- Refactor tree-sitter-highlight to work like the atom one, recomputing partial tree updates.
- syntax errors highlight query

- UI work: completion popup


------

1
- [ ] respect view fullscreen flag
- [ ] Implement marks (superset of Selection/Range)
- [ ] ctrl-v/ctrl-x on file picker
- [ ] linewise selection work
- [ ] nixos packaging
- [ ] CI binary builds

- [ ] regex search / select next
- [ ] open_above (O) command
- [ ] = for auto indent line/selection
- [x] q should only close the view, if all are closed, close the editor
- [ ] buffers should sit on editor.buffers, view simply refs them
- [ ] yank on delete

- [ ] load toml configs, themes, tabsize/identation

- [ ] draw separator line between views

- [ ] lsp: signature help
- [x] lsp: hover
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
