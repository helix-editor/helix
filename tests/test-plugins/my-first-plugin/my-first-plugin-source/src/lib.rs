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

// Function called when the plugin is loaded
#[no_mangle]
pub extern "C" fn on_load() {
    show_message("Meu plugin WASM foi carregado!");
    register_command("meu-plugin:saudacao", "on_saudacao_command");
}

// Function of callback for the registered command
// This function will receive arguments as pointers and lengths
#[no_mangle]
pub extern "C" fn on_saudacao_command(args_ptr: *const u8, args_len: usize) {
    let args_str = unsafe {
        let slice = slice::from_raw_parts(args_ptr, args_len);
        std::str::from_utf8_unchecked(slice)
    };
    show_message(&format!("Olá do comando WASM! Argumentos: {}", args_str));
}
