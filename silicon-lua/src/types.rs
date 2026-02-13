use mlua::{Lua, Result, Table, Value};
use toml::Value as TomlValue;

/// Try to get a typed value from a Lua table, returning `None` for nil/missing keys.
///
/// **CRITICAL**: Do NOT use `table.get::<T>(key)` directly for optional fields.
/// In mlua, `get::<bool>()` on a nil key returns `Ok(false)`, silently
/// overwriting defaults. Always use this helper instead.
pub fn get_opt<T: mlua::FromLua>(lua: &Lua, table: &Table, key: &str) -> Option<T> {
    match table.get::<Value>(key) {
        Ok(Value::Nil) | Err(_) => None,
        Ok(val) => T::from_lua(val, lua).ok(),
    }
}

/// Get an optional char from a Lua string (first character).
pub fn get_opt_char(lua: &Lua, table: &Table, key: &str) -> Option<char> {
    get_opt::<String>(lua, table, key).and_then(|s| s.chars().next())
}

/// Convert a Lua sequence table to `Vec<String>`.
pub fn table_to_string_vec(table: &Table) -> Result<Vec<String>> {
    let mut vec = Vec::new();
    for value in table.sequence_values::<String>() {
        vec.push(value?);
    }
    Ok(vec)
}

/// Convert a Lua sequence table to `Vec<u16>`.
pub fn table_to_u16_vec(table: &Table) -> Result<Vec<u16>> {
    let mut vec = Vec::new();
    for value in table.sequence_values::<u16>() {
        vec.push(value?);
    }
    Ok(vec)
}

/// Convert a Lua sequence table to `Vec<PathBuf>`.
pub fn table_to_pathbuf_vec(table: &Table) -> Result<Vec<std::path::PathBuf>> {
    Ok(table_to_string_vec(table)?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Check if a Lua table is a sequence (array-like) vs a map (dict-like).
/// A table is considered a sequence if its first key is the integer 1.
pub fn is_sequence(table: &Table) -> bool {
    // If the table has a key `1` (integer), treat it as a sequence.
    matches!(table.get::<Value>(1i64), Ok(v) if v != Value::Nil)
}

fn snake_to_kebab(s: &str) -> String {
    s.replace('_', "-")
}

/// Convert a Lua Value to a `toml::Value` without modifying keys.
/// Suitable for pass-through config (e.g., LSP `config` subtable) and reuse in migration.
pub fn lua_to_toml(value: &Value) -> Result<TomlValue> {
    lua_to_toml_inner(value, false)
}

/// Convert a Lua Value to a `toml::Value`, converting `snake_case` keys to `kebab-case`.
/// Used for Silicon's own language config fields that use `#[serde(rename_all = "kebab-case")]`.
/// The `config` key (LSP pass-through) is excluded from conversion.
pub fn lua_to_toml_kebab(value: &Value) -> Result<TomlValue> {
    lua_to_toml_inner(value, true)
}

fn lua_to_toml_inner(value: &Value, kebab_keys: bool) -> Result<TomlValue> {
    match value {
        Value::Boolean(b) => Ok(TomlValue::Boolean(*b)),
        Value::Integer(i) => Ok(TomlValue::Integer(*i)),
        Value::Number(n) => Ok(TomlValue::Float(*n)),
        Value::String(s) => Ok(TomlValue::String(s.to_str()?.to_string())),
        Value::Table(t) => {
            if is_sequence(t) {
                let mut arr = Vec::new();
                for v in t.sequence_values::<Value>() {
                    arr.push(lua_to_toml_inner(&v?, kebab_keys)?);
                }
                Ok(TomlValue::Array(arr))
            } else {
                let mut map = toml::map::Map::new();
                for pair in t.pairs::<String, Value>() {
                    let (k, v) = pair?;
                    // The "config" subtable is passed through to LSP servers as-is.
                    let is_config_key = k == "config";
                    let key = if kebab_keys && !is_config_key {
                        snake_to_kebab(&k)
                    } else {
                        k
                    };
                    let child_kebab = kebab_keys && !is_config_key;
                    map.insert(key, lua_to_toml_inner(&v, child_kebab)?);
                }
                Ok(TomlValue::Table(map))
            }
        }
        Value::Nil => Ok(TomlValue::String(String::new())),
        _ => Err(mlua::Error::RuntimeError(
            "cannot convert Lua value to TOML (functions, userdata not supported)".into(),
        )),
    }
}
