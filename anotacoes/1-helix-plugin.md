# Design Proposal: Plugin Architecture for the Helix Editor

This document describes an architecture for a robust, secure, and high-performance plugin system for the Helix editor. The proposal aims to allow the community to extend the editor's functionalities while maintaining the project's stability and philosophy.

## 1. Philosophy and Goals

The plugin system must adhere to the same principles as Helix:

*   **Security:** Plugin code should never be able to crash or corrupt the editor. Plugins must operate within a strict sandbox, with controlled access to system resources and the editor's state.
*   **Performance:** Loading and executing plugins should have a minimal impact on startup time and editing latency. The use of high-performance JIT runtimes is essential.
*   **Ergonomics:** Creating plugins should be a pleasant experience. The API should be well-documented, idiomatic, and powerful, with a fast development cycle.
*   **Flexibility:** The architecture must support both simple scripts for quick automation and complex, high-performance plugins written in compiled languages.

## 2. Proposed Architecture

The solution is based on a new central crate, `helix-plugin`, which orchestrates the discovery, loading, and execution of plugins. We propose support for two types of plugins to cover different use cases:

1.  **WebAssembly (WASM)-based Plugins:** For complex and high-performance functionalities. It allows developers to use Rust, C++, Go, Zig, etc., compiling to `wasm32-wasi`. The WASM runtime provides a first-class security sandbox.
2.  **Lua-based Plugins:** For configurations, automations, and rapid prototyping. Lua is a lightweight, fast, and easy-to-embed scripting language in Rust, ideal for simpler tasks.

### Main Components

#### a. `helix-plugin` Crate

This new crate will be the heart of the system.

*   **Responsibilities:**
    *   Manage the complete lifecycle of plugins (discovery, loading, reloading, unloading).
    *   Contain the runtimes for WASM (`wasmtime`) and Lua (`mlua`).
    *   Expose a secure and stable host API for plugins.
    *   Act as a bridge between the Helix core and the plugins.

#### b. Plugin Manifest (`plugin.toml`)

Each plugin must include a manifest file for metadata and configuration.

*   **Location:** `~/.config/helix/plugins/my-plugin/plugin.toml`
*   **Example Structure:**
    ```toml
    # Basic plugin metadata.
    name = "my-awesome-plugin"
    version = "0.1.0"
    authors = ["Your Name <your@email.com>"]
    description = "A plugin that does something awesome."

    # The entry point for the plugin's code.
    # The system will determine the plugin type by the extension.
    entrypoint = "main.wasm" # or "main.lua"

    # (Optional) Defines when the plugin should be loaded to save resources.
    # If omitted, the plugin is loaded on startup.
    [activation]
    on_command = ["my-plugin:my-action"] # Loads when a specific command is called.
    on_language = ["rust", "toml"]        # Loads when a file of a language is opened.
    on_event = ["buffer_save"]            # Loads on specific editor events.
    ```

#### c. The Plugin API (`helix::api`)

This is the contact surface between a plugin and the editor. It will be a secure facade over the internal Helix crates (`helix-core`, `helix-view`, etc.), ensuring that no plugin can access the internal state in a dangerous way.

*   **API Modules (Examples):**
    *   `helix::api::editor`: Functions to interact with buffers, selections, and the general state.
        *   `get_buffer_content(buf_id) -> Result<String, Error>`
        *   `get_selections(view_id) -> Result<Vec<Range>, Error>`
        *   `apply_transaction(view_id, transaction)`
    *   `helix::api::commands`: To register new commands in the editor.
        *   `register(name, callback)`
    *   `helix::api::events`: To react to editor events.
        *   `subscribe(event_name, callback)`
    *   `helix::api::ui`: To interact with the Helix user interface.
        *   `show_picker(items, callback)`
        *   `show_message(level, text)`

## 3. Implementation Requirements

#### a. New Crate: `helix-plugin`

*   **Module Structure:**
    *   `lib.rs`: Crate entry point.
    *   `manager.rs`: `PluginManager`, responsible for the lifecycle.
    *   `api.rs`: Definition of the public host API.
    *   `host/mod.rs`, `host/wasm.rs`, `host/lua.rs`: Implementations of the WASM and Lua runtimes.
    *   `config.rs`: Logic to read and interpret `plugin.toml`.

#### b. Cargo Dependencies (Cargo.toml)

*   **WASM Runtime:**
    *   `wasmtime`: JIT runtime for `wasm32-wasi`.
*   **Lua Runtime:**
    *   `mlua`: High-level safe bindings for Lua.
*   **Utilities:**
    *   `serde` & `serde_json`: For data serialization between host and plugins.
    *   `toml`: To parse `plugin.toml` manifests.
    *   `walkdir`: For efficient discovery of plugins in the file system.
    *   `anyhow`: For cleaner and more ergonomic error handling.

#### c. Modifications to Existing Code

*   **`Cargo.toml` (Workspace):**
    *   Add `helix-plugin` to the `members` list.

*   **`helix-term` (Main Binary):**
    *   **Initialization:** Instantiate and initialize the `PluginManager` in `main.rs`.
    *   **Event Loop:** Integrate the `PluginManager` into the main event loop to dispatch events (keys, commands, etc.) to the plugins.

*   **`helix-event`:**
    *   Allow the `PluginManager` to register as a "listener" for global editor events.

*   **`helix-core` and `helix-view`:**
    *   **API Exposure (Secure Facade):** This is the most critical part. Instead of making internal functions `pub`, we will create facade functions in `helix::api` that perform validations and expose only the necessary functionality. This prevents a malformed or malicious plugin from causing a `panic` or corrupting the editor's state.

*   **Command Dispatcher (`helix-view/src/commands.rs`):**
    *   Modify the dispatcher so that if a command is not found internally, it queries the `PluginManager` to check if a plugin has registered the command.

## 4. Suggested Implementation Roadmap

1.  **Phase 1: Foundation and Basic WASM**
    *   Create the `helix-plugin` crate and add the dependencies (`wasmtime`, `toml`, `walkdir`).
    *   Implement plugin discovery and `plugin.toml` parsing.
    *   Implement a basic WASM host that can load and execute a `.wasm` file.
    *   Define a minimal API: `helix::api::commands::register` and `helix::api::ui::show_message`.
    *   Create a "hello world" plugin in Rust (compiled to WASM) for testing.

2.  **Phase 2: Integration with the Core**
    *   Integrate the `PluginManager` into `helix-term` and the event loop.
    *   Modify the command dispatcher to call plugin commands.
    *   Expand the API with read access to the editor's state (e.g., `get_buffer_content`).

3.  **Phase 3: Lua Host and API Expansion**
    *   Add the `mlua` dependency and implement the `LuaHost`.
    *   Expose the same `helix::api` to the Lua environment.
    *   Expand the API with write/modification functionalities (e.g., `apply_transaction`), ensuring that all operations are safe and reversible (undo/redo).
    *   Create an example plugin in Lua.

4.  **Phase 4: Documentation and Ecosystem**
    *   Write the documentation for plugin developers in the Helix `book/`.
    *   Document the entire `helix::api` in detail.
    *   Create project templates for plugins in Rust and Lua.
    *   Develop some useful plugins and include them in the `runtime/plugins` directory.

## 5. Plugin Developer Experience

#### Example: "Hello World" Plugin (Rust/WASM)

```rust
// In the plugin (lib.rs)
use helix::api;

// The `helix::plugin` macro would handle the boilerplate of WASM export.
#[helix::plugin]
fn on_load() {
    api::commands::register("hello-plugin", |args| {
        api::ui::show_message(api::ui::Level::Info, "Hello, from my plugin!");
    });
}
```

#### Example: "Hello World" Plugin (Lua)

```lua
-- In the plugin (main.lua)
local editor = helix.api.editor
local ui = helix.api.ui

helix.api.commands.register("hello-lua", function(args)
    ui.show_message("info", "Hello, from my Lua plugin!")
end)
```

This architecture provides a solid foundation for a thriving plugin ecosystem, empowering users to adapt Helix to their needs while maintaining the project's quality and performance standards.