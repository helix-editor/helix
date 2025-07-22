use crate::{compositor::Context, config::Config};

use anyhow::Error;
use libloading::Library;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

enum PluginInstance {}
type InitFn = fn(&Config) -> &'static mut PluginInstance;
type ConfigUpdatedFn = fn(&mut PluginInstance, &Config);
type AvailableCmdsFn = fn(&mut PluginInstance) -> Vec<String>;
type CmdFn = fn(&mut PluginInstance, &mut Context, &str);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginInfo {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub lib_source: String,
    pub build_command: String,
    pub min_helix_version: Option<String>,
    pub dependencies: Option<Vec<String>>,
}

struct Plugin {
    info: PluginInfo,
    lib: Library,
    instance: &'static mut PluginInstance,
}

static PLUGINS: Mutex<Option<HashMap<String, Plugin>>> = Mutex::new(None);
static PLUGINS_CONFIG: Mutex<Option<toml::Table>> = Mutex::new(None);

pub struct Plugins {}

impl Plugins {
    // Utils for plugin developers

    pub fn alloc<T>() -> &'static mut T {
        let layout = std::alloc::Layout::new::<T>();
        unsafe {
            let t = std::alloc::alloc(layout) as *mut T;
            t.as_mut().unwrap()
        }
    }

    pub fn get_config_for(id: &str) -> Option<toml::Table> {
        if let Some(plugins_config) = PLUGINS_CONFIG.lock().unwrap().as_ref() {
            if let Some(config) = plugins_config.get(id) {
                return Some(config.as_table().unwrap().to_owned());
            }
        }
        None
    }

    // Internal plugin API

    pub fn reconfigure(config: &Config) -> Result<(), Error> {
        let plugin_config_path = helix_loader::config_dir().join("plugins.toml");
        let plugin_config_str = std::fs::read_to_string(plugin_config_path)?;
        let plugin_config: toml::Table = toml::from_str(&plugin_config_str)?;

        let sources = plugin_config.get("sources").unwrap().as_array().unwrap();
        let enabled = plugin_config.get("enabled").unwrap().as_array().unwrap();
        let enabled: Vec<String> = enabled
            .iter()
            .map(|e| e.as_str().unwrap().to_owned())
            .collect();

        for source in sources {
            // TODO: HTTP req or git fetch or etc. if necessary
            let cwd = Path::new(source.as_str().unwrap());
            let source = Path::join(cwd, "hx.toml");
            let source = std::fs::read_to_string(source).unwrap();

            let source: toml::Table = toml::from_str(&source).unwrap();
            for (id, plugin_info) in source {
                if enabled.contains(&id) {
                    Self::load(cwd, id, plugin_info.try_into().unwrap(), config);
                }
            }
        }

        Ok(())
    }

    fn load(cwd: &Path, id: String, info: PluginInfo, config: &Config) {
        let mut plugins_opt = PLUGINS.lock().unwrap();
        if plugins_opt.is_none() {
            plugins_opt.replace(HashMap::new());
        }
        let plugins = plugins_opt.as_mut().unwrap();

        // TODO: do nothing if this version is too low
        // if (VERSION_AND_GIT_HASH < info.min_helix_version) {
        // 	return;
        // }

        // load the dynamic lib
        let lib_path = cwd.join(&info.lib_source);
        let lib = unsafe { Library::new(lib_path).unwrap() };

        // call the plugin's init method (if it has one)
        let func_opt = unsafe { lib.get::<InitFn>(b"init") };
        let instance = func_opt.unwrap()(config);

        plugins.insert(
            id,
            Plugin {
                info,
                lib,
                instance,
            },
        );
    }

    pub fn available_commands() -> Vec<String> {
        let mut commands = Vec::<String>::new();
        if let Some(plugins) = PLUGINS.lock().unwrap().as_mut() {
            for plugin in plugins.values_mut() {
                let func_opt = unsafe { plugin.lib.get::<AvailableCmdsFn>(b"available_commands") };
                if let Ok(func) = func_opt {
                    let plugin_commands = func(plugin.instance);
                    for command in plugin_commands {
                        commands.push(command.to_string());
                    }
                }
            }
        }
        commands
    }

    pub fn call_typed_command(cx: &mut Context, name: &str, args: &str) -> bool {
        if let Some(plugins) = PLUGINS.lock().unwrap().as_mut() {
            for plugin in plugins.values_mut() {
                let func_opt = unsafe { plugin.lib.get::<CmdFn>(name.as_bytes()) };
                if let Ok(func) = func_opt {
                    func(plugin.instance, cx, args);
                    return true;
                }
            }
        }
        false
    }
}
