use mlua::{Lua, Table, Value};
use silicon_view::{document::Mode, input::KeyEvent};
use std::collections::HashMap;

use crate::types::{get_opt, is_sequence};

/// Intermediate representation of a keybinding, independent of `silicon-term` types.
///
/// `silicon-lua` cannot depend on `silicon-term` (circular dependency), so we define
/// this enum here. `silicon-term` converts it to `KeyTrie` / `MappableCommand` at load time.
#[derive(Debug, Clone)]
pub enum KeyBinding {
    /// A single command name, e.g. `"move_line_down"`.
    Command(String),
    /// A sequence of command names, e.g. `["select_all", "yank"]`.
    Sequence(Vec<String>),
    /// A submenu node with labeled children.
    Node {
        label: String,
        is_sticky: bool,
        map: HashMap<KeyEvent, KeyBinding>,
        order: Vec<KeyEvent>,
    },
}

/// Register the `si.keymap.set()` and `si.keymap.set_many()` functions.
///
/// Internally stores bindings in a Lua registry table `si_keybindings` with
/// sub-tables for each mode: `{ normal = {}, insert = {}, select = {} }`.
pub fn register_keymap_api(lua: &Lua, keymap_table: &Table) -> mlua::Result<()> {
    // Create the registry table for accumulating keybindings.
    let registry = lua.create_table()?;
    registry.set("normal", lua.create_table()?)?;
    registry.set("insert", lua.create_table()?)?;
    registry.set("select", lua.create_table()?)?;
    lua.set_named_registry_value("si_keybindings", registry)?;

    // si.keymap.set(mode, key, action)
    let set_fn = lua.create_function(keymap_set)?;
    keymap_table.set("set", set_fn)?;

    // si.keymap.set_many(mode, mappings)
    let set_many_fn = lua.create_function(keymap_set_many)?;
    keymap_table.set("set_many", set_many_fn)?;

    Ok(())
}

/// `si.keymap.set(mode, key, action)` — store a single keybinding.
fn keymap_set(lua: &Lua, (mode, key, action): (String, String, Value)) -> mlua::Result<()> {
    validate_mode(&mode)?;
    let registry: Table = lua.named_registry_value("si_keybindings")?;
    let mode_table: Table = registry.get(mode.as_str())?;
    mode_table.set(key, action)?;
    Ok(())
}

/// `si.keymap.set_many(mode, mappings)` — store multiple keybindings at once.
fn keymap_set_many(lua: &Lua, (mode, mappings): (String, Table)) -> mlua::Result<()> {
    validate_mode(&mode)?;
    let registry: Table = lua.named_registry_value("si_keybindings")?;
    let mode_table: Table = registry.get(mode.as_str())?;
    for pair in mappings.pairs::<String, Value>() {
        let (key, action) = pair?;
        mode_table.set(key, action)?;
    }
    Ok(())
}

/// Validate that a mode string is one of "normal", "insert", "select".
fn validate_mode(mode: &str) -> mlua::Result<()> {
    match mode {
        "normal" | "insert" | "select" => Ok(()),
        _ => Err(mlua::Error::runtime(format!(
            "invalid mode '{}': expected 'normal', 'insert', or 'select'",
            mode
        ))),
    }
}

/// Extract accumulated keybindings from the Lua registry after script execution.
///
/// Returns a map from `Mode` to a `KeyBinding::Node` for each mode that has bindings.
pub fn extract_keybindings(lua: &Lua) -> mlua::Result<HashMap<Mode, KeyBinding>> {
    let registry: Table = lua.named_registry_value("si_keybindings")?;
    let mut result = HashMap::new();

    for (mode_str, mode) in [
        ("normal", Mode::Normal),
        ("insert", Mode::Insert),
        ("select", Mode::Select),
    ] {
        let mode_table: Table = registry.get(mode_str)?;
        // Check if the mode table has any entries using pairs().
        // Do NOT use table.len() — it only counts sequence (integer) keys.
        if mode_table.pairs::<Value, Value>().next().is_some() {
            let binding = lua_table_to_node(lua, &mode_table, &format!("{} mode", mode_str))?;
            result.insert(mode, binding);
        }
    }

    Ok(result)
}

/// Convert a Lua table to a `KeyBinding::Node`.
///
/// Iterates all key-value pairs, skipping metadata keys ("label", "is_sticky").
/// Each key is parsed as a `KeyEvent`, each value is converted recursively.
fn lua_table_to_node(lua: &Lua, table: &Table, default_label: &str) -> mlua::Result<KeyBinding> {
    let label: String = get_opt::<String>(lua, table, "label").unwrap_or_else(|| default_label.to_string());
    let is_sticky: bool = get_opt::<bool>(lua, table, "is_sticky").unwrap_or(false);

    let mut map = HashMap::new();
    let mut order = Vec::new();

    for pair in table.pairs::<Value, Value>() {
        let (k, v) = pair?;

        // Skip metadata keys.
        if let Value::String(ref s) = k {
            let key_str = s.to_str()?.to_string();
            if key_str == "label" || key_str == "is_sticky" {
                continue;
            }
        }

        // Skip integer keys — those are sequence entries, not key bindings.
        if let Value::Integer(_) = k {
            continue;
        }

        let key_str = match k {
            Value::String(s) => s.to_str()?.to_string(),
            _ => {
                return Err(mlua::Error::runtime(format!(
                    "keymap key must be a string, got {:?}",
                    k
                )));
            }
        };

        let key_event: KeyEvent = key_str
            .parse()
            .map_err(|e| mlua::Error::runtime(format!("{e}")))?;

        let binding = lua_value_to_binding(lua, v)?;
        map.insert(key_event, binding);
        order.push(key_event);
    }

    Ok(KeyBinding::Node {
        label,
        is_sticky,
        map,
        order,
    })
}

/// Convert a Lua value to a `KeyBinding`.
///
/// - String → `KeyBinding::Command`
/// - Sequence table (array) → `KeyBinding::Sequence`
/// - Map table → `KeyBinding::Node` (recurse)
fn lua_value_to_binding(lua: &Lua, value: Value) -> mlua::Result<KeyBinding> {
    match value {
        Value::String(s) => Ok(KeyBinding::Command(s.to_str()?.to_string())),
        Value::Table(t) => {
            if is_sequence_table(&t) {
                // It's a sequence of commands.
                let mut commands = Vec::new();
                for val in t.sequence_values::<String>() {
                    commands.push(val?);
                }
                Ok(KeyBinding::Sequence(commands))
            } else {
                // It's a submenu node.
                lua_table_to_node(lua, &t, "")
            }
        }
        _ => Err(mlua::Error::runtime(format!(
            "keymap action must be a string or table, got {:?}",
            value.type_name()
        ))),
    }
}

/// Check if a Lua table is a sequence (array of commands) rather than a map (submenu).
///
/// A table is a sequence if it has integer key 1 AND does not have a "label" key
/// (which would indicate a submenu node).
fn is_sequence_table(table: &Table) -> bool {
    if !is_sequence(table) {
        return false;
    }
    // If it has a "label" key, it's a node, not a sequence.
    !matches!(table.get::<Value>("label"), Ok(v) if v != Value::Nil)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::create_lua_state;
    use silicon_view::input::{KeyCode, KeyModifiers};

    /// Helper: create Lua state, run source, extract keybindings.
    fn run_and_extract(source: &str) -> HashMap<Mode, KeyBinding> {
        let lua = create_lua_state().unwrap();
        lua.load(source).exec().unwrap();
        extract_keybindings(&lua).unwrap()
    }

    #[test]
    fn simple_command_binding() {
        let keys = run_and_extract(r#"si.keymap.set("normal", "j", "move_line_down")"#);
        assert!(keys.contains_key(&Mode::Normal));
        if let KeyBinding::Node { map, .. } = &keys[&Mode::Normal] {
            let j = KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            };
            assert!(map.contains_key(&j));
            assert!(matches!(&map[&j], KeyBinding::Command(name) if name == "move_line_down"));
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn sequence_binding() {
        let keys = run_and_extract(
            r#"si.keymap.set("normal", "Q", {"select_all", "yank"})"#,
        );
        if let KeyBinding::Node { map, .. } = &keys[&Mode::Normal] {
            let q: KeyEvent = "Q".parse().unwrap();
            assert!(matches!(&map[&q], KeyBinding::Sequence(cmds) if cmds == &["select_all", "yank"]));
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn submenu_binding() {
        let source = r#"
            si.keymap.set("normal", "g", {
                label = "goto",
                d = "goto_definition",
                r = "goto_reference",
            })
        "#;
        let keys = run_and_extract(source);
        if let KeyBinding::Node { map, .. } = &keys[&Mode::Normal] {
            let g = KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::NONE,
            };
            if let KeyBinding::Node { label, map: sub, .. } = &map[&g] {
                assert_eq!(label, "goto");
                let d = KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::NONE,
                };
                assert!(matches!(&sub[&d], KeyBinding::Command(name) if name == "goto_definition"));
            } else {
                panic!("Expected Node for submenu");
            }
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn nested_submenu() {
        let source = r#"
            si.keymap.set("normal", "space", {
                label = "space",
                f = {
                    label = "file",
                    f = "file_picker",
                    s = "save_selection",
                },
            })
        "#;
        let keys = run_and_extract(source);
        if let KeyBinding::Node { map, .. } = &keys[&Mode::Normal] {
            let space: KeyEvent = "space".parse().unwrap();
            if let KeyBinding::Node { map: sub, .. } = &map[&space] {
                let f = KeyEvent {
                    code: KeyCode::Char('f'),
                    modifiers: KeyModifiers::NONE,
                };
                if let KeyBinding::Node { label, map: sub2, .. } = &sub[&f] {
                    assert_eq!(label, "file");
                    assert!(matches!(&sub2[&f], KeyBinding::Command(name) if name == "file_picker"));
                } else {
                    panic!("Expected nested Node");
                }
            } else {
                panic!("Expected Node for space");
            }
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn set_many_multiple_keys() {
        let source = r#"
            si.keymap.set_many("normal", {
                j = "move_line_down",
                k = "move_line_up",
            })
        "#;
        let keys = run_and_extract(source);
        if let KeyBinding::Node { map, .. } = &keys[&Mode::Normal] {
            assert_eq!(map.len(), 2);
            let j = KeyEvent { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
            let k = KeyEvent { code: KeyCode::Char('k'), modifiers: KeyModifiers::NONE };
            assert!(matches!(&map[&j], KeyBinding::Command(name) if name == "move_line_down"));
            assert!(matches!(&map[&k], KeyBinding::Command(name) if name == "move_line_up"));
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn modifier_key_parsing() {
        let keys = run_and_extract(r#"si.keymap.set("normal", "C-a", "select_all")"#);
        if let KeyBinding::Node { map, .. } = &keys[&Mode::Normal] {
            let ctrl_a = KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            };
            assert!(map.contains_key(&ctrl_a));
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn invalid_mode_errors() {
        let lua = create_lua_state().unwrap();
        let result = lua
            .load(r#"si.keymap.set("bogus", "j", "move_line_down")"#)
            .exec();
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("invalid mode"), "got: {err}");
    }

    #[test]
    fn empty_config_no_keys() {
        let keys = run_and_extract("");
        assert!(keys.is_empty());
    }

    #[test]
    fn is_sticky_flag() {
        let source = r#"
            si.keymap.set("normal", "z", {
                label = "view",
                is_sticky = true,
                j = "scroll_down",
                k = "scroll_up",
            })
        "#;
        let keys = run_and_extract(source);
        if let KeyBinding::Node { map, .. } = &keys[&Mode::Normal] {
            let z = KeyEvent { code: KeyCode::Char('z'), modifiers: KeyModifiers::NONE };
            if let KeyBinding::Node { is_sticky, .. } = &map[&z] {
                assert!(is_sticky);
            } else {
                panic!("Expected Node for z");
            }
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn multiple_modes() {
        let source = r#"
            si.keymap.set("normal", "j", "move_line_down")
            si.keymap.set("insert", "C-w", "delete_word_backward")
        "#;
        let keys = run_and_extract(source);
        assert!(keys.contains_key(&Mode::Normal));
        assert!(keys.contains_key(&Mode::Insert));
        assert!(!keys.contains_key(&Mode::Select));
    }
}
