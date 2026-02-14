# Silicon Roadmap

Tracked improvements and features, roughly ordered by implementation difficulty.

## Completed

- [x] `:terminal` always opens+focuses (never hides); `Space+t` still toggles
- [x] `:write` prompts to create parent directories when they don't exist
- [x] Theme loading errors surfaced as editor notifications instead of silent fallback
- [x] Removed dead `trigger_offset` field from Completion, clarified stale TODOs
- [x] Hardened 8 high-risk unwraps across commands, application, indent, LSP, word index

## Medium

- [ ] **Split `commands.rs` into submodules** — 7K lines is too large. Extract movement, selection, and editing commands into separate submodules alongside the existing `typed.rs`, `lsp.rs`, `dap.rs`, `syntax.rs`.
- [ ] **Add tests for `silicon-lua`** — The Lua config system has no dedicated test suite. Write tests covering config parsing, keybinding extraction, theme definition, and error cases.
- [ ] **Add tests for `silicon-terminal`** — Terminal emulation is stateful and async. Add integration tests for spawn, input/output, tab management, and resize.
- [ ] **`:checkhealth` command** — Verify LSP binaries exist, grammars are built, config parses cleanly, runtime directory is present. Render results in a scratch buffer.

## Medium-Hard

- [ ] **Session/workspace restore** — Serialize open buffers, split layout, cursor positions, and terminal tabs to disk. Restore on launch. Add `--session` flag and `:session-save` / `:session-load` commands.
- [ ] **Performance benchmark suite** — Measure startup time, large file open (200MB XML), syntax highlighting latency, keystroke-to-render latency. Automate with CI and publish results.

## Hard

- [ ] **Integrated git UI** — `silicon-vcs` currently only supports diff gutters and `goto_next_change`. Add inline blame, commit log viewer, hunk staging, and branch switcher.
- [ ] **Floating/popup terminal** — Current terminal is bottom-dock only. Add a centered floating terminal mode (useful for lazygit, etc). Requires Compositor integration since the terminal panel currently lives outside it.
- [ ] **Plugin API via Lua** — Expose editor APIs to Lua: `si.editor.open()`, `si.editor.insert()`, `si.selection.*`, `si.command.run()`. Add hooks for `on_save`, `on_open`, `on_mode_change`. This is the biggest differentiator vs Helix.
- [ ] **Fuzzy finder improvements** — Live grep with preview (telescope-style), recent files picker, command palette with fuzzy search across all commands.
- [ ] **AI completions API** — Expose a hook in the completion system for external AI providers (Copilot, local models via Ollama). Ghost text rendering depends on virtual text support.

## Very Hard

- [ ] **Soft wrap and virtual text** — Soft wrap for markdown/prose editing. Virtual text support for inline diagnostics, git blame annotations, and AI ghost text. Affects rendering pipeline, cursor movement, and line number calculations throughout `silicon-view` and `silicon-tui`.
- [ ] **Remote editing (SSH)** — Run `si` locally, edit files on a remote host. Requires a client-server protocol, file synchronization, and latency-aware rendering.
