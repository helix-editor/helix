# Silicon Plugin API — Complete Architecture

This document specifies the complete architecture for Silicon's Lua plugin API. It is grounded in the actual codebase as of February 2026 and is intended to be directly implementable.

Silicon's plugin API is its defining feature — "the customizable Helix." Full Neovim-level extensibility using Lua 5.4, built on top of the existing `si.*` config API. One API layer, one set of conventions, no historical baggage.

---

## Table of Contents

1. [Persistent Lua Runtime](#1-persistent-lua-runtime)
2. [Event Hook System](#2-event-hook-system)
3. [Editor APIs (`si.editor`)](#3-editor-apis-sieditor)
4. [Document APIs (`si.document`)](#4-document-apis-sidocument)
5. [Selection APIs (`si.selection`)](#5-selection-apis-siselection)
6. [Terminal APIs (`si.terminal`)](#6-terminal-apis-siterminal)
7. [Async APIs (`si.async`)](#7-async-apis-siasync)
8. [UI APIs (`si.ui`)](#8-ui-apis-siui)
9. [LSP Middleware (`si.lsp`)](#9-lsp-middleware-silsp)
10. [Custom Modes](#10-custom-modes)
11. [Plugin Lifecycle & Packaging](#11-plugin-lifecycle--packaging)
12. [Rust Implementation Blueprint](#12-rust-implementation-blueprint)
13. [Implementation Phases](#13-implementation-phases)
14. [Complete API Reference](#14-complete-api-reference)
15. [Example Plugins](#15-example-plugins)

---

## 1. Persistent Lua Runtime

### Where the VM Lives

The Lua VM lives as a field on the `Editor` struct in `silicon-view/src/editor.rs`:

```rust
// silicon-view/src/editor.rs
pub struct Editor {
    // ... existing fields ...

    /// The persistent Lua VM. Created at startup, lives until editor exit.
    /// `None` only if Lua initialization failed (editor still runs, plugins disabled).
    pub lua: Option<mlua::Lua>,

    /// Registry of plugin hook callbacks, keyed by event name.
    /// Each entry is a Vec of LuaHookRef (lightweight, Clone-able handle).
    /// The actual Lua callback functions are stored in a Lua-side registry
    /// table, keyed by integer ID. This avoids storing mlua::RegistryKey
    /// (which is not Clone) in Rust data structures.
    pub lua_hooks: HashMap<String, Vec<LuaHookRef>>,

    /// Counter for generating unique hook IDs.
    pub lua_hook_counter: u64,
}
```

Why `Editor` and not `Application`: The `Editor` is the natural owner because (a) it already holds all the state plugins need to read (documents, views, config, language servers), (b) it lives in `silicon-view` which `silicon-lua` can depend on without circularity, and (c) the Compositor's `Context` already has `&mut Editor`, so every command handler already has access.

### Lifecycle

1. **Creation**: `Application::new()` calls `Editor::new()`. Editor creates the Lua VM via `silicon_lua::state::create_lua_state()`, but with modifications for persistence (see below). The VM is stored as `editor.lua = Some(lua)`.

2. **Plugin loading**: After the VM is created and the `si` global table is populated, `Application::new()` loads plugins in order: global `init.lua` → workspace `init.lua` → each plugin's `init.lua` (dependency-ordered).

3. **Config reload** (`:config-reload`): The config values are re-extracted from a fresh execution of `init.lua`, but the VM itself is NOT destroyed. Plugin state in `si.plugin.*` survives config reload. Only `si.config.*`, `si.keymap.*`, `si.theme.*`, and `si.language*` are re-evaluated. Hooks registered by the user's `init.lua` (not plugins) are re-registered.

4. **Plugin hot-reload** (`:plugin-reload <name>`): Only that plugin's hooks are unregistered (by owner tag). Its `si.plugin.<name>` namespace is cleared. Its `init.lua` is re-executed in the existing VM. Other plugins keep their state.

5. **Destruction**: VM is dropped when Editor is dropped (editor exit). No special cleanup needed — Lua's GC handles it.

### Changes to `create_lua_state()` for Persistence

The current `create_lua_state()` in `silicon-lua/src/state.rs` sets a 1M instruction limit hook that fires once. For a persistent VM, we need:

```rust
// silicon-lua/src/state.rs — modified for persistent VM

/// Create a Lua VM suitable for long-lived plugin use.
/// The instruction limit is per-callback-invocation, not global.
pub fn create_persistent_lua_state() -> Result<Lua, LuaConfigError> {
    let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default())?;
    let _ = lua.set_memory_limit(MEMORY_LIMIT); // 64 MiB

    // NO global instruction hook — we set it per-callback instead.
    // See `invoke_lua_hook()` which wraps each callback in a scoped limit.

    // Build the `si` global table (same as current code).
    build_si_table(&lua)?;

    // Add plugin namespace table.
    let si: Table = lua.globals().get("si")?;
    si.set("plugin", lua.create_table()?)?;

    // Register si.on() and si.off() for event hooks.
    register_hook_api(&lua, &si)?;

    Ok(lua)
}
```

### Concurrency Model

Silicon processes events sequentially via `tokio::select! { biased; ... }` in `Application::run()`. The Lua VM is single-threaded. This is a **perfect** fit:

```
Event arrives (keyboard, LSP response, terminal output, timer)
    ↓
Application::handle_*() — runs on main thread
    ↓
If event triggers a Lua hook:
    editor.dispatch_lua_event("DocumentDidOpen", data)
        ↓
    For each registered hook callback:
        Set instruction limit (1M instructions)
        Retrieve callback from Lua registry table by integer ID
        Call Lua function directly (lua.registry_value → Function::call)
        If error: log it, show statusline warning, continue to next hook
        Clear instruction limit
    ↓
Continue Rust event handling
    ↓
render()
```

Key properties:
- **No mutex needed.** The Lua VM is only ever accessed from the main thread, inside event handlers that already have `&mut Editor`.
- **No Send/Sync on Lua values.** Since callbacks are called synchronously from the thread that owns the VM, we never need to send Lua values across threads.
- **Instruction limit per callback.** Each hook invocation gets its own instruction budget (1M). A plugin doing expensive work in one callback can't starve others because the limit resets per call.
- **Async results come back via the Job system.** When a plugin calls `si.async.spawn()`, the Rust side spawns a tokio task. When it completes, it sends a `Callback` through `JOB_QUEUE` → the main event loop picks it up → calls the Lua callback on the main thread. The Lua VM is never touched from the async task.

### Memory Strategy

- **64 MiB limit** (current value in `state.rs`). Sufficient for plugin code + data. If a plugin tries to allocate beyond this, `mlua` returns an error which is caught and reported as a statusline warning.
- **GC policy**: Use Lua's default incremental GC. Optionally expose `si.gc()` for plugins that create many temporary objects. No custom GC tuning unless profiling shows a need.
- **Userdata lifecycle**: Lua userdata objects (Document, Selection, etc.) hold lightweight handles (IDs, indices) rather than references. They look up the real data from `Editor` at call time. This avoids dangling reference problems entirely.

---

## 2. Event Hook System

### Lua API

```lua
-- Register a hook. Returns a numeric ID for later removal.
local id = si.on("DocumentDidOpen", function(event)
    -- event is a table with event-specific fields
    print(event.path)
end)

-- Remove a specific hook by ID.
si.off(id)
```

### Every Exposed Event

These are all events currently defined in `silicon-view/src/events.rs`, plus new events added for the plugin API:

| Event Name | Callback Argument Table | When It Fires |
|---|---|---|
| `DocumentDidOpen` | `{ doc_id, path, language }` | After a document is opened and added to the editor |
| `DocumentDidChange` | `{ doc_id, view_id }` | After a document's text changes (edit, undo, redo) |
| `DocumentDidClose` | `{ doc_id, path, language }` | After a document is closed and removed |
| `DocumentFocusLost` | `{ doc_id }` | When a document loses focus (but isn't closed) |
| `DocumentDidSave` | `{ doc_id, path }` | After a document is written to disk (new event) |
| `SelectionDidChange` | `{ doc_id, view_id }` | After the selection changes in a document |
| `DiagnosticsDidChange` | `{ doc_id }` | When LSP diagnostics update for a document |
| `OnModeSwitch` | `{ old_mode, new_mode, doc_id }` | When the editor mode changes (new event) |
| `PostCommand` | `{ name, doc_id }` | After any editor command executes (new event) |
| `PostInsertChar` | `{ char, doc_id }` | After a character is inserted in insert mode (new event) |
| `LanguageServerInitialized` | `{ server_id, name }` | When an LSP server finishes initialization |
| `LanguageServerExited` | `{ server_id, name }` | When an LSP server process exits |
| `ConfigDidChange` | `{}` | After config reload completes |
| `EditorDidStart` | `{}` | After editor initialization completes (new event) |
| `EditorWillQuit` | `{}` | Before the editor exits (new event) |
| `TerminalDidOpen` | `{ tab_index }` | After a terminal tab is spawned (new event) |
| `TerminalDidClose` | `{ tab_index }` | After a terminal tab is closed (new event) |
| `BufEnter` | `{ doc_id, path, language }` | When focus switches to a buffer (new event) |
| `BufLeave` | `{ doc_id }` | When focus leaves a buffer (new event) |

### How Events Are Dispatched

Each Rust event dispatch site already calls `silicon_event::dispatch(SomeEvent { ... })`. We add a Lua dispatch step after the Rust hooks complete.

**Why `mlua::RegistryKey` can't be stored directly:** `RegistryKey` is intentionally `!Clone` — mlua uses it for ref-counted GC prevention. We can't clone hook lists for iteration. Instead, we store callbacks in a **Lua-side registry table** keyed by integer ID, and keep only lightweight `LuaHookRef` handles on the Rust side.

#### Hook Data Structures

```rust
// silicon-view/src/lua_bridge.rs (new file)

/// Lightweight, Clone-able handle to a Lua hook callback.
/// The actual Lua function lives in a Lua-side table (`si._hooks[id]`),
/// NOT in an mlua::RegistryKey on the Rust side.
#[derive(Debug, Clone)]
pub struct LuaHookRef {
    /// Unique ID for this hook. Also the integer key in `si._hooks`.
    pub id: u64,
    /// Which plugin registered this hook (empty string for user init.lua).
    pub owner: String,
    /// Which event this hook is registered for.
    pub event: String,
}
```

When `si.on(event, callback)` is called from Lua, the Rust implementation:
1. Increments `editor.lua_hook_counter` to get a new ID.
2. Stores the Lua function in `si._hooks[id] = callback` (a Lua table).
3. Pushes a `LuaHookRef { id, owner, event }` into `editor.lua_hooks[event]`.

When `si.off(id)` is called:
1. Sets `si._hooks[id] = nil` (releases the Lua function reference).
2. Removes the matching `LuaHookRef` from `editor.lua_hooks`.

#### Dispatch Implementation

```rust
// silicon-view/src/lua_bridge.rs

impl Editor {
    /// Dispatch an event to all registered Lua hooks.
    ///
    /// Takes the Lua VM out of `self` temporarily (extract-and-reinject)
    /// so we can pass `&mut self` to callbacks via the light userdata pointer.
    pub fn dispatch_lua_event(&mut self, event_name: &str, build_data: impl FnOnce(&mlua::Lua) -> mlua::Result<mlua::Value>) {
        // Get hook list. Clone is cheap — it's just Vec<LuaHookRef> (u64 + String + String).
        let hooks = match self.lua_hooks.get(event_name) {
            Some(hooks) if !hooks.is_empty() => hooks.clone(),
            _ => return,
        };

        // Extract the Lua VM from self so we can pass &mut self to callbacks.
        let Some(lua) = self.lua.take() else { return };

        // Set the editor pointer for si.editor.* calls inside callbacks.
        // See "The Unsafe Boundary" section for safety analysis.
        let self_ptr = self as *mut Editor;
        let _ = lua.set_named_registry_value("si_editor_ptr",
            mlua::LightUserData(self_ptr as *mut std::ffi::c_void));

        // Build the event data table.
        let data = match build_data(&lua) {
            Ok(data) => data,
            Err(err) => {
                log::error!("Failed to build event data for '{}': {}", event_name, err);
                self.lua = Some(lua);
                return;
            }
        };

        // Retrieve the hook callback table from Lua.
        let hooks_table: mlua::Table = lua.globals()
            .get::<mlua::Table>("si").unwrap()
            .get("_hooks").unwrap();

        for hook_ref in &hooks {
            // Set per-callback instruction limit.
            let _ = lua.set_hook(
                mlua::HookTriggers::new().every_nth_instruction(1_000_000),
                |_lua, _debug| Err(mlua::Error::runtime("instruction limit exceeded")),
            );

            // Look up the callback function by integer ID in the Lua table.
            let result: mlua::Result<()> = (|| {
                let callback: mlua::Function = hooks_table.get(hook_ref.id)?;
                callback.call::<()>(data.clone())?;
                Ok(())
            })();

            // Clear instruction limit.
            let _ = lua.remove_hook();

            if let Err(err) = result {
                log::error!("[{}] {} hook failed: {}", hook_ref.owner, event_name, err);
                // Can't call self.set_error() here because self.lua is taken.
                // Store the error message; we'll set it after reinserting lua.
                // (In practice, use a local Vec<String> to collect errors.)
            }
        }

        // Clear the editor pointer — no dangling access after this point.
        let _ = lua.set_named_registry_value("si_editor_ptr",
            mlua::LightUserData(std::ptr::null_mut()));

        // Reinsert the VM.
        self.lua = Some(lua);
    }
}
```

### Hook Ordering

1. Internal Rust hooks run first (via `silicon_event::dispatch`). This ensures internal state (selections, diagnostics, LSP state) is consistent before Lua sees it.
2. Lua hooks run in registration order.
3. Within a plugin, hooks run in the order they were registered.

### Error Isolation

If a Lua hook throws:
1. The error is logged via `log::error!`.
2. A brief message is shown in the statusline: `[plugin-name] DocumentDidOpen: error message`.
3. The full error (including Lua traceback) is stored in a message buffer accessible via `:messages`.
4. The event chain continues — the next hook runs normally.
5. The editor **never** crashes from a plugin error.

---

## 3. Editor APIs (`si.editor`)

### API Surface

```lua
si.editor.open(path, opts?)        -- Open a file
si.editor.close(doc_id?)           -- Close current or specific document
si.editor.documents()              -- List all open documents
si.editor.current()                -- Get current Document object
si.editor.mode()                   -- Get current mode ("normal", "insert", "select")
si.editor.set_mode(mode)           -- Switch mode
si.editor.transaction(fn)          -- Group edits into one undo step
si.editor.notify(msg, level?)      -- Show notification
si.editor.insert(text)             -- Insert at cursors
si.editor.command(name, args?)     -- Run built-in command by name
si.editor.focus(doc_id)            -- Focus a specific document
si.editor.split(direction?)        -- Create a new split ("vertical" or "horizontal")
si.editor.cwd()                    -- Get current working directory
```

### Borrowing Strategy

The critical challenge: Lua callbacks need `&mut Editor`, but the callback is called from code that already has `&mut Editor`. This is the standard Rust aliased-mutability problem.

**Solution: Extract-and-reinject pattern.**

When we call Lua hooks, we temporarily take the `lua` field out of `Editor`:

```rust
impl Editor {
    pub fn with_lua<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mlua::Lua, &mut Editor) -> R,
    {
        // Take the Lua VM out of self. self.lua is now None.
        let lua = self.lua.take()?;
        let result = f(&lua, self);
        // Put it back.
        self.lua = Some(lua);
        Some(result)
    }
}
```

Inside the Lua callback, the `si.editor` methods receive `Editor` via a light userdata pointer stored in the Lua registry:

```rust
// Inside si.editor.mode() implementation:
fn lua_editor_mode(lua: &Lua, _: ()) -> mlua::Result<String> {
    let editor = get_editor(lua)?;
    Ok(editor.mode.to_string())
}

fn get_editor(lua: &Lua) -> mlua::Result<&mut Editor> {
    let ptr: mlua::LightUserData = lua.named_registry_value("si_editor_ptr")?;
    if ptr.0.is_null() {
        return Err(mlua::Error::runtime(
            "si.editor is only available during event callbacks"
        ));
    }
    // SAFETY: see "The Unsafe Boundary" below.
    unsafe { Ok(&mut *(ptr.0 as *mut Editor)) }
}
```

### The Unsafe Boundary

**`get_editor()` is the single `unsafe` point in the entire plugin system.** Every other piece of Lua-Rust interop uses safe mlua APIs. This one spot exists because Rust's borrow checker can't express "the Lua VM was taken out of Editor, so there's no aliasing."

The raw pointer dereference is sound because all four of these invariants hold simultaneously:

1. **Set-before, cleared-after.** `dispatch_lua_event()` sets `si_editor_ptr` to a valid `&mut Editor` pointer immediately before calling any Lua callback, and sets it to `null` immediately after the last callback returns. The pointer is never stale.

2. **Single-threaded only.** The Lua VM is only accessed from the main event loop thread. mlua's `Lua` handle is `!Send` when the `send` feature is used with proper scoping (we take the VM out of Editor, use it on the same thread, put it back). No other thread can call `get_editor()`.

3. **No aliasing.** When `dispatch_lua_event()` runs, `self.lua` is `None` (we took it out). The only reference to `Editor` is the raw pointer. Lua callbacks get `&mut Editor` through this pointer. Since callbacks are called sequentially (not concurrently), there is never more than one `&mut Editor` at a time.

4. **Lifetime is bounded.** The pointer's validity is scoped to the `dispatch_lua_event()` call. A Lua plugin cannot store the pointer and use it later — `get_editor()` checks for null and errors if called outside a callback context (e.g., from a timer callback where the pointer isn't set; timer callbacks set the pointer via their own dispatch path).

**Where the pointer is set** (exhaustive list):
- `Editor::dispatch_lua_event()` — for event hooks
- `Application::handle_lua_keymap_callback()` — for function keybindings
- `Application::handle_lua_async_result()` — for async callback results (timers, spawned processes)
- `Application::handle_lua_custom_mode_key()` — for custom mode key dispatch

Each of these follows the same pattern: set pointer → call Lua → clear pointer.

For commands that need `Application` state (terminal panel), we use the existing `Callback` system: the Lua API enqueues a `Callback` variant, and the event loop processes it on the next iteration.

### `si.editor.transaction(fn)` — Grouped Undo

```lua
si.editor.transaction(function()
    local doc = si.editor.current()
    doc:insert("// Header\n")
    doc:insert("// Footer\n")
    -- Both inserts are one undo step
end)
```

Implementation: Before calling `fn`, create a transaction savepoint on the current document. After `fn` returns, commit the transaction. If `fn` errors, roll back the savepoint. The Rust side uses `Document::apply()` with grouped undo (setting `UndoKind::GroupWith` on the transaction).

### `si.editor.command(name, args?)` — Run Built-in Commands

```lua
si.editor.command("write")                    -- :write
si.editor.command("buffer_close")             -- :buffer-close
si.editor.command("open", { "file.rs" })      -- :open file.rs
```

Implementation: Look up the command name in `typed::TYPABLE_COMMAND_MAP`. Construct a `compositor::Context` from the current `Editor` + `Jobs` state. Call the typed command function. This gives plugins access to every ex-mode command.

For static commands (movement, editing), use the `MappableCommand` dispatch:

```lua
si.editor.command("move_line_down")  -- same as pressing 'j'
```

Look up in `STATIC_COMMAND_LIST`, construct a `commands::Context`, call `execute()`.

### `si.keymap.set()` with Lua Function Callbacks

Currently `si.keymap.set("normal", "key", action)` only accepts string command names or table submenus. The action value is stored in a Lua registry table during `init.lua` execution, then extracted as a `KeyBinding` enum (`Command(String)`, `Sequence(Vec<String>)`, `Node {...}`) by `keymap::extract_keybindings()`. These are converted to `KeyTrie` / `MappableCommand` at load time in `silicon-term`. At keypress time, the keymap system resolves the trie and calls the command's function pointer — no Lua involvement.

**Supporting Lua function callbacks requires a new variant in the keymap pipeline:**

```rust
// silicon-lua/src/keymap.rs — add variant:
#[derive(Debug, Clone)]
pub enum KeyBinding {
    Command(String),
    Sequence(Vec<String>),
    Node { label: String, is_sticky: bool, map: HashMap<KeyEvent, KeyBinding>, order: Vec<KeyEvent> },
    /// A Lua function callback. The u64 is the hook ID in the `si._hooks` table.
    LuaCallback(u64),
}
```

```rust
// silicon-term/src/keymap.rs — add variant to MappableCommand:
pub enum MappableCommand {
    Typable { name: String, args: Vec<String>, doc: &'static str },
    Static { name: &'static str, fun: fn(&mut Context), doc: &'static str },
    Macro { name: String, keys: Vec<KeyEvent> },
    /// A Lua function callback, dispatched to the VM at keypress time.
    LuaCallback { id: u64, description: String },
}
```

**At keypress time**, when `MappableCommand::LuaCallback { id, .. }` is encountered:

```rust
// silicon-term/src/commands.rs — in MappableCommand::execute():
MappableCommand::LuaCallback { id, .. } => {
    // Can't call Lua directly from here — we don't have access to the Lua VM
    // from a keymap command context. Use the Job callback pattern.
    cx.jobs.callback(async move {
        Ok(Callback::LuaKeymapCallback { hook_id: id })
    });
}
```

```rust
// silicon-term/src/job.rs — add variant:
pub enum Callback {
    // ... existing variants ...
    /// Invoke a Lua function registered as a keybinding.
    LuaKeymapCallback { hook_id: u64 },
}
```

```rust
// silicon-term/src/application.rs — handle it:
Callback::LuaKeymapCallback { hook_id } => {
    self.editor.with_lua(|lua, editor| {
        // Set editor pointer for si.editor.* access.
        let _ = lua.set_named_registry_value("si_editor_ptr",
            mlua::LightUserData(editor as *mut Editor as *mut std::ffi::c_void));

        let _ = lua.set_hook(
            mlua::HookTriggers::new().every_nth_instruction(1_000_000),
            |_, _| Err(mlua::Error::runtime("instruction limit exceeded")),
        );

        let result: mlua::Result<()> = (|| {
            let hooks_table: mlua::Table = lua.globals()
                .get::<mlua::Table>("si")?.get("_hooks")?;
            let callback: mlua::Function = hooks_table.get(hook_id)?;
            callback.call::<()>(())?;
            Ok(())
        })();

        let _ = lua.remove_hook();
        let _ = lua.set_named_registry_value("si_editor_ptr",
            mlua::LightUserData(std::ptr::null_mut()));

        if let Err(err) = result {
            editor.set_error(format!("Keymap callback failed: {}", err));
        }
    });
    self.render().await;
}
```

**The dispatch is one event loop iteration deferred** (keypress → enqueue callback → event loop picks it up → calls Lua). This adds negligible latency (sub-millisecond) since the callback channel drains on the next `tokio::select!` iteration, and it avoids needing `&mut Application` + Lua VM access in the keymap command context.

**`keymap_set` changes in `silicon-lua/src/keymap.rs`:**

```rust
// In lua_value_to_binding(), add Function handling:
fn lua_value_to_binding(lua: &Lua, value: Value) -> mlua::Result<KeyBinding> {
    match value {
        Value::String(s) => Ok(KeyBinding::Command(s.to_str()?.to_string())),
        Value::Function(f) => {
            // Store the function in si._hooks and return the ID.
            let si: Table = lua.globals().get("si")?;
            let hooks_table: Table = si.get("_hooks")?;
            let counter: u64 = si.get("_hook_counter")?;
            let id = counter + 1;
            si.set("_hook_counter", id)?;
            hooks_table.set(id, f)?;
            Ok(KeyBinding::LuaCallback(id))
        }
        Value::Table(t) => {
            // ... existing submenu/sequence logic
        }
        _ => Err(mlua::Error::runtime("keymap action must be string, function, or table")),
    }
}
```

**User override priority** is preserved: user `init.lua` runs after plugins. If a plugin sets `si.keymap.set("normal", "space g", plugin_fn)` and the user sets `si.keymap.set("normal", "space g", "goto_line")`, the user's string binding wins because it overwrites the same key in the registry table during extraction. This is the existing behavior — function callbacks don't change it.

---

## 4. Document APIs (`si.document`)

### Userdata Design

`Document` objects in Lua are lightweight handles — they store a `DocumentId` and look up the real `Document` from `Editor` at call time:

```rust
// silicon-lua/src/api/document.rs (new file)

struct LuaDocument {
    id: DocumentId,
}

impl mlua::UserData for LuaDocument {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| Ok(this.id.0));

        methods.add_method("path", |lua, this, ()| {
            let editor = get_editor(lua)?;
            let doc = editor.document(this.id)
                .ok_or_else(|| mlua::Error::runtime("document closed"))?;
            match doc.path() {
                Some(p) => Ok(Some(p.to_string_lossy().to_string())),
                None => Ok(None),
            }
        });

        methods.add_method("name", |lua, this, ()| {
            let editor = get_editor(lua)?;
            let doc = editor.document(this.id)
                .ok_or_else(|| mlua::Error::runtime("document closed"))?;
            Ok(doc.display_name().to_string())
        });

        methods.add_method("language", |lua, this, ()| {
            let editor = get_editor(lua)?;
            let doc = editor.document(this.id)
                .ok_or_else(|| mlua::Error::runtime("document closed"))?;
            Ok(doc.language_name().to_string())
        });

        methods.add_method("line_count", |lua, this, ()| {
            let editor = get_editor(lua)?;
            let doc = editor.document(this.id)
                .ok_or_else(|| mlua::Error::runtime("document closed"))?;
            Ok(doc.text().len_lines())
        });

        // ... more methods below
    }
}
```

### API Surface

```lua
-- Metadata
doc:id()                           -- DocumentId (integer)
doc:path()                         -- File path or nil for scratch buffers
doc:name()                         -- Display name
doc:language()                     -- Language name ("rust", "python", etc.)
doc:is_modified()                  -- Has unsaved changes?

-- Reading text
doc:get_line(n)                    -- Get line n (1-indexed). Returns string.
doc:get_lines(start, finish)       -- Get lines [start, end) as table of strings
doc:get_text()                     -- Entire document text as string
doc:line_count()                   -- Number of lines

-- Selections
doc:selections()                   -- List of Selection userdata objects
doc:primary_selection()            -- The primary selection
doc:set_selections(selections)     -- Replace all selections
doc:set_primary_selection(sel)     -- Replace primary selection only

-- Editing (all go through Transaction system)
doc:insert(text, sel?)             -- Insert text at selection(s)
doc:delete(sel?)                   -- Delete selected text
doc:replace(sel, text)             -- Replace selection content
doc:apply(transaction)             -- Apply a raw Transaction

-- History
doc:undo()                         -- Undo last change
doc:redo()                         -- Redo last undone change
```

### Line Indexing

All line numbers in the Lua API are **1-indexed** (Lua convention). Column numbers are **1-indexed**. Internally, these are converted to 0-indexed char offsets when interfacing with the Rope.

### Write Operations Through Transactions

Every write operation constructs a `Transaction`:

```rust
// doc:insert(text) implementation
methods.add_method("insert", |lua, this, (text, sel): (String, Option<LuaSelection>)| {
    let editor = get_editor_mut(lua)?;
    let doc = editor.document_mut(this.id)
        .ok_or_else(|| mlua::Error::runtime("document closed"))?;
    let view_id = editor.tree.focus;  // current view

    let selection = match sel {
        Some(s) => s.to_selection(),
        None => doc.selection(view_id).clone(),
    };

    let text = doc.text();
    let transaction = Transaction::insert(text, &selection, text.into());
    doc.apply(&transaction, view_id);
    Ok(())
});
```

The `doc.apply()` method:
1. Applies the `ChangeSet` to the `Rope`.
2. Updates all selections to map over the change.
3. Records in undo history.
4. Dispatches `DocumentDidChange` event.
5. Notifies attached language servers.

---

## 5. Selection APIs (`si.selection`)

### Userdata Design

```rust
// silicon-lua/src/api/selection.rs (new file)

/// A single selection range in Lua.
/// Stores anchor and head as char offsets.
struct LuaSelection {
    anchor: usize,
    head: usize,
}

impl mlua::UserData for LuaSelection {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("start", |lua, this, ()| {
            // Return {line, col} of the earlier position (1-indexed)
            let editor = get_editor(lua)?;
            let doc = editor.document(get_current_doc_id(lua)?)
                .ok_or_else(|| mlua::Error::runtime("no document"))?;
            let pos = std::cmp::min(this.anchor, this.head);
            let line = doc.text().char_to_line(pos);
            let col = pos - doc.text().line_to_char(line);
            let t = lua.create_table()?;
            t.set("line", line + 1)?;  // 1-indexed
            t.set("col", col + 1)?;    // 1-indexed
            Ok(t)
        });

        methods.add_method("end", |lua, this, ()| {
            // Return {line, col} of the later position (1-indexed)
            // ... same pattern with max(anchor, head)
        });

        methods.add_method("text", |lua, this, ()| {
            let editor = get_editor(lua)?;
            let doc = editor.document(get_current_doc_id(lua)?)
                .ok_or_else(|| mlua::Error::runtime("no document"))?;
            let from = std::cmp::min(this.anchor, this.head);
            let to = std::cmp::max(this.anchor, this.head);
            let text: String = doc.text().slice(from..to).into();
            Ok(text)
        });

        methods.add_method("is_empty", |_, this, ()| {
            Ok(this.anchor == this.head)
        });

        methods.add_method("anchor", |_, this, ()| Ok(this.anchor));
        methods.add_method("head", |_, this, ()| Ok(this.head));
    }
}
```

### API Surface

```lua
-- Properties
sel:start()                          -- { line = 1, col = 1 } (earlier position)
sel:finish()                         -- { line = 1, col = 5 } (later position)
sel:anchor()                         -- raw char offset (anchor)
sel:head()                           -- raw char offset (head)
sel:text()                           -- selected text as string
sel:is_empty()                       -- cursor (zero-width)?

-- Construction
si.selection.new(start_line, start_col, end_line, end_col)  -- from line/col
si.selection.point(line, col)                                -- cursor at position
si.selection.from_offsets(anchor, head)                      -- from raw char offsets
```

### Multi-Cursor Behavior

The API gives plugins the full selection list. Plugins decide how to operate:

```lua
-- Operate on primary selection only
local sel = doc:primary_selection()
doc:insert("hello", sel)

-- Operate on all selections
for _, sel in ipairs(doc:selections()) do
    doc:replace(sel, string.upper(sel:text()))
end
```

### Mapping to Silicon's Range Type

| Lua | Rust |
|---|---|
| `LuaSelection { anchor, head }` | `silicon_core::Range { anchor, head, old_visual_position: None }` |
| `sel:start()` → `{ line, col }` | `Range::from()` → `Position { row, col }` via Rope |
| `sel:text()` | `doc.text().slice(range.from()..range.to())` |

---

## 6. Terminal APIs (`si.terminal`)

### API Surface

```lua
si.terminal.open(opts?)              -- Show terminal panel (toggle or show)
si.terminal.new(opts?)               -- New terminal tab
si.terminal.send(text, tab?)         -- Send text to terminal
si.terminal.close(tab?)              -- Close terminal tab
si.terminal.list()                   -- List terminal tabs
si.terminal.focus()                  -- Focus the terminal panel
si.terminal.unfocus()                -- Return focus to editor
si.terminal.on_output(callback)      -- Hook for terminal output (future)
```

**`opts` table for `si.terminal.new()`:**
```lua
{
    cmd = "lazygit",        -- Command to run (default: $SHELL)
    cwd = "/some/path",     -- Working directory
    env = { FOO = "bar" },  -- Extra environment variables
}
```

### Mapping to Existing Infrastructure

The terminal panel is owned by `Application`, not `Editor`. Terminal commands from Lua use the existing `Callback` system:

```rust
// When Lua calls si.terminal.new({cmd = "lazygit"}):
fn lua_terminal_new(lua: &Lua, opts: mlua::Value) -> mlua::Result<()> {
    // Parse opts...
    let cmd = /* extract cmd from opts */;

    // Enqueue callback for Application to process.
    // This uses the existing JOB_QUEUE channel.
    let callback = if let Some(cmd) = cmd {
        Callback::RunInTerminal {
            shell: vec!["/bin/sh".into(), "-c".into()],
            cmd,
        }
    } else {
        Callback::NewTerminalTab
    };

    // Use dispatch_blocking since we're on the main thread.
    crate::job::dispatch_blocking_callback(callback);
    Ok(())
}
```

**`si.terminal.send(text)`** requires a new `Callback` variant:

```rust
// silicon-term/src/job.rs — add:
pub enum Callback {
    // ... existing variants ...
    SendToTerminal { text: String, tab: Option<usize> },
}

// silicon-term/src/application.rs — handle:
Callback::SendToTerminal { text, tab } => {
    let idx = tab.unwrap_or(self.terminal_panel.active_tab());
    if let Some(instance) = self.terminal_panel.instance_mut(idx) {
        instance.input(text.as_bytes());
    }
}
```

**`si.terminal.list()`** returns terminal tab info directly. Since this is read-only, we can expose `TerminalPanel` info through the `Editor` struct (which already has `terminal_panel_focused`):

```rust
// Add to Editor:
pub terminal_tab_info: Vec<TerminalTabInfo>,

// Application syncs this each frame:
self.editor.terminal_tab_info = self.terminal_panel.tab_info();
```

---

## 7. Async APIs (`si.async`)

### API Surface

```lua
si.async.spawn(cmd, opts?, callback)   -- Run shell command async
si.async.read_file(path, callback)     -- Async file read
si.async.write_file(path, content, callback)  -- Async file write
si.async.timer(ms, callback)           -- One-shot timer
si.async.interval(ms, callback)        -- Repeating timer, returns cancel_id
si.async.cancel(cancel_id)             -- Cancel a timer/interval
```

### How Async Rust Communicates Back to Lua

The flow for `si.async.spawn()`:

```
Lua calls si.async.spawn("cargo build", {}, callback)
    ↓
Rust implementation:
    1. Store `callback` in Lua registry → get registry_key
    2. Spawn tokio task:
       tokio::spawn(async move {
           let output = tokio::process::Command::new("sh")
               .arg("-c").arg(cmd)
               .output().await;
           dispatch_callback(Callback::LuaAsyncResult {
               registry_key,
               result: output,
           }).await;
       });
    ↓
Event loop picks up Callback::LuaAsyncResult:
    3. Retrieve Lua callback from registry_key
    4. Call callback with result table:
       callback({ code = 0, stdout = "...", stderr = "..." })
    ↓
Editor re-renders
```

New Callback variant:

```rust
pub enum Callback {
    // ... existing variants ...
    LuaCallback {
        /// Registry key for the Lua callback function.
        registry_key: LuaRegistryKey,
        /// Serialized result data (as JSON or a simple enum).
        result: LuaAsyncResult,
    },
}

pub enum LuaAsyncResult {
    ProcessOutput { code: i32, stdout: String, stderr: String },
    FileContent(Result<String, String>),
    FileWritten(Result<(), String>),
    Timer,
}
```

The registry key is a serializable handle (`mlua::RegistryKey` wrapped in a newtype that implements `Send`). mlua's `RegistryKey` is `Send` when the `send` feature is enabled (which Silicon already uses: `mlua = { features = ["send", ...] }`).

### Timer Implementation

```rust
fn lua_timer(lua: &Lua, (ms, callback): (u64, mlua::Function)) -> mlua::Result<()> {
    let key = lua.create_registry_value(callback)?;
    let key = SendableRegistryKey(key);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        dispatch_callback(Callback::LuaCallback {
            registry_key: key,
            result: LuaAsyncResult::Timer,
        }).await;
    });

    Ok(())
}
```

### Interval with Cancellation

```rust
fn lua_interval(lua: &Lua, (ms, callback): (u64, mlua::Function)) -> mlua::Result<u64> {
    let key = lua.create_registry_value(callback)?;
    let cancel_id = NEXT_CANCEL_ID.fetch_add(1, Ordering::Relaxed);

    // Store cancel token
    CANCEL_TOKENS.lock().insert(cancel_id, CancellationToken::new());
    let token = CANCEL_TOKENS.lock().get(&cancel_id).unwrap().clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(ms));
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                _ = interval.tick() => {
                    dispatch_callback(Callback::LuaCallback {
                        registry_key: key.clone(),
                        result: LuaAsyncResult::Timer,
                    }).await;
                }
            }
        }
    });

    Ok(cancel_id)
}
```

---

## 8. UI APIs (`si.ui`)

### API Surface

```lua
si.ui.float(opts)                    -- Create floating window
si.ui.picker(items, opts, on_select) -- Fuzzy picker
si.ui.prompt(text, on_submit)        -- Input prompt
si.ui.menu(items, on_select)         -- Selection menu
si.ui.notify(msg, opts?)             -- Notification popup
```

### Floating Windows

```lua
si.ui.float({
    width = 60,
    height = 20,
    title = "My Plugin",
    content = "Hello from a plugin!",  -- string or table of lines
    border = "rounded",                -- "none", "single", "rounded", "double"
    filetype = "markdown",             -- for syntax highlighting
})
```

**Implementation:** Push a `Popup<Markdown>` or `Popup<Text>` component onto the Compositor. This uses the existing `Popup` and `Overlay` component infrastructure in `silicon-term/src/ui/`.

Since the Compositor is owned by `Application` (not accessible from Lua callbacks directly), we use the `Callback::EditorCompositor` variant:

```rust
fn lua_float(lua: &Lua, opts: mlua::Table) -> mlua::Result<()> {
    let width = opts.get::<u16>("width").unwrap_or(60);
    let height = opts.get::<u16>("height").unwrap_or(20);
    let content: String = opts.get("content")?;
    let title: Option<String> = get_opt(lua, &opts, "title");

    crate::job::dispatch_blocking(move |_editor, compositor| {
        let popup = Popup::new("plugin-float", PluginFloat::new(content, title))
            .max_width(width)
            .max_height(height);
        compositor.push(Box::new(popup));
    });

    Ok(())
}
```

### Fuzzy Picker

```lua
si.ui.picker(
    {"file1.rs", "file2.rs", "file3.rs"},   -- items (strings or tables)
    { prompt = "Select file:" },             -- options
    function(selected)                        -- callback
        si.editor.open(selected)
    end
)
```

**Implementation:** Reuse the existing `Picker` component from `silicon-term/src/ui/picker.rs`. The picker already supports arbitrary items with custom rendering. Wrap the Lua callback in a `Callback::EditorCompositor` that retrieves the selection and calls the Lua function:

```rust
fn lua_picker(lua: &Lua, (items, opts, callback): (mlua::Table, mlua::Table, mlua::Function)) -> mlua::Result<()> {
    let items_vec: Vec<String> = items.sequence_values::<String>().collect::<mlua::Result<_>>()?;
    let prompt: String = get_opt(lua, &opts, "prompt").unwrap_or_default();
    let callback_key = lua.create_registry_value(callback)?;

    crate::job::dispatch_blocking(move |editor, compositor| {
        // Build picker using Silicon's existing picker infrastructure
        let picker = ui::Picker::new(
            items_vec.into_iter().map(|s| PickerItem(s)).collect(),
            prompt,
            move |cx, item, _action| {
                // When user selects an item, call the Lua callback
                let callback = Callback::LuaCallback {
                    registry_key: callback_key.clone(),
                    result: LuaAsyncResult::PickerSelection(item.0.clone()),
                };
                cx.jobs.callback(async move { Ok(callback) });
            },
        );
        compositor.push(Box::new(overlaid(picker)));
    });

    Ok(())
}
```

### Prompt

```lua
si.ui.prompt("Enter filename: ", function(input)
    si.editor.open(input)
end)
```

### Menu

```lua
si.ui.menu(
    {
        { label = "Stage file", action = "git_stage" },
        { label = "Unstage file", action = "git_unstage" },
    },
    function(selected)
        si.editor.command(selected.action)
    end
)
```

### Mapping to Compositor

| Lua API | Compositor Component |
|---|---|
| `si.ui.float()` | `Popup<PluginFloat>` (new lightweight component) |
| `si.ui.picker()` | `Overlay<Picker<PluginPickerItem>>` (reuse existing `Picker`) |
| `si.ui.prompt()` | `Prompt` (existing component from `silicon-term/src/ui/prompt.rs`) |
| `si.ui.menu()` | `Menu` (existing component from `silicon-term/src/ui/menu.rs`) |
| `si.ui.notify()` | Set `editor.status_msg` (existing mechanism) |

---

## 9. LSP Middleware (`si.lsp`)

### API Surface

```lua
-- Transform completions before they're shown
si.lsp.on_completion(function(items)
    -- items is a table of completion items
    -- Return modified items (filter, reorder, add custom items)
    return vim.tbl_filter(function(item)
        return item.kind ~= "Snippet"  -- remove snippets
    end, items)
end)

-- Transform diagnostics before they're displayed
si.lsp.on_diagnostics(function(diagnostics)
    return vim.tbl_filter(function(d)
        return d.severity ~= "hint"  -- hide hints
    end, diagnostics)
end)

-- Transform hover info
si.lsp.on_hover(function(hover)
    hover.contents = hover.contents .. "\n\n---\nCustom note from plugin"
    return hover
end)

-- Transform code actions
si.lsp.on_code_action(function(actions)
    -- Add a custom action
    table.insert(actions, {
        title = "My Custom Action",
        callback = function() si.editor.command("write") end
    })
    return actions
end)
```

### Middleware Pipeline

Multiple plugins can register middleware. They compose in registration order (FIFO):

```
LSP Server Response
    ↓
Plugin A's on_completion (registered first)
    ↓ (returns modified items)
Plugin B's on_completion (registered second)
    ↓ (returns further modified items)
Editor displays result
```

### Implementation

LSP middleware hooks into the existing response handling in `silicon-term/src/commands/lsp.rs`. After the Rust side deserializes an LSP response, it calls the Lua middleware chain before processing:

```rust
// silicon-term/src/commands/lsp.rs — modified completion handler

fn handle_completion_response(editor: &mut Editor, response: CompletionResponse) {
    let items = match response {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    };

    // Convert to Lua tables, run through middleware chain.
    let items = editor.with_lua(|lua, editor| {
        run_lsp_middleware(lua, editor, "completion", items_to_lua_table(lua, &items))
    }).unwrap_or_else(|| items_to_lua_table(&items));

    // Continue with normal completion display...
}
```

### Performance

LSP responses are latency-sensitive. Middleware adds overhead:
- **Completion**: Must be < 5ms total for all middleware. If a middleware exceeds this, log a warning.
- **Diagnostics**: Less time-critical (can be async).
- **Hover/Code actions**: User-initiated, slight delay acceptable.

The instruction limit per middleware callback is set lower (100K instructions instead of 1M) to prevent slow plugins from making the editor feel laggy.

### Storage

```rust
// silicon-view/src/editor.rs

pub struct Editor {
    // ... existing fields ...

    /// LSP middleware chains. Key is the method name.
    pub lsp_middleware: HashMap<String, Vec<LuaHook>>,
}
```

---

## 10. Custom Modes

### API Surface

```lua
-- Define a new mode
si.mode.define("git", {
    label = "GIT",           -- Shown in statusline
    keymap = {
        s = function() si.async.spawn("git add %", {}, function() end) end,
        c = function() si.terminal.new({ cmd = "git commit" }) end,
        p = function() si.async.spawn("git push", {}, function(r)
            si.editor.notify("Push: " .. r.stdout)
        end) end,
        d = function() si.editor.command("diff") end,
        q = function() si.mode.enter("normal") end,
    },
})

-- Enter custom mode
si.mode.enter("git")
```

### Integration with the Mode Enum

Silicon's `Mode` enum (`silicon-view/src/document.rs`) currently has three variants: `Normal`, `Select`, `Insert`. Custom modes are **not** new enum variants. Instead, they are implemented as a **keymap overlay** on Normal mode:

```rust
// silicon-view/src/editor.rs

pub struct Editor {
    // ... existing fields ...

    /// Currently active custom mode, if any. When set, this mode's
    /// keymap takes priority over the normal-mode keymap.
    pub custom_mode: Option<CustomMode>,
}

pub struct CustomMode {
    pub name: String,
    pub label: String,
    /// Lua registry keys for the keymap callbacks.
    pub keymap: HashMap<KeyEvent, mlua::RegistryKey>,
}
```

When a custom mode is active:
1. The statusline shows the custom mode's `label` instead of "NOR".
2. Keypresses are first checked against the custom mode's keymap.
3. If the key is found, the Lua callback is invoked.
4. If the key is NOT found, it falls through to the normal-mode keymap.
5. Pressing Escape or `q` (configurable) exits the custom mode.

### Why Not a New Mode Enum Variant?

Adding variants to `Mode` would require changes throughout the codebase (every `match` on `Mode`). Instead, custom modes are an overlay on Normal mode. This is simpler, doesn't touch internal Rust code for each new mode, and matches how plugins actually use modes (temporary keybinding overrides).

---

## 11. Plugin Lifecycle & Packaging

### Directory Structure

```
~/.config/silicon/
├── init.lua                 # User config (runs first)
├── lua/                     # User modules (in package.path)
│   └── myutils.lua
└── plugins/
    ├── auto-formatter/
    │   ├── plugin.toml      # Plugin metadata
    │   └── init.lua         # Plugin entry point
    ├── lazygit/
    │   ├── plugin.toml
    │   └── init.lua
    └── git-mode/
        ├── plugin.toml
        ├── init.lua
        └── lua/             # Plugin-local modules
            └── helpers.lua
```

### `plugin.toml` Format

```toml
[plugin]
name = "auto-formatter"
version = "0.1.0"
description = "Run formatters on save"
author = "Jane Doe"
license = "MIT"
min_silicon_version = "26.3.0"

[dependencies]
# Other plugins this depends on (loaded first)
# silicon-utils = { git = "https://github.com/user/silicon-utils" }

[lazy]
# Optional: only load when these conditions are met
events = ["DocumentDidSave"]    # Load on first DocumentDidSave
commands = ["format-buffer"]    # Load when :format-buffer is invoked
filetypes = ["rust", "python"]  # Load when these filetypes are opened
```

### Load Order

1. Create persistent Lua VM.
2. Execute `~/.config/silicon/init.lua` (user global config).
3. Execute `.silicon/init.lua` (workspace config, if exists).
4. Discover plugins in `~/.config/silicon/plugins/`.
5. Parse all `plugin.toml` files.
6. Topologically sort by dependencies.
7. Load each plugin's `init.lua` in dependency order.
   - Lazy plugins: register triggers, defer `init.lua` until triggered.
   - Eager plugins: execute `init.lua` immediately.

### Plugin `init.lua` Contract

```lua
-- plugins/my-plugin/init.lua

local M = {}

function M.setup(opts)
    -- opts comes from the user's config:
    -- si.plugin.configure("my-plugin", { option = value })

    -- Register hooks
    si.on("DocumentDidSave", function(event)
        -- plugin logic
    end)

    -- Set default keybindings (user can override)
    si.keymap.set("normal", "space p f", function()
        -- plugin command
    end)
end

return M
```

The plugin system calls `M.setup(opts)` with any user-provided options.

### `:plugin-*` Commands

| Command | Description |
|---|---|
| `:plugin-install <url>` | `git clone` into plugins dir. `<url>` can be `github-user/repo` (expands to `https://github.com/...`) |
| `:plugin-remove <name>` | Delete plugin directory after confirmation |
| `:plugin-list` | Show installed plugins with name, version, status (enabled/disabled/error) |
| `:plugin-reload <name>` | Hot-reload one plugin: unregister hooks, clear state, re-execute init.lua |
| `:plugin-reload` | Hot-reload all plugins |
| `:plugin-disable <name>` | Disable without removing (creates `.disabled` marker file) |
| `:plugin-enable <name>` | Re-enable a disabled plugin |
| `:plugin-update <name?>` | `git pull` in plugin dir (all plugins if no name) |

### Hot-Reload Mechanics

When `:plugin-reload my-plugin` is called:

1. Find all hooks owned by `"my-plugin"` in `editor.lua_hooks`.
2. Remove them (unregister from all event lists).
3. Clear `si.plugin.my_plugin` table in Lua.
4. Remove the plugin's entries from `package.loaded` (so `require()` re-reads files).
5. Re-execute `plugins/my-plugin/init.lua`.
6. Call `M.setup(opts)` again with the user's options.

### Per-Plugin State

```lua
-- In plugin code:
si.plugin.my_plugin = si.plugin.my_plugin or {}
si.plugin.my_plugin.counter = 0

si.on("DocumentDidOpen", function(event)
    si.plugin.my_plugin.counter = si.plugin.my_plugin.counter + 1
end)
```

On reload, `si.plugin.my_plugin` is set to `nil` before the plugin re-initializes, giving the plugin a clean slate.

---

## 12. Rust Implementation Blueprint

### Crate Ownership

| API Namespace | Owning Crate | Rationale |
|---|---|---|
| Lua VM, `si.on/off`, `si.plugin` | `silicon-lua` | Already owns Lua state creation |
| `si.editor`, `si.document`, `si.selection` | `silicon-lua` (types) + `silicon-view` (implementations via traits) | Core editor types live in `silicon-view` |
| `si.terminal` | `silicon-term` (callback dispatch) | Terminal panel owned by Application |
| `si.async` | `silicon-lua` (API) + `silicon-term` (callback handling) | Tokio tasks need the event loop |
| `si.ui` | `silicon-term` (callback dispatch to Compositor) | Compositor owned by Application |
| `si.lsp` | `silicon-lua` (middleware storage) + `silicon-term` (hook points) | LSP calls happen in command handlers |
| `si.mode` | `silicon-lua` (API) + `silicon-view` (CustomMode struct) | Mode state in Editor |
| `si.config`, `si.keymap`, `si.theme` | `silicon-lua` (existing) | Already implemented |

### New Files

```
silicon-lua/src/
├── api/                    # NEW directory
│   ├── mod.rs              # Re-exports
│   ├── editor.rs           # si.editor.* implementations
│   ├── document.rs         # LuaDocument userdata
│   ├── selection.rs        # LuaSelection userdata
│   ├── terminal.rs         # si.terminal.* implementations
│   ├── async_api.rs        # si.async.* implementations
│   ├── ui.rs               # si.ui.* implementations
│   ├── lsp.rs              # si.lsp.* middleware
│   ├── mode.rs             # si.mode.* implementations
│   └── hooks.rs            # si.on(), si.off(), hook dispatch
├── plugin.rs               # NEW: Plugin discovery, loading, lifecycle
├── persistent.rs           # NEW: Persistent VM creation (replaces parts of state.rs)
└── (existing files unchanged)

silicon-view/src/
├── lua_bridge.rs           # NEW: LuaHook struct, dispatch_lua_event(), CustomMode
└── editor.rs               # MODIFIED: add lua, lua_hooks, custom_mode fields

silicon-term/src/
├── job.rs                  # MODIFIED: add LuaCallback, SendToTerminal variants
├── application.rs          # MODIFIED: add Lua VM init, hook dispatch calls
├── commands/
│   └── typed.rs            # MODIFIED: add :plugin-* commands
└── ui/
    └── plugin_float.rs     # NEW: PluginFloat component for si.ui.float()
```

### Key mlua Patterns

**UserData for Document/Selection:**
```rust
impl mlua::UserData for LuaDocument {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("path", |lua, this, ()| { ... });
        // Method receiver is &Self, not &mut Self.
        // Mutation happens through get_editor_mut(lua) pattern.
    }
}
```

**Registering API functions on the `si` table:**
```rust
pub fn register_editor_api(lua: &Lua, si: &Table) -> mlua::Result<()> {
    let editor_table = lua.create_table()?;

    editor_table.set("open", lua.create_function(lua_editor_open)?)?;
    editor_table.set("close", lua.create_function(lua_editor_close)?)?;
    editor_table.set("current", lua.create_function(lua_editor_current)?)?;
    editor_table.set("mode", lua.create_function(lua_editor_mode)?)?;
    // ...

    si.set("editor", editor_table)?;
    Ok(())
}
```

**Function callbacks stored in registry:**
```rust
// Store a Lua function for later invocation:
let key = lua.create_registry_value(callback_fn)?;

// Later, retrieve and call:
let callback: mlua::Function = lua.registry_value(&key)?;
callback.call::<()>(args)?;

// Clean up when no longer needed:
lua.remove_registry_value(key)?;
```

**The `get_editor` pattern (accessing Editor from Lua callbacks):**
```rust
/// Retrieve the Editor reference from the Lua registry.
/// Only valid during a plugin callback invocation.
fn get_editor(lua: &Lua) -> mlua::Result<&mut Editor> {
    let ptr: mlua::LightUserData = lua.named_registry_value("si_editor_ptr")?;
    if ptr.0.is_null() {
        return Err(mlua::Error::runtime(
            "si.editor is only available during event callbacks"
        ));
    }
    // Safety: The pointer is set by dispatch_lua_event/with_lua immediately
    // before the callback and cleared after. The callback runs synchronously
    // on the owning thread.
    unsafe { Ok(&mut *(ptr.0 as *mut Editor)) }
}
```

---

## 13. Implementation Phases

### Phase 1: Persistent Lua Runtime (Foundation)

**What it enables:** The Lua VM survives across events. Plugin state persists. The `si.on()` and `si.off()` APIs work but no events are dispatched yet.

**Files created:**
- `silicon-lua/src/persistent.rs` — `create_persistent_lua_state()`
- `silicon-lua/src/api/mod.rs` — API module structure
- `silicon-lua/src/api/hooks.rs` — `si.on()`, `si.off()` registration
- `silicon-view/src/lua_bridge.rs` — `LuaHook` struct

**Files modified:**
- `silicon-view/src/editor.rs` — add `lua: Option<Lua>`, `lua_hooks`, `lua_hook_counter` fields to `Editor`
- `silicon-term/src/application.rs` — create persistent VM in `Application::new()`, store in `Editor`
- `silicon-lua/src/lib.rs` — export new modules

**Test plan:**
- Unit test: VM created, `si` global exists, `si.on()` stores callback, `si.off()` removes it.
- Integration test: Start editor, hook registered in init.lua, verify hook list populated.
- Edge case: VM creation failure → editor starts without Lua, no crash.

**Demo plugin at this phase:**
```lua
-- Can register hooks (they just don't fire yet)
local id = si.on("DocumentDidOpen", function(event)
    print("opened!")
end)
si.off(id)

-- Plugin state persists
si.plugin.my_plugin = { initialized = true }
```

### Phase 2: Event Hooks + Read-Only Editor API

**What it enables:** Lua hooks fire on real editor events. Plugins can observe (but not modify) the editor state.

**Dependencies:** Phase 1.

**Files created:**
- `silicon-lua/src/api/editor.rs` — `si.editor.current()`, `si.editor.mode()`, `si.editor.documents()`
- `silicon-lua/src/api/document.rs` — `LuaDocument` userdata (read-only methods)
- `silicon-lua/src/api/selection.rs` — `LuaSelection` userdata (read-only methods)

**Files modified:**
- `silicon-view/src/editor.rs` — add `dispatch_lua_event()`, `with_lua()`
- `silicon-term/src/application.rs` — add hook dispatch after Rust event dispatch
- `silicon-view/src/events.rs` — add new events: `DocumentDidSave`, `OnModeSwitch`, `BufEnter`, `BufLeave`

**Test plan:**
- Unit test: `dispatch_lua_event("DocumentDidOpen", data)` → Lua callback fires, receives correct data.
- Unit test: `doc:path()`, `doc:language()`, `doc:line_count()` return correct values.
- Integration test: Open a file → `DocumentDidOpen` hook fires in Lua.
- Error isolation test: Hook throws error → next hook still runs, editor doesn't crash.

**Demo plugin at this phase:**
```lua
si.on("DocumentDidOpen", function(event)
    local doc = si.editor.current()
    si.editor.notify(string.format("Opened %s (%s, %d lines)",
        doc:name(), doc:language(), doc:line_count()))
end)
```

### Phase 3: Write APIs + Commands

**What it enables:** Plugins can modify buffer contents, run commands, manipulate selections. This is where plugins become truly useful.

**Dependencies:** Phase 2.

**Files modified:**
- `silicon-lua/src/api/editor.rs` — add `si.editor.open()`, `si.editor.close()`, `si.editor.insert()`, `si.editor.command()`, `si.editor.transaction()`
- `silicon-lua/src/api/document.rs` — add `doc:insert()`, `doc:delete()`, `doc:replace()`, `doc:set_selections()`
- `silicon-lua/src/api/selection.rs` — add `si.selection.new()`, `si.selection.point()`

**Test plan:**
- Unit test: `doc:insert("hello")` → text appears in document rope.
- Unit test: `si.editor.command("write")` → file written.
- Unit test: `si.editor.transaction(fn)` → multiple edits are one undo step.
- Integration test: Plugin inserts text, user presses `u` → all plugin edits undone in one step.

**Demo plugin at this phase:**
```lua
-- Auto-insert header comment for new files
si.on("DocumentDidOpen", function(event)
    local doc = si.editor.current()
    if doc:line_count() == 1 and doc:get_line(1) == "" then
        if doc:language() == "rust" then
            doc:insert("// TODO: Add description\n\n")
        end
    end
end)
```

### Phase 4: Plugin Packaging & Lifecycle

**What it enables:** `plugin.toml` discovery, `:plugin-list`, `:plugin-reload`, `:plugin-disable/enable`. This is deliberately early so that the plugin lifecycle (load, reload, error reporting, hook cleanup) is dogfooded while building every subsequent API. `:plugin-install` (git clone) and `:plugin-update` (git pull) are synchronous initially; they become async in Phase 5.

**Dependencies:** Phase 3 (needs `si.editor.command()` for typed commands, hook system for cleanup).

**Files created:**
- `silicon-lua/src/plugin.rs` — plugin discovery, `plugin.toml` parsing, load ordering, reload mechanics

**Files modified:**
- `silicon-term/src/commands/typed.rs` — add `:plugin-list`, `:plugin-reload`, `:plugin-disable`, `:plugin-enable`, `:plugin-install` (sync git clone), `:plugin-remove`
- `silicon-term/src/application.rs` — call plugin loader after VM creation

**Test plan:**
- Unit test: `plugin.toml` parsing (name, version, dependencies, lazy triggers).
- Unit test: Topological sort of plugin dependencies.
- Integration test: Create a test plugin in a temp dir, `:plugin-reload` → hooks unregistered and re-registered.
- Integration test: `:plugin-disable` → plugin hooks stop firing. `:plugin-enable` → they resume.

**Demo at this phase:**
```
:plugin-install user/silicon-lazygit
:plugin-list
  auto-formatter  0.1.0  enabled
  lazygit         0.2.0  enabled
:plugin-reload lazygit
:plugin-disable auto-formatter
:plugin-remove lazygit
```

### Phase 5: Async APIs + Terminal

**What it enables:** Plugins can spawn processes, do async I/O, interact with the terminal panel. Enables build runners, REPL integration, lazygit. Also upgrades `:plugin-install` and `:plugin-update` to async.

**Dependencies:** Phase 3.

**Files created:**
- `silicon-lua/src/api/async_api.rs` — `si.async.spawn()`, `si.async.timer()`, etc.
- `silicon-lua/src/api/terminal.rs` — `si.terminal.open()`, `si.terminal.send()`, etc.

**Files modified:**
- `silicon-term/src/job.rs` — add `Callback::LuaCallback`, `Callback::SendToTerminal`
- `silicon-term/src/application.rs` — handle new callback variants

**Test plan:**
- Unit test: `si.async.spawn("echo hello", {}, cb)` → callback receives `{ code=0, stdout="hello\n" }`.
- Unit test: `si.async.timer(100, cb)` → callback fires after ~100ms.
- Integration test: `si.terminal.send("ls\n")` → command runs in terminal.

**Demo plugin at this phase:**
```lua
-- Lazygit integration
si.keymap.set("normal", "space g g", function()
    si.terminal.new({ cmd = "lazygit" })
end)
```

### Phase 6: UI APIs

**What it enables:** Plugins can create floating windows, pickers, prompts. Full UI contribution capability.

**Dependencies:** Phase 3.

**Files created:**
- `silicon-lua/src/api/ui.rs` — `si.ui.float()`, `si.ui.picker()`, `si.ui.prompt()`, `si.ui.menu()`
- `silicon-term/src/ui/plugin_float.rs` — `PluginFloat` component

**Files modified:**
- `silicon-term/src/ui/mod.rs` — register new component

**Test plan:**
- Integration test: `si.ui.float({ content = "hello" })` → popup appears on screen.
- Integration test: `si.ui.picker(items, {}, cb)` → picker opens, user selects, callback fires.

**Demo plugin at this phase:**
```lua
-- Quick file template selector
si.keymap.set("normal", "space n", function()
    si.ui.picker(
        {"Rust module", "Python script", "Markdown doc"},
        { prompt = "Template:" },
        function(choice)
            if choice == "Rust module" then
                local doc = si.editor.current()
                doc:insert("mod {};\n")
            end
        end
    )
end)
```

### Phase 7: LSP Middleware + Custom Modes

**What it enables:** Plugins can filter/transform LSP responses. Plugins can define custom modes with dedicated keymaps.

**Dependencies:** Phase 3.

**Files created:**
- `silicon-lua/src/api/lsp.rs` — `si.lsp.on_completion()`, `si.lsp.on_diagnostics()`, etc.
- `silicon-lua/src/api/mode.rs` — `si.mode.define()`, `si.mode.enter()`

**Files modified:**
- `silicon-view/src/editor.rs` — add `lsp_middleware`, `custom_mode` fields
- `silicon-term/src/commands/lsp.rs` — insert middleware calls in response handlers

**Demo plugin at this phase:**
```lua
-- Hide hint-level diagnostics
si.lsp.on_diagnostics(function(diagnostics)
    local filtered = {}
    for _, d in ipairs(diagnostics) do
        if d.severity ~= "hint" then
            table.insert(filtered, d)
        end
    end
    return filtered
end)

-- Git mode
si.mode.define("git", {
    label = "GIT",
    keymap = {
        s = function() si.async.spawn("git add " .. si.editor.current():path()) end,
        q = function() si.mode.enter("normal") end,
    },
})
si.keymap.set("normal", "space g", function() si.mode.enter("git") end)
```

---

## 14. Complete API Reference

### `si` (Global)

| Function | Signature | Description |
|---|---|---|
| `si.on` | `(event: string, callback: function) → id: integer` | Register an event hook |
| `si.off` | `(id: integer) → nil` | Remove an event hook by ID |
| `si.platform` | `string` (read-only) | OS name: `"macos"`, `"linux"`, `"windows"` |
| `si.config_dir` | `string` (read-only) | Path to `~/.config/silicon/` |
| `si.home_dir` | `string` (read-only) | User's home directory |
| `si.hostname` | `string` (read-only) | Machine hostname |
| `si.version` | `string` (read-only) | Silicon version (e.g. `"26.3.0"`) |

### `si.editor`

| Function | Signature | Description | Example |
|---|---|---|---|
| `open` | `(path: string, opts?: {split?: string}) → Document` | Open a file | `si.editor.open("file.rs")` |
| `close` | `(doc_id?: integer) → nil` | Close document (current if nil) | `si.editor.close()` |
| `documents` | `() → Document[]` | All open documents | `for _, d in ipairs(si.editor.documents()) do ... end` |
| `current` | `() → Document` | Current focused document | `local doc = si.editor.current()` |
| `mode` | `() → string` | Current mode | `if si.editor.mode() == "normal" then ... end` |
| `set_mode` | `(mode: string) → nil` | Switch mode | `si.editor.set_mode("insert")` |
| `transaction` | `(fn: function) → nil` | Group edits as one undo step | `si.editor.transaction(function() ... end)` |
| `notify` | `(msg: string, level?: string) → nil` | Show statusline message | `si.editor.notify("Saved!", "info")` |
| `insert` | `(text: string) → nil` | Insert at cursors | `si.editor.insert("hello")` |
| `command` | `(name: string, args?: string[]) → nil` | Run built-in command | `si.editor.command("write")` |
| `focus` | `(doc_id: integer) → nil` | Focus a document | `si.editor.focus(3)` |
| `split` | `(direction?: string) → nil` | Create split | `si.editor.split("vertical")` |
| `cwd` | `() → string` | Current working directory | `local dir = si.editor.cwd()` |

### `si.document` / Document Userdata

| Method | Signature | Description | Example |
|---|---|---|---|
| `id` | `() → integer` | Document ID | `doc:id()` |
| `path` | `() → string?` | File path (nil for scratch) | `doc:path()` |
| `name` | `() → string` | Display name | `doc:name()` |
| `language` | `() → string` | Language name | `doc:language()` |
| `is_modified` | `() → boolean` | Has unsaved changes | `doc:is_modified()` |
| `get_line` | `(n: integer) → string` | Get line (1-indexed) | `doc:get_line(1)` |
| `get_lines` | `(start: integer, finish: integer) → string[]` | Get line range | `doc:get_lines(1, 10)` |
| `get_text` | `() → string` | Entire document text | `doc:get_text()` |
| `line_count` | `() → integer` | Number of lines | `doc:line_count()` |
| `selections` | `() → Selection[]` | All selections | `doc:selections()` |
| `primary_selection` | `() → Selection` | Primary selection | `doc:primary_selection()` |
| `set_selections` | `(sels: Selection[]) → nil` | Replace selections | `doc:set_selections(sels)` |
| `insert` | `(text: string, sel?: Selection) → nil` | Insert text | `doc:insert("hello")` |
| `delete` | `(sel?: Selection) → nil` | Delete selected text | `doc:delete()` |
| `replace` | `(sel: Selection, text: string) → nil` | Replace selection | `doc:replace(sel, "new")` |
| `undo` | `() → nil` | Undo | `doc:undo()` |
| `redo` | `() → nil` | Redo | `doc:redo()` |

### `si.selection` / Selection Userdata

| Method/Function | Signature | Description | Example |
|---|---|---|---|
| `si.selection.new` | `(sl, sc, el, ec: integer) → Selection` | Create from line/col | `si.selection.new(1,1,1,10)` |
| `si.selection.point` | `(line, col: integer) → Selection` | Cursor at position | `si.selection.point(5, 1)` |
| `si.selection.from_offsets` | `(anchor, head: integer) → Selection` | From char offsets | `si.selection.from_offsets(0, 10)` |
| `sel:start` | `() → {line: int, col: int}` | Start position | `sel:start().line` |
| `sel:finish` | `() → {line: int, col: int}` | End position | `sel:finish().col` |
| `sel:anchor` | `() → integer` | Anchor char offset | `sel:anchor()` |
| `sel:head` | `() → integer` | Head char offset | `sel:head()` |
| `sel:text` | `() → string` | Selected text | `sel:text()` |
| `sel:is_empty` | `() → boolean` | Zero-width? | `sel:is_empty()` |

### `si.terminal`

| Function | Signature | Description | Example |
|---|---|---|---|
| `open` | `(opts?: {}) → nil` | Show terminal panel | `si.terminal.open()` |
| `new` | `(opts?: {cmd?, cwd?, env?}) → nil` | New terminal tab | `si.terminal.new({cmd="htop"})` |
| `send` | `(text: string, tab?: integer) → nil` | Send text to terminal | `si.terminal.send("ls\n")` |
| `close` | `(tab?: integer) → nil` | Close terminal tab | `si.terminal.close()` |
| `list` | `() → {index, title, exited}[]` | List terminal tabs | `si.terminal.list()` |
| `focus` | `() → nil` | Focus terminal panel | `si.terminal.focus()` |
| `unfocus` | `() → nil` | Return focus to editor | `si.terminal.unfocus()` |

### `si.async`

| Function | Signature | Description | Example |
|---|---|---|---|
| `spawn` | `(cmd: string, opts?: {cwd?, env?}, cb: function) → nil` | Async shell command | `si.async.spawn("cargo build", {}, function(r) end)` |
| `read_file` | `(path: string, cb: function) → nil` | Async file read | `si.async.read_file("f.txt", function(content) end)` |
| `write_file` | `(path, content: string, cb: function) → nil` | Async file write | `si.async.write_file("f.txt", "data", function() end)` |
| `timer` | `(ms: integer, cb: function) → nil` | One-shot timer | `si.async.timer(1000, function() end)` |
| `interval` | `(ms: integer, cb: function) → integer` | Repeating timer | `local id = si.async.interval(500, cb)` |
| `cancel` | `(id: integer) → nil` | Cancel interval | `si.async.cancel(id)` |

### `si.ui`

| Function | Signature | Description | Example |
|---|---|---|---|
| `float` | `(opts: {width?, height?, title?, content, border?, filetype?}) → nil` | Floating window | `si.ui.float({content="hello"})` |
| `picker` | `(items: string[], opts?: {prompt?}, on_select: function) → nil` | Fuzzy picker | `si.ui.picker({"a","b"}, {}, function(s) end)` |
| `prompt` | `(text: string, on_submit: function) → nil` | Input prompt | `si.ui.prompt("Name:", function(s) end)` |
| `menu` | `(items: {label, value?}[], on_select: function) → nil` | Selection menu | `si.ui.menu(items, function(s) end)` |
| `notify` | `(msg: string, opts?: {level?, timeout?}) → nil` | Notification | `si.ui.notify("Done!")` |

### `si.lsp`

| Function | Signature | Description | Example |
|---|---|---|---|
| `on_completion` | `(fn: function(items) → items) → id` | Completion middleware | `si.lsp.on_completion(function(items) return items end)` |
| `on_diagnostics` | `(fn: function(diags) → diags) → id` | Diagnostics middleware | `si.lsp.on_diagnostics(function(d) return d end)` |
| `on_hover` | `(fn: function(hover) → hover) → id` | Hover middleware | `si.lsp.on_hover(function(h) return h end)` |
| `on_code_action` | `(fn: function(actions) → actions) → id` | Code action middleware | `si.lsp.on_code_action(function(a) return a end)` |

### `si.mode`

| Function | Signature | Description | Example |
|---|---|---|---|
| `define` | `(name: string, opts: {label, keymap}) → nil` | Create custom mode | `si.mode.define("git", {...})` |
| `enter` | `(name: string) → nil` | Switch to mode | `si.mode.enter("git")` |

### `si.keymap` (existing, extended)

| Function | Signature | Description | Example |
|---|---|---|---|
| `set` | `(mode, key, action: string\|function\|table) → nil` | Set keybinding | `si.keymap.set("normal", "g d", "goto_definition")` |
| `set_many` | `(mode: string, mappings: table) → nil` | Set multiple bindings | `si.keymap.set_many("normal", {j="move_line_down"})` |

**Extension for functions:** Currently `si.keymap.set()` only accepts string command names or tables. For the plugin API, it also accepts Lua functions:

```lua
si.keymap.set("normal", "space x", function()
    si.editor.notify("Custom action!")
end)
```

### `si.theme` (existing)

| Function | Signature | Description |
|---|---|---|
| `set` | `(name: string) → nil` | Set theme |
| `adaptive` | `(opts: {light, dark, fallback?}) → nil` | Auto light/dark |
| `define` | `(name: string, spec: table) → nil` | Define inline theme |

### `si.config` (existing)

Direct field assignment: `si.config.scrolloff = 8`

### `si.plugin`

| Function | Signature | Description | Example |
|---|---|---|---|
| `configure` | `(name: string, opts: table) → nil` | Pass options to a plugin | `si.plugin.configure("formatter", {on_save=true})` |

Plugins access their namespace via `si.plugin.<name>`.

---

## 15. Example Plugins

### 1. Auto-Formatter

Runs a formatter on save, replaces buffer content:

```lua
-- plugins/auto-formatter/init.lua
local M = {}

local formatters = {
    rust = "rustfmt --edition 2021",
    python = "black -q -",
    lua = "stylua -",
    javascript = "prettier --parser babel",
    typescript = "prettier --parser typescript",
}

function M.setup(opts)
    opts = opts or {}
    local user_formatters = opts.formatters or {}

    -- Merge user formatters with defaults
    for lang, cmd in pairs(user_formatters) do
        formatters[lang] = cmd
    end

    si.on("DocumentDidSave", function(event)
        local doc = si.editor.current()
        local lang = doc:language()
        local formatter = formatters[lang]

        if not formatter then return end
        if not doc:path() then return end

        local path = doc:path()

        si.async.spawn(formatter .. " < " .. path, {}, function(result)
            if result.code == 0 and result.stdout ~= "" then
                si.editor.transaction(function()
                    local doc = si.editor.current()
                    -- Select all text and replace with formatted output
                    local all = si.selection.new(1, 1, doc:line_count(), 1)
                    doc:replace(all, result.stdout)
                end)
                si.editor.command("write")
            elseif result.code ~= 0 then
                si.editor.notify("Format failed: " .. result.stderr, "error")
            end
        end)
    end)
end

return M
```

User config:
```lua
-- ~/.config/silicon/init.lua
si.plugin.configure("auto-formatter", {
    formatters = {
        go = "gofmt",
    },
})
```

### 2. Custom Statusline Segment

Shows git branch + diagnostics count:

```lua
-- plugins/statusline-extra/init.lua
local M = {}

-- State persists across events
si.plugin.statusline_extra = {
    branch = "",
    error_count = 0,
    warning_count = 0,
}

function M.setup(opts)
    -- Update git branch periodically
    local function update_branch()
        si.async.spawn("git branch --show-current 2>/dev/null", {}, function(result)
            if result.code == 0 then
                si.plugin.statusline_extra.branch = result.stdout:gsub("%s+$", "")
            end
        end)
    end

    update_branch()
    si.async.interval(5000, update_branch)  -- refresh every 5s

    -- Track diagnostics count
    si.on("DiagnosticsDidChange", function(event)
        local doc = si.editor.current()
        -- Count would come from the diagnostics list
        -- (simplified here — real implementation reads from doc diagnostics)
        si.plugin.statusline_extra.error_count = 0
        si.plugin.statusline_extra.warning_count = 0
    end)

    -- Note: Custom statusline segment rendering requires the statusline
    -- to support plugin-provided segments (future enhancement).
    -- For now, use si.editor.notify or a floating window.
end

return M
```

### 3. File Template

Inserts boilerplate when opening a new file by extension:

```lua
-- plugins/file-template/init.lua
local M = {}

local templates = {
    rs = [[
//! TODO: Module description

]],
    py = [[
#!/usr/bin/env python3
"""TODO: Module description."""


def main():
    pass


if __name__ == "__main__":
    main()
]],
    sh = [[
#!/usr/bin/env bash
set -euo pipefail

]],
}

function M.setup(opts)
    -- Merge user templates
    if opts and opts.templates then
        for ext, tmpl in pairs(opts.templates) do
            templates[ext] = tmpl
        end
    end

    si.on("DocumentDidOpen", function(event)
        local doc = si.editor.current()

        -- Only for new empty files
        if doc:line_count() > 1 then return end
        if doc:get_line(1) ~= "" then return end

        local path = doc:path()
        if not path then return end

        local ext = path:match("%.(%w+)$")
        if not ext then return end

        local template = templates[ext]
        if not template then return end

        si.editor.transaction(function()
            doc:insert(template)
        end)
    end)
end

return M
```

### 4. Lazygit Integration

Opens lazygit in a terminal, closes on exit:

```lua
-- plugins/lazygit/init.lua
local M = {}

function M.setup(opts)
    opts = opts or {}
    local cmd = opts.cmd or "lazygit"
    local key = opts.key or "space g g"

    si.keymap.set("normal", key, function()
        si.terminal.new({ cmd = cmd })
    end)

    -- Optional: reload buffers after lazygit exits
    -- (Would need terminal exit hook — TerminalDidClose event)
    si.on("TerminalDidClose", function(event)
        -- Refresh any open buffers that might have changed
        for _, doc in ipairs(si.editor.documents()) do
            if doc:path() and doc:is_modified() == false then
                -- Reload from disk if unchanged
                si.editor.command("revert", { doc:path() })
            end
        end
    end)
end

return M
```

### 5. Diagnostic Filter

Hides "hint" level diagnostics from LSP:

```lua
-- plugins/diagnostic-filter/init.lua
local M = {}

function M.setup(opts)
    opts = opts or {}
    local min_severity = opts.min_severity or "info"

    local severity_order = {
        hint = 1,
        info = 2,
        warning = 3,
        error = 4,
    }

    local min_level = severity_order[min_severity] or 2

    si.lsp.on_diagnostics(function(diagnostics)
        local filtered = {}
        for _, diag in ipairs(diagnostics) do
            local level = severity_order[diag.severity] or 0
            if level >= min_level then
                table.insert(filtered, diag)
            end
        end
        return filtered
    end)
end

return M
```

User config:
```lua
si.plugin.configure("diagnostic-filter", {
    min_severity = "warning",  -- hide hints and info
})
```

### 6. Custom "Git Mode"

Defines a mode with s/c/p/d keybindings for git operations:

```lua
-- plugins/git-mode/init.lua
local M = {}

function M.setup(opts)
    opts = opts or {}
    local key = opts.key or "space g"

    si.mode.define("git", {
        label = "GIT",
        keymap = {
            s = function()
                local path = si.editor.current():path()
                if path then
                    si.async.spawn("git add " .. path, {}, function(r)
                        if r.code == 0 then
                            si.editor.notify("Staged: " .. path)
                        else
                            si.editor.notify("Stage failed: " .. r.stderr, "error")
                        end
                    end)
                end
            end,

            S = function()
                si.async.spawn("git add -A", {}, function(r)
                    if r.code == 0 then
                        si.editor.notify("Staged all files")
                    else
                        si.editor.notify("Stage all failed: " .. r.stderr, "error")
                    end
                end)
            end,

            c = function()
                si.terminal.new({ cmd = "git commit" })
                si.mode.enter("normal")
            end,

            p = function()
                si.async.spawn("git push", {}, function(r)
                    if r.code == 0 then
                        si.editor.notify("Pushed successfully")
                    else
                        si.editor.notify("Push failed: " .. r.stderr, "error")
                    end
                end)
            end,

            d = function()
                si.async.spawn("git diff --stat", {}, function(r)
                    if r.code == 0 then
                        si.ui.float({
                            title = "Git Diff",
                            content = r.stdout,
                            width = 80,
                            height = 20,
                        })
                    end
                end)
            end,

            l = function()
                si.async.spawn("git log --oneline -20", {}, function(r)
                    if r.code == 0 then
                        si.ui.float({
                            title = "Git Log",
                            content = r.stdout,
                            width = 80,
                            height = 20,
                        })
                    end
                end)
            end,

            q = function()
                si.mode.enter("normal")
            end,

            ["escape"] = function()
                si.mode.enter("normal")
            end,
        },
    })

    -- Enter git mode with space g
    si.keymap.set("normal", key, function()
        si.mode.enter("git")
    end)
end

return M
```

---

## Design Decisions Summary

| Decision | Choice | Rationale |
|---|---|---|
| VM location | `Editor.lua` field | Accessible from all command handlers via `&mut Editor` |
| VM lifetime | Persistent (editor lifetime) | Plugin state must survive events |
| Threading | Single-threaded, main thread only | Matches tokio::select! event loop; no mutex needed |
| Userdata strategy | Lightweight handles (IDs) | Avoids dangling references; looks up real data at call time |
| Editor access from Lua | Light userdata pointer | Same pattern as Neovim; valid for callback duration |
| Line indexing | 1-indexed (Lua convention) | Consistency with Lua ecosystem |
| Error handling | Log + statusline warning | Never crash the editor |
| Custom modes | Keymap overlay on Normal | Avoids modifying Mode enum throughout codebase |
| Plugin isolation | Shared VM, namespaced state | Simpler than separate VMs; plugins can cooperate |
| Async results | Channel → event loop → Lua callback | Keeps Lua single-threaded |
| Instruction limit | Per-callback (1M) | Prevents infinite loops without global limit |
| UI contributions | Callback to Compositor | Compositor not accessible from Lua directly |
| LSP middleware | Ordered pipeline (FIFO) | Predictable composition |
| Plugin distribution | Git-based | Simple, no custom registry needed |
