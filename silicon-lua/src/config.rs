use std::collections::{HashMap, HashSet};
use std::num::NonZeroU8;
use std::time::Duration;

use mlua::{Lua, Table, Value};

use silicon_core::diagnostic::Severity;
use silicon_core::syntax::config::{AutoPairConfig, IndentationHeuristic, SoftWrap};
use silicon_view::annotations::diagnostics::{DiagnosticFilter, InlineDiagnosticsConfig};
use silicon_view::clipboard::ClipboardProvider;
use silicon_view::editor::{
    AutoSave, AutoSaveAfterDelay, BufferLine, BufferPickerConfig, Config as EditorConfig,
    CursorShapeConfig, FileExplorerConfig, FilePickerConfig, GutterConfig, GutterLineNumbersConfig,
    GutterType, IndentGuidesConfig, KittyKeyboardProtocolConfig, LineEndingConfig, LineNumber,
    LspConfig, ModeConfig, PickerStartPosition, PopupBorderConfig, SearchConfig, SmartTabConfig,
    StatusLineConfig, StatusLineElement, TerminalConfig, WhitespaceCharacters, WhitespaceConfig,
    WhitespaceRender, WhitespaceRenderValue, WordCompletion,
};
use silicon_view::graphics::CursorKind;

use crate::error::LuaConfigError;
use crate::state::get_config_table;
use crate::types::*;

/// Editor config extracted from Lua, with tracking of which fields were explicitly set.
#[derive(Debug)]
pub struct LuaEditorConfig {
    pub config: EditorConfig,
    pub explicit_fields: HashSet<String>,
}

/// All known top-level config field names.
const KNOWN_FIELDS: &[&str] = &[
    "scrolloff",
    "scroll_lines",
    "mouse",
    "shell",
    "line_number",
    "cursorline",
    "cursorcolumn",
    "gutters",
    "middle_click_paste",
    "auto_pairs",
    "auto_completion",
    "path_completion",
    "word_completion",
    "auto_format",
    "default_yank_register",
    "auto_save",
    "text_width",
    "idle_timeout",
    "completion_timeout",
    "preview_completion_insert",
    "completion_trigger_len",
    "completion_replace",
    "continue_comments",
    "auto_info",
    "file_picker",
    "file_explorer",
    "statusline",
    "cursor_shape",
    "true_color",
    "undercurl",
    "search",
    "lsp",
    "terminal",
    "rulers",
    "whitespace",
    "bufferline",
    "indent_guides",
    "color_modes",
    "soft_wrap",
    "workspace_lsp_roots",
    "default_line_ending",
    "insert_final_newline",
    "atomic_save",
    "trim_final_newlines",
    "trim_trailing_whitespace",
    "smart_tab",
    "popup_border",
    "indent_heuristic",
    "jump_label_alphabet",
    "inline_diagnostics",
    "end_of_line_diagnostics",
    "clipboard_provider",
    "editor_config",
    "rainbow_brackets",
    "kitty_keyboard_protocol",
    "buffer_picker",
    "commandline",
];

/// Extract the complete editor configuration from `si.config`.
///
/// Missing keys silently keep defaults from `EditorConfig::default()`.
/// Unknown keys log a warning.
pub fn extract_editor_config(lua: &Lua) -> Result<LuaEditorConfig, LuaConfigError> {
    let table = get_config_table(lua)?;
    let mut ec = EditorConfig::default();
    let mut explicit = HashSet::new();

    // --- Scalar fields (bool, integer, float) ---

    if let Some(v) = get_opt::<usize>(lua, &table, "scrolloff") {
        ec.scrolloff = v;
        explicit.insert("scrolloff".into());
    }
    if let Some(v) = get_opt::<isize>(lua, &table, "scroll_lines") {
        ec.scroll_lines = v;
        explicit.insert("scroll_lines".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "mouse") {
        ec.mouse = v;
        explicit.insert("mouse".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "cursorline") {
        ec.cursorline = v;
        explicit.insert("cursorline".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "cursorcolumn") {
        ec.cursorcolumn = v;
        explicit.insert("cursorcolumn".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "middle_click_paste") {
        ec.middle_click_paste = v;
        explicit.insert("middle_click_paste".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "auto_completion") {
        ec.auto_completion = v;
        explicit.insert("auto_completion".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "path_completion") {
        ec.path_completion = v;
        explicit.insert("path_completion".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "auto_format") {
        ec.auto_format = v;
        explicit.insert("auto_format".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "preview_completion_insert") {
        ec.preview_completion_insert = v;
        explicit.insert("preview_completion_insert".into());
    }
    if let Some(v) = get_opt::<u8>(lua, &table, "completion_trigger_len") {
        ec.completion_trigger_len = v;
        explicit.insert("completion_trigger_len".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "completion_replace") {
        ec.completion_replace = v;
        explicit.insert("completion_replace".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "continue_comments") {
        ec.continue_comments = v;
        explicit.insert("continue_comments".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "auto_info") {
        ec.auto_info = v;
        explicit.insert("auto_info".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "true_color") {
        ec.true_color = v;
        explicit.insert("true_color".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "undercurl") {
        ec.undercurl = v;
        explicit.insert("undercurl".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "color_modes") {
        ec.color_modes = v;
        explicit.insert("color_modes".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "insert_final_newline") {
        ec.insert_final_newline = v;
        explicit.insert("insert_final_newline".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "atomic_save") {
        ec.atomic_save = v;
        explicit.insert("atomic_save".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "trim_final_newlines") {
        ec.trim_final_newlines = v;
        explicit.insert("trim_final_newlines".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "trim_trailing_whitespace") {
        ec.trim_trailing_whitespace = v;
        explicit.insert("trim_trailing_whitespace".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "editor_config") {
        ec.editor_config = v;
        explicit.insert("editor_config".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "rainbow_brackets") {
        ec.rainbow_brackets = v;
        explicit.insert("rainbow_brackets".into());
    }
    if let Some(v) = get_opt::<usize>(lua, &table, "text_width") {
        ec.text_width = v;
        explicit.insert("text_width".into());
    }
    if let Some(v) = get_opt::<bool>(lua, &table, "commandline") {
        ec.commandline = v;
        explicit.insert("commandline".into());
    }

    // --- Duration fields (milliseconds) ---

    if let Some(v) = get_opt::<u64>(lua, &table, "idle_timeout") {
        ec.idle_timeout = Duration::from_millis(v);
        explicit.insert("idle_timeout".into());
    }
    if let Some(v) = get_opt::<u64>(lua, &table, "completion_timeout") {
        ec.completion_timeout = Duration::from_millis(v);
        explicit.insert("completion_timeout".into());
    }

    // --- Char fields ---

    if let Some(v) = get_opt_char(lua, &table, "default_yank_register") {
        ec.default_yank_register = v;
        explicit.insert("default_yank_register".into());
    }

    // --- String → Vec<char> ---

    if let Some(v) = get_opt::<String>(lua, &table, "jump_label_alphabet") {
        ec.jump_label_alphabet = v.chars().collect();
        explicit.insert("jump_label_alphabet".into());
    }

    // --- Vec fields ---

    if let Some(v) = get_opt::<Table>(lua, &table, "shell") {
        ec.shell = table_to_string_vec(&v)?;
        explicit.insert("shell".into());
    }
    if let Some(v) = get_opt::<Table>(lua, &table, "rulers") {
        ec.rulers = table_to_u16_vec(&v)?;
        explicit.insert("rulers".into());
    }
    if let Some(v) = get_opt::<Table>(lua, &table, "workspace_lsp_roots") {
        ec.workspace_lsp_roots = table_to_pathbuf_vec(&v)?;
        explicit.insert("workspace_lsp_roots".into());
    }

    // --- String enum fields ---

    if let Some(v) = get_opt::<String>(lua, &table, "line_number") {
        ec.line_number = parse_line_number(&v)?;
        explicit.insert("line_number".into());
    }
    if let Some(v) = get_opt::<String>(lua, &table, "bufferline") {
        ec.bufferline = parse_bufferline(&v)?;
        explicit.insert("bufferline".into());
    }
    if let Some(v) = get_opt::<String>(lua, &table, "popup_border") {
        ec.popup_border = parse_popup_border(&v)?;
        explicit.insert("popup_border".into());
    }
    if let Some(v) = get_opt::<String>(lua, &table, "default_line_ending") {
        ec.default_line_ending = parse_line_ending(&v)?;
        explicit.insert("default_line_ending".into());
    }
    if let Some(v) = get_opt::<String>(lua, &table, "indent_heuristic") {
        ec.indent_heuristic = parse_indent_heuristic(&v)?;
        explicit.insert("indent_heuristic".into());
    }
    if let Some(v) = get_opt::<String>(lua, &table, "end_of_line_diagnostics") {
        ec.end_of_line_diagnostics = parse_diagnostic_filter(&v)?;
        explicit.insert("end_of_line_diagnostics".into());
    }
    if let Some(v) = get_opt::<String>(lua, &table, "clipboard_provider") {
        ec.clipboard_provider = parse_clipboard_provider(&v)?;
        explicit.insert("clipboard_provider".into());
    }
    if let Some(v) = get_opt::<String>(lua, &table, "kitty_keyboard_protocol") {
        ec.kitty_keyboard_protocol = parse_kitty_keyboard_protocol(&v)?;
        explicit.insert("kitty_keyboard_protocol".into());
    }

    // --- Special fields (bool or table) ---

    // auto_pairs: true/false or table of char pairs
    match table.get::<Value>("auto_pairs") {
        Ok(Value::Boolean(b)) => {
            ec.auto_pairs = AutoPairConfig::Enable(b);
            explicit.insert("auto_pairs".into());
        }
        Ok(Value::Table(t)) => {
            ec.auto_pairs = extract_auto_pairs(lua, &t)?;
            explicit.insert("auto_pairs".into());
        }
        _ => {}
    }

    // gutters: array of names or full config table
    if let Ok(Value::Table(t)) = table.get::<Value>("gutters") {
        ec.gutters = extract_gutters(lua, &t)?;
        explicit.insert("gutters".into());
    }

    // smart_tab: nil (None), false (None), true (Some(default)), or table
    match table.get::<Value>("smart_tab") {
        Ok(Value::Boolean(false)) | Ok(Value::Nil) => {
            // Only set explicit if it was actually `false`, not nil
            if matches!(table.get::<Value>("smart_tab"), Ok(Value::Boolean(false))) {
                ec.smart_tab = None;
                explicit.insert("smart_tab".into());
            }
        }
        Ok(Value::Boolean(true)) => {
            ec.smart_tab = Some(SmartTabConfig::default());
            explicit.insert("smart_tab".into());
        }
        Ok(Value::Table(t)) => {
            ec.smart_tab = Some(extract_smart_tab(lua, &t)?);
            explicit.insert("smart_tab".into());
        }
        _ => {}
    }

    // auto_save: table with after_delay and focus_lost
    if let Some(t) = get_opt::<Table>(lua, &table, "auto_save") {
        ec.auto_save = extract_auto_save(lua, &t)?;
        explicit.insert("auto_save".into());
    }

    // word_completion: table with enable and trigger_length
    if let Some(t) = get_opt::<Table>(lua, &table, "word_completion") {
        ec.word_completion = extract_word_completion(lua, &t)?;
        explicit.insert("word_completion".into());
    }

    // --- Nested table fields ---

    if let Some(t) = get_opt::<Table>(lua, &table, "file_picker") {
        ec.file_picker = extract_file_picker(lua, &t)?;
        explicit.insert("file_picker".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "file_explorer") {
        ec.file_explorer = extract_file_explorer(lua, &t)?;
        explicit.insert("file_explorer".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "statusline") {
        ec.statusline = extract_statusline(lua, &t)?;
        explicit.insert("statusline".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "cursor_shape") {
        ec.cursor_shape = extract_cursor_shape(lua, &t)?;
        explicit.insert("cursor_shape".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "search") {
        ec.search = extract_search(lua, &t)?;
        explicit.insert("search".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "lsp") {
        ec.lsp = extract_lsp(lua, &t)?;
        explicit.insert("lsp".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "whitespace") {
        ec.whitespace = extract_whitespace(lua, &t)?;
        explicit.insert("whitespace".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "indent_guides") {
        ec.indent_guides = extract_indent_guides(lua, &t)?;
        explicit.insert("indent_guides".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "soft_wrap") {
        ec.soft_wrap = extract_soft_wrap(lua, &t)?;
        explicit.insert("soft_wrap".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "inline_diagnostics") {
        ec.inline_diagnostics = extract_inline_diagnostics(lua, &t)?;
        explicit.insert("inline_diagnostics".into());
    }
    if let Some(t) = get_opt::<Table>(lua, &table, "buffer_picker") {
        ec.buffer_picker = extract_buffer_picker(lua, &t)?;
        explicit.insert("buffer_picker".into());
    }

    // terminal: nil or table
    match table.get::<Value>("terminal") {
        Ok(Value::Table(t)) => {
            ec.terminal = Some(extract_terminal_config(lua, &t)?);
            explicit.insert("terminal".into());
        }
        Ok(Value::Boolean(false)) => {
            ec.terminal = None;
            explicit.insert("terminal".into());
        }
        _ => {}
    }

    // Warn about unknown fields
    warn_unknown_fields(&table);

    Ok(LuaEditorConfig {
        config: ec,
        explicit_fields: explicit,
    })
}

// ---------------------------------------------------------------------------
// Enum parsers
// ---------------------------------------------------------------------------

fn parse_line_number(s: &str) -> Result<LineNumber, mlua::Error> {
    match s {
        "absolute" | "abs" => Ok(LineNumber::Absolute),
        "relative" | "rel" => Ok(LineNumber::Relative),
        other => Err(mlua::Error::runtime(format!(
            "invalid line_number: '{other}' (expected 'absolute' or 'relative')"
        ))),
    }
}

fn parse_bufferline(s: &str) -> Result<BufferLine, mlua::Error> {
    match s {
        "never" => Ok(BufferLine::Never),
        "always" => Ok(BufferLine::Always),
        "multiple" => Ok(BufferLine::Multiple),
        other => Err(mlua::Error::runtime(format!(
            "invalid bufferline: '{other}' (expected 'never', 'always', or 'multiple')"
        ))),
    }
}

fn parse_popup_border(s: &str) -> Result<PopupBorderConfig, mlua::Error> {
    match s {
        "none" => Ok(PopupBorderConfig::None),
        "all" => Ok(PopupBorderConfig::All),
        "popup" => Ok(PopupBorderConfig::Popup),
        "menu" => Ok(PopupBorderConfig::Menu),
        other => Err(mlua::Error::runtime(format!(
            "invalid popup_border: '{other}' (expected 'none', 'all', 'popup', or 'menu')"
        ))),
    }
}

fn parse_line_ending(s: &str) -> Result<LineEndingConfig, mlua::Error> {
    match s {
        "native" => Ok(LineEndingConfig::Native),
        "lf" => Ok(LineEndingConfig::LF),
        "crlf" => Ok(LineEndingConfig::Crlf),
        other => Err(mlua::Error::runtime(format!(
            "invalid default_line_ending: '{other}' (expected 'native', 'lf', or 'crlf')"
        ))),
    }
}

fn parse_indent_heuristic(s: &str) -> Result<IndentationHeuristic, mlua::Error> {
    match s {
        "simple" => Ok(IndentationHeuristic::Simple),
        "tree-sitter" => Ok(IndentationHeuristic::TreeSitter),
        "hybrid" => Ok(IndentationHeuristic::Hybrid),
        other => Err(mlua::Error::runtime(format!(
            "invalid indent_heuristic: '{other}' (expected 'simple', 'tree-sitter', or 'hybrid')"
        ))),
    }
}

fn parse_diagnostic_filter(s: &str) -> Result<DiagnosticFilter, mlua::Error> {
    match s {
        "disable" => Ok(DiagnosticFilter::Disable),
        "hint" => Ok(DiagnosticFilter::Enable(Severity::Hint)),
        "info" => Ok(DiagnosticFilter::Enable(Severity::Info)),
        "warning" => Ok(DiagnosticFilter::Enable(Severity::Warning)),
        "error" => Ok(DiagnosticFilter::Enable(Severity::Error)),
        other => Err(mlua::Error::runtime(format!(
            "invalid diagnostic filter: '{other}' (expected 'disable', 'hint', 'info', 'warning', or 'error')"
        ))),
    }
}

fn parse_clipboard_provider(s: &str) -> Result<ClipboardProvider, mlua::Error> {
    match s {
        "pasteboard" => Ok(ClipboardProvider::Pasteboard),
        "wayland" => Ok(ClipboardProvider::Wayland),
        "xclip" => Ok(ClipboardProvider::XClip),
        "xsel" => Ok(ClipboardProvider::XSel),
        "win32-yank" => Ok(ClipboardProvider::Win32Yank),
        "tmux" => Ok(ClipboardProvider::Tmux),
        "termux" => Ok(ClipboardProvider::Termux),
        "none" => Ok(ClipboardProvider::None),
        other => Err(mlua::Error::runtime(format!(
            "invalid clipboard_provider: '{other}' (expected 'pasteboard', 'wayland', 'xclip', 'xsel', 'win32-yank', 'tmux', 'termux', or 'none')"
        ))),
    }
}

fn parse_cursor_kind(s: &str) -> Result<CursorKind, mlua::Error> {
    match s {
        "block" => Ok(CursorKind::Block),
        "bar" => Ok(CursorKind::Bar),
        "underline" => Ok(CursorKind::Underline),
        "hidden" => Ok(CursorKind::Hidden),
        other => Err(mlua::Error::runtime(format!(
            "invalid cursor shape: '{other}' (expected 'block', 'bar', 'underline', or 'hidden')"
        ))),
    }
}

fn parse_gutter_type(s: &str) -> Result<GutterType, mlua::Error> {
    match s {
        "diagnostics" => Ok(GutterType::Diagnostics),
        "line-numbers" => Ok(GutterType::LineNumbers),
        "spacer" => Ok(GutterType::Spacer),
        "diff" => Ok(GutterType::Diff),
        other => Err(mlua::Error::runtime(format!(
            "invalid gutter type: '{other}' (expected 'diagnostics', 'line-numbers', 'spacer', or 'diff')"
        ))),
    }
}

fn parse_statusline_element(s: &str) -> Result<StatusLineElement, mlua::Error> {
    match s {
        "mode" => Ok(StatusLineElement::Mode),
        "spinner" => Ok(StatusLineElement::Spinner),
        "file-base-name" => Ok(StatusLineElement::FileBaseName),
        "file-name" => Ok(StatusLineElement::FileName),
        "file-absolute-path" => Ok(StatusLineElement::FileAbsolutePath),
        "file-modification-indicator" => Ok(StatusLineElement::FileModificationIndicator),
        "read-only-indicator" => Ok(StatusLineElement::ReadOnlyIndicator),
        "file-encoding" => Ok(StatusLineElement::FileEncoding),
        "file-line-ending" => Ok(StatusLineElement::FileLineEnding),
        "file-indent-style" => Ok(StatusLineElement::FileIndentStyle),
        "file-type" => Ok(StatusLineElement::FileType),
        "diagnostics" => Ok(StatusLineElement::Diagnostics),
        "workspace-diagnostics" => Ok(StatusLineElement::WorkspaceDiagnostics),
        "selections" => Ok(StatusLineElement::Selections),
        "primary-selection-length" => Ok(StatusLineElement::PrimarySelectionLength),
        "position" => Ok(StatusLineElement::Position),
        "separator" => Ok(StatusLineElement::Separator),
        "position-percentage" => Ok(StatusLineElement::PositionPercentage),
        "total-line-numbers" => Ok(StatusLineElement::TotalLineNumbers),
        "spacer" => Ok(StatusLineElement::Spacer),
        "version-control" => Ok(StatusLineElement::VersionControl),
        "register" => Ok(StatusLineElement::Register),
        "current-working-directory" => Ok(StatusLineElement::CurrentWorkingDirectory),
        other => Err(mlua::Error::runtime(format!(
            "invalid statusline element: '{other}'"
        ))),
    }
}

fn parse_whitespace_render_value(s: &str) -> Result<WhitespaceRenderValue, mlua::Error> {
    match s {
        "none" => Ok(WhitespaceRenderValue::None),
        "all" => Ok(WhitespaceRenderValue::All),
        other => Err(mlua::Error::runtime(format!(
            "invalid whitespace render: '{other}' (expected 'none' or 'all')"
        ))),
    }
}

fn parse_severity(s: &str) -> Result<Severity, mlua::Error> {
    match s {
        "hint" => Ok(Severity::Hint),
        "info" => Ok(Severity::Info),
        "warning" => Ok(Severity::Warning),
        "error" => Ok(Severity::Error),
        other => Err(mlua::Error::runtime(format!(
            "invalid severity: '{other}' (expected 'hint', 'info', 'warning', or 'error')"
        ))),
    }
}

fn parse_kitty_keyboard_protocol(s: &str) -> Result<KittyKeyboardProtocolConfig, mlua::Error> {
    match s {
        "auto" => Ok(KittyKeyboardProtocolConfig::Auto),
        "disabled" => Ok(KittyKeyboardProtocolConfig::Disabled),
        "enabled" => Ok(KittyKeyboardProtocolConfig::Enabled),
        other => Err(mlua::Error::runtime(format!(
            "invalid kitty_keyboard_protocol: '{other}' (expected 'auto', 'disabled', or 'enabled')"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Nested struct extractors
// ---------------------------------------------------------------------------

fn extract_auto_pairs(_lua: &Lua, table: &Table) -> Result<AutoPairConfig, mlua::Error> {
    let mut pairs = HashMap::new();
    for pair in table.pairs::<String, String>() {
        let (k, v) = pair?;
        let open = k
            .chars()
            .next()
            .ok_or_else(|| mlua::Error::runtime("auto_pairs key must be a single character"))?;
        let close = v
            .chars()
            .next()
            .ok_or_else(|| mlua::Error::runtime("auto_pairs value must be a single character"))?;
        pairs.insert(open, close);
    }
    Ok(AutoPairConfig::Pairs(pairs))
}

fn extract_gutters(lua: &Lua, table: &Table) -> Result<GutterConfig, mlua::Error> {
    if is_sequence(table) {
        // Array of gutter name strings
        let mut layout = Vec::new();
        for val in table.sequence_values::<String>() {
            layout.push(parse_gutter_type(&val?)?);
        }
        Ok(GutterConfig::from(layout))
    } else {
        // Full config table: { layout = {...}, line_numbers = { min_width = N } }
        let mut gc = GutterConfig::default();
        if let Some(layout_table) = get_opt::<Table>(lua, table, "layout") {
            let mut layout = Vec::new();
            for val in layout_table.sequence_values::<String>() {
                layout.push(parse_gutter_type(&val?)?);
            }
            gc.layout = layout;
        }
        if let Some(ln_table) = get_opt::<Table>(lua, table, "line_numbers") {
            if let Some(mw) = get_opt::<usize>(lua, &ln_table, "min_width") {
                gc.line_numbers = GutterLineNumbersConfig { min_width: mw };
            }
        }
        Ok(gc)
    }
}

fn extract_smart_tab(lua: &Lua, table: &Table) -> Result<SmartTabConfig, mlua::Error> {
    let mut st = SmartTabConfig::default();
    if let Some(v) = get_opt::<bool>(lua, table, "enable") {
        st.enable = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "supersede_menu") {
        st.supersede_menu = v;
    }
    Ok(st)
}

fn extract_auto_save(lua: &Lua, table: &Table) -> Result<AutoSave, mlua::Error> {
    let mut auto_save = AutoSave::default();
    if let Some(v) = get_opt::<bool>(lua, table, "focus_lost") {
        auto_save.focus_lost = v;
    }
    if let Some(t) = get_opt::<Table>(lua, table, "after_delay") {
        let mut ad = AutoSaveAfterDelay::default();
        if let Some(v) = get_opt::<bool>(lua, &t, "enable") {
            ad.enable = v;
        }
        if let Some(v) = get_opt::<u64>(lua, &t, "timeout") {
            ad.timeout = v;
        }
        auto_save.after_delay = ad;
    }
    Ok(auto_save)
}

fn extract_word_completion(lua: &Lua, table: &Table) -> Result<WordCompletion, mlua::Error> {
    let mut wc = WordCompletion::default();
    if let Some(v) = get_opt::<bool>(lua, table, "enable") {
        wc.enable = v;
    }
    if let Some(v) = get_opt::<u8>(lua, table, "trigger_length") {
        wc.trigger_length =
            NonZeroU8::new(v).ok_or_else(|| mlua::Error::runtime("trigger_length must be > 0"))?;
    }
    Ok(wc)
}

fn extract_file_picker(lua: &Lua, table: &Table) -> Result<FilePickerConfig, mlua::Error> {
    let mut fp = FilePickerConfig::default();
    if let Some(v) = get_opt::<bool>(lua, table, "hidden") {
        fp.hidden = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "follow_symlinks") {
        fp.follow_symlinks = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "deduplicate_links") {
        fp.deduplicate_links = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "parents") {
        fp.parents = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "ignore") {
        fp.ignore = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "git_ignore") {
        fp.git_ignore = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "git_global") {
        fp.git_global = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "git_exclude") {
        fp.git_exclude = v;
    }
    // max_depth: nil → keep default (Some(24)), explicit integer → set it
    match table.get::<Value>("max_depth") {
        Ok(Value::Integer(n)) => {
            fp.max_depth = if n <= 0 {
                None
            } else {
                Some(n as usize)
            };
        }
        Ok(Value::Boolean(false)) | Ok(Value::Nil) => {
            // Check if explicitly set to false (meaning no limit)
            if matches!(table.get::<Value>("max_depth"), Ok(Value::Boolean(false))) {
                fp.max_depth = None;
            }
        }
        _ => {}
    }
    Ok(fp)
}

fn extract_file_explorer(lua: &Lua, table: &Table) -> Result<FileExplorerConfig, mlua::Error> {
    let mut fe = FileExplorerConfig::default();
    if let Some(v) = get_opt::<bool>(lua, table, "hidden") {
        fe.hidden = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "follow_symlinks") {
        fe.follow_symlinks = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "parents") {
        fe.parents = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "ignore") {
        fe.ignore = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "git_ignore") {
        fe.git_ignore = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "git_global") {
        fe.git_global = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "git_exclude") {
        fe.git_exclude = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "flatten_dirs") {
        fe.flatten_dirs = v;
    }
    Ok(fe)
}

fn extract_statusline_elements(table: &Table) -> Result<Vec<StatusLineElement>, mlua::Error> {
    let mut elems = Vec::new();
    for val in table.sequence_values::<String>() {
        elems.push(parse_statusline_element(&val?)?);
    }
    Ok(elems)
}

fn extract_severity_vec(table: &Table) -> Result<Vec<Severity>, mlua::Error> {
    let mut sevs = Vec::new();
    for val in table.sequence_values::<String>() {
        sevs.push(parse_severity(&val?)?);
    }
    Ok(sevs)
}

fn extract_statusline(lua: &Lua, table: &Table) -> Result<StatusLineConfig, mlua::Error> {
    let mut sl = StatusLineConfig::default();
    if let Some(t) = get_opt::<Table>(lua, table, "left") {
        sl.left = extract_statusline_elements(&t)?;
    }
    if let Some(t) = get_opt::<Table>(lua, table, "center") {
        sl.center = extract_statusline_elements(&t)?;
    }
    if let Some(t) = get_opt::<Table>(lua, table, "right") {
        sl.right = extract_statusline_elements(&t)?;
    }
    if let Some(v) = get_opt::<String>(lua, table, "separator") {
        sl.separator = v;
    }
    if let Some(t) = get_opt::<Table>(lua, table, "mode") {
        sl.mode = extract_mode_config(lua, &t)?;
    }
    if let Some(t) = get_opt::<Table>(lua, table, "diagnostics") {
        sl.diagnostics = extract_severity_vec(&t)?;
    }
    if let Some(t) = get_opt::<Table>(lua, table, "workspace_diagnostics") {
        sl.workspace_diagnostics = extract_severity_vec(&t)?;
    }
    Ok(sl)
}

fn extract_mode_config(lua: &Lua, table: &Table) -> Result<ModeConfig, mlua::Error> {
    let mut mc = ModeConfig::default();
    if let Some(v) = get_opt::<String>(lua, table, "normal") {
        mc.normal = v;
    }
    if let Some(v) = get_opt::<String>(lua, table, "insert") {
        mc.insert = v;
    }
    if let Some(v) = get_opt::<String>(lua, table, "select") {
        mc.select = v;
    }
    Ok(mc)
}

fn extract_cursor_shape(lua: &Lua, table: &Table) -> Result<CursorShapeConfig, mlua::Error> {
    let normal = match get_opt::<String>(lua, table, "normal") {
        Some(s) => parse_cursor_kind(&s)?,
        None => CursorKind::default(),
    };
    let insert = match get_opt::<String>(lua, table, "insert") {
        Some(s) => parse_cursor_kind(&s)?,
        None => CursorKind::default(),
    };
    let select = match get_opt::<String>(lua, table, "select") {
        Some(s) => parse_cursor_kind(&s)?,
        None => CursorKind::default(),
    };
    // CursorShapeConfig([normal, select, insert]) — indexed by Mode enum order
    Ok(CursorShapeConfig::from_modes(normal, select, insert))
}

fn extract_search(lua: &Lua, table: &Table) -> Result<SearchConfig, mlua::Error> {
    let mut sc = SearchConfig::default();
    if let Some(v) = get_opt::<bool>(lua, table, "smart_case") {
        sc.smart_case = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "wrap_around") {
        sc.wrap_around = v;
    }
    Ok(sc)
}

fn extract_lsp(lua: &Lua, table: &Table) -> Result<LspConfig, mlua::Error> {
    let mut lsp = LspConfig::default();
    if let Some(v) = get_opt::<bool>(lua, table, "enable") {
        lsp.enable = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "display_progress_messages") {
        lsp.display_progress_messages = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "display_messages") {
        lsp.display_messages = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "auto_signature_help") {
        lsp.auto_signature_help = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "display_signature_help_docs") {
        lsp.display_signature_help_docs = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "display_inlay_hints") {
        lsp.display_inlay_hints = v;
    }
    if let Some(v) = get_opt::<u8>(lua, table, "inlay_hints_length_limit") {
        lsp.inlay_hints_length_limit = NonZeroU8::new(v);
    }
    if let Some(v) = get_opt::<bool>(lua, table, "display_color_swatches") {
        lsp.display_color_swatches = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "snippets") {
        lsp.snippets = v;
    }
    if let Some(v) = get_opt::<bool>(lua, table, "goto_reference_include_declaration") {
        lsp.goto_reference_include_declaration = v;
    }
    Ok(lsp)
}

fn extract_terminal_config(lua: &Lua, table: &Table) -> Result<TerminalConfig, mlua::Error> {
    let mut tc = TerminalConfig::default();
    if let Some(v) = get_opt::<String>(lua, table, "command") {
        tc.command = v;
    }
    if let Some(t) = get_opt::<Table>(lua, table, "args") {
        tc.args = table_to_string_vec(&t)?;
    }
    Ok(tc)
}

fn extract_whitespace(lua: &Lua, table: &Table) -> Result<WhitespaceConfig, mlua::Error> {
    let mut ws = WhitespaceConfig::default();

    // render: string ("none"/"all") or table { default = ..., space = ..., ... }
    match table.get::<Value>("render") {
        Ok(Value::String(s)) => {
            let s = s.to_str()?.to_owned();
            ws.render = WhitespaceRender::Basic(parse_whitespace_render_value(&s)?);
        }
        Ok(Value::Table(t)) => {
            let default = match get_opt::<String>(lua, &t, "default") {
                Some(s) => Some(parse_whitespace_render_value(&s)?),
                None => None,
            };
            let space = match get_opt::<String>(lua, &t, "space") {
                Some(s) => Some(parse_whitespace_render_value(&s)?),
                None => None,
            };
            let nbsp = match get_opt::<String>(lua, &t, "nbsp") {
                Some(s) => Some(parse_whitespace_render_value(&s)?),
                None => None,
            };
            let nnbsp = match get_opt::<String>(lua, &t, "nnbsp") {
                Some(s) => Some(parse_whitespace_render_value(&s)?),
                None => None,
            };
            let tab = match get_opt::<String>(lua, &t, "tab") {
                Some(s) => Some(parse_whitespace_render_value(&s)?),
                None => None,
            };
            let newline = match get_opt::<String>(lua, &t, "newline") {
                Some(s) => Some(parse_whitespace_render_value(&s)?),
                None => None,
            };
            ws.render = WhitespaceRender::Specific {
                default,
                space,
                nbsp,
                nnbsp,
                tab,
                newline,
            };
        }
        _ => {}
    }

    if let Some(t) = get_opt::<Table>(lua, table, "characters") {
        ws.characters = extract_whitespace_characters(lua, &t)?;
    }

    Ok(ws)
}

fn extract_whitespace_characters(
    lua: &Lua,
    table: &Table,
) -> Result<WhitespaceCharacters, mlua::Error> {
    let mut wc = WhitespaceCharacters::default();
    if let Some(c) = get_opt_char(lua, table, "space") {
        wc.space = c;
    }
    if let Some(c) = get_opt_char(lua, table, "nbsp") {
        wc.nbsp = c;
    }
    if let Some(c) = get_opt_char(lua, table, "nnbsp") {
        wc.nnbsp = c;
    }
    if let Some(c) = get_opt_char(lua, table, "tab") {
        wc.tab = c;
    }
    if let Some(c) = get_opt_char(lua, table, "tabpad") {
        wc.tabpad = c;
    }
    if let Some(c) = get_opt_char(lua, table, "newline") {
        wc.newline = c;
    }
    Ok(wc)
}

fn extract_indent_guides(lua: &Lua, table: &Table) -> Result<IndentGuidesConfig, mlua::Error> {
    let mut ig = IndentGuidesConfig::default();
    if let Some(v) = get_opt::<bool>(lua, table, "render") {
        ig.render = v;
    }
    if let Some(c) = get_opt_char(lua, table, "character") {
        ig.character = c;
    }
    if let Some(v) = get_opt::<u8>(lua, table, "skip_levels") {
        ig.skip_levels = v;
    }
    Ok(ig)
}

fn extract_soft_wrap(lua: &Lua, table: &Table) -> Result<SoftWrap, mlua::Error> {
    let mut sw = SoftWrap::default();
    if let Some(v) = get_opt::<bool>(lua, table, "enable") {
        sw.enable = Some(v);
    }
    if let Some(v) = get_opt::<u16>(lua, table, "max_wrap") {
        sw.max_wrap = Some(v);
    }
    if let Some(v) = get_opt::<u16>(lua, table, "max_indent_retain") {
        sw.max_indent_retain = Some(v);
    }
    if let Some(v) = get_opt::<String>(lua, table, "wrap_indicator") {
        sw.wrap_indicator = Some(v);
    }
    if let Some(v) = get_opt::<bool>(lua, table, "wrap_at_text_width") {
        sw.wrap_at_text_width = Some(v);
    }
    Ok(sw)
}

fn extract_inline_diagnostics(
    lua: &Lua,
    table: &Table,
) -> Result<InlineDiagnosticsConfig, mlua::Error> {
    let mut id = InlineDiagnosticsConfig::default();
    if let Some(v) = get_opt::<String>(lua, table, "cursor_line") {
        id.cursor_line = parse_diagnostic_filter(&v)?;
    }
    if let Some(v) = get_opt::<String>(lua, table, "other_lines") {
        id.other_lines = parse_diagnostic_filter(&v)?;
    }
    if let Some(v) = get_opt::<u16>(lua, table, "min_diagnostic_width") {
        id.min_diagnostic_width = v;
    }
    if let Some(v) = get_opt::<u16>(lua, table, "prefix_len") {
        id.prefix_len = v;
    }
    if let Some(v) = get_opt::<u16>(lua, table, "max_wrap") {
        id.max_wrap = v;
    }
    if let Some(v) = get_opt::<usize>(lua, table, "max_diagnostics") {
        id.max_diagnostics = v;
    }
    Ok(id)
}

fn extract_buffer_picker(lua: &Lua, table: &Table) -> Result<BufferPickerConfig, mlua::Error> {
    let mut bp = BufferPickerConfig::default();
    if let Some(v) = get_opt::<String>(lua, table, "start_position") {
        bp.start_position = match v.as_str() {
            "current" => PickerStartPosition::Current,
            "previous" => PickerStartPosition::Previous,
            other => {
                return Err(mlua::Error::runtime(format!(
                    "invalid start_position: '{other}' (expected 'current' or 'previous')"
                )))
            }
        };
    }
    Ok(bp)
}

// ---------------------------------------------------------------------------
// Merge support
// ---------------------------------------------------------------------------

/// Copy a single field from `src` to `dst` by name.
pub fn apply_editor_field(dst: &mut EditorConfig, src: &EditorConfig, field: &str) {
    match field {
        "scrolloff" => dst.scrolloff = src.scrolloff,
        "scroll_lines" => dst.scroll_lines = src.scroll_lines,
        "mouse" => dst.mouse = src.mouse,
        "shell" => dst.shell.clone_from(&src.shell),
        "line_number" => dst.line_number = src.line_number,
        "cursorline" => dst.cursorline = src.cursorline,
        "cursorcolumn" => dst.cursorcolumn = src.cursorcolumn,
        "gutters" => dst.gutters = src.gutters.clone(),
        "middle_click_paste" => dst.middle_click_paste = src.middle_click_paste,
        "auto_pairs" => dst.auto_pairs = src.auto_pairs.clone(),
        "auto_completion" => dst.auto_completion = src.auto_completion,
        "path_completion" => dst.path_completion = src.path_completion,
        "word_completion" => dst.word_completion = src.word_completion,
        "auto_format" => dst.auto_format = src.auto_format,
        "default_yank_register" => dst.default_yank_register = src.default_yank_register,
        "auto_save" => dst.auto_save = src.auto_save.clone(),
        "text_width" => dst.text_width = src.text_width,
        "idle_timeout" => dst.idle_timeout = src.idle_timeout,
        "completion_timeout" => dst.completion_timeout = src.completion_timeout,
        "preview_completion_insert" => {
            dst.preview_completion_insert = src.preview_completion_insert
        }
        "completion_trigger_len" => dst.completion_trigger_len = src.completion_trigger_len,
        "completion_replace" => dst.completion_replace = src.completion_replace,
        "continue_comments" => dst.continue_comments = src.continue_comments,
        "auto_info" => dst.auto_info = src.auto_info,
        "file_picker" => dst.file_picker = src.file_picker.clone(),
        "file_explorer" => dst.file_explorer = src.file_explorer.clone(),
        "statusline" => dst.statusline = src.statusline.clone(),
        "cursor_shape" => dst.cursor_shape = src.cursor_shape.clone(),
        "true_color" => dst.true_color = src.true_color,
        "undercurl" => dst.undercurl = src.undercurl,
        "search" => dst.search = src.search.clone(),
        "lsp" => dst.lsp = src.lsp.clone(),
        "terminal" => dst.terminal = src.terminal.clone(),
        "rulers" => dst.rulers.clone_from(&src.rulers),
        "whitespace" => dst.whitespace = src.whitespace.clone(),
        "bufferline" => dst.bufferline = src.bufferline.clone(),
        "indent_guides" => dst.indent_guides = src.indent_guides.clone(),
        "color_modes" => dst.color_modes = src.color_modes,
        "soft_wrap" => dst.soft_wrap = src.soft_wrap.clone(),
        "workspace_lsp_roots" => dst.workspace_lsp_roots.clone_from(&src.workspace_lsp_roots),
        "default_line_ending" => dst.default_line_ending = src.default_line_ending,
        "insert_final_newline" => dst.insert_final_newline = src.insert_final_newline,
        "atomic_save" => dst.atomic_save = src.atomic_save,
        "trim_final_newlines" => dst.trim_final_newlines = src.trim_final_newlines,
        "trim_trailing_whitespace" => {
            dst.trim_trailing_whitespace = src.trim_trailing_whitespace
        }
        "smart_tab" => dst.smart_tab = src.smart_tab.clone(),
        "popup_border" => dst.popup_border = src.popup_border.clone(),
        "indent_heuristic" => dst.indent_heuristic = src.indent_heuristic.clone(),
        "jump_label_alphabet" => dst.jump_label_alphabet.clone_from(&src.jump_label_alphabet),
        "inline_diagnostics" => dst.inline_diagnostics = src.inline_diagnostics.clone(),
        "end_of_line_diagnostics" => dst.end_of_line_diagnostics = src.end_of_line_diagnostics,
        "clipboard_provider" => dst.clipboard_provider = src.clipboard_provider.clone(),
        "editor_config" => dst.editor_config = src.editor_config,
        "rainbow_brackets" => dst.rainbow_brackets = src.rainbow_brackets,
        "kitty_keyboard_protocol" => dst.kitty_keyboard_protocol = src.kitty_keyboard_protocol,
        "buffer_picker" => dst.buffer_picker = src.buffer_picker,
        "commandline" => dst.commandline = src.commandline,
        _ => log::warn!("unknown config field in merge: {field}"),
    }
}

// ---------------------------------------------------------------------------
// Unknown field detection
// ---------------------------------------------------------------------------

fn warn_unknown_fields(table: &Table) {
    let known: HashSet<&str> = KNOWN_FIELDS.iter().copied().collect();
    if let Ok(pairs) = table.pairs::<String, Value>().collect::<Result<Vec<_>, _>>() {
        for (key, _) in pairs {
            if !known.contains(key.as_str()) {
                log::warn!("unknown config field: si.config.{key}");
            }
        }
    }
}
