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

        // si.keymap — stub table with set() and set_many() (Phase 3).
        let keymap = lua.create_table()?;
        let stub_set = lua.create_function(|_, _args: mlua::MultiValue| Ok(()))?;
        let stub_set_many = lua.create_function(|_, _args: mlua::MultiValue| Ok(()))?;
        keymap.set("set", stub_set)?;
        keymap.set("set_many", stub_set_many)?;
        si.set("keymap", keymap)?;

        // si.theme — stub table with set(), adaptive(), define() (Phase 4).
        let theme = lua.create_table()?;
        let stub_theme_set = lua.create_function(|_, _args: mlua::MultiValue| Ok(()))?;
        let stub_theme_adaptive = lua.create_function(|_, _args: mlua::MultiValue| Ok(()))?;
        let stub_theme_define = lua.create_function(|_, _args: mlua::MultiValue| Ok(()))?;
        theme.set("set", stub_theme_set)?;
        theme.set("adaptive", stub_theme_adaptive)?;
        theme.set("define", stub_theme_define)?;
        si.set("theme", theme)?;

        // si.language() and si.language_server() — stubs (Phase 5).
        let stub_language = lua.create_function(|_, _args: mlua::MultiValue| Ok(()))?;
        let stub_language_server = lua.create_function(|_, _args: mlua::MultiValue| Ok(()))?;
        si.set("language", stub_language)?;
        si.set("language_server", stub_language_server)?;

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
    let lua_dir_str = lua_dir.to_string_lossy();
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
