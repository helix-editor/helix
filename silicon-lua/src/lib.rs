pub mod config;
pub mod error;
pub mod keymap;
pub mod state;
pub mod theme;
pub mod types;

use std::collections::{HashMap, HashSet};
use std::path::Path;

pub use config::{apply_editor_field, LuaEditorConfig};
pub use error::LuaConfigError;
pub use keymap::KeyBinding;
pub use theme::ThemeConfig;
use silicon_view::document::Mode;

/// Configuration extracted from Lua init files.
#[derive(Debug)]
pub struct LuaConfig {
    pub editor: silicon_view::editor::Config,
    pub explicit_editor_fields: HashSet<String>,
    pub keys: HashMap<Mode, KeyBinding>,
    pub theme: Option<ThemeConfig>,
}

impl LuaConfig {
    /// Merge another config into this one.
    /// Only fields that were explicitly set in `other` will override fields in `self`.
    pub fn merge(&mut self, other: LuaConfig) {
        for field in &other.explicit_editor_fields {
            apply_editor_field(&mut self.editor, &other.editor, field);
        }
        self.explicit_editor_fields
            .extend(other.explicit_editor_fields);
        // Merge keybindings: other's modes override/extend self's.
        for (mode, binding) in other.keys {
            self.keys.insert(mode, binding);
        }
        // Workspace theme overrides global theme.
        if other.theme.is_some() {
            self.theme = other.theme;
        }
    }
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
    let result = config::extract_editor_config(&lua)?;
    let keys = keymap::extract_keybindings(&lua)?;
    let theme = theme::extract_theme_config(&lua)?;
    Ok(LuaConfig {
        editor: result.config,
        explicit_editor_fields: result.explicit_fields,
        keys,
        theme,
    })
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
    let workspace_path = silicon_loader::find_workspace()
        .0
        .join(".silicon")
        .join("init.lua");

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

    let result = config::extract_editor_config(&lua)?;
    let keys = keymap::extract_keybindings(&lua)?;
    let theme = theme::extract_theme_config(&lua)?;
    Ok(LuaConfig {
        editor: result.config,
        explicit_editor_fields: result.explicit_fields,
        keys,
        theme,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use silicon_core::diagnostic::Severity;
    use silicon_view::editor::{
        BufferLine, Config as EditorConfig, LineEndingConfig, LineNumber, PopupBorderConfig,
    };
    use std::time::Duration;

    #[test]
    fn empty_config_uses_defaults() {
        let config = load_config_from_str("").unwrap();
        let defaults = EditorConfig::default();
        assert_eq!(config.editor.scrolloff, defaults.scrolloff);
        assert_eq!(config.editor.mouse, defaults.mouse);
        assert_eq!(config.editor.cursorline, defaults.cursorline);
        assert_eq!(config.editor.auto_format, defaults.auto_format);
        assert!(config.explicit_editor_fields.is_empty());
    }

    #[test]
    fn scrolloff_override() {
        let config = load_config_from_str("si.config.scrolloff = 10").unwrap();
        assert_eq!(config.editor.scrolloff, 10);
        assert!(config.explicit_editor_fields.contains("scrolloff"));
        // Other fields remain default.
        let defaults = EditorConfig::default();
        assert_eq!(config.editor.mouse, defaults.mouse);
    }

    #[test]
    fn mouse_false_is_explicit() {
        let config = load_config_from_str("si.config.mouse = false").unwrap();
        assert!(!config.editor.mouse);
        assert!(config.explicit_editor_fields.contains("mouse"));
    }

    #[test]
    fn mouse_true_is_explicit() {
        let config = load_config_from_str("si.config.mouse = true").unwrap();
        assert!(config.editor.mouse);
        assert!(config.explicit_editor_fields.contains("mouse"));
    }

    #[test]
    fn bool_nil_not_overwritten() {
        // mouse defaults to true. Not setting it should keep it true.
        let config = load_config_from_str("si.config.scrolloff = 10").unwrap();
        assert!(config.editor.mouse); // default is true
        assert!(!config.explicit_editor_fields.contains("mouse"));
    }

    // --- String enum fields ---

    #[test]
    fn line_number_absolute() {
        let config = load_config_from_str(r#"si.config.line_number = "absolute""#).unwrap();
        assert_eq!(config.editor.line_number, LineNumber::Absolute);
        assert!(config.explicit_editor_fields.contains("line_number"));
    }

    #[test]
    fn line_number_relative() {
        let config = load_config_from_str(r#"si.config.line_number = "relative""#).unwrap();
        assert_eq!(config.editor.line_number, LineNumber::Relative);
    }

    #[test]
    fn line_number_invalid() {
        let result = load_config_from_str(r#"si.config.line_number = "bogus""#);
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("invalid line_number"), "got: {err}");
    }

    #[test]
    fn bufferline_enum() {
        let config = load_config_from_str(r#"si.config.bufferline = "multiple""#).unwrap();
        assert_eq!(config.editor.bufferline, BufferLine::Multiple);

        let config = load_config_from_str(r#"si.config.bufferline = "always""#).unwrap();
        assert_eq!(config.editor.bufferline, BufferLine::Always);

        let config = load_config_from_str(r#"si.config.bufferline = "never""#).unwrap();
        assert_eq!(config.editor.bufferline, BufferLine::Never);
    }

    #[test]
    fn popup_border_enum() {
        let config = load_config_from_str(r#"si.config.popup_border = "all""#).unwrap();
        assert_eq!(config.editor.popup_border, PopupBorderConfig::All);
    }

    #[test]
    fn default_line_ending_enum() {
        let config = load_config_from_str(r#"si.config.default_line_ending = "lf""#).unwrap();
        assert_eq!(config.editor.default_line_ending, LineEndingConfig::LF);
    }

    // --- Duration fields ---

    #[test]
    fn idle_timeout_millis() {
        let config = load_config_from_str("si.config.idle_timeout = 500").unwrap();
        assert_eq!(config.editor.idle_timeout, Duration::from_millis(500));
        assert!(config.explicit_editor_fields.contains("idle_timeout"));
    }

    #[test]
    fn completion_timeout_millis() {
        let config = load_config_from_str("si.config.completion_timeout = 100").unwrap();
        assert_eq!(
            config.editor.completion_timeout,
            Duration::from_millis(100)
        );
    }

    // --- Array fields ---

    #[test]
    fn shell_array() {
        let config =
            load_config_from_str(r#"si.config.shell = { "/bin/zsh", "-c" }"#).unwrap();
        assert_eq!(config.editor.shell, vec!["/bin/zsh", "-c"]);
        assert!(config.explicit_editor_fields.contains("shell"));
    }

    #[test]
    fn rulers_array() {
        let config = load_config_from_str("si.config.rulers = { 80, 120 }").unwrap();
        assert_eq!(config.editor.rulers, vec![80u16, 120]);
        assert!(config.explicit_editor_fields.contains("rulers"));
    }

    // --- Char fields ---

    #[test]
    fn default_yank_register() {
        let config =
            load_config_from_str(r#"si.config.default_yank_register = "+""#).unwrap();
        assert_eq!(config.editor.default_yank_register, '+');
    }

    #[test]
    fn jump_label_alphabet() {
        let config =
            load_config_from_str(r#"si.config.jump_label_alphabet = "asdfjkl""#).unwrap();
        assert_eq!(
            config.editor.jump_label_alphabet,
            vec!['a', 's', 'd', 'f', 'j', 'k', 'l']
        );
    }

    // --- Nested table: indent_guides ---

    #[test]
    fn indent_guides_nested() {
        let source = r#"
            si.config.indent_guides = { render = true, character = "│", skip_levels = 1 }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert!(config.editor.indent_guides.render);
        assert_eq!(config.editor.indent_guides.character, '│');
        assert_eq!(config.editor.indent_guides.skip_levels, 1);
        assert!(config.explicit_editor_fields.contains("indent_guides"));
    }

    // --- Nested table: lsp ---

    #[test]
    fn lsp_nested() {
        let source = r#"
            si.config.lsp = { enable = true, display_inlay_hints = true, snippets = false }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert!(config.editor.lsp.enable);
        assert!(config.editor.lsp.display_inlay_hints);
        assert!(!config.editor.lsp.snippets);
    }

    // --- Nested table: search ---

    #[test]
    fn search_nested() {
        let source = r#"
            si.config.search = { smart_case = false, wrap_around = false }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert!(!config.editor.search.smart_case);
        assert!(!config.editor.search.wrap_around);
    }

    // --- Nested table: statusline ---

    #[test]
    fn statusline_nested() {
        let source = r#"
            si.config.statusline = {
                left = { "mode", "file-name" },
                center = {},
                right = { "position" },
                separator = "|",
            }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert_eq!(config.editor.statusline.left.len(), 2);
        assert!(config.editor.statusline.center.is_empty());
        assert_eq!(config.editor.statusline.right.len(), 1);
        assert_eq!(config.editor.statusline.separator, "|");
    }

    // --- Nested table: soft_wrap ---

    #[test]
    fn soft_wrap_nested() {
        let source = r#"
            si.config.soft_wrap = { enable = true, max_wrap = 30 }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert_eq!(config.editor.soft_wrap.enable, Some(true));
        assert_eq!(config.editor.soft_wrap.max_wrap, Some(30));
    }

    // --- Special: auto_pairs ---

    #[test]
    fn auto_pairs_bool_true() {
        let config = load_config_from_str("si.config.auto_pairs = true").unwrap();
        assert_eq!(
            config.editor.auto_pairs,
            silicon_core::syntax::config::AutoPairConfig::Enable(true)
        );
    }

    #[test]
    fn auto_pairs_bool_false() {
        let config = load_config_from_str("si.config.auto_pairs = false").unwrap();
        assert_eq!(
            config.editor.auto_pairs,
            silicon_core::syntax::config::AutoPairConfig::Enable(false)
        );
    }

    #[test]
    fn auto_pairs_table() {
        let source = r#"
            si.config.auto_pairs = { ["("] = ")", ["{"] = "}" }
        "#;
        let config = load_config_from_str(source).unwrap();
        if let silicon_core::syntax::config::AutoPairConfig::Pairs(pairs) =
            &config.editor.auto_pairs
        {
            assert_eq!(pairs.get(&'('), Some(&')'));
            assert_eq!(pairs.get(&'{'), Some(&'}'));
        } else {
            panic!("Expected AutoPairConfig::Pairs");
        }
    }

    // --- Special: gutters ---

    #[test]
    fn gutters_array() {
        let source = r#"
            si.config.gutters = { "diagnostics", "line-numbers", "diff" }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert_eq!(config.editor.gutters.layout.len(), 3);
    }

    #[test]
    fn gutters_table() {
        let source = r#"
            si.config.gutters = {
                layout = { "diagnostics", "spacer", "line-numbers" },
                line_numbers = { min_width = 5 },
            }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert_eq!(config.editor.gutters.layout.len(), 3);
        assert_eq!(config.editor.gutters.line_numbers.min_width, 5);
    }

    // --- Special: smart_tab ---

    #[test]
    fn smart_tab_false_disables() {
        let config = load_config_from_str("si.config.smart_tab = false").unwrap();
        assert!(config.editor.smart_tab.is_none());
        assert!(config.explicit_editor_fields.contains("smart_tab"));
    }

    #[test]
    fn smart_tab_true_enables_defaults() {
        let config = load_config_from_str("si.config.smart_tab = true").unwrap();
        assert!(config.editor.smart_tab.is_some());
        assert!(config.editor.smart_tab.as_ref().unwrap().enable);
    }

    #[test]
    fn smart_tab_table() {
        let source = r#"
            si.config.smart_tab = { enable = true, supersede_menu = true }
        "#;
        let config = load_config_from_str(source).unwrap();
        let st = config.editor.smart_tab.unwrap();
        assert!(st.enable);
        assert!(st.supersede_menu);
    }

    // --- Special: auto_save ---

    #[test]
    fn auto_save_nested() {
        let source = r#"
            si.config.auto_save = {
                focus_lost = true,
                after_delay = { enable = true, timeout = 5000 },
            }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert!(config.editor.auto_save.focus_lost);
        assert!(config.editor.auto_save.after_delay.enable);
        assert_eq!(config.editor.auto_save.after_delay.timeout, 5000);
    }

    // --- Diagnostic filters ---

    #[test]
    fn end_of_line_diagnostics() {
        let config = load_config_from_str(
            r#"si.config.end_of_line_diagnostics = "warning""#,
        )
        .unwrap();
        assert_eq!(
            config.editor.end_of_line_diagnostics,
            silicon_view::annotations::diagnostics::DiagnosticFilter::Enable(Severity::Warning)
        );
    }

    #[test]
    fn end_of_line_diagnostics_disable() {
        let config = load_config_from_str(
            r#"si.config.end_of_line_diagnostics = "disable""#,
        )
        .unwrap();
        assert_eq!(
            config.editor.end_of_line_diagnostics,
            silicon_view::annotations::diagnostics::DiagnosticFilter::Disable
        );
    }

    // --- Inline diagnostics ---

    #[test]
    fn inline_diagnostics_nested() {
        let source = r#"
            si.config.inline_diagnostics = {
                cursor_line = "error",
                other_lines = "disable",
                min_diagnostic_width = 60,
            }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert_eq!(
            config.editor.inline_diagnostics.cursor_line,
            silicon_view::annotations::diagnostics::DiagnosticFilter::Enable(Severity::Error)
        );
        assert_eq!(
            config.editor.inline_diagnostics.other_lines,
            silicon_view::annotations::diagnostics::DiagnosticFilter::Disable
        );
        assert_eq!(config.editor.inline_diagnostics.min_diagnostic_width, 60);
    }

    // --- Cursor shape ---

    #[test]
    fn cursor_shape_nested() {
        let source = r#"
            si.config.cursor_shape = { normal = "block", insert = "bar", select = "underline" }
        "#;
        let config = load_config_from_str(source).unwrap();
        use silicon_view::document::Mode;
        use silicon_view::graphics::CursorKind;
        assert_eq!(
            config.editor.cursor_shape.from_mode(Mode::Normal),
            CursorKind::Block
        );
        assert_eq!(
            config.editor.cursor_shape.from_mode(Mode::Insert),
            CursorKind::Bar
        );
        assert_eq!(
            config.editor.cursor_shape.from_mode(Mode::Select),
            CursorKind::Underline
        );
    }

    // --- Merge ---

    #[test]
    fn merge_only_overrides_explicit_fields() {
        let mut global = load_config_from_str(
            r#"
            si.config.scrolloff = 8
            si.config.mouse = false
        "#,
        )
        .unwrap();

        let workspace = load_config_from_str("si.config.scrolloff = 15").unwrap();

        global.merge(workspace);
        assert_eq!(global.editor.scrolloff, 15); // overridden by workspace
        assert!(!global.editor.mouse); // kept from global (not in workspace explicit)
    }

    // --- Whitespace ---

    #[test]
    fn whitespace_basic_render() {
        let source = r#"
            si.config.whitespace = { render = "all" }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert_eq!(
            config.editor.whitespace.render,
            silicon_view::editor::WhitespaceRender::Basic(
                silicon_view::editor::WhitespaceRenderValue::All
            )
        );
    }

    // --- Word completion ---

    #[test]
    fn word_completion_nested() {
        let source = r#"
            si.config.word_completion = { enable = false, trigger_length = 3 }
        "#;
        let config = load_config_from_str(source).unwrap();
        assert!(!config.editor.word_completion.enable);
        assert_eq!(
            config.editor.word_completion.trigger_length.get(),
            3
        );
    }

    // --- Comprehensive config ---

    #[test]
    fn comprehensive_config() {
        let source = r#"
            si.config.scrolloff = 8
            si.config.mouse = true
            si.config.line_number = "relative"
            si.config.cursorline = true
            si.config.bufferline = "multiple"
            si.config.true_color = true
            si.config.indent_guides = { render = true, character = "│" }
            si.config.lsp = { enable = true, display_inlay_hints = true }
            si.config.rulers = { 80, 120 }
            si.config.shell = { "/bin/zsh", "-c" }
            si.config.popup_border = "all"
            si.config.color_modes = true
            si.config.auto_format = false
            si.config.completion_trigger_len = 3
            si.config.idle_timeout = 400
        "#;
        let config = load_config_from_str(source).unwrap();
        assert_eq!(config.editor.scrolloff, 8);
        assert!(config.editor.mouse);
        assert_eq!(config.editor.line_number, LineNumber::Relative);
        assert!(config.editor.cursorline);
        assert_eq!(config.editor.bufferline, BufferLine::Multiple);
        assert!(config.editor.true_color);
        assert!(config.editor.indent_guides.render);
        assert!(config.editor.lsp.display_inlay_hints);
        assert_eq!(config.editor.rulers, vec![80u16, 120]);
        assert_eq!(config.editor.shell, vec!["/bin/zsh", "-c"]);
        assert_eq!(config.editor.popup_border, PopupBorderConfig::All);
        assert!(config.editor.color_modes);
        assert!(!config.editor.auto_format);
        assert_eq!(config.editor.completion_trigger_len, 3);
        assert_eq!(config.editor.idle_timeout, Duration::from_millis(400));
        // Should have tracked all 15 fields
        assert_eq!(config.explicit_editor_fields.len(), 15);
    }

    // --- Phase 1 tests kept ---

    #[test]
    fn syntax_error_is_caught() {
        let result = load_config_from_str("si.config.scrolloff = ");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LuaConfigError::Lua(_)));
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
            si.keymap.set("normal", "g", "goto_file")
            si.keymap.set_many("normal", { k = "move_line_up" })
            si.theme.set("onedark")
            si.theme.adaptive({ light = "onelight", dark = "onedark" })
            si.theme.define("mytheme", {})
            si.language({})
            si.language_server({})
        "#;
        let config = load_config_from_str(source).unwrap();
        let defaults = EditorConfig::default();
        assert_eq!(config.editor.scrolloff, defaults.scrolloff);
        // Keybindings should be populated.
        assert!(config.keys.contains_key(&silicon_view::document::Mode::Normal));
        // Last theme call was define(), so theme should be Custom.
        assert!(matches!(config.theme, Some(ThemeConfig::Custom { .. })));
    }
}
