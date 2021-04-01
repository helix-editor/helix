- Refactor tree-sitter-highlight to work like the atom one, recomputing partial tree updates.
- syntax errors highlight query

------

- [ ] use signature_help_provider and completion_provider trigger characters in
    a hook to trigger signature help text / autocompletion
- [ ] document_on_type provider triggers
- [ ] completion isIncomplete support

1
- [ ] respect view fullscreen flag
- [ ] Implement marks (superset of Selection/Range)
- [ ] nixos packaging
- [ ] CI binary builds

- [ ] regex search / select next
- [ ] = for auto indent line/selection
- [ ] yank on delete
- [ ]  :x for closing buffers

- [ ] jumplist (push selections on goto / select on the view)
- [ ] repeat insert/command -> transaction
- [ ] repeat selection

- [] jump to alt buffer

- [ ] load toml configs, themes, tabsize/identation

- [ ] draw separator line between views

- [ ] lsp: signature help
- [x] lsp: hover
- [ ] lsp: document symbols (nested/vec)
- [ ] lsp: code actions
- [ ] lsp: formatting
- [x] lsp: goto

- decide if markdown should have vertical padding too

- the hooks system should be better for pre/post insert.

2
- [ ] surround bindings (select + surround ( wraps selection in parens )
- [ ] macro recording
- [x] tab completion for paths on the prompt
- [ ] extend selection (treesitter select parent node) (replaces viw, vi(, va( etc )
- [ ] bracket pairs
- [x] comment block (gcc)
- [ ] completion signature popups/docs
- [ ] multiple views into the same file
- [ ] selection align
- [ ] store some state between restarts: file positions, prompt history
- [ ] highlight matched characters in completion

3
- [x] diagnostics popups
- [ ] diff mode with highlighting?
- [ ] snippet support (tab to jump between marks)
- [ ] gamelisp/wasm scripting

X
- [ ] rendering via skulpin/skia or raw wgpu
