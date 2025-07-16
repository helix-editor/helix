use crate::{config::PluginManifest, host::{wasm::WasmHost, lua::LuaHost}, api::HelixApi};
use std::path::PathBuf;
use walkdir::WalkDir;
use anyhow::Result;
use std::collections::HashMap;

pub enum PluginHost {
    Wasm(WasmHost),
    Lua(LuaHost),
}

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub host: PluginHost,
}

#[derive(Debug)]
pub struct PluginManager {
    pub loaded_plugins: Vec<LoadedPlugin>,
    api_sender: tokio::sync::mpsc::UnboundedSender<helix_view::editor::EditorEvent>,
    // Mapeia o nome do comando para (nome da função de callback, índice do plugin)
    registered_commands: HashMap<String, (String, usize)>,
    next_request_id: u32,
    pending_requests: HashMap<u32, (usize, String)>, // request_id -> (plugin_idx, callback_fn_name)
}

impl PluginManager {
    pub fn new(api_sender: tokio::sync::mpsc::UnboundedSender<helix_view::editor::EditorEvent>) -> Self {
        Self { loaded_plugins: vec![], api_sender, registered_commands: HashMap::new(), next_request_id: 0, pending_requests: HashMap::new() }
    }

    pub fn discover_plugins_in(&mut self, directory: &PathBuf) -> Result<()> {
        if !directory.is_dir() {
            return Ok(()); // Não é um erro se o diretório não existir
        }

        for entry in WalkDir::new(directory).min_depth(1).max_depth(2).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name().to_str() == Some("plugin.toml") {
                let path = entry.path().to_path_buf();
                match PluginManifest::load_from(&path) {
                    Ok(manifest) => {
                        let plugin_dir = path.parent().unwrap();
                        let entrypoint_path = plugin_dir.join(&manifest.entrypoint);

                        if !entrypoint_path.exists() {
                            log::warn!("Plugin entrypoint not found for '{}': {:?}", manifest.name, entrypoint_path);
                            continue;
                        }

                        let plugin_idx = self.loaded_plugins.len();
                        let helix_api = HelixApi::new(self.api_sender.clone(), plugin_idx);

                        let host = if entrypoint_path.extension().map_or(false, |ext| ext == "wasm") {
                            match WasmHost::new(&entrypoint_path, helix_api) {
                                Ok(mut host) => {
                                    // Chamar a função de inicialização do plugin
                                    if let Err(e) = host.call_function("on_load", &[]) {
                                        log::error!("Error calling on_load for plugin '{}': {}", manifest.name, e);
                                    }
                                    PluginHost::Wasm(host)
                                }
                                Err(e) => {
                                    log::error!("Failed to load wasm host for plugin '{}': {}", manifest.name, e);
                                    continue;
                                }
                            }
                        } else if entrypoint_path.extension().map_or(false, |ext| ext == "lua") {
                            match LuaHost::new(&entrypoint_path, helix_api) {
                                Ok(mut host) => {
                                    // Chamar a função de inicialização do plugin
                                    if let Err(e) = host.call_function("on_load", &[]) {
                                        log::error!("Error calling on_load for plugin '{}': {}", manifest.name, e);
                                    }
                                    PluginHost::Lua(host)
                                }
                                Err(e) => {
                                    log::error!("Failed to load lua host for plugin '{}': {}", manifest.name, e);
                                    continue;
                                }
                            }
                        } else {
                            log::warn!("Unsupported plugin entrypoint type for '{}': {:?}", manifest.name, entrypoint_path);
                            continue;
                        };

                        log::info!("Successfully loaded plugin '{}'", manifest.name);

                        self.loaded_plugins.push(LoadedPlugin { manifest, host });

                        // Registrar comandos do plugin (se houver)
                        // Isso será feito de forma mais genérica depois.
                        // Por enquanto, apenas para o exemplo de teste.
                        if self.loaded_plugins[plugin_idx].manifest.name == "my-first-plugin" {
                            self.registered_commands.insert(
                                "my-plugin:test-command".to_string(),
                                ("on_saudacao_command".to_string(), plugin_idx),
                            );
                        }
                    }
                    Err(e) => log::error!("Failed to load plugin manifest from {:?}: {}", path, e),
                }
            }
        }

        Ok(())
    }

    pub fn execute_command(&mut self, name: &str, args: &[String]) {
        if let Some((callback_fn_name, plugin_idx)) = self.registered_commands.get(name) {
            let plugin = &mut self.loaded_plugins[*plugin_idx];
            match &mut plugin.host {
                PluginHost::Wasm(host) => {
                    if let Err(e) = host.call_function(callback_fn_name, args) {
                        log::error!("Error executing WASM plugin command '{}': {}", name, e);
                    }
                }
                PluginHost::Lua(host) => {
                    if let Err(e) = host.call_function(callback_fn_name, args) {
                        log::error!("Error executing Lua plugin command '{}': {}", name, e);
                    }
                }
            }
        } else {
            log::warn!("Plugin command not found: {}", name);
        }
    }

    pub fn register_command(&mut self, command_name: String, callback_function_name: String, plugin_idx: usize) {
        self.registered_commands.insert(command_name, (callback_function_name, plugin_idx));
        log::info!("Registered plugin command: {}", command_name);
    }

    pub fn handle_plugin_response(&mut self, request_id: u32, response_data: String) {
        if let Some((plugin_idx, callback_fn_name)) = self.pending_requests.remove(&request_id) {
            let plugin = &mut self.loaded_plugins[plugin_idx];
            match &mut plugin.host {
                PluginHost::Wasm(host) => {
                    if let Err(e) = host.on_response(request_id, response_data.clone()) {
                        log::error!("Error executing WASM plugin response callback '{}': {}", callback_fn_name, e);
                    }
                }
                PluginHost::Lua(host) => {
                    if let Err(e) = host.on_response(request_id, response_data.clone()) {
                        log::error!("Error executing Lua plugin response callback '{}': {}", callback_fn_name, e);
                    }
                }
            }
        } else {
            log::warn!("Received response for unknown request_id: {}", request_id);
        }
    }

    pub fn subscribe_to_event(&mut self, event_name: String, callback_function_name: String, plugin_idx: usize) {
        self.event_subscribers.entry(event_name).or_default().push((plugin_idx, callback_function_name));
        log::info!("Plugin {} subscribed to event '{}'".to_string(), plugin_idx, event_name);
    }

    pub fn get_next_request_id(&mut self) -> u32 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }

    pub fn add_pending_request(&mut self, request_id: u32, plugin_idx: usize, callback_fn_name: String) {
        self.pending_requests.insert(request_id, (plugin_idx, callback_fn_name));
    }

    pub fn dispatch_event(&mut self, event_name: &str, event_data: &str) {
        if let Some(subscribers) = self.event_subscribers.get(event_name) {
            for (plugin_idx, callback_fn_name) in subscribers.clone() {
                let plugin = &mut self.loaded_plugins[plugin_idx];
                match &mut plugin.host {
                    PluginHost::Wasm(host) => {
                        if let Err(e) = host.call_function(&callback_fn_name, &[event_data.to_string()]) {
                            log::error!("Error executing WASM plugin event callback '{}': {}", callback_fn_name, e);
                        }
                    }
                    PluginHost::Lua(host) => {
                        if let Err(e) = host.call_function(&callback_fn_name, &[event_data.to_string()]) {
                            log::error!("Error executing Lua plugin event callback '{}': {}", callback_fn_name, e);
                        }
                    }
                }
            }
        }
    }
}
}