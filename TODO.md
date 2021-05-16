- Refactor tree-sitter-highlight to work like the atom one, recomputing partial tree updates.

------

- tree sitter:
  - lua
  - markdown
  - zig
  - regex
  - vue
  - kotlin
  - julia
  - clojure
  - erlang

- [ ] use signature_help_provider and completion_provider trigger characters in
    a hook to trigger signature help text / autocompletion
- [ ] document.on_type provider triggers
- [ ] completion isIncomplete support

1
- [ ] respect view fullscreen flag
- [ ] Implement marks (superset of Selection/Range)

- [ ] nixos packaging

- [ ] = for auto indent line/selection
- [ ]  :x for closing buffers

- [ ] repeat selection

- [] jump to alt buffer

- [ ] lsp: signature help
- [x] lsp: hover
- [ ] lsp: document symbols (nested/vec)
- [ ] lsp: code actions
- [ ] lsp: formatting
- [x] lsp: goto

- [ ] search: smart case by default: insensitive unless upper detected

- [ ] move Compositor into tui

2
- [ ] surround bindings (select + surround ( wraps selection in parens )
- [ ] macro recording
- [ ] extend selection (treesitter select parent node) (replaces viw, vi(, va( etc )
- [x] bracket pairs
- [x] comment block (gcc)
- [ ] selection align
- [ ] store some state between restarts: file positions, prompt history
- [ ] highlight matched characters in completion

3
- [ ] diff mode with highlighting?
- [ ] snippet support (tab to jump between marks)
- [ ] gamelisp/wasm scripting

X
- [ ] rendering via skulpin/skia or raw wgpu
