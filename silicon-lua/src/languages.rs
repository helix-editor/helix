use mlua::{Lua, Result, Table, Value};
use toml::Value as TomlValue;

use crate::types::lua_to_toml_kebab;

/// Register `si.language()` and `si.language_server()` APIs.
/// Both accumulate config in Lua registry tables.
pub fn register_language_api(lua: &Lua) -> Result<()> {
    let languages = lua.create_table()?;
    let servers = lua.create_table()?;
    lua.set_named_registry_value("si_languages", &languages)?;
    lua.set_named_registry_value("si_language_servers", &servers)?;
    Ok(())
}

/// `si.language(name, config)` — store a language override in the registry.
pub fn language_builder(lua: &Lua, (name, config): (String, Table)) -> Result<()> {
    let registry: Table = lua.named_registry_value("si_languages")?;
    config.set("name", name.clone())?;
    registry.set(name, config)?;
    Ok(())
}

/// `si.language_server(name, config)` — store a language server definition in the registry.
pub fn language_server_builder(lua: &Lua, (name, config): (String, Table)) -> Result<()> {
    let registry: Table = lua.named_registry_value("si_language_servers")?;
    registry.set(name, config)?;
    Ok(())
}

/// After `init.lua` execution, extract language configs as `toml::Value`
/// for merging with built-in `languages.toml` defaults via `merge_toml_values()`.
///
/// Returns `None` if no language or language server config was defined.
pub fn extract_language_config(lua: &Lua) -> Result<Option<TomlValue>> {
    let languages: Table = lua.named_registry_value("si_languages")?;
    let servers: Table = lua.named_registry_value("si_language_servers")?;

    let has_languages = languages.pairs::<String, Value>().next().is_some();
    let has_servers = servers.pairs::<String, Value>().next().is_some();

    if !has_languages && !has_servers {
        return Ok(None);
    }

    let mut result = toml::map::Map::new();

    // Convert language definitions to TOML array (matches [[language]] in languages.toml).
    if has_languages {
        let mut lang_array = Vec::new();
        for pair in languages.pairs::<String, Value>() {
            let (_name, config) = pair?;
            lang_array.push(lua_to_toml_kebab(&config)?);
        }
        result.insert("language".into(), TomlValue::Array(lang_array));
    }

    // Convert language server definitions to TOML table (matches [language-server] in languages.toml).
    if has_servers {
        let mut server_map = toml::map::Map::new();
        for pair in servers.pairs::<String, Value>() {
            let (name, config) = pair?;
            server_map.insert(name, lua_to_toml_kebab(&config)?);
        }
        result.insert("language-server".into(), TomlValue::Table(server_map));
    }

    Ok(Some(TomlValue::Table(result)))
}
