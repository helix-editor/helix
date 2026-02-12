pub mod config;
pub mod error;
pub mod state;

use std::path::Path;

pub use error::LuaConfigError;

/// Configuration extracted from Lua init files.
#[derive(Debug)]
pub struct LuaConfig {
    pub editor: silicon_view::editor::Config,
}

/// Load config from a specific Lua file path.
pub fn load_config(path: &Path) -> Result<LuaConfig, LuaConfigError> {
    let source = std::fs::read_to_string(path)?;
    load_config_from_str(&source)
}

/// Load config from a Lua source string.
pub fn load_config_from_str(source: &str) -> Result<LuaConfig, LuaConfigError> {
    let lua = state::create_lua_state()?;
    lua.load(source).exec()?;
    let editor = config::extract_editor_config(&lua)?;
    Ok(LuaConfig { editor })
}

/// Load config from default locations.
///
/// Searches for:
///   1. `~/.config/silicon/init.lua` (global)
///   2. `.silicon/init.lua` (workspace)
///
/// If neither exists but `config.toml` does, returns `TomlDetected`.
/// If neither exists at all, returns `NotFound`.
/// If both exist, loads global first, then workspace (workspace wins).
pub fn load_config_default() -> Result<LuaConfig, LuaConfigError> {
    let global_path = silicon_loader::config_dir().join("init.lua");
    let workspace_path = silicon_loader::find_workspace().0.join(".silicon").join("init.lua");

    let global_exists = global_path.is_file();
    let workspace_exists = workspace_path.is_file();

    if !global_exists && !workspace_exists {
        // Check for legacy TOML config.
        let toml_path = silicon_loader::config_dir().join("config.toml");
        if toml_path.is_file() {
            return Err(LuaConfigError::TomlDetected(toml_path));
        }
        return Err(LuaConfigError::NotFound);
    }

    let lua = state::create_lua_state()?;

    if global_exists {
        let source = std::fs::read_to_string(&global_path)?;
        lua.load(&source)
            .set_name(global_path.to_string_lossy())
            .exec()?;
    }

    if workspace_exists {
        let source = std::fs::read_to_string(&workspace_path)?;
        lua.load(&source)
            .set_name(workspace_path.to_string_lossy())
            .exec()?;
    }

    let editor = config::extract_editor_config(&lua)?;
    Ok(LuaConfig { editor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_uses_defaults() {
        let config = load_config_from_str("").unwrap();
        let defaults = silicon_view::editor::Config::default();
        assert_eq!(config.editor.scrolloff, defaults.scrolloff);
        assert_eq!(config.editor.mouse, defaults.mouse);
        assert_eq!(config.editor.cursorline, defaults.cursorline);
        assert_eq!(config.editor.auto_format, defaults.auto_format);
    }

    #[test]
    fn scrolloff_override() {
        let config = load_config_from_str("si.config.scrolloff = 10").unwrap();
        assert_eq!(config.editor.scrolloff, 10);
        // Other fields remain default.
        let defaults = silicon_view::editor::Config::default();
        assert_eq!(config.editor.mouse, defaults.mouse);
    }

    #[test]
    fn mouse_override() {
        let config = load_config_from_str("si.config.mouse = false").unwrap();
        assert!(!config.editor.mouse);
    }

    #[test]
    fn syntax_error_is_caught() {
        let result = load_config_from_str("si.config.scrolloff = ");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LuaConfigError::Lua(_)));
    }

    #[test]
    fn infinite_loop_protection() {
        let result = load_config_from_str("while true do end");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("instruction limit"),
            "Expected instruction limit error, got: {err_msg}"
        );
    }

    #[test]
    fn platform_conditional() {
        let source = r#"
            if si.platform == "macos" then
                si.config.scrolloff = 20
            else
                si.config.scrolloff = 15
            end
        "#;
        let config = load_config_from_str(source).unwrap();
        let expected = if cfg!(target_os = "macos") { 20 } else { 15 };
        assert_eq!(config.editor.scrolloff, expected);
    }

    #[test]
    fn stub_apis_dont_crash() {
        let source = r#"
            si.keymap.set("normal", "gf", "goto_file")
            si.keymap.set_many({})
            si.theme.set("onedark")
            si.theme.adaptive("onedark", "onelight")
            si.theme.define("mytheme", {})
            si.language({})
            si.language_server({})
        "#;
        let config = load_config_from_str(source).unwrap();
        // Should not crash; config should be defaults.
        let defaults = silicon_view::editor::Config::default();
        assert_eq!(config.editor.scrolloff, defaults.scrolloff);
    }
}
