#[rustfmt::skip]
#[allow(clippy::all)]
mod generated;

use anyhow::{anyhow, Error, Result};
use crossterm::event::Event;
use helix_loader::plugin_dir;
use std::ffi::OsStr;
use std::fs::DirEntry;
use std::path::PathBuf;
use wasmtime::{Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::{add_to_linker, WasiCtx, WasiCtxBuilder};

use generated::messages::KeyEvent;
use protobuf::Message;

/// References
/// - https://zellij.dev/documentation/plugin-rust.html
/// - https://github.com/zellij-org/zellij/blob/main/zellij-server/src/wasm_vm.rs
/// - https://docs.wasmtime.dev/examples-rust-wasi.html

pub struct PluginManager {
    plugins: Vec<Plugin>,
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
        if let Event::Key(key) = term_event {
            let mut key_event = KeyEvent::new();
            key_event.code = format!("{:?}", key.code);
            key_event.mod_shift = key
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT);
            key_event.mod_ctrl = key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL);
            key_event.mod_alt = key.modifiers.contains(crossterm::event::KeyModifiers::ALT);
            let bytes = key_event.write_to_bytes().unwrap();

            let mut event = generated::messages::Event::new();
            event.event_type = generated::messages::event::EventType::KEY_EVENT.into();
            event.payload = bytes;

            self.handle_event(event);
        }
    }

    pub fn handle_event(&mut self, event: generated::messages::Event) {
        for plugin in &mut self.plugins {
            match plugin.handle_event(event.clone()) {
                Ok(_) => {
                    log::info!("Plugin '{}' handled event '{}'", plugin.name, event)
                }
                Err(e) => log::error!(
                    "Plugin '{}' failed to handle event '{}': {}",
                    plugin.name,
                    event,
                    e
                ),
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
        let mut event = generated::messages::Event::new();
        event.event_type = generated::messages::event::EventType::PLUGIN_STARTED.into();
        event.payload = Vec::new();

        self.handle_event(event)?;

        Ok(())
    }

    fn get_plugin_name_and_bytes(
        entry: Result<DirEntry, std::io::Error>,
    ) -> Result<(String, Vec<u8>)> {
        let entry: DirEntry = entry?;

        let path: PathBuf = entry.path();
        let extension: &OsStr = path.extension().ok_or(Error::msg(format!(
            "Plugin at path '{}' did not have an extension",
            path.to_string_lossy().to_string()
        )))?;

        if extension != "wasm" {
            return Err(Error::msg(format!(
                "Plugin at path '{}' did not have 'wasm' extension",
                path.to_string_lossy().to_string()
            )));
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let wasm_bytes = std::fs::read(entry.path())?;

        Ok((name, wasm_bytes))
    }

    fn handle_event(&mut self, event: generated::messages::Event) -> Result<()> {
        let bytes = event.write_to_bytes()?;

        let addr = self.allocate_memory(&bytes)?;
        self.copy_data_to_memory(&bytes, addr)?;
        self.call_handle_event_func(addr, bytes.len())?;
        self.deallocate_memory(addr, bytes)?;

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

    fn call_handle_event_func(&mut self, addr: u32, len: usize) -> Result<(), Error> {
        let func = self
            .instance
            .get_typed_func::<(u32, u32), (), _>(&mut self.store, "handle_event")?;
        let params = (addr as u32, len as u32);
        func.call(&mut self.store, params)?;
        Ok(())
    }

    fn copy_data_to_memory(&mut self, bytes: &Vec<u8>, addr: u32) -> Result<(), Error> {
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
