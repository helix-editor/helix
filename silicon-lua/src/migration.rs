use std::fmt::Write;
use std::path::{Path, PathBuf};

/// Convert a TOML config string to equivalent Lua source.
pub fn config_toml_to_lua(toml_content: &str) -> Result<String, String> {
    let value: toml::Value =
        toml::from_str(toml_content).map_err(|e| format!("Failed to parse TOML: {e}"))?;

    let mut lua = String::new();
    let _ = writeln!(lua, "-- Silicon Editor Configuration");
    let _ = writeln!(lua, "-- Auto-converted from config.toml\n");

    if let Some(theme) = value.get("theme") {
        convert_theme(&mut lua, theme);
        lua.push('\n');
    }

    if let Some(editor) = value.get("editor") {
        convert_editor(&mut lua, editor);
        lua.push('\n');
    }

    if let Some(keys) = value.get("keys") {
        convert_keys(&mut lua, keys);
        lua.push('\n');
    }

    Ok(lua)
}

/// Convert a TOML languages config to Lua.
pub fn languages_toml_to_lua(toml_content: &str) -> Result<String, String> {
    let value: toml::Value =
        toml::from_str(toml_content).map_err(|e| format!("Failed to parse languages TOML: {e}"))?;

    let mut lua = String::new();
    let _ = writeln!(lua, "-- Language configuration");
    let _ = writeln!(lua, "-- Auto-converted from languages.toml\n");

    if let Some(servers) = value.get("language-server") {
        if let Some(table) = servers.as_table() {
            for (name, config) in table {
                convert_language_server(&mut lua, name, config);
            }
            lua.push('\n');
        }
    }

    if let Some(langs) = value.get("language") {
        if let Some(array) = langs.as_array() {
            for lang in array {
                convert_language(&mut lua, lang);
            }
        }
    }

    Ok(lua)
}

/// Run the full migration: convert all TOML configs to init.lua.
/// Returns the path to the generated init.lua.
pub fn run_migration(config_dir: &Path) -> Result<PathBuf, String> {
    let init_lua = config_dir.join("init.lua");
    let mut content = String::new();

    let config_toml = config_dir.join("config.toml");
    if config_toml.exists() {
        let toml_str = std::fs::read_to_string(&config_toml)
            .map_err(|e| format!("Failed to read config.toml: {e}"))?;
        content.push_str(&config_toml_to_lua(&toml_str)?);
    }

    let lang_toml = config_dir.join("languages.toml");
    if lang_toml.exists() {
        let toml_str = std::fs::read_to_string(&lang_toml)
            .map_err(|e| format!("Failed to read languages.toml: {e}"))?;
        content.push_str(&languages_toml_to_lua(&toml_str)?);
    }

    if content.is_empty() {
        content = DEFAULT_INIT_LUA.to_string();
    }

    std::fs::write(&init_lua, &content)
        .map_err(|e| format!("Failed to write init.lua: {e}"))?;

    // Backup originals
    if config_toml.exists() {
        std::fs::rename(&config_toml, config_dir.join("config.toml.bak"))
            .map_err(|e| format!("Failed to backup config.toml: {e}"))?;
    }
    if lang_toml.exists() {
        std::fs::rename(&lang_toml, config_dir.join("languages.toml.bak"))
            .map_err(|e| format!("Failed to backup languages.toml: {e}"))?;
    }

    Ok(init_lua)
}

const DEFAULT_INIT_LUA: &str = "\
-- Silicon Editor Configuration

si.theme.set(\"default\")
si.config.scrolloff = 5
si.config.mouse = true
";

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

fn convert_theme(lua: &mut String, theme: &toml::Value) {
    match theme {
        toml::Value::String(name) => {
            let _ = writeln!(lua, "si.theme.set(\"{}\")", escape_lua_string(name));
        }
        toml::Value::Table(t) => {
            // Adaptive theme
            let light = t
                .get("light")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let dark = t
                .get("dark")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let _ = writeln!(lua, "si.theme.adaptive({{");
            let _ = writeln!(lua, "    light = \"{}\",", escape_lua_string(light));
            let _ = writeln!(lua, "    dark = \"{}\",", escape_lua_string(dark));
            if let Some(fb) = t.get("fallback").and_then(|v| v.as_str()) {
                let _ = writeln!(lua, "    fallback = \"{}\",", escape_lua_string(fb));
            }
            let _ = writeln!(lua, "}})");
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Editor config
// ---------------------------------------------------------------------------

fn convert_editor(lua: &mut String, editor: &toml::Value) {
    if let Some(table) = editor.as_table() {
        for (key, value) in table {
            let lua_key = kebab_to_snake(key);
            let _ = writeln!(
                lua,
                "si.config.{} = {}",
                lua_key,
                toml_to_lua(value, true)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Keybindings
// ---------------------------------------------------------------------------

fn convert_keys(lua: &mut String, keys: &toml::Value) {
    if let Some(modes) = keys.as_table() {
        for (mode, bindings) in modes {
            convert_mode_keybindings(lua, mode, bindings);
        }
    }
}

fn convert_mode_keybindings(lua: &mut String, mode: &str, bindings: &toml::Value) {
    if let Some(table) = bindings.as_table() {
        for (key, action) in table {
            if key == "label" || key == "is_sticky" {
                continue;
            }
            match action {
                toml::Value::String(cmd) => {
                    let _ = writeln!(
                        lua,
                        "si.keymap.set(\"{}\", \"{}\", \"{}\")",
                        mode,
                        escape_lua_string(key),
                        escape_lua_string(cmd)
                    );
                }
                toml::Value::Array(cmds) => {
                    let items: Vec<String> = cmds
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| format!("\"{}\"", escape_lua_string(s)))
                        .collect();
                    let _ = writeln!(
                        lua,
                        "si.keymap.set(\"{}\", \"{}\", {{ {} }})",
                        mode,
                        escape_lua_string(key),
                        items.join(", ")
                    );
                }
                toml::Value::Table(submenu) => {
                    let label = submenu
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or(key);
                    let _ = writeln!(
                        lua,
                        "si.keymap.set(\"{}\", \"{}\", {{",
                        mode,
                        escape_lua_string(key)
                    );
                    let _ = writeln!(lua, "    label = \"{}\",", escape_lua_string(label));
                    for (sub_key, sub_action) in submenu {
                        if sub_key == "label" || sub_key == "is_sticky" {
                            continue;
                        }
                        write_keybinding_entry(lua, sub_key, sub_action, 1);
                    }
                    let _ = writeln!(lua, "}})");
                }
                _ => {}
            }
        }
    }
}

fn write_keybinding_entry(lua: &mut String, key: &str, value: &toml::Value, indent: usize) {
    let prefix = "    ".repeat(indent);
    let lk = lua_table_key(key);
    match value {
        toml::Value::String(cmd) => {
            let _ = writeln!(
                lua,
                "    {}{} = \"{}\",",
                prefix,
                lk,
                escape_lua_string(cmd)
            );
        }
        toml::Value::Array(cmds) => {
            let items: Vec<String> = cmds
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("\"{}\"", escape_lua_string(s)))
                .collect();
            let _ = writeln!(
                lua,
                "    {}{} = {{ {} }},",
                prefix,
                lk,
                items.join(", ")
            );
        }
        toml::Value::Table(submenu) => {
            let label = submenu
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(key);
            let _ = writeln!(lua, "    {}{} = {{", prefix, lk);
            let _ = writeln!(
                lua,
                "    {}    label = \"{}\",",
                prefix,
                escape_lua_string(label)
            );
            for (sub_key, sub_action) in submenu {
                if sub_key == "label" || sub_key == "is_sticky" {
                    continue;
                }
                write_keybinding_entry(lua, sub_key, sub_action, indent + 1);
            }
            let _ = writeln!(lua, "    {}}},", prefix);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Language config
// ---------------------------------------------------------------------------

fn convert_language_server(lua: &mut String, name: &str, config: &toml::Value) {
    if let Some(table) = config.as_table() {
        let _ = writeln!(
            lua,
            "si.language_server(\"{}\", {{",
            escape_lua_string(name)
        );
        for (key, value) in table {
            let lua_key = kebab_to_snake(key);
            let lk = lua_table_key(&lua_key);
            if key == "config" {
                // Preserve camelCase keys in LSP server config
                let _ = writeln!(lua, "    {} = {},", lk, toml_to_lua(value, false));
            } else {
                let _ = writeln!(lua, "    {} = {},", lk, toml_to_lua(value, true));
            }
        }
        let _ = writeln!(lua, "}})");
    }
}

fn convert_language(lua: &mut String, lang: &toml::Value) {
    if let Some(table) = lang.as_table() {
        let name = table
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let _ = writeln!(lua, "si.language(\"{}\", {{", escape_lua_string(name));
        for (key, value) in table {
            if key == "name" {
                continue;
            }
            let lua_key = kebab_to_snake(key);
            let lk = lua_table_key(&lua_key);
            let _ = writeln!(lua, "    {} = {},", lk, toml_to_lua(value, true));
        }
        let _ = writeln!(lua, "}})");
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kebab_to_snake(s: &str) -> String {
    s.replace('-', "_")
}

fn escape_lua_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\0', "\\0")
}

/// Convert a TOML value to an inline Lua expression.
///
/// When `convert_keys` is true, table keys are converted from kebab-case to snake_case.
/// When false, keys are preserved as-is (used for LSP `config` sub-tables with camelCase).
fn toml_to_lua(value: &toml::Value, convert_keys: bool) -> String {
    match value {
        toml::Value::String(s) => format!("\"{}\"", escape_lua_string(s)),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => {
            let s = f.to_string();
            if s.contains('.') {
                s
            } else {
                format!("{s}.0")
            }
        }
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            if arr.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> =
                    arr.iter().map(|v| toml_to_lua(v, convert_keys)).collect();
                format!("{{ {} }}", items.join(", "))
            }
        }
        toml::Value::Table(t) => {
            if t.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> = t
                    .iter()
                    .map(|(k, v)| {
                        let key = if convert_keys {
                            kebab_to_snake(k)
                        } else {
                            k.clone()
                        };
                        format!("{} = {}", lua_table_key(&key), toml_to_lua(v, convert_keys))
                    })
                    .collect();
                format!("{{ {} }}", items.join(", "))
            }
        }
        toml::Value::Datetime(dt) => format!("\"{dt}\""),
    }
}

/// Format a key for a Lua table literal. Bare identifiers are used when valid,
/// otherwise quoted bracket syntax `["key"]`.
fn lua_table_key(key: &str) -> String {
    if is_lua_identifier(key) {
        key.to_string()
    } else {
        format!("[\"{}\"]", escape_lua_string(key))
    }
}

fn is_lua_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kebab_to_snake() {
        assert_eq!(kebab_to_snake("line-number"), "line_number");
        assert_eq!(kebab_to_snake("auto-format"), "auto_format");
        assert_eq!(kebab_to_snake("indent-guides"), "indent_guides");
        assert_eq!(kebab_to_snake("mouse"), "mouse");
    }

    #[test]
    fn test_escape_lua_string() {
        assert_eq!(escape_lua_string("hello"), "hello");
        assert_eq!(escape_lua_string(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(escape_lua_string("line\nnew"), "line\\nnew");
        assert_eq!(escape_lua_string(r"back\slash"), r"back\\slash");
    }

    #[test]
    fn test_is_lua_identifier() {
        assert!(is_lua_identifier("foo"));
        assert!(is_lua_identifier("_bar"));
        assert!(is_lua_identifier("F1"));
        assert!(is_lua_identifier("snake_case"));
        assert!(!is_lua_identifier(""));
        assert!(!is_lua_identifier("C-a"));
        assert!(!is_lua_identifier("123"));
        assert!(!is_lua_identifier("has space"));
    }

    #[test]
    fn test_simple_config_conversion() {
        let toml = r##"
theme = "onedark"

[editor]
line-number = "relative"
mouse = true
scrolloff = 5
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(lua.contains(r#"si.theme.set("onedark")"#));
        assert!(lua.contains(r#"si.config.line_number = "relative""#));
        assert!(lua.contains("si.config.mouse = true"));
        assert!(lua.contains("si.config.scrolloff = 5"));
    }

    #[test]
    fn test_nested_config_conversion() {
        let toml = r##"
[editor.indent-guides]
render = true
character = "│"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(lua.contains("si.config.indent_guides"));
        assert!(lua.contains("render = true"));
        assert!(
            lua.contains("character = \"│\""),
            "generated lua:\n{lua}"
        );
    }

    #[test]
    fn test_keybinding_conversion_simple() {
        let toml = r##"
[keys.normal]
j = "move_line_down"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(
            lua.contains(r#"si.keymap.set("normal", "j", "move_line_down")"#),
            "generated lua:\n{lua}"
        );
    }

    #[test]
    fn test_keybinding_conversion_modifier() {
        let toml = r##"
[keys.normal]
C-a = "select_all"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(
            lua.contains(r#"si.keymap.set("normal", "C-a", "select_all")"#),
            "generated lua:\n{lua}"
        );
    }

    #[test]
    fn test_keybinding_submenu() {
        let toml = r##"
[keys.normal.g]
d = "goto_definition"
r = "goto_reference"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(
            lua.contains(r#"si.keymap.set("normal", "g", {"#),
            "generated lua:\n{lua}"
        );
        assert!(lua.contains(r#"d = "goto_definition""#));
        assert!(lua.contains(r#"r = "goto_reference""#));
    }

    #[test]
    fn test_keybinding_sequence() {
        let toml = r##"
[keys.normal]
Q = ["select_all", "yank"]
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(
            lua.contains(r#"si.keymap.set("normal", "Q", { "select_all", "yank" })"#),
            "generated lua:\n{lua}"
        );
    }

    #[test]
    fn test_adaptive_theme_conversion() {
        let toml = r##"
[theme]
light = "solarized_light"
dark = "onedark"
fallback = "onedark"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(lua.contains("si.theme.adaptive"));
        assert!(lua.contains(r#"light = "solarized_light""#));
        assert!(lua.contains(r#"dark = "onedark""#));
        assert!(lua.contains(r#"fallback = "onedark""#));
    }

    #[test]
    fn test_languages_conversion() {
        let toml = r##"
[language-server.pyright]
command = "pyright-langserver"
args = ["--stdio"]

[[language]]
name = "python"
language-servers = ["pyright"]
auto-format = true
"##;
        let lua = languages_toml_to_lua(toml).unwrap();
        assert!(
            lua.contains(r#"si.language_server("pyright""#),
            "generated lua:\n{lua}"
        );
        assert!(lua.contains(r#"command = "pyright-langserver""#));
        assert!(
            lua.contains(r#"si.language("python""#),
            "generated lua:\n{lua}"
        );
        assert!(lua.contains("language_servers"));
        assert!(lua.contains("auto_format = true"));
    }

    #[test]
    fn test_language_server_config_preserves_camel_case() {
        let toml = r##"
[language-server.rust-analyzer]
command = "rust-analyzer"

[language-server.rust-analyzer.config]
checkOnSave = { command = "clippy" }
cargo = { allFeatures = true }
"##;
        let lua = languages_toml_to_lua(toml).unwrap();
        // camelCase keys in config sub-table must be preserved
        assert!(
            lua.contains("checkOnSave"),
            "generated lua:\n{lua}"
        );
        assert!(lua.contains("allFeatures"));
        // But standard keys should be snake_case
        assert!(lua.contains(r#"command = "rust-analyzer""#));
    }

    #[test]
    fn test_roundtrip_correctness() {
        // The generated Lua should load without errors and produce correct values.
        let toml = r##"
theme = "onedark"
[editor]
scrolloff = 5
mouse = true
[keys.normal]
j = "move_line_down"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        let config = crate::load_config_from_str(&lua).unwrap();
        assert_eq!(config.editor.scrolloff, 5);
        assert!(config.editor.mouse);
        assert!(matches!(
            config.theme,
            Some(crate::ThemeConfig::Named(ref n)) if n == "onedark"
        ));
        assert!(config
            .keys
            .contains_key(&silicon_view::document::Mode::Normal));
    }

    #[test]
    fn test_roundtrip_with_nested_config() {
        let toml = r##"
theme = "onedark"
[editor]
scrolloff = 8
line-number = "relative"
mouse = false
[editor.indent-guides]
render = true
character = "|"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        let config = crate::load_config_from_str(&lua).unwrap();
        assert_eq!(config.editor.scrolloff, 8);
        assert_eq!(
            config.editor.line_number,
            silicon_view::editor::LineNumber::Relative
        );
        assert!(!config.editor.mouse);
        assert!(config.editor.indent_guides.render);
        assert_eq!(config.editor.indent_guides.character, '|');
    }

    #[test]
    fn test_roundtrip_with_keybindings_and_submenu() {
        let toml = r##"
[keys.normal]
j = "move_line_down"
k = "move_line_up"

[keys.normal.g]
d = "goto_definition"
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        let config = crate::load_config_from_str(&lua).unwrap();
        assert!(config
            .keys
            .contains_key(&silicon_view::document::Mode::Normal));
    }

    #[test]
    fn test_roundtrip_languages() {
        let toml = r##"
[language-server.pyright]
command = "pyright-langserver"
args = ["--stdio"]

[[language]]
name = "python"
language-servers = ["pyright"]
auto-format = true
"##;
        let lua_src = languages_toml_to_lua(toml).unwrap();
        // Load as config — languages are appended after editor config
        let config = crate::load_config_from_str(&lua_src).unwrap();
        let lang_config = config.language_config.unwrap();
        let langs = lang_config.get("language").unwrap().as_array().unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(
            langs[0].get("name").unwrap().as_str().unwrap(),
            "python"
        );
        let servers = lang_config
            .get("language-server")
            .unwrap()
            .as_table()
            .unwrap();
        assert!(servers.contains_key("pyright"));
    }

    #[test]
    fn test_run_migration() {
        let dir = tempfile::tempdir().unwrap();
        let config_toml = dir.path().join("config.toml");
        std::fs::write(
            &config_toml,
            r##"
theme = "onedark"
[editor]
scrolloff = 8
"##,
        )
        .unwrap();

        let result = run_migration(dir.path()).unwrap();
        assert_eq!(result, dir.path().join("init.lua"));
        assert!(result.exists());
        assert!(!config_toml.exists());
        assert!(dir.path().join("config.toml.bak").exists());

        // Verify the generated Lua is loadable
        let lua_content = std::fs::read_to_string(&result).unwrap();
        crate::load_config_from_str(&lua_content).unwrap();
    }

    #[test]
    fn test_run_migration_with_languages() {
        let dir = tempfile::tempdir().unwrap();
        let config_toml = dir.path().join("config.toml");
        let lang_toml = dir.path().join("languages.toml");
        std::fs::write(&config_toml, "theme = \"onedark\"\n").unwrap();
        std::fs::write(
            &lang_toml,
            r##"
[language-server.pyright]
command = "pyright-langserver"
"##,
        )
        .unwrap();

        let result = run_migration(dir.path()).unwrap();
        assert!(result.exists());
        assert!(!config_toml.exists());
        assert!(!lang_toml.exists());
        assert!(dir.path().join("config.toml.bak").exists());
        assert!(dir.path().join("languages.toml.bak").exists());

        let content = std::fs::read_to_string(&result).unwrap();
        assert!(content.contains("si.theme.set"));
        assert!(content.contains("si.language_server"));
    }

    #[test]
    fn test_run_migration_empty_creates_default() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_migration(dir.path()).unwrap();
        assert!(result.exists());
        let content = std::fs::read_to_string(&result).unwrap();
        assert!(content.contains("si.theme.set"));
    }

    #[test]
    fn test_toml_to_lua_values() {
        assert_eq!(toml_to_lua(&toml::Value::Boolean(true), true), "true");
        assert_eq!(toml_to_lua(&toml::Value::Integer(42), true), "42");
        assert_eq!(
            toml_to_lua(&toml::Value::String("hello".into()), true),
            "\"hello\""
        );
        assert_eq!(toml_to_lua(&toml::Value::Float(1.5), true), "1.5");
    }

    #[test]
    fn test_toml_to_lua_array() {
        let arr = toml::Value::Array(vec![
            toml::Value::String("a".into()),
            toml::Value::String("b".into()),
        ]);
        assert_eq!(toml_to_lua(&arr, true), r#"{ "a", "b" }"#);
    }

    #[test]
    fn test_toml_to_lua_empty_array() {
        let arr = toml::Value::Array(vec![]);
        assert_eq!(toml_to_lua(&arr, true), "{}");
    }

    #[test]
    fn test_toml_to_lua_table_with_kebab_keys() {
        let mut t = toml::map::Map::new();
        t.insert("tab-width".into(), toml::Value::Integer(4));
        t.insert("unit".into(), toml::Value::String("  ".into()));
        let result = toml_to_lua(&toml::Value::Table(t), true);
        assert!(result.contains("tab_width = 4"));
        assert!(result.contains("unit = \"  \""));
    }

    #[test]
    fn test_toml_to_lua_table_preserve_keys() {
        let mut t = toml::map::Map::new();
        t.insert("checkOnSave".into(), toml::Value::Boolean(true));
        let result = toml_to_lua(&toml::Value::Table(t), false);
        assert!(result.contains("checkOnSave = true"));
    }

    #[test]
    fn test_editor_shell_array() {
        let toml = r##"
[editor]
shell = ["/bin/zsh", "-c"]
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(
            lua.contains(r#"si.config.shell = { "/bin/zsh", "-c" }"#),
            "generated lua:\n{lua}"
        );
        // Verify roundtrip
        let config = crate::load_config_from_str(&lua).unwrap();
        assert_eq!(config.editor.shell, vec!["/bin/zsh", "-c"]);
    }

    #[test]
    fn test_editor_rulers_array() {
        let toml = r##"
[editor]
rulers = [80, 120]
"##;
        let lua = config_toml_to_lua(toml).unwrap();
        assert!(
            lua.contains("si.config.rulers = { 80, 120 }"),
            "generated lua:\n{lua}"
        );
        let config = crate::load_config_from_str(&lua).unwrap();
        assert_eq!(config.editor.rulers, vec![80u16, 120]);
    }
}
