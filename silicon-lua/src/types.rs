use mlua::{Lua, Result, Table, Value};

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
