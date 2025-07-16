### Complete Report: Implementation of the Plugins/Scripts Module for the Helix Editor

**Date:** July 11, 2025

**Objective:** Develop an extensible plugin system for the Helix editor, allowing the community to add new functionalities and customize the editor through plugins based on WebAssembly (WASM) and Lua.

---

#### **1. Phase 1: Foundation and Basic Structure**

**Purpose:** Establish the foundation of the plugin system, including the creation of the new crate, the addition of essential dependencies, and the definition of data structures for plugin discovery and loading.

**Changes and Progress:**

*   **Creation of the `helix-plugin` Crate:**
    *   A new Rust library crate, `helix-plugin`, was created in the Helix project workspace.
    *   **Command:** `cargo new --lib helix-plugin`
    *   **Impact:** Added `helix-plugin/Cargo.toml` and `helix-plugin/src/lib.rs`.
*   **Addition of Initial Dependencies:**
    *   The dependencies `wasmtime`, `toml`, `walkdir`, `serde`, `anyhow`, and `mlua` (with the `lua54` feature) were added to `helix-plugin/Cargo.toml`.
    *   **Command:** `cargo add -p helix-plugin wasmtime toml walkdir serde anyhow mlua --features "lua54"`
    *   **Impact:** Configuration of the development environment for the WASM and Lua hosts, as well as utilities for parsing and file manipulation.
*   **Module Structure:**
    *   Directories `helix-plugin/src/host` and `helix-plugin/tests` were created.
    *   Files `helix-plugin/src/manager.rs`, `helix-plugin/src/config.rs`, `helix-plugin/src/host/wasm.rs`, `helix-plugin/src/host/lua.rs`, and `helix-plugin/src/api.rs` were created.
    *   The modules were declared in `helix-plugin/src/lib.rs` and `helix-plugin/src/host/mod.rs` for code organization.
*   **Definition of the Plugin Manifest (`plugin.toml`):**
    *   In `helix-plugin/src/config.rs`, the `PluginManifest` and `Activation` structs were defined using `serde` for deserialization of TOML files. This allows plugins to declare their metadata and entry points.
*   **Plugin Discovery Logic:**
    *   In `helix-plugin/src/manager.rs`, the `discover_plugins_in` function was implemented. It uses `walkdir` to scan a directory (`~/.config/helix/plugins/`) for `plugin.toml` files, reads them, and parses their contents into `PluginManifest`s.
*   **Initial Hosts (WASM and Lua):**
    *   In `helix-plugin/src/host/wasm.rs`, the `WasmHost` struct was created with the ability to load a `.wasm` file using `wasmtime`.
    *   In `helix-plugin/src/host/lua.rs`, the `LuaHost` struct was created with the ability to load a `.lua` file using `mlua`.
*   **Discovery Tests:**
    *   A test directory `tests/test-plugins/my-first-plugin` was created with an example `plugin.toml`.
    *   An integration test (`helix-plugin/tests/discovery.rs`) was added and successfully executed, confirming that the `PluginManager` can discover and parse plugin manifests.

---

#### **2. Phase 2: Integration with the Editor Core**

**Purpose:** Connect the plugin system to the main lifecycle of Helix, allowing the editor to initialize the plugin manager and for plugin commands to be dispatched.

**Changes and Progress:**

*   **Initialization of the `PluginManager` in `helix-term`:**
    *   In `helix-term/src/main.rs`, the `PluginManager` is now instantiated in `main_impl` before the creation of the `Application`.
    *   An MPSC channel (`tokio::sync::mpsc::unbounded_channel`) is created, and the `sender` is passed to `PluginManager::new`. The `receiver` is later assigned to `editor.editor_events.1`.
*   **Integration into the `Application` Struct:**
    *   In `helix-term/src/application.rs`, the field `plugin_manager: helix_plugin::manager::PluginManager` was added to the `Application` struct.
    *   The `Application::new` constructor was modified to accept the `plugin_manager` as an argument and store it.
*   **Event System for Communication with Plugins:**
    *   In `helix-view/src/editor.rs`, the `EditorEvent` enum was extended with two new variants:
        *   `EditorEvent::PluginCommand(String, Vec<String>, Option<u32>)`: To dispatch plugin commands from `helix-term` to the `Application`, including an optional `request_id` for responses.
        *   `EditorEvent::RegisterPluginCommand(String, String, usize)`: To allow plugins to register commands in the editor, including the command name, the callback function name, and the plugin index.
        *   `EditorEvent::PluginResponse(u32, String)`: To send responses back to the plugins.
        *   `EditorEvent::PluginEvent(String, String)`: To dispatch editor events to the plugins.
    *   A new MPSC channel (`editor_events: (UnboundedSender<EditorEvent>, UnboundedReceiver<EditorEvent>)`) was added to the `Editor` struct to manage event communication.
    *   A method `dispatch_editor_event(&mut self, event: EditorEvent)` was added to the `Editor` to send events through this channel.
    *   The `tokio::select!` in `Editor::wait_event` was updated to listen to the new `editor_events` channel.
*   **Plugin Command Dispatch Mechanism:**
    *   In `helix-term/src/commands.rs`, the `MappableCommand` enum was extended with the variant `Plugin { name: String, args: Vec<String> }`.
    *   The `MappableCommand::from_str` method was modified so that if a command is not found among the static or typable commands, it is interpreted as a `MappableCommand::Plugin`.
    *   The `MappableCommand::name()` and `MappableCommand::doc()` methods were updated to handle the new `Plugin` variant.
    *   The `MappableCommand::execute()` method was modified to dispatch an `EditorEvent::PluginCommand` (using `cx.editor.dispatch_editor_event`) when a `MappableCommand::Plugin` is executed.
*   **Command Execution in the `PluginManager`:**
    *   In `helix-plugin/src/manager.rs`, a method `execute_command(&mut self, name: &str, args: &[String])` was added. This method is responsible for finding the plugin that registered the command and calling the appropriate callback function in the plugin's host, passing the arguments.

---

#### **3. Phase 3: API Expansion and Full Host Integration**

**Purpose:** Enhance the communication API between the editor and plugins, allowing plugins to register commands dynamically and interact with the editor in a richer way.

**Changes and Progress:**

*   **Plugin API (`helix-plugin/src/api.rs`):**
    *   The `HelixApi` struct was enhanced. Now, its `new` constructor receives the `UnboundedSender<EditorEvent>` and a `plugin_idx` (plugin identifier).
    *   The `show_message(message: String)` function was implemented to send an `EditorEvent::PluginCommand` to the editor.
    *   The `register_command(command_name: String, callback_function_name: String)` function was implemented to send an `EditorEvent::RegisterPluginCommand` to the editor, including the `plugin_idx` to identify the source plugin.
    *   The `get_buffer_content(doc_id: u32, request_id: u32)` function was added, sending an `EditorEvent::PluginCommand` with a `request_id` to the editor.
    *   New functions `insert_text(doc_id: u32, position: u32, text: String)` and `delete_text(doc_id: u32, start: u32, end: u32)` were added, sending `EditorEvent::PluginCommand`s to the editor.
    *   The `subscribe_to_event(event_name: String, callback_function_name: String)` function was added, sending an `EditorEvent::PluginCommand` to the editor.
*   **API Integration in `WasmHost` (`helix-plugin/src/host/wasm.rs`):**
    *   The `WasmHost::new` constructor now receives an instance of `HelixApi`.
    *   The `HelixApi` is stored as state (`Store<HelixApi>`) in the `wasmtime::Store`.
    *   The `show_message`, `register_command`, `get_buffer_content`, `insert_text`, `delete_text`, and `subscribe_to_event` functions of the `HelixApi` are exposed to WASM plugins through `linker.func_wrap`, allowing WASM plugins to call these host functions.
    *   The `call_function(&mut self, name: &str, args: &[String])` function was updated to pass arguments to WASM functions, including allocation and deallocation of WASM memory for strings.
*   **API Integration in `LuaHost` (`helix-plugin/src/host/lua.rs`):
    *   The `LuaHost::new` constructor now receives an instance of `HelixApi`.
    *   The `HelixApi` is registered as a `UserData` and exposed as a global object `helix` in the Lua environment. This allows Lua scripts to call `helix.show_message()`, `helix.register_command()`, `helix.get_buffer_content()`, `helix.insert_text()`, `helix.delete_text()`, and `helix.subscribe_to_event()`.
    *   The `call_function(&mut self, name: &str, args: &[String])` method was updated to pass arguments to Lua functions.
*   **Host and Command Management in `PluginManager` (`helix-plugin/src/manager.rs`):**
    *   A `PluginHost` enum was introduced to encapsulate `WasmHost` and `LuaHost`, allowing for generic management of different types of hosts.
    *   The `LoadedPlugin` struct now stores the generic `PluginHost` and the `PluginManifest`.
    *   The `discover_plugins_in` function was updated to:
        *   Create a `HelixApi` for each plugin, associating it with the correct `plugin_idx`.
        *   Instantiate the appropriate `WasmHost` or `LuaHost`, passing the `HelixApi`.
        *   Call the `on_load` function in the plugins (WASM and Lua) after loading, if exported.
        *   A `HashMap<String, (String, usize)>` (`registered_commands`) was added to the `PluginManager` struct to map command names to the callback function name and the loaded plugin's index.
        *   A `next_request_id` and `pending_requests` (`HashMap`) were added to manage asynchronous requests and their responses.
        *   An example command registration (`my-plugin:test-command`) was added for testing purposes.
    *   The `execute_command` function was enhanced to look up the command in `registered_commands` and, if found, call the corresponding callback function in the appropriate `PluginHost`, passing the arguments.
    *   A new method `register_command(&mut self, command_name: String, callback_function_name: String, plugin_idx: usize)` was added to the `PluginManager` to register commands dynamically.
    *   A new method `handle_plugin_response(&mut self, request_id: u32, response_data: String)` was added to process plugin responses.
    *   Methods `get_next_request_id` and `add_pending_request` were added to manage the request/response flow.
*   **Handling of `RegisterPluginCommand` and `PluginResponse` in `Application`:**
    *   In `helix-term/src/application.rs`, `handle_editor_event` was updated to process `EditorEvent::RegisterPluginCommand` and `EditorEvent::PluginResponse`, calling the appropriate methods in the `PluginManager`.
    *   The `Application` now has a `next_request_id` to generate request IDs.

---

#### **4. Phase 4: Documentation and Ecosystem**

**Purpose:** Provide resources for plugin developers and organize the plugin ecosystem.

**Changes and Progress:**

*   **Plugin Documentation:**
    *   The `book/src/plugins.md` file was updated to reflect the new API functions (`insert_text`, `delete_text`, `subscribe_to_event`, `get_buffer_content` with `request_id`) and the `request_id`/`PluginResponse` mechanics.
    *   The plugin examples in Rust (WASM) and Lua were updated to demonstrate passing arguments and using the new API functions.
*   **Project Templates for Plugins:**
    *   The templates in `helix-plugin/templates/rust-wasm-plugin` and `helix-plugin/templates/lua-plugin` were updated to include examples of using the new API functions and the structure for handling arguments and responses.

---

#### **5. Basic Event System (Subscription/Dispatch)**

**Purpose:** Implement a mechanism for plugins to subscribe to and react to specific editor events.

**Changes and Progress:**

*   **`EditorEvent::PluginEvent`:** Added to `helix-view/src/editor.rs` to dispatch editor events to plugins.
*   **`HelixApi::subscribe_to_event`:** Added to `helix-plugin/src/api.rs` to allow plugins to subscribe to events.
*   **Host Integration:** `WasmHost` and `LuaHost` were updated to expose `subscribe_to_event`.
*   **`Application::event_subscribers`:** Added a `HashMap` in `helix-term/src/application.rs` to manage plugins subscribed to events.
*   **`Application::handle_editor_event`:** Updated to dispatch `PluginEvent` to subscribed plugins.

---

#### **6. Error Handling and Reporting (Robustness)**

**Purpose:** Improve error handling and user feedback for plugin failures.

**Changes and Progress:**

*   **Detailed Logs:** `log::error!` and `log::warn!` are used extensively to report plugin loading, execution, and communication failures.
*   **Host Error Capturing:** Errors from `wasmtime` and `mlua` are captured and converted to `anyhow::Error` for consistent handling.

---
