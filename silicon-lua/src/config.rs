use mlua::{Lua, Value};
use silicon_view::editor::Config as EditorConfig;

use crate::error::LuaConfigError;
use crate::state::get_config_table;

/// Try to get a typed value from a Lua table, returning `None` for nil/missing keys.
fn get_opt<T: mlua::FromLua>(lua: &Lua, table: &mlua::Table, key: &str) -> Option<T> {
    match table.get::<Value>(key) {
        Ok(Value::Nil) | Err(_) => None,
        Ok(val) => T::from_lua(val, lua).ok(),
    }
}

/// Extract editor configuration from `si.config` table.
///
/// Missing keys silently keep defaults from `EditorConfig::default()`.
pub fn extract_editor_config(lua: &Lua) -> Result<EditorConfig, LuaConfigError> {
    let config = get_config_table(lua)?;
    let mut editor = EditorConfig::default();

    if let Some(v) = get_opt::<usize>(lua, &config, "scrolloff") {
        editor.scrolloff = v;
    }
    if let Some(v) = get_opt::<bool>(lua, &config, "mouse") {
        editor.mouse = v;
    }
    if let Some(v) = get_opt::<bool>(lua, &config, "cursorline") {
        editor.cursorline = v;
    }
    if let Some(v) = get_opt::<bool>(lua, &config, "auto_format") {
        editor.auto_format = v;
    }

    Ok(editor)
}
