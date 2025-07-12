use helix_plugin::host::wasm::WasmHost;
use std::path::PathBuf;

#[test]
fn test_load_and_call_wasm_plugin() {
    // Este teste assume que o `main.wasm` existe e foi compilado.
    let wasm_file = PathBuf::from("../tests/test-plugins/my-first-plugin/main.wasm");

    // Se o arquivo não existir, o teste falhará, o que é esperado neste cenário.
    if !wasm_file.exists() {
        panic!("Test WASM file not found at {:?}. Please compile the test plugin first.", wasm_file);
    }

    let mut host = WasmHost::new(&wasm_file).expect("Failed to create WasmHost");

    // Tenta chamar a função `on_load` exportada.
    host.call_function("on_load").expect("Failed to call on_load function");
}
