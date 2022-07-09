use anyhow::{anyhow, Error, Result};
use crossterm::event::Event;
use helix_loader::plugin_dir;
use std::ffi::OsStr;
use std::fs::DirEntry;
use std::path::PathBuf;
use wasmtime::{AsContextMut, Caller, Engine, Extern, Instance, Linker, Module, Store};
use wasmtime_wasi::{add_to_linker, WasiCtx, WasiCtxBuilder};

/// References
/// - https://zellij.dev/documentation/plugin-rust.html
/// - https://github.com/zellij-org/zellij/blob/main/zellij-server/src/wasm_vm.rs
/// - https://docs.wasmtime.dev/examples-rust-wasi.html

pub struct PluginManager {
    plugins: Vec<Plugin>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: Vec::new(),
        }
    }

    pub fn load_plugins(&mut self) -> Result<()> {
        log::info!("Loading plugins");

        let plugin_dir = plugin_dir();
        std::fs::create_dir_all(plugin_dir.clone())?;

        let entries = plugin_dir.read_dir()?;
        for entry in entries {
            match Plugin::try_from(entry) {
                Ok(plugin) => {
                    log::info!("Loaded plugin '{}'", plugin.name);
                    self.plugins.push(plugin);
                }
                Err(e) => log::error!("Failed to load plugin at entry: {}", e),
            }
        }

        log::info!("Finished loading plugins");

        Ok(())
    }

    pub fn start_plugins(&mut self) {
        log::info!("Starting plugins");
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.start() {
                log::error!("Failed to start plugin '{}': {}", plugin.name, e);
            }
        }
        log::info!("Finished starting plugins");
    }

    pub fn handle_term_event(&mut self, term_event: Event) {
        for plugin in &mut self.plugins {
            let result = match term_event {
                Event::Key(event) => {
                    let event_json = serde_json::to_string(&event).unwrap();
                    plugin.on_key_press(event_json)
                }
                Event::Mouse(event) => {
                    let event_json = serde_json::to_string(&event).unwrap();
                    plugin.on_mouse_event(event_json)
                }
                Event::Resize(cols, rows) => plugin.on_resize(cols, rows),
            };

            if let Err(e) = result {
                log::error!(
                    "Plugin '{}' failed to handle term event: {}",
                    plugin.name,
                    e
                )
            }
        }
    }
}

pub struct Plugin {
    name: String,
    instance: Instance,
    store: Store<WasiCtx>,
}

impl TryFrom<Result<DirEntry, std::io::Error>> for Plugin {
    type Error = Error;

    fn try_from(value: Result<DirEntry, std::io::Error>) -> Result<Self, Self::Error> {
        let (name, wasm_bytes) = Plugin::get_plugin_name_and_bytes(value)?;

        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        add_to_linker(&mut linker, |s| s)?;

        let wasi = WasiCtxBuilder::new().build();
        let mut store = Store::new(&engine, wasi);
        let module = Module::new(&engine, wasm_bytes)?;

        linker.func_wrap(
            "helix",
            "log",
            |mut caller: Caller<_>, ptr: u32, len: u32| {
                let memory_export = match caller.get_export("memory") {
                    Some(memory_export) => memory_export,
                    None => {
                        log::warn!("Failed to get memory for plugin");
                        return;
                    }
                };

                let memory = match memory_export {
                    Extern::Memory(memory) => memory,
                    _ => {
                        log::warn!("Plugin memory export was wrong type");
                        return;
                    }
                };

                let store = caller.as_context_mut();
                let data = memory.data_mut(store);
                let start = ptr as usize;
                let end = (ptr + len) as usize;
                let bytes = &mut data[start..end];

                let text = unsafe {
                    String::from_raw_parts(bytes.as_mut_ptr(), len as usize, len as usize)
                };

                log::info!("(Plugin) info: {}", text);

                // String memory is free'd by the plugin
                std::mem::forget(text);
            },
        )?;

        linker.module(&mut store, "", &module)?;

        let instance = linker.instantiate(&mut store, &module)?;

        Ok(Plugin {
            name,
            instance,
            store,
        })
    }
}

impl Plugin {
    fn start(&mut self) -> Result<()> {
        self.on_start()?;

        Ok(())
    }

    fn get_plugin_name_and_bytes(
        entry: Result<DirEntry, std::io::Error>,
    ) -> Result<(String, Vec<u8>)> {
        let entry: DirEntry = entry?;

        let path: PathBuf = entry.path();
        let extension: &OsStr = path.extension().ok_or_else(|| {
            Error::msg(format!(
                "Plugin at path '{}' did not have an extension",
                path.to_string_lossy()
            ))
        })?;

        if extension != "wasm" {
            return Err(Error::msg(format!(
                "Plugin at path '{}' did not have 'wasm' extension",
                path.to_string_lossy()
            )));
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let wasm_bytes = std::fs::read(entry.path())?;

        Ok((name, wasm_bytes))
    }

    fn on_key_press(&mut self, key_press_event: String) -> Result<()> {
        let bytes = key_press_event.into_bytes();
        let len = bytes.len();
        let addr = self.allocate_memory(&bytes)?;

        self.copy_data_to_memory(&bytes, addr)?;

        let on_key_press = self
            .instance
            .get_typed_func::<(u32, u32), (), _>(&mut self.store, "on_key_press")?;

        let params = (addr as u32, len as u32);
        on_key_press.call(&mut self.store, params)?;

        self.deallocate_memory(addr, bytes)?;

        Ok(())
    }

    fn on_mouse_event(&mut self, mouse_event: String) -> Result<()> {
        let bytes = mouse_event.into_bytes();
        let len = bytes.len();
        let addr = self.allocate_memory(&bytes)?;

        self.copy_data_to_memory(&bytes, addr)?;

        let on_key_press = self
            .instance
            .get_typed_func::<(u32, u32), (), _>(&mut self.store, "on_mouse_event")?;

        let params = (addr as u32, len as u32);
        on_key_press.call(&mut self.store, params)?;

        self.deallocate_memory(addr, bytes)?;

        Ok(())
    }

    fn on_start(&mut self) -> Result<()> {
        let on_start = self
            .instance
            .get_typed_func::<(), (), _>(&mut self.store, "on_start")?;
        on_start.call(&mut self.store, ())?;

        Ok(())
    }

    fn on_resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        let on_resize_fn = self
            .instance
            .get_typed_func::<(u32, u32), (), _>(&mut self.store, "on_resize")?;
        let params = (cols as u32, rows as u32);
        on_resize_fn.call(&mut self.store, params)?;
        Ok(())
    }

    fn deallocate_memory(&mut self, addr: u32, bytes: Vec<u8>) -> Result<(), Error> {
        let dealloc_func = self
            .instance
            .get_typed_func::<(u32, u32), (), _>(&mut self.store, "deallocate")?;
        let params = (addr, bytes.len() as u32);
        dealloc_func.call(&mut self.store, params)?;
        Ok(())
    }

    fn copy_data_to_memory(&mut self, bytes: &[u8], addr: u32) -> Result<(), Error> {
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .ok_or_else(|| anyhow!("Failed to get memory"))?;

        let dst_slice: &mut [u8] = memory.data_mut(&mut self.store);
        for (idx, val) in bytes.iter().enumerate() {
            dst_slice[(addr as usize + idx) as usize] = *val;
        }

        Ok(())
    }

    fn allocate_memory(&mut self, bytes: &Vec<u8>) -> Result<u32, Error> {
        let alloc_func = self
            .instance
            .get_typed_func::<u32, u32, _>(&mut self.store, "allocate")?;
        let addr: u32 = alloc_func.call(&mut self.store, bytes.len() as u32)?;

        Ok(addr)
    }
}
