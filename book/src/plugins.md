# Plugins

Helix offers an extensible plugin system that allows users and developers to add new functionalities and customize the editor. Plugins can be written in WebAssembly (WASM) for high performance or in Lua for rapid prototyping and automation.

## How to Develop Plugins

### Overview

Plugins interact with Helix through a host API (`helix::api`) that exposes editor functionalities in a safe and controlled manner. Each plugin is defined by a `plugin.toml` file that describes its metadata and entry point.

### Structure of a Plugin

A typical plugin consists of:

*   **`plugin.toml`**: The plugin manifest, containing metadata and the path to the entry point (e.g., `main.wasm` or `main.lua`).
*   **Entry Point**: The `.wasm` or `.lua` file that contains the plugin's logic.

### The Helix API for Plugins (`helix::api`)

The `helix::api` is the main interface for the plugin's interaction with the editor. It is exposed to plugins through imported functions (for WASM) or a global object (`helix`) (for Lua).

#### Currently Available Functions:

*   `helix.show_message(message: String)`: Displays a status message in the editor.
*   `helix.register_command(command_name: String, callback_function_name: String)`: Registers a new command in the editor. When this command is executed by the user, the `callback_function_name` function in your plugin will be called.
*   `helix.get_buffer_content(doc_id: u32, request_id: u32)`: Requests the content of a buffer. The response will be sent back to the plugin via `on_response(request_id, data)`.
*   `helix.insert_text(doc_id: u32, position: u32, text: String)`: Inserts text into a buffer at the specified position.
*   `helix.delete_text(doc_id: u32, start: u32, end: u32)`: Deletes text from a buffer within the specified range.
*   `helix.subscribe_to_event(event_name: String, callback_function_name: String)`: Subscribes to an editor event. When the event occurs, the `callback_function_name` function in your plugin will be called with the event data.

### WebAssembly (WASM) Plugins

WASM plugins are compiled from languages like Rust, C++, Go, etc., to the `wasm32-wasi` target. They offer the best performance and security, being executed in an isolated sandbox.

#### Template Example (Rust/WASM):

```rust
// my-plugin/src/lib.rs

use std::slice;
use std::ffi::{CStr, CString};

// Exported function to allocate memory in WASM linear memory
#[no_mangle]
pub extern "C" fn allocate(size: usize) -> *mut u8 {
    let mut buffer = Vec::with_capacity(size);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer); // Prevent Rust from deallocating the memory
    ptr
}

// Exported function to deallocate memory in WASM linear memory
#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, size, size); // Reconstruct Vec to deallocate
    }
}

// Import host functions (defined in helix-plugin/src/host/wasm.rs)
#[link(wasm_import_module = "helix_api")]
extern "C" {
    #[link_name = "show_message"]
    fn host_show_message(ptr: *const u8, len: usize);

    #[link_name = "register_command"]
    fn host_register_command(name_ptr: *const u8, name_len: usize, callback_ptr: *const u8, callback_len: usize);

    #[link_name = "get_buffer_content"]
    fn host_get_buffer_content(doc_id: u32, request_id: u32);

    #[link_name = "insert_text"]
    fn host_insert_text(doc_id: u32, position: u32, text_ptr: *const u8, text_len: usize);

    #[link_name = "delete_text"]
    fn host_delete_text(doc_id: u32, start: u32, end: u32);

    #[link_name = "subscribe_to_event"]
    fn host_subscribe_to_event(event_name_ptr: *const u8, event_name_len: usize, callback_ptr: *const u8, callback_len: usize);
}

// Helper to call show_message
fn show_message(message: &str) {
    unsafe {
        host_show_message(message.as_ptr(), message.len());
    }
}

// Helper to call register_command
fn register_command(command_name: &str, callback_function_name: &str) {
    unsafe {
        host_register_command(
            command_name.as_ptr(), command_name.len(),
            callback_function_name.as_ptr(), callback_function_name.len(),
        );
    }
}

// Helper to call get_buffer_content
fn get_buffer_content(doc_id: u32, request_id: u32) {
    unsafe {
        host_get_buffer_content(doc_id, request_id);
    }
}

// Helper to call insert_text
fn insert_text(doc_id: u32, position: u32, text: &str) {
    unsafe {
        host_insert_text(doc_id, position, text.as_ptr(), text.len());
    }
}

// Helper to call delete_text
fn delete_text(doc_id: u32, start: u32, end: u32) {
    unsafe {
        host_delete_text(doc_id, start, end);
    }
}

// Helper to call subscribe_to_event
fn subscribe_to_event(event_name: &str, callback_function_name: &str) {
    unsafe {
        host_subscribe_to_event(
            event_name.as_ptr(), event_name.len(),
            callback_function_name.as_ptr(), callback_function_name.len(),
        );
    }
}

// Function called when the plugin is loaded
#[no_mangle]
pub extern "C" fn on_load() {
    show_message("My WASM plugin has been loaded!");
    register_command("my-plugin:greeting", "on_greeting_command");
    subscribe_to_event("buffer_save", "on_buffer_save_event");
}

// Callback function for the registered command
#[no_mangle]
pub extern "C" fn on_greeting_command(args_ptr: *const u8, args_len: usize) {
    let args_str = unsafe {
        let slice = slice::from_raw_parts(args_ptr, args_len);
        std::str::from_utf8_unchecked(slice)
    };
    show_message(&format!("Hello from the WASM command! Arguments: {}", args_str));
    get_buffer_content(0, 123); // Example of requesting buffer content
}

// Callback function for editor events
#[no_mangle]
pub extern "C" fn on_buffer_save_event(event_data_ptr: *const u8, event_data_len: usize) {
    let event_data_str = unsafe {
        let slice = slice::from_raw_parts(event_data_ptr, event_data_len);
        std::str::from_utf8_unchecked(slice)
    };
    show_message(&format!("buffer_save event received: {}", event_data_str));
}

// Callback function for request responses
#[no_mangle]
pub extern "C" fn on_response(request_id: u32, response_data_ptr: *const u8, response_data_len: usize) {
    let response_data_str = unsafe {
        let slice = slice::from_raw_parts(response_data_ptr, response_data_len);
        std::str::from_utf8_unchecked(slice)
    };
    show_message(&format!("Response for request {}: {}", request_id, response_data_str));
}
```

### Lua Plugins

Lua plugins are simple scripts that are executed in an embedded Lua environment. They are ideal for automation and quick customization, without the need for compilation.

#### Template Example (Lua):

```lua
-- my-plugin/main.lua

-- The 'helix' object is injected into the global Lua environment
-- and provides access to the Helix API.

-- Function called when the plugin is loaded
function on_load()
    helix.show_message("My Lua plugin has been loaded!")
    helix.register_command("my-plugin:lua-greeting", "on_lua_greeting_command")
    helix.subscribe_to_event("buffer_save", "on_lua_buffer_save_event")
end

-- Callback function for the registered command
function on_lua_greeting_command(...)
    local args = {...}
    local args_str = table.concat(args, ", ")
    helix.show_message("Hello from the Lua command! Arguments: " .. args_str)
    helix.get_buffer_content(0, 456) -- Example of requesting buffer content
end

-- Callback function for editor events
function on_lua_buffer_save_event(event_data)
    helix.show_message("buffer_save event received: " .. event_data)
end

-- Callback function for request responses
function on_response(request_id, response_data)
    helix.show_message("Response for request " .. request_id .. ": " .. response_data)
end
```

## Installing Plugins

To install a plugin, create a directory for it inside `~/.config/helix/plugins/` (or the Helix configuration directory on your operating system) and place the `plugin.toml` and the entry point file (`.wasm` or `.lua`) there.

Example:

```
~/.config/helix/plugins/
└── my-plugin/
    ├── plugin.toml
    └── main.wasm  # or main.lua
```

After installation, restart Helix for the plugin to be discovered and loaded.

## Next Steps

*   Expansion of the `helix::api` with more editor functionalities.
*   Mechanisms for passing arguments and returning structured values between the host and plugins.
*   Support for editor events for plugins.
*   Tools for debugging plugins.
