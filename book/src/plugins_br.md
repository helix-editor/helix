# Plugins

O Helix oferece um sistema de plugins extensível que permite aos usuários e desenvolvedores adicionar novas funcionalidades e personalizar o editor. Os plugins podem ser escritos em WebAssembly (WASM) para alto desempenho ou em Lua para prototipagem rápida e automação.

## Como Desenvolver Plugins

### Visão Geral

Os plugins interagem com o Helix através de uma API de host (`helix::api`) que expõe funcionalidades do editor de forma segura e controlada. Cada plugin é definido por um arquivo `plugin.toml` que descreve seus metadados e ponto de entrada.

### Estrutura de um Plugin

Um plugin típico consiste em:

*   **`plugin.toml`**: O manifesto do plugin, contendo metadados e o caminho para o ponto de entrada (e.g., `main.wasm` ou `main.lua`).
*   **Ponto de Entrada**: O arquivo `.wasm` ou `.lua` que contém a lógica do plugin.

### API do Helix para Plugins (`helix::api`)

A `helix::api` é a interface principal para a interação do plugin com o editor. Ela é exposta aos plugins através de funções importadas (para WASM) ou de um objeto global (`helix`) (para Lua).

#### Funções Atualmente Disponíveis:

*   `helix.show_message(message: String)`: Exibe uma mensagem de status no editor.
*   `helix.register_command(command_name: String, callback_function_name: String)`: Registra um novo comando no editor. Quando este comando é executado pelo usuário, a função `callback_function_name` no seu plugin será chamada.
*   `helix.get_buffer_content(doc_id: u32, request_id: u32)`: Solicita o conteúdo de um buffer. A resposta será enviada de volta ao plugin via `on_response(request_id, data)`.
*   `helix.insert_text(doc_id: u32, position: u32, text: String)`: Insere texto em um buffer na posição especificada.
*   `helix.delete_text(doc_id: u32, start: u32, end: u32)`: Deleta texto de um buffer dentro do range especificado.
*   `helix.subscribe_to_event(event_name: String, callback_function_name: String)`: Assina um evento do editor. Quando o evento ocorre, a função `callback_function_name` no seu plugin será chamada com os dados do evento.

### Plugins WebAssembly (WASM)

Plugins WASM são compilados a partir de linguagens como Rust, C++, Go, etc., para o target `wasm32-wasi`. Eles oferecem o melhor desempenho e segurança, sendo executados em um sandbox isolado.

#### Exemplo de Template (Rust/WASM):

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

// Função chamada quando o plugin é carregado
#[no_mangle]
pub extern "C" fn on_load() {
    show_message("Meu plugin WASM foi carregado!");
    register_command("meu-plugin:saudacao", "on_saudacao_command");
    subscribe_to_event("buffer_save", "on_buffer_save_event");
}

// Função de callback para o comando registrado
#[no_mangle]
pub extern "C" fn on_saudacao_command(args_ptr: *const u8, args_len: usize) {
    let args_str = unsafe {
        let slice = slice::from_raw_parts(args_ptr, args_len);
        std::str::from_utf8_unchecked(slice)
    };
    show_message(&format!("Olá do comando WASM! Argumentos: {}", args_str));
    get_buffer_content(0, 123); // Exemplo de solicitação de conteúdo do buffer
}

// Função de callback para eventos do editor
#[no_mangle]
pub extern "C" fn on_buffer_save_event(event_data_ptr: *const u8, event_data_len: usize) {
    let event_data_str = unsafe {
        let slice = slice::from_raw_parts(event_data_ptr, event_data_len);
        std::str::from_utf8_unchecked(slice)
    };
    show_message(&format!("Evento buffer_save recebido: {}", event_data_str));
}

// Função de callback para respostas de solicitações
#[no_mangle]
pub extern "C" fn on_response(request_id: u32, response_data_ptr: *const u8, response_data_len: usize) {
    let response_data_str = unsafe {
        let slice = slice::from_raw_parts(response_data_ptr, response_data_len);
        std::str::from_utf8_unchecked(slice)
    };
    show_message(&format!("Resposta para solicitação {}: {}", request_id, response_data_str));
}
```

### Plugins Lua

Plugins Lua são scripts simples que são executados em um ambiente Lua embarcado. Eles são ideais para automação e personalização rápida, sem a necessidade de compilação.

#### Exemplo de Template (Lua):

```lua
-- my-plugin/main.lua

-- O objeto 'helix' é injetado no ambiente global do Lua
-- e fornece acesso à API do Helix.

-- Função chamada quando o plugin é carregado
function on_load()
    helix.show_message("Meu plugin Lua foi carregado!")
    helix.register_command("meu-plugin:lua-saudacao", "on_lua_saudacao_command")
    helix.subscribe_to_event("buffer_save", "on_lua_buffer_save_event")
end

-- Função de callback para o comando registrado
function on_lua_saudacao_command(...)
    local args = {...}
    local args_str = table.concat(args, ", ")
    helix.show_message("Olá do comando Lua! Argumentos: " .. args_str)
    helix.get_buffer_content(0, 456) -- Exemplo de solicitação de conteúdo do buffer
end

-- Função de callback para eventos do editor
function on_lua_buffer_save_event(event_data)
    helix.show_message("Evento buffer_save recebido: " .. event_data)
end

-- Função de callback para respostas de solicitações
function on_response(request_id, response_data)
    helix.show_message("Resposta para solicitação " .. request_id .. ": " .. response_data)
end
```

## Instalação de Plugins

Para instalar um plugin, crie um diretório para ele dentro de `~/.config/helix/plugins/` (ou o diretório de configuração do Helix no seu sistema operacional) e coloque o `plugin.toml` e o arquivo de ponto de entrada (`.wasm` ou `.lua`) lá.

Exemplo:

```
~/.config/helix/plugins/
└── meu-plugin/
    ├── plugin.toml
    └── main.wasm  # ou main.lua
```

Após a instalação, reinicie o Helix para que o plugin seja descoberto e carregado.

## Próximos Passos

*   Expansão da `helix::api` com mais funcionalidades do editor.
*   Mecanismos para passar argumentos e retornar valores de forma estruturada entre o host e os plugins.
*   Suporte a eventos do editor para plugins.
*   Ferramentas para depuração de plugins.
