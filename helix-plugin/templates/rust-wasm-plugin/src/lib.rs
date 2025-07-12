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
