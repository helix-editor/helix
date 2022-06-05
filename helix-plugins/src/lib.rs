mod generated;

use anyhow::{anyhow, Error, Result};
use crossterm::event::Event;
use helix_loader::plugin_dir;
use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;
use std::{fs::DirEntry, io::Read};
use wasmer::{Instance, Module, Store, Value};
use wasmer_wasi::{Pipe, WasiEnv, WasiState};

use generated::messages::KeyEvent;
use protobuf::Message;

/// References
/// - https://zellij.dev/documentation/plugin-rust.html
/// - https://github.com/zellij-org/zellij/blob/main/zellij-server/src/wasm_vm.rs
/// - https://github.com/wasmerio/wasmer/blob/master/examples/wasi_pipes.rs

#[derive(Debug)]
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

    pub async fn next_plugin_request(&mut self) -> Option<String> {
        for plugin in &mut self.plugins {
            if let Ok(message) = plugin.read_string() {
                if !message.is_empty() {
                    log::info!(
                        "Received message from plugin '{}': {}",
                        plugin.name,
                        message
                    );
                    return Some(message);
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct Plugin {
    name: String,
    wasi_env: WasiEnv,
    instance: Instance,
}

impl TryFrom<Result<DirEntry, std::io::Error>> for Plugin {
    type Error = Error;

    fn try_from(value: Result<DirEntry, std::io::Error>) -> Result<Self, Self::Error> {
        let (name, wasm_bytes) = Plugin::get_plugin_name_and_bytes(value)?;

        let store = Store::default();

        let module = Module::new(&store, wasm_bytes)?;

        let mut wasi_env = WasiState::new(name.clone())
            .stdout(Box::new(Pipe::new()))
            .stderr(Box::new(Pipe::new()))
            .stdin(Box::new(Pipe::new()))
            .finalize()?;

        let import_object = wasi_env.import_object(&module)?;

        let instance = Instance::new(&module, &import_object)?;

        Ok(Plugin {
            name,
            wasi_env,
            instance,
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
        let dealloc_func = self.instance.exports.get_function("deallocate")?;
        dealloc_func.call(&[Value::I32(addr as i32), Value::I32(bytes.len() as i32)])?;
        Ok(())
    }

    fn call_handle_event_func(&mut self, addr: u32, len: usize) -> Result<(), Error> {
        let func = self.instance.exports.get_function("handle_event")?;
        func.call(&[Value::I32(addr as i32), Value::I32(len as i32)])?;
        Ok(())
    }

    fn copy_data_to_memory(&mut self, bytes: &Vec<u8>, addr: u32) -> Result<(), Error> {
        let memory = self.instance.exports.get_memory("memory")?;

        Ok(unsafe {
            let dst_slice: &mut [u8] = memory.data_unchecked_mut();
            for (idx, val) in bytes.iter().enumerate() {
                dst_slice[(addr as usize + idx) as usize] = *val;
            }
        })
    }

    fn allocate_memory(&mut self, bytes: &Vec<u8>) -> Result<u32, Error> {
        let alloc_func = self.instance.exports.get_function("allocate")?;
        let values: Vec<wasmer::Val> = alloc_func.call(&[Value::I32(bytes.len() as i32)])?.to_vec();
        let addr: u32 = match values[0] {
            Value::I32(val) => val as u32,
            _ => return Err(anyhow!("Invalid return from 'allocate' function")),
        };

        Ok(addr)
    }

    fn read_string(&mut self) -> Result<String> {
        let mut buf = String::new();
        self.read_to_string(&mut buf)?;

        Ok(buf)
    }
}

impl Read for Plugin {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut state = self.wasi_env.state();
        let stdout = state
            .fs
            .stdout_mut()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let wasi_stdout = stdout.as_mut().ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to call stdout.as_mut",
        ))?;

        wasi_stdout.read(buf)
    }
}

impl Write for Plugin {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut state = self.wasi_env.state();
        let stdin = state
            .fs
            .stdin_mut()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let wasi_stdin = stdin.as_mut().ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to call stdin.as_mut",
        ))?;

        wasi_stdin.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
