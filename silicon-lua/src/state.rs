use mlua::{Lua, LuaOptions, StdLib, Table};

use crate::error::LuaConfigError;

/// Maximum memory the Lua VM can allocate (64 MiB).
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

/// Maximum number of Lua instructions before we abort (guards against infinite loops).
const INSTRUCTION_LIMIT: u32 = 1_000_000;

/// Create a sandboxed Lua 5.4 VM with the `si` global table pre-registered.
pub fn create_lua_state() -> Result<Lua, LuaConfigError> {
    let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default())?;

    let _ = lua.set_memory_limit(MEMORY_LIMIT);

    // Instruction-count hook to catch infinite loops.
    lua.set_hook(
        mlua::HookTriggers::new().every_nth_instruction(INSTRUCTION_LIMIT),
        |_lua, _debug| {
            Err(mlua::Error::runtime(
                "instruction limit exceeded (possible infinite loop)",
            ))
        },
    )?;

    // Build the `si` global table.
    {
        let si = lua.create_table()?;

        // si.config — empty table; user sets fields, missing fields keep Rust defaults.
        let config = lua.create_table()?;
        si.set("config", config)?;

        // si.runners — empty table; user sets extension → command template pairs.
        let runners = lua.create_table()?;
        si.set("runners", runners)?;

        // si.keymap — set() and set_many() for keybinding configuration.
        let keymap = lua.create_table()?;
        crate::keymap::register_keymap_api(&lua, &keymap)?;
        si.set("keymap", keymap)?;

        // si.theme — set(), adaptive(), define() for theme configuration.
        let theme = lua.create_table()?;
        crate::theme::register_theme_api(&lua, &theme)?;
        si.set("theme", theme)?;

        // si.language() and si.language_server() — language config builders.
        crate::languages::register_language_api(&lua)?;
        let language_fn =
            lua.create_function(crate::languages::language_builder)?;
        let language_server_fn =
            lua.create_function(crate::languages::language_server_builder)?;
        si.set("language", language_fn)?;
        si.set("language_server", language_server_fn)?;

        // Runtime constants.
        si.set("platform", std::env::consts::OS)?;
        si.set(
            "config_dir",
            silicon_loader::config_dir()
                .to_string_lossy()
                .to_string(),
        )?;
        if let Ok(home) = std::env::var("HOME") {
            si.set("home_dir", home)?;
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            si.set("home_dir", home)?;
        }
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            si.set("hostname", hostname)?;
        } else if let Ok(output) = std::process::Command::new("hostname").output() {
            if let Ok(h) = String::from_utf8(output.stdout) {
                si.set("hostname", h.trim().to_string())?;
            }
        }

        lua.globals().set("si", si)?;
    }

    // Add ~/.config/silicon/lua/ to package.path so users can `require("mymodule")`.
    let lua_dir = silicon_loader::config_dir().join("lua");
    let lua_dir_str = lua_dir.to_string_lossy().replace('\\', "\\\\");
    lua.load(format!(
        r#"package.path = package.path .. ";{0}/?.lua;{0}/?/init.lua""#,
        lua_dir_str
    ))
    .exec()?;

    Ok(lua)
}

/// Get the `si.config` table from a Lua state.
pub fn get_config_table(lua: &Lua) -> Result<Table, LuaConfigError> {
    let si: Table = lua.globals().get("si")?;
    let config: Table = si.get("config")?;
    Ok(config)
}
