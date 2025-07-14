# Helix Editor - Plugin Module (WASM & Lua)

This repository is a fork of the [Helix Editor](https://github.com/helix-editor/helix) and contains a proposed implementation for an extensible plugin system. The primary new functionality aims to allow the community to extend and customize the editor through plugins based on **WebAssembly (WASM)** and **Lua**.

The motivation behind this initiative is to introduce a robust, secure, and high-performance extensibility mechanism, aligned with Helix's core philosophy.

## 🚀 Plugin Proposal Overview

The proposed architecture for Helix's plugin system revolves around a new central `crate`, `helix-plugin`, which orchestrates the discovery, loading, and execution of plugins.

### Supported Plugin Types

1.  **WebAssembly (WASM):** For complex and high-performance functionalities. It allows developers to use languages like Rust, C++, Go, Zig (compiling to `wasm32-wasi`), ensuring a first-class security sandbox.
2.  **Lua:** Ideal for configurations, automations, and rapid prototyping, offering a lightweight, fast, and easy-to-embed scripting language in Rust.

### Core Principles of the Plugin System

The plugin system design adheres to the same principles as Helix:

* **Security:** Plugin code should never be able to crash or corrupt the editor. Plugins must operate within a strict sandbox, with controlled access to system resources and the editor's state.
* **Performance:** Loading and executing plugins should have a minimal impact on startup time and editing latency. The use of high-performance JIT runtimes is essential.
* **Ergonomics:** Creating plugins should be a pleasant experience. The API should be well-documented, idiomatic, and powerful, with a fast development cycle.
* **Flexibility:** The architecture must support both simple scripts for quick automation and complex, high-performance plugins written in compiled languages.

### Key Components

* **`helix-plugin` Crate:** This new crate will be the heart of the system, responsible for managing the complete lifecycle of plugins (discovery, loading, reloading, unloading), containing WASM (`wasmtime`) and Lua (`mlua`) runtimes, exposing a secure host API, and acting as a bridge between the Helix core and plugins.
* **Plugin Manifest (`plugin.toml`):** Each plugin must include a manifest file for metadata and configuration, defining its name, version, authors, description, entrypoint (`main.wasm` or `main.lua`), and optional activation conditions (on command, on language, on event).
* **The Plugin API (`helix::api`):** This is the contact surface between a plugin and the editor, a secure facade over internal Helix crates (`helix-core`, `helix-view`), ensuring no dangerous access to internal state. It includes functions like `show_message`, `register_command`, `get_buffer_content`, `insert_text`, `delete_text`, and `subscribe_to_event`.

## 🛠️ Implementation Status (Progress Report)

This proposal and its implementation have been developed in phases, as detailed in the full report:

* **Phase 1: Foundation and Basic Structure:** Involved the creation of the `helix-plugin` crate, adding essential dependencies (`wasmtime`, `toml`, `walkdir`, `serde`, `anyhow`, `mlua`), defining the `plugin.toml` manifest, and implementing initial plugin discovery and basic WASM and Lua hosts.
* **Phase 2: Integration with the Editor Core:** Focused on connecting the plugin system to Helix's main lifecycle, including `PluginManager` initialization in `helix-term`, integration into the `Application` struct, and establishing an event system (`EditorEvent::PluginCommand`, `EditorEvent::RegisterPluginCommand`, `EditorEvent::PluginResponse`, `EditorEvent::PluginEvent`) for communication. The command dispatcher was modified to handle plugin commands dynamically.
* **Phase 3: API Expansion and Full Host Integration:** Enhanced the `HelixApi` with core functions and exposed them to both `WasmHost` and `LuaHost`, enabling plugins to interact richly with the editor. This phase also included robust argument passing and response handling.
* **Phase 4: Documentation and Ecosystem:** Initiated plugin developer documentation (`book/src/plugins.md`) and updated project templates for Rust (WASM) and Lua plugins.
* **Phase 5: Basic Event System:** Implemented mechanisms for plugins to subscribe to and react to specific editor events.
* **Phase 6: Error Handling and Reporting:** Improved error handling and user feedback for plugin failures, utilizing detailed logs and consistent error capturing.

## 🛣️ Suggested Implementation Roadmap (Robust Version)

For a first robust version, we prioritize the following areas:

* **WASM Compilation Environment:** Ensure the Helix development environment can compile Rust plugins for the `wasm32-wasi` target.
* **Robust Bidirectional Communication:** Implement a robust mechanism for host functions (Helix) to return values to plugins (WASM and Lua).
* **Essential API Expansion:** Expand the `HelixApi` to expose the most basic and crucial editor functionalities for plugins (read and write access).
* **Dynamic Command Management:** Finalize the implementation of dynamic command registration and execution, including refined argument passing.
* **Basic Event System:** Implement a mechanism for plugins to subscribe to and react to specific editor events.
* **Error Handling and Reporting:** Improve error handling and user feedback for plugin failures to ensure editor robustness.

## 🤝 How to Contribute and Test

If you are interested in testing this proposal or contributing, please follow these steps (assuming you have already cloned this fork):

1.  **Build Helix with the Plugin Module:**
    ```bash
    # Ensure you have Rust and the wasm32-wasi toolchain installed
    rustup target add wasm32-wasi
    
    # From the root directory of this repository
    cargo build --release
    ```
2.  **Install an Example Plugin:**
    Create a directory for the plugin inside `~/.config/helix/plugins/` (or your operating system's Helix configuration directory) and place the `plugin.toml` and the entry point file (`.wasm` or `.lua`) there.
    Example:
    ```
    ~/.config/helix/plugins/
    └── my-plugin/
        ├── plugin.toml
        └── main.wasm  # or main.lua
    ```
    You can find example templates in `helix-plugin/templates/`.
3.  **Run the Modified Helix:**
    ```bash
    /path/to/your/helix/target/release/hx
    ```
4.  **Try Plugin Commands:**
    Inside Helix, you will be able to use commands registered by your plugins. For example, for a Lua greeting plugin: `:my-plugin:lua-greeting`.

## 🔗 Useful Links

* [Official Helix Editor Repository](https://github.com/helix-editor/helix)
* [Helix Documentation](https://docs.helix-editor.com/)
* [Create a Pull Request (GitHub Documentation)](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-with-pull-requests/creating-a-pull-request)

---
*This `README.md` focuses on my plugin module proposal. For the complete Helix documentation, please refer to the original repository.*
