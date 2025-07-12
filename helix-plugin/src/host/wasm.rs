use anyhow::Result;
use std::path::PathBuf;
use wasmtime::*;
use crate::api::HelixApi;

pub struct WasmHost {
    engine: Engine,
    store: Store<HelixApi>, // Agora o Store armazena a HelixApi
    instance: Instance,
}

impl WasmHost {
    pub fn new(wasm_file: &PathBuf, helix_api: HelixApi) -> Result<Self> {
        let engine = Engine::default();
        let mut store = Store::new(&engine, helix_api); // Passa a HelixApi para o Store

        // Compila o módulo
        let module = Module::from_file(&engine, wasm_file)?;

        // Cria um linker para vincular funções do host (API do Helix)
        let mut linker = Linker::new(&engine);

        // Linkar a função `show_message` da HelixApi
        linker.func_wrap(
            "helix_api",
            "show_message",
            |mut caller: Caller<HelixApi>, ptr: i32, len: i32| {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return Err(Trap::new("failed to find host memory")), // Erro se a memória não for encontrada
                };
                let data = mem.data(&caller).get(ptr as usize..ptr as usize + len as usize)
                    .and_then(|arr| std::str::from_utf8(arr).ok())
                    .ok_or_else(|| Trap::new("failed to read string from WASM memory"))?;

                // Acessa a HelixApi do estado do Store
                let helix_api = caller.data();
                helix_api.show_message(data.to_string())
                    .map_err(|e| Trap::new(format!("Host error: {}", e)))?;
                Ok(())
            },
        )?;

        // Linkar a função `register_command` da HelixApi
        linker.func_wrap(
            "helix_api",
            "register_command",
            |mut caller: Caller<HelixApi>, name_ptr: i32, name_len: i32, callback_ptr: i32, callback_len: i32| {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(mem)) => mem,
                    _ => return Err(Trap::new("failed to find host memory")), // Erro se a memória não for encontrada
                };

                let name = mem.data(&caller).get(name_ptr as usize..name_ptr as usize + name_len as usize)
                    .and_then(|arr| std::str::from_utf8(arr).ok())
                    .ok_or_else(|| Trap::new("failed to read command name from WASM memory"))?;

                let callback = mem.data(&caller).get(callback_ptr as usize..callback_ptr as usize + callback_len as usize)
                    .and_then(|arr| std::str::from_utf8(arr).ok())
                    .ok_or_else(|| Trap::new("failed to read callback name from WASM memory"))?;

                let helix_api = caller.data();
                helix_api.register_command(name.to_string(), callback.to_string())
                    .map_err(|e| Trap::new(format!("Host error: {}", e)))?;
                Ok(())
            },
        )?;

        linker.func_wrap(
            "helix_api",
            "get_buffer_content",
            |mut caller: Caller<HelixApi>, doc_id: u32, request_id: u32| {
                let helix_api = caller.data();
                log::info!("WASM plugin called get_buffer_content for doc_id: {} with request_id: {}", doc_id, request_id);
                helix_api.get_buffer_content(doc_id, request_id)
                    .map_err(|e| Trap::new(format!("Host error: {}", e)))?;
                Ok(())
            },
        )?;

        // Instancia o módulo
        let instance = linker.instantiate(&mut store, &module)?;

        Ok(Self {
            engine,
            store,
            instance,
        })
    }

    /// Chama uma função exportada no módulo WASM, passando argumentos.
    pub fn call_function(&mut self, name: &str, args: &[String]) -> Result<()> {
        let func = self.instance.get_func(&mut self.store, name)
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export function: {}", name))?;

        // Obter funções de alocação/desalocação de memória do módulo WASM
        let allocate = self.instance.get_func(&mut self.store, "allocate")
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export `allocate` function"))?;
        let deallocate = self.instance.get_func(&mut self.store, "deallocate")
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export `deallocate` function"))?;

        // Serializar argumentos para uma única string JSON (ou outro formato)
        let args_json = serde_json::to_string(args)?;

        // Alocar memória no WASM para a string de argumentos
        let args_ptr_len = allocate.call(&mut self.store, &[args_json.len().into()])?;
        let args_ptr = args_ptr_len[0].i32().unwrap();

        // Escrever a string de argumentos na memória WASM
        let memory = self.instance.get_memory(&mut self.store, "memory")
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export `memory`"))?;
        memory.write(&mut self.store, args_ptr as usize, args_json.as_bytes())?;

        // Chamar a função WASM com o ponteiro e o tamanho dos argumentos
        func.call(&mut self.store, &[args_ptr.into(), (args_json.len() as i32).into()], &mut [])?;

        // Liberar a memória alocada no WASM
        deallocate.call(&mut self.store, &[args_ptr.into(), (args_json.len() as i32).into()])?;

        Ok(())
    }

    /// Chama a função `on_response` no módulo WASM.
    pub fn on_response(&mut self, request_id: u32, response_data: String) -> Result<()> {
        let func = self.instance.get_func(&mut self.store, "on_response")
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export `on_response` function"))?;

        // Obter funções de alocação/desalocação de memória do módulo WASM
        let allocate = self.instance.get_func(&mut self.store, "allocate")
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export `allocate` function"))?;
        let deallocate = self.instance.get_func(&mut self.store, "deallocate")
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export `deallocate` function"))?;

        // Alocar memória no WASM para a string de resposta
        let response_ptr_len = allocate.call(&mut self.store, &[response_data.len().into()])?;
        let response_ptr = response_ptr_len[0].i32().unwrap();

        // Escrever a string de resposta na memória WASM
        let memory = self.instance.get_memory(&mut self.store, "memory")
            .ok_or_else(|| anyhow::anyhow!("Wasm module does not export `memory`"))?;
        memory.write(&mut self.store, response_ptr as usize, response_data.as_bytes())?;

        // Chamar a função WASM com o request_id, ponteiro e tamanho da resposta
        func.call(&mut self.store, &[request_id.into(), response_ptr.into(), (response_data.len() as i32).into()], &mut [])?;

        // Liberar a memória alocada no WASM
        deallocate.call(&mut self.store, &[response_ptr.into(), (response_data.len() as i32).into()])?;

        Ok(())
    }
}