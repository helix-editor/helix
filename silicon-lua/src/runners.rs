use std::collections::HashMap;

use mlua::Lua;

use crate::error::LuaConfigError;

/// Extract user-defined runners from `si.runners` table.
///
/// Each key is a file extension (e.g. `"py"`, `"rs"`), and each value is a
/// command template string with `{file}`, `{name}`, `{dir}` placeholders.
pub fn extract_runners(lua: &Lua) -> Result<HashMap<String, String>, LuaConfigError> {
    let si: mlua::Table = lua.globals().get("si")?;
    let runners: mlua::Table = si.get("runners")?;

    let mut map = HashMap::new();
    for pair in runners.pairs::<String, String>() {
        let (ext, cmd) = pair?;
        map.insert(ext, cmd);
    }
    Ok(map)
}
