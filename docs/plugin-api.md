# Plugin API Vision

Silicon is a customizable version of Helix. The core editing model — selection-first, tree-sitter, batteries-included LSP — stays. What changes is that users can actually extend it.

This document describes the vision, strategy, and implementation plan for Silicon's Lua plugin API.

## Why This Matters

Helix's most controversial decision is refusing extensibility. The community has asked for a plugin system [for years](https://github.com/helix-editor/helix/discussions/3806). Helix's answer is [Steel](https://github.com/helix-editor/helix/pull/8675), a Scheme dialect — an [unpopular choice](https://github.com/helix-editor/helix/discussions/13464) with no stable release in sight.

Silicon uses **Lua 5.4**. The same language Neovim chose, the language plugin authors already know. The config system (`si.config.*`, `si.keymap.*`, `si.theme.*`) already proves it works. The goal is to extend that foundation into a full plugin API.

## Target Audience

- **Helix refugees** who love the editing model but left because they couldn't extend it.
- **Neovim users tired of config** who want that level of power without the setup overhead.
- **Power users and tinkerers** who want to shape their editor to fit their workflow.
- **Everyone else** who just wants an editor that works out of the box — plugins are optional, not required.

## The Competitive Landscape (2026)

| | Helix | Silicon | Neovim |
|---|---|---|---|
| Editing model | Selection-first | Selection-first | Verb-object |
| Scripting | Steel (Scheme, unreleased) | **Lua 5.4** | Lua 5.1 (LuaJIT) |
| Config | TOML | **Lua (shipped)** | Lua |
| Built-in terminal | No | **Yes (Alacritty-quality)** | Yes (basic) |
| LSP out of the box | Yes | **Yes** | Needs config |
| Plugin ecosystem | None | **Planned** | Massive |
| Zero-config usable | Yes | **Yes** | No |

Silicon's niche: **Helix's editing model + Neovim's extensibility.** No one else occupies this space.

## Design Principles

- **Plugins are optional.** Silicon must remain fully functional with zero plugins. Batteries included means batteries *included*, not "install 30 plugins to get started."
- **Lua is the only plugin language.** No fragmentation. One language, one API, one ecosystem.
- **Safe by default.** Plugins run in a sandboxed Lua VM with memory limits (64 MB) and instruction limits. A bad plugin should never crash the editor.
- **Config and plugins use the same API.** The `si.*` namespace serves both. A user's `init.lua` is just a plugin that runs first.
- **Expose, don't invent.** The Rust internals already have the right abstractions (transactions, selections, events). The plugin API should expose them, not reinvent them.

## What Exists Today

The Lua config system is shipped and stable:

- `si.config.*` — all editor settings
- `si.keymap.set()` / `si.keymap.set_many()` — keybindings
- `si.theme.set()` / `si.theme.adaptive()` / `si.theme.define()` — themes
- `si.language()` / `si.language_server()` — language and LSP config
- `si.runners` — file runner templates
- Hot reload via file watcher and `:config-reload`
- Sandboxing (64 MB memory, 1M instruction limit)

The Rust-side event system (`silicon-event`) is also mature:

- `DocumentDidOpen`, `DocumentDidChange`, `DocumentDidClose`
- `SelectionDidChange`, `OnModeSwitch`, `PostCommand`, `PostInsertChar`
- `LanguageServerInitialized`, `LanguageServerExited`
- `ConfigDidChange`

These events exist in Rust but are **not yet exposed to Lua**. Connecting them is the critical next step.

## Implementation Phases

### Phase 1: Persistent Lua Runtime

**The foundation.** Currently the Lua VM is created, executes `init.lua`, and is discarded. For plugins to work, the VM must live for the entire editor session.

**What this requires:**
- Keep the `mlua::Lua` instance alive in the editor state
- Solve the concurrency model: Lua is single-threaded, Silicon is async (tokio). Options include running Lua on a dedicated thread with message passing, or using a mutex-guarded VM invoked from the main thread.
- Ensure plugin state persists between events

**Key files:**
- `silicon-lua/src/state.rs` — where the VM is created
- `silicon-lua/src/lib.rs` — config loading entry point
- `silicon-view/src/editor.rs` — where the VM would be stored

### Phase 2: Event Hooks

Wire the existing Rust event system to Lua callbacks.

```lua
si.on("DocumentDidOpen", function(event)
  if event.path:match("%.md$") then
    si.config.soft_wrap = true
  end
end)

si.on("OnModeSwitch", function(event)
  -- change cursor color per mode, etc.
end)
```

**What this requires:**
- A `si.on(event_name, callback)` function that stores Lua function references
- A dispatch path from `silicon-event` hooks into the Lua VM
- Event data marshaling from Rust structs to Lua tables

**Key files:**
- `silicon-event/src/lib.rs` — `register_hook!` macro
- `silicon-view/src/events.rs` — event definitions

### Phase 3: Read-Only Editor APIs

Let plugins observe the editor state without modifying it.

```lua
local doc = si.editor.current_document()
local path = doc:path()
local line = doc:get_line(10)
local text = doc:get_text()
local sels = doc:selections()
local lang = doc:language()
local mode = si.editor.mode()
local docs = si.editor.documents()
```

**What this requires:**
- Lua userdata wrappers around `Document` and `Editor`
- Safe read access through the existing lock/borrow mechanisms
- Position and selection types exposed as Lua tables

**Key files:**
- `silicon-view/src/document.rs` — document state
- `silicon-view/src/editor.rs` — editor state
- `silicon-core/src/selection.rs` — selection types

### Phase 4: Write APIs

Let plugins modify the editor through the transaction system.

```lua
-- Insert text at cursor
si.editor.insert("hello")

-- Run any built-in command
si.command.run("write")
si.command.run("buffer_close")

-- Open files
si.editor.open("~/project/README.md")

-- Manipulate selections
local doc = si.editor.current_document()
doc:select_all()
doc:set_selection(1, 10, 1, 20)  -- line, col, line, col
```

**What this requires:**
- Transaction construction from Lua (insert, delete, replace)
- Command dispatch from Lua into the command system
- Careful handling of undo history (plugin edits should be undoable)

**Key files:**
- `silicon-core/src/transaction.rs` — transaction system
- `silicon-term/src/commands.rs` — command registry

### Phase 5: Plugin Packaging and Loading

A simple, practical plugin system.

```
~/.config/silicon/plugins/
  my-plugin/
    init.lua        -- entry point
    plugin.toml     -- metadata (name, version, description)
```

```lua
-- plugin.toml
-- name = "my-plugin"
-- version = "0.1.0"
-- description = "Does a thing"

-- init.lua
local M = {}

function M.setup(opts)
  si.on("DocumentDidOpen", function(event)
    -- plugin logic
  end)
  si.keymap.set("normal", "space p", function()
    -- plugin command
  end)
end

return M
```

**What this requires:**
- Plugin discovery and loading order
- A `require()` path that includes the plugins directory
- Optional: dependency declaration between plugins
- `:plugin-list`, `:plugin-reload` commands

## What This Is NOT

- **Not Emacs.** We're not replacing the entire editor with Lua. The core stays in Rust. Plugins extend behavior at well-defined boundaries.
- **Not a package manager.** Plugin distribution and installation is out of scope for now. Users can manage plugins with git, symlinks, or whatever they prefer.
- **Not mandatory.** Silicon with zero plugins should always be a complete, productive editor. Plugins are for people who want more, not a tax on everyone.

## Open Questions

- **Concurrency model:** Should the Lua VM run on its own thread (message passing, more complex) or be mutex-guarded on the main thread (simpler, potential latency)?
- **API stability:** When do we commit to a stable plugin API? Too early locks in bad decisions. Too late discourages plugin authors.
- **Async in Lua:** Should plugins be able to do async work (HTTP requests, subprocess spawning)? Neovim exposes `vim.uv` (libuv bindings). Do we want something similar?
- **UI contributions:** Can plugins render custom UI (floating windows, sidebars, picker entries)? This is Phase 6+ territory but worth thinking about early.
