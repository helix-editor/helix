use crate::{
    annotations::diagnostics::{DiagnosticFilter, InlineDiagnosticsConfig},
    clipboard::ClipboardProvider,
    document::{
        DocumentOpenError, DocumentSavedEventFuture, DocumentSavedEventResult, Mode, SavePoint,
    },
    events::{DocumentDidClose, DocumentDidOpen, DocumentFocusLost},
    graphics::{CursorKind, Rect},
    handlers::Handlers,
    info::Info,
    input::KeyEvent,
    register::Registers,
    theme::{self, Theme},
    tree::{self, Tree},
    Document, DocumentId, View, ViewId,
};
use helix_event::dispatch;
use helix_vcs::DiffProviderRegistry;

use futures_util::stream::select_all::SelectAll;
use futures_util::{future, StreamExt};
use helix_lsp::{Call, LanguageServerId};
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::{
    borrow::Cow,
    cell::Cell,
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{self, stdin},
    num::{NonZeroU8, NonZeroUsize},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::{sleep, Duration, Instant, Sleep},
};

use anyhow::{anyhow, bail, Error};

pub use helix_core::diagnostic::Severity;
use helix_core::{
    auto_pairs::AutoPairs,
    diagnostic::DiagnosticProvider,
    syntax::{
        self,
        config::{AutoPairConfig, IndentationHeuristic, LanguageServerFeature, SoftWrap},
    },
    Change, LineEnding, Position, Range, Selection, Uri, NATIVE_LINE_ENDING,
};
use helix_dap::{self as dap, registry::DebugAdapterId};
use helix_lsp::lsp;
use helix_stdx::path::canonicalize;

use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};

use arc_swap::{
    access::{DynAccess, DynGuard},
    ArcSwap,
};

pub const DEFAULT_AUTO_SAVE_DELAY: u64 = 3000;

fn deserialize_duration_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}

fn serialize_duration_millis<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(
        duration
            .as_millis()
            .try_into()
            .map_err(|_| serde::ser::Error::custom("duration value overflowed u64"))?,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct GutterConfig {
    /// Gutter Layout
    pub layout: Vec<GutterType>,
    /// Options specific to the "line-numbers" gutter
    pub line_numbers: GutterLineNumbersConfig,
}

impl Default for GutterConfig {
    fn default() -> Self {
        Self {
            layout: vec![
                GutterType::Diagnostics,
                GutterType::Spacer,
                GutterType::LineNumbers,
                GutterType::Spacer,
                GutterType::Diff,
            ],
            line_numbers: GutterLineNumbersConfig::default(),
        }
    }
}

impl From<Vec<GutterType>> for GutterConfig {
    fn from(x: Vec<GutterType>) -> Self {
        GutterConfig {
            layout: x,
            ..Default::default()
        }
    }
}

fn deserialize_gutter_seq_or_struct<'de, D>(deserializer: D) -> Result<GutterConfig, D::Error>
where
    D: Deserializer<'de>,
{
    struct GutterVisitor;

    impl<'de> serde::de::Visitor<'de> for GutterVisitor {
        type Value = GutterConfig;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "an array of gutter names or a detailed gutter configuration"
            )
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
        where
            S: serde::de::SeqAccess<'de>,
        {
            let mut gutters = Vec::new();
            while let Some(gutter) = seq.next_element::<String>()? {
                gutters.push(
                    gutter
                        .parse::<GutterType>()
                        .map_err(serde::de::Error::custom)?,
                )
            }

            Ok(gutters.into())
        }

        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let deserializer = serde::de::value::MapAccessDeserializer::new(map);
            Deserialize::deserialize(deserializer)
        }
    }

    deserializer.deserialize_any(GutterVisitor)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct GutterLineNumbersConfig {
    /// Minimum number of characters to use for line number gutter. Defaults to 3.
    pub min_width: usize,
}

impl Default for GutterLineNumbersConfig {
    fn default() -> Self {
        Self { min_width: 3 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct FilePickerConfig {
    /// IgnoreOptions
    /// Enables ignoring hidden files.
    /// Whether to hide hidden files in file picker and global search results. Defaults to true.
    pub hidden: bool,
    /// Enables following symlinks.
    /// Whether to follow symbolic links in file picker and file or directory completions. Defaults to true.
    pub follow_symlinks: bool,
    /// Hides symlinks that point into the current directory. Defaults to true.
    pub deduplicate_links: bool,
    /// Enables reading ignore files from parent directories. Defaults to true.
    pub parents: bool,
    /// Enables reading `.ignore` files.
    /// Whether to hide files listed in .ignore in file picker and global search results. Defaults to true.
    pub ignore: bool,
    /// Enables reading `.gitignore` files.
    /// Whether to hide files listed in .gitignore in file picker and global search results. Defaults to true.
    pub git_ignore: bool,
    /// Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option.
    /// Whether to hide files listed in global .gitignore in file picker and global search results. Defaults to true.
    pub git_global: bool,
    /// Enables reading `.git/info/exclude` files.
    /// Whether to hide files listed in .git/info/exclude in file picker and global search results. Defaults to true.
    pub git_exclude: bool,
    /// WalkBuilder options
    /// Maximum Depth to recurse directories in file picker and global search. Defaults to `None`.
    pub max_depth: Option<usize>,
}

impl Default for FilePickerConfig {
    fn default() -> Self {
        Self {
            hidden: true,
            follow_symlinks: true,
            deduplicate_links: true,
            parents: true,
            ignore: true,
            git_ignore: true,
            git_global: true,
            git_exclude: true,
            max_depth: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct FileExplorerConfig {
    /// IgnoreOptions
    /// Enables ignoring hidden files.
    /// Whether to hide hidden files in file explorer and global search results. Defaults to false.
    pub hidden: bool,
    /// Enables following symlinks.
    /// Whether to follow symbolic links in file picker and file or directory completions. Defaults to false.
    pub follow_symlinks: bool,
    /// Enables reading ignore files from parent directories. Defaults to false.
    pub parents: bool,
    /// Enables reading `.ignore` files.
    /// Whether to hide files listed in .ignore in file picker and global search results. Defaults to false.
    pub ignore: bool,
    /// Enables reading `.gitignore` files.
    /// Whether to hide files listed in .gitignore in file picker and global search results. Defaults to false.
    pub git_ignore: bool,
    /// Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option.
    /// Whether to hide files listed in global .gitignore in file picker and global search results. Defaults to false.
    pub git_global: bool,
    /// Enables reading `.git/info/exclude` files.
    /// Whether to hide files listed in .git/info/exclude in file picker and global search results. Defaults to false.
    pub git_exclude: bool,
    /// Whether to flatten single-child directories in file explorer. Defaults to true.
    pub flatten_dirs: bool,
}

impl Default for FileExplorerConfig {
    fn default() -> Self {
        Self {
            hidden: false,
            follow_symlinks: false,
            parents: false,
            ignore: false,
            git_ignore: false,
            git_global: false,
            git_exclude: false,
            flatten_dirs: true,
        }
    }
}

fn serialize_alphabet<S>(alphabet: &[char], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let alphabet: String = alphabet.iter().collect();
    serializer.serialize_str(&alphabet)
}

fn deserialize_alphabet<'de, D>(deserializer: D) -> Result<Vec<char>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let str = String::deserialize(deserializer)?;
    let chars: Vec<_> = str.chars().collect();
    let unique_chars: HashSet<_> = chars.iter().copied().collect();
    if unique_chars.len() != chars.len() {
        return Err(<D::Error as Error>::custom(
            "jump-label-alphabet must contain unique characters",
        ));
    }
    Ok(chars)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    /// Padding to keep between the edge of the screen and the cursor when scrolling. Defaults to 5.
    pub scrolloff: usize,
    /// Number of lines to scroll at once. Defaults to 3
    pub scroll_lines: isize,
    /// Mouse support. Defaults to true.
    pub mouse: bool,
    /// Shell to use for shell commands. Defaults to ["cmd", "/C"] on Windows and ["sh", "-c"] otherwise.
    pub shell: Vec<String>,
    /// Line number mode.
    pub line_number: LineNumber,
    /// Highlight the lines cursors are currently on. Defaults to false.
    pub cursorline: bool,
    /// Highlight the columns cursors are currently on. Defaults to false.
    pub cursorcolumn: bool,
    #[serde(deserialize_with = "deserialize_gutter_seq_or_struct")]
    pub gutters: GutterConfig,
    /// Middle click paste support. Defaults to true.
    pub middle_click_paste: bool,
    /// Automatic insertion of pairs to parentheses, brackets,
    /// etc. Optionally, this can be a list of 2-tuples to specify a
    /// global list of characters to pair. Defaults to true.
    pub auto_pairs: AutoPairConfig,
    /// Automatic auto-completion, automatically pop up without user trigger. Defaults to true.
    pub auto_completion: bool,
    /// Enable filepath completion.
    /// Show files and directories if an existing path at the cursor was recognized,
    /// either absolute or relative to the current opened document or current working directory (if the buffer is not yet saved).
    /// Defaults to true.
    pub path_completion: bool,
    /// Configures completion of words from open buffers.
    /// Defaults to enabled with a trigger length of 7.
    pub word_completion: WordCompletion,
    /// Automatic formatting on save. Defaults to true.
    pub auto_format: bool,
    /// Default register used for yank/paste. Defaults to '"'
    pub default_yank_register: char,
    /// Automatic save on focus lost and/or after delay.
    /// Time delay in milliseconds since last edit after which auto save timer triggers.
    /// Time delay defaults to false with 3000ms delay. Focus lost defaults to false.
    #[serde(deserialize_with = "deserialize_auto_save")]
    pub auto_save: AutoSave,
    /// Set a global text_width
    pub text_width: usize,
    /// Time in milliseconds since last keypress before idle timers trigger.
    /// Used for various UI timeouts. Defaults to 250ms.
    #[serde(
        serialize_with = "serialize_duration_millis",
        deserialize_with = "deserialize_duration_millis"
    )]
    pub idle_timeout: Duration,
    /// Time in milliseconds after typing a word character before auto completions
    /// are shown, set to 5 for instant. Defaults to 250ms.
    #[serde(
        serialize_with = "serialize_duration_millis",
        deserialize_with = "deserialize_duration_millis"
    )]
    pub completion_timeout: Duration,
    /// Whether to insert the completion suggestion on hover. Defaults to true.
    pub preview_completion_insert: bool,
    pub completion_trigger_len: u8,
    /// Whether to instruct the LSP to replace the entire word when applying a completion
    /// or to only insert new text
    pub completion_replace: bool,
    /// `true` if helix should automatically add a line comment token if you're currently in a comment
    /// and press `enter`.
    pub continue_comments: bool,
    /// Whether to display infoboxes. Defaults to true.
    pub auto_info: bool,
    pub file_picker: FilePickerConfig,
    pub file_explorer: FileExplorerConfig,
    /// Configuration of the statusline elements
    pub statusline: StatusLineConfig,
    /// Shape for cursor in each mode
    pub cursor_shape: CursorShapeConfig,
    /// Set to `true` to override automatic detection of terminal truecolor support in the event of a false negative. Defaults to `false`.
    pub true_color: bool,
    /// Set to `true` to override automatic detection of terminal undercurl support in the event of a false negative. Defaults to `false`.
    pub undercurl: bool,
    /// Search configuration.
    #[serde(default)]
    pub search: SearchConfig,
    pub lsp: LspConfig,
    pub terminal: Option<TerminalConfig>,
    /// Column numbers at which to draw the rulers. Defaults to `[]`, meaning no rulers.
    pub rulers: Vec<u16>,
    #[serde(default)]
    pub whitespace: WhitespaceConfig,
    /// Persistently display open buffers along the top
    pub bufferline: BufferLine,
    /// Vertical indent width guides.
    pub indent_guides: IndentGuidesConfig,
    /// Whether to color modes with different colors. Defaults to `false`.
    pub color_modes: bool,
    pub soft_wrap: SoftWrap,
    /// Workspace specific lsp ceiling dirs
    pub workspace_lsp_roots: Vec<PathBuf>,
    /// Which line ending to choose for new documents. Defaults to `native`. i.e. `crlf` on Windows, otherwise `lf`.
    pub default_line_ending: LineEndingConfig,
    /// Whether to automatically insert a trailing line-ending on write if missing. Defaults to `true`.
    pub insert_final_newline: bool,
    /// Whether to use atomic operations to write documents to disk.
    /// This prevents data loss if the editor is interrupted while writing the file, but may
    /// confuse some file watching/hot reloading programs. Defaults to `true`.
    pub atomic_save: bool,
    /// Whether to automatically remove all trailing line-endings after the final one on write.
    /// Defaults to `false`.
    pub trim_final_newlines: bool,
    /// Whether to automatically remove all whitespace characters preceding line-endings on write.
    /// Defaults to `false`.
    pub trim_trailing_whitespace: bool,
    /// Enables smart tab
    pub smart_tab: Option<SmartTabConfig>,
    /// Draw border around popups.
    pub popup_border: PopupBorderConfig,
    /// Draw rounded border corners
    pub rounded_corners: bool,
    /// Which indent heuristic to use when a new line is inserted
    #[serde(default)]
    pub indent_heuristic: IndentationHeuristic,
    /// labels characters used in jumpmode
    #[serde(
        serialize_with = "serialize_alphabet",
        deserialize_with = "deserialize_alphabet"
    )]
    pub jump_label_alphabet: Vec<char>,
    /// Display diagnostic below the line they occur.
    pub inline_diagnostics: InlineDiagnosticsConfig,
    pub end_of_line_diagnostics: DiagnosticFilter,
    // Set to override the default clipboard provider
    pub clipboard_provider: ClipboardProvider,
    /// Whether to read settings from [EditorConfig](https://editorconfig.org) files. Defaults to
    /// `true`.
    pub editor_config: bool,
    /// Whether to render rainbow colors for matching brackets. Defaults to `false`.
    pub rainbow_brackets: bool,
    /// Whether to enable Kitty Keyboard Protocol
    pub kitty_keyboard_protocol: KittyKeyboardProtocolConfig,
    pub buffer_picker: BufferPickerConfig,
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub struct BufferPickerConfig {
    pub start_position: PickerStartPosition,
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum PickerStartPosition {
    #[default]
    Current,
    Previous,
}

impl PickerStartPosition {
    #[must_use]
    pub fn is_previous(self) -> bool {
        matches!(self, Self::Previous)
    }

    #[must_use]
    pub fn is_current(self) -> bool {
        matches!(self, Self::Current)
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum KittyKeyboardProtocolConfig {
    #[default]
    Auto,
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Eq, PartialOrd, Ord)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct SmartTabConfig {
    pub enable: bool,
    pub supersede_menu: bool,
}

impl Default for SmartTabConfig {
    fn default() -> Self {
        SmartTabConfig {
            enable: true,
            supersede_menu: false,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct TerminalConfig {
    pub command: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[cfg(windows)]
pub fn get_terminal_provider() -> Option<TerminalConfig> {
    use helix_stdx::env::binary_exists;

    if binary_exists("wt") {
        return Some(TerminalConfig {
            command: "wt".to_string(),
            args: vec![
                "new-tab".to_string(),
                "--title".to_string(),
                "DEBUG".to_string(),
                "cmd".to_string(),
                "/C".to_string(),
            ],
        });
    }

    Some(TerminalConfig {
        command: "conhost".to_string(),
        args: vec!["cmd".to_string(), "/C".to_string()],
    })
}

#[cfg(not(any(windows, target_arch = "wasm32")))]
pub fn get_terminal_provider() -> Option<TerminalConfig> {
    use helix_stdx::env::{binary_exists, env_var_is_set};

    if env_var_is_set("TMUX") && binary_exists("tmux") {
        return Some(TerminalConfig {
            command: "tmux".to_string(),
            args: vec!["split-window".to_string()],
        });
    }

    if env_var_is_set("WEZTERM_UNIX_SOCKET") && binary_exists("wezterm") {
        return Some(TerminalConfig {
            command: "wezterm".to_string(),
            args: vec!["cli".to_string(), "split-pane".to_string()],
        });
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct LspConfig {
    /// Enables LSP
    pub enable: bool,
    /// Display LSP messagess from $/progress below statusline
    pub display_progress_messages: bool,
    /// Display LSP messages from window/showMessage below statusline
    pub display_messages: bool,
    /// Enable automatic pop up of signature help (parameter hints)
    pub auto_signature_help: bool,
    /// Display docs under signature help popup
    pub display_signature_help_docs: bool,
    /// Display inlay hints
    pub display_inlay_hints: bool,
    /// Maximum displayed length of inlay hints (excluding the added trailing `…`).
    /// If it's `None`, there's no limit
    pub inlay_hints_length_limit: Option<NonZeroU8>,
    /// Display document color swatches
    pub display_color_swatches: bool,
    /// Whether to enable snippet support
    pub snippets: bool,
    /// Whether to include declaration in the goto reference query
    pub goto_reference_include_declaration: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enable: true,
            display_progress_messages: false,
            display_messages: true,
            auto_signature_help: true,
            display_signature_help_docs: true,
            display_inlay_hints: false,
            inlay_hints_length_limit: None,
            snippets: true,
            goto_reference_include_declaration: true,
            display_color_swatches: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct SearchConfig {
    /// Smart case: Case insensitive searching unless pattern contains upper case characters. Defaults to true.
    pub smart_case: bool,
    /// Whether the search should wrap after depleting the matches. Default to true.
    pub wrap_around: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct StatusLineConfig {
    pub left: Vec<StatusLineElement>,
    pub center: Vec<StatusLineElement>,
    pub right: Vec<StatusLineElement>,
    pub separator: String,
    pub mode: ModeConfig,
    pub diagnostics: Vec<Severity>,
    pub workspace_diagnostics: Vec<Severity>,
}

impl Default for StatusLineConfig {
    fn default() -> Self {
        use StatusLineElement as E;

        Self {
            left: vec![
                E::Mode,
                E::Spinner,
                E::FileName,
                E::ReadOnlyIndicator,
                E::FileModificationIndicator,
            ],
            center: vec![],
            right: vec![
                E::Diagnostics,
                E::Selections,
                E::Register,
                E::Position,
                E::FileEncoding,
            ],
            separator: String::from("│"),
            mode: ModeConfig::default(),
            diagnostics: vec![Severity::Warning, Severity::Error],
            workspace_diagnostics: vec![Severity::Warning, Severity::Error],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct ModeConfig {
    pub normal: String,
    pub insert: String,
    pub select: String,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            normal: String::from("NOR"),
            insert: String::from("INS"),
            select: String::from("SEL"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatusLineElement {
    /// The editor mode (Normal, Insert, Visual/Selection)
    Mode,

    /// The LSP activity spinner
    Spinner,

    /// The file basename (the leaf of the open file's path)
    FileBaseName,

    /// The relative file path
    FileName,

    /// The file absolute path
    FileAbsolutePath,

    // The file modification indicator
    FileModificationIndicator,

    /// An indicator that shows `"[readonly]"` when a file cannot be written
    ReadOnlyIndicator,

    /// The file encoding
    FileEncoding,

    /// The file line endings (CRLF or LF)
    FileLineEnding,

    /// The file indentation style
    FileIndentStyle,

    /// The file type (language ID or "text")
    FileType,

    /// A summary of the number of errors and warnings
    Diagnostics,

    /// A summary of the number of errors and warnings on file and workspace
    WorkspaceDiagnostics,

    /// The number of selections (cursors)
    Selections,

    /// The number of characters currently in primary selection
    PrimarySelectionLength,

    /// The cursor position
    Position,

    /// The separator string
    Separator,

    /// The cursor position as a percent of the total file
    PositionPercentage,

    /// The total line numbers of the current file
    TotalLineNumbers,

    /// A single space
    Spacer,

    /// Current version control information
    VersionControl,

    /// Indicator for selected register
    Register,

    /// The base of current working directory
    CurrentWorkingDirectory,
}

// Cursor shape is read and used on every rendered frame and so needs
// to be fast. Therefore we avoid a hashmap and use an enum indexed array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorShapeConfig([CursorKind; 3]);

impl CursorShapeConfig {
    pub fn from_mode(&self, mode: Mode) -> CursorKind {
        self.get(mode as usize).copied().unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for CursorShapeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let m = HashMap::<Mode, CursorKind>::deserialize(deserializer)?;
        let into_cursor = |mode: Mode| m.get(&mode).copied().unwrap_or_default();
        Ok(CursorShapeConfig([
            into_cursor(Mode::Normal),
            into_cursor(Mode::Select),
            into_cursor(Mode::Insert),
        ]))
    }
}

impl Serialize for CursorShapeConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        let modes = [Mode::Normal, Mode::Select, Mode::Insert];
        for mode in modes {
            map.serialize_entry(&mode, &self.from_mode(mode))?;
        }
        map.end()
    }
}

impl std::ops::Deref for CursorShapeConfig {
    type Target = [CursorKind; 3];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for CursorShapeConfig {
    fn default() -> Self {
        Self([CursorKind::Block; 3])
    }
}

/// bufferline render modes
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BufferLine {
    /// Don't render bufferline
    #[default]
    Never,
    /// Always render
    Always,
    /// Only if multiple buffers are open
    Multiple,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineNumber {
    /// Show absolute line number
    Absolute,

    /// If focused and in normal/select mode, show relative line number to the primary cursor.
    /// If unfocused or in insert mode, show absolute line number.
    Relative,
}

impl std::str::FromStr for LineNumber {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "absolute" | "abs" => Ok(Self::Absolute),
            "relative" | "rel" => Ok(Self::Relative),
            _ => anyhow::bail!("Line number can only be `absolute` or `relative`."),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GutterType {
    /// Show diagnostics and other features like breakpoints
    Diagnostics,
    /// Show line numbers
    LineNumbers,
    /// Show one blank space
    Spacer,
    /// Highlight local changes
    Diff,
}

impl std::str::FromStr for GutterType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "diagnostics" => Ok(Self::Diagnostics),
            "spacer" => Ok(Self::Spacer),
            "line-numbers" => Ok(Self::LineNumbers),
            "diff" => Ok(Self::Diff),
            _ => anyhow::bail!(
                "Gutter type can only be `diagnostics`, `spacer`, `line-numbers` or `diff`."
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WhitespaceConfig {
    pub render: WhitespaceRender,
    pub characters: WhitespaceCharacters,
}

impl Default for WhitespaceConfig {
    fn default() -> Self {
        Self {
            render: WhitespaceRender::Basic(WhitespaceRenderValue::None),
            characters: WhitespaceCharacters::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum WhitespaceRender {
    Basic(WhitespaceRenderValue),
    Specific {
        default: Option<WhitespaceRenderValue>,
        space: Option<WhitespaceRenderValue>,
        nbsp: Option<WhitespaceRenderValue>,
        nnbsp: Option<WhitespaceRenderValue>,
        tab: Option<WhitespaceRenderValue>,
        newline: Option<WhitespaceRenderValue>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WhitespaceRenderValue {
    None,
    // TODO
    // Selection,
    All,
}

impl WhitespaceRender {
    pub fn space(&self) -> WhitespaceRenderValue {
        match *self {
            Self::Basic(val) => val,
            Self::Specific { default, space, .. } => {
                space.or(default).unwrap_or(WhitespaceRenderValue::None)
            }
        }
    }
    pub fn nbsp(&self) -> WhitespaceRenderValue {
        match *self {
            Self::Basic(val) => val,
            Self::Specific { default, nbsp, .. } => {
                nbsp.or(default).unwrap_or(WhitespaceRenderValue::None)
            }
        }
    }
    pub fn nnbsp(&self) -> WhitespaceRenderValue {
        match *self {
            Self::Basic(val) => val,
            Self::Specific { default, nnbsp, .. } => {
                nnbsp.or(default).unwrap_or(WhitespaceRenderValue::None)
            }
        }
    }
    pub fn tab(&self) -> WhitespaceRenderValue {
        match *self {
            Self::Basic(val) => val,
            Self::Specific { default, tab, .. } => {
                tab.or(default).unwrap_or(WhitespaceRenderValue::None)
            }
        }
    }
    pub fn newline(&self) -> WhitespaceRenderValue {
        match *self {
            Self::Basic(val) => val,
            Self::Specific {
                default, newline, ..
            } => newline.or(default).unwrap_or(WhitespaceRenderValue::None),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AutoSave {
    /// Auto save after a delay in milliseconds. Defaults to disabled.
    #[serde(default)]
    pub after_delay: AutoSaveAfterDelay,
    /// Auto save on focus lost. Defaults to false.
    #[serde(default)]
    pub focus_lost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AutoSaveAfterDelay {
    #[serde(default)]
    /// Enable auto save after delay. Defaults to false.
    pub enable: bool,
    #[serde(default = "default_auto_save_delay")]
    /// Time delay in milliseconds. Defaults to [DEFAULT_AUTO_SAVE_DELAY].
    pub timeout: u64,
}

impl Default for AutoSaveAfterDelay {
    fn default() -> Self {
        Self {
            enable: false,
            timeout: DEFAULT_AUTO_SAVE_DELAY,
        }
    }
}

fn default_auto_save_delay() -> u64 {
    DEFAULT_AUTO_SAVE_DELAY
}

fn deserialize_auto_save<'de, D>(deserializer: D) -> Result<AutoSave, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize, Serialize)]
    #[serde(untagged, deny_unknown_fields, rename_all = "kebab-case")]
    enum AutoSaveToml {
        EnableFocusLost(bool),
        AutoSave(AutoSave),
    }

    match AutoSaveToml::deserialize(deserializer)? {
        AutoSaveToml::EnableFocusLost(focus_lost) => Ok(AutoSave {
            focus_lost,
            ..Default::default()
        }),
        AutoSaveToml::AutoSave(auto_save) => Ok(auto_save),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WhitespaceCharacters {
    pub space: char,
    pub nbsp: char,
    pub nnbsp: char,
    pub tab: char,
    pub tabpad: char,
    pub newline: char,
}

impl Default for WhitespaceCharacters {
    fn default() -> Self {
        Self {
            space: '·',   // U+00B7
            nbsp: '⍽',    // U+237D
            nnbsp: '␣',   // U+2423
            tab: '→',     // U+2192
            newline: '⏎', // U+23CE
            tabpad: ' ',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IndentGuidesConfig {
    pub render: bool,
    pub character: char,
    pub skip_levels: u8,
}

impl Default for IndentGuidesConfig {
    fn default() -> Self {
        Self {
            skip_levels: 0,
            render: false,
            character: '│',
        }
    }
}

/// Line ending configuration.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEndingConfig {
    /// The platform's native line ending.
    ///
    /// `crlf` on Windows, otherwise `lf`.
    #[default]
    Native,
    /// Line feed.
    LF,
    /// Carriage return followed by line feed.
    Crlf,
    /// Form feed.
    #[cfg(feature = "unicode-lines")]
    FF,
    /// Carriage return.
    #[cfg(feature = "unicode-lines")]
    CR,
    /// Next line.
    #[cfg(feature = "unicode-lines")]
    Nel,
}

impl From<LineEndingConfig> for LineEnding {
    fn from(line_ending: LineEndingConfig) -> Self {
        match line_ending {
            LineEndingConfig::Native => NATIVE_LINE_ENDING,
            LineEndingConfig::LF => LineEnding::LF,
            LineEndingConfig::Crlf => LineEnding::Crlf,
            #[cfg(feature = "unicode-lines")]
            LineEndingConfig::FF => LineEnding::FF,
            #[cfg(feature = "unicode-lines")]
            LineEndingConfig::CR => LineEnding::CR,
            #[cfg(feature = "unicode-lines")]
            LineEndingConfig::Nel => LineEnding::Nel,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PopupBorderConfig {
    None,
    All,
    Popup,
    Menu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct WordCompletion {
    pub enable: bool,
    pub trigger_length: NonZeroU8,
}

impl Default for WordCompletion {
    fn default() -> Self {
        Self {
            enable: true,
            trigger_length: NonZeroU8::new(7).unwrap(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scrolloff: 5,
            scroll_lines: 3,
            mouse: true,
            shell: if cfg!(windows) {
                vec!["cmd".to_owned(), "/C".to_owned()]
            } else {
                vec!["sh".to_owned(), "-c".to_owned()]
            },
            line_number: LineNumber::Absolute,
            cursorline: false,
            cursorcolumn: false,
            gutters: GutterConfig::default(),
            middle_click_paste: true,
            auto_pairs: AutoPairConfig::default(),
            auto_completion: true,
            path_completion: true,
            word_completion: WordCompletion::default(),
            auto_format: true,
            default_yank_register: '"',
            auto_save: AutoSave::default(),
            idle_timeout: Duration::from_millis(250),
            completion_timeout: Duration::from_millis(250),
            preview_completion_insert: true,
            completion_trigger_len: 2,
            auto_info: true,
            file_picker: FilePickerConfig::default(),
            file_explorer: FileExplorerConfig::default(),
            statusline: StatusLineConfig::default(),
            cursor_shape: CursorShapeConfig::default(),
            true_color: false,
            undercurl: false,
            search: SearchConfig::default(),
            lsp: LspConfig::default(),
            terminal: get_terminal_provider(),
            rulers: Vec::new(),
            whitespace: WhitespaceConfig::default(),
            bufferline: BufferLine::default(),
            indent_guides: IndentGuidesConfig::default(),
            color_modes: false,
            soft_wrap: SoftWrap {
                enable: Some(false),
                ..SoftWrap::default()
            },
            text_width: 80,
            completion_replace: false,
            continue_comments: true,
            workspace_lsp_roots: Vec::new(),
            default_line_ending: LineEndingConfig::default(),
            insert_final_newline: true,
            atomic_save: true,
            trim_final_newlines: false,
            trim_trailing_whitespace: false,
            smart_tab: Some(SmartTabConfig::default()),
            popup_border: PopupBorderConfig::None,
            rounded_corners: false,
            indent_heuristic: IndentationHeuristic::default(),
            jump_label_alphabet: ('a'..='z').collect(),
            inline_diagnostics: InlineDiagnosticsConfig::default(),
            end_of_line_diagnostics: DiagnosticFilter::Enable(Severity::Hint),
            clipboard_provider: ClipboardProvider::default(),
            editor_config: true,
            rainbow_brackets: false,
            kitty_keyboard_protocol: Default::default(),
            buffer_picker: BufferPickerConfig::default(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            wrap_around: true,
            smart_case: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Breakpoint {
    pub id: Option<usize>,
    pub verified: bool,
    pub message: Option<String>,

    pub line: usize,
    pub column: Option<usize>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

use futures_util::stream::{Flatten, Once};

type Diagnostics = BTreeMap<Uri, Vec<(lsp::Diagnostic, DiagnosticProvider)>>;

pub struct Editor {
    /// Current editing mode.
    pub mode: Mode,
    pub tree: Tree,
    pub next_document_id: DocumentId,
    pub documents: BTreeMap<DocumentId, Document>,

    // We Flatten<> to resolve the inner DocumentSavedEventFuture. For that we need a stream of streams, hence the Once<>.
    // https://stackoverflow.com/a/66875668
    pub saves: HashMap<DocumentId, UnboundedSender<Once<DocumentSavedEventFuture>>>,
    pub save_queue: SelectAll<Flatten<UnboundedReceiverStream<Once<DocumentSavedEventFuture>>>>,
    pub write_count: usize,

    pub count: Option<std::num::NonZeroUsize>,
    pub selected_register: Option<char>,
    pub registers: Registers,
    pub macro_recording: Option<(char, Vec<KeyEvent>)>,
    pub macro_replaying: Vec<char>,
    pub language_servers: helix_lsp::Registry,
    pub diagnostics: Diagnostics,
    pub diff_providers: DiffProviderRegistry,

    pub debug_adapters: dap::registry::Registry,
    pub breakpoints: HashMap<PathBuf, Vec<Breakpoint>>,

    pub syn_loader: Arc<ArcSwap<syntax::Loader>>,
    pub theme_loader: Arc<theme::Loader>,
    /// last_theme is used for theme previews. We store the current theme here,
    /// and if previewing is cancelled, we can return to it.
    pub last_theme: Option<Theme>,
    /// The currently applied editor theme. While previewing a theme, the previewed theme
    /// is set here.
    pub theme: Theme,

    /// The primary Selection prior to starting a goto_line_number preview. This is
    /// restored when the preview is aborted, or added to the jumplist when it is
    /// confirmed.
    pub last_selection: Option<Selection>,

    pub status_msg: Option<(Cow<'static, str>, Severity)>,
    pub autoinfo: Option<Info>,

    pub config: Arc<dyn DynAccess<Config>>,
    pub auto_pairs: Option<AutoPairs>,

    pub idle_timer: Pin<Box<Sleep>>,
    redraw_timer: Pin<Box<Sleep>>,
    last_motion: Option<Motion>,
    pub last_completion: Option<CompleteAction>,
    last_cwd: Option<PathBuf>,

    pub exit_code: i32,

    pub config_events: (UnboundedSender<ConfigEvent>, UnboundedReceiver<ConfigEvent>),
    pub needs_redraw: bool,
    /// Cached position of the cursor calculated during rendering.
    /// The content of `cursor_cache` is returned by `Editor::cursor` if
    /// set to `Some(_)`. The value will be cleared after it's used.
    /// If `cursor_cache` is `None` then the `Editor::cursor` function will
    /// calculate the cursor position.
    ///
    /// `Some(None)` represents a cursor position outside of the visible area.
    /// This will just cause `Editor::cursor` to return `None`.
    ///
    /// This cache is only a performance optimization to
    /// avoid calculating the cursor position multiple
    /// times during rendering and should not be set by other functions.
    pub handlers: Handlers,

    pub mouse_down_range: Option<Range>,
    pub cursor_cache: CursorCache,
}

pub type Motion = Box<dyn Fn(&mut Editor)>;

#[derive(Debug)]
pub enum EditorEvent {
    DocumentSaved(DocumentSavedEventResult),
    ConfigEvent(ConfigEvent),
    LanguageServerMessage((LanguageServerId, Call)),
    DebuggerEvent((DebugAdapterId, dap::Payload)),
    IdleTimer,
    Redraw,
}

#[derive(Debug, Clone)]
pub enum ConfigEvent {
    Refresh,
    Update(Box<Config>),
}

enum ThemeAction {
    Set,
    Preview,
}

#[derive(Debug, Clone)]
pub enum CompleteAction {
    Triggered,
    /// A savepoint of the currently selected completion. The savepoint
    /// MUST be restored before sending any event to the LSP
    Selected {
        savepoint: Arc<SavePoint>,
    },
    Applied {
        trigger_offset: usize,
        changes: Vec<Change>,
        placeholder: bool,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Load,
    Replace,
    HorizontalSplit,
    VerticalSplit,
}

impl Action {
    /// Whether to align the view to the cursor after executing this action
    pub fn align_view(&self, view: &View, new_doc: DocumentId) -> bool {
        !matches!((self, view.doc == new_doc), (Action::Load, false))
    }
}

/// Error thrown on failed document closed
pub enum CloseError {
    /// Document doesn't exist
    DoesNotExist,
    /// Buffer is modified
    BufferModified(String),
    /// Document failed to save
    SaveError(anyhow::Error),
}

impl Editor {
    pub fn new(
        mut area: Rect,
        theme_loader: Arc<theme::Loader>,
        syn_loader: Arc<ArcSwap<syntax::Loader>>,
        config: Arc<dyn DynAccess<Config>>,
        handlers: Handlers,
    ) -> Self {
        let language_servers = helix_lsp::Registry::new(syn_loader.clone());
        let conf = config.load();
        let auto_pairs = (&conf.auto_pairs).into();

        // HAXX: offset the render area height by 1 to account for prompt/commandline
        area.height -= 1;

        Self {
            mode: Mode::Normal,
            tree: Tree::new(area),
            next_document_id: DocumentId::default(),
            documents: BTreeMap::new(),
            saves: HashMap::new(),
            save_queue: SelectAll::new(),
            write_count: 0,
            count: None,
            selected_register: None,
            macro_recording: None,
            macro_replaying: Vec::new(),
            theme: theme_loader.default(),
            language_servers,
            diagnostics: Diagnostics::new(),
            diff_providers: DiffProviderRegistry::default(),
            debug_adapters: dap::registry::Registry::new(),
            breakpoints: HashMap::new(),
            syn_loader,
            theme_loader,
            last_theme: None,
            last_selection: None,
            registers: Registers::new(Box::new(arc_swap::access::Map::new(
                Arc::clone(&config),
                |config: &Config| &config.clipboard_provider,
            ))),
            status_msg: None,
            autoinfo: None,
            idle_timer: Box::pin(sleep(conf.idle_timeout)),
            redraw_timer: Box::pin(sleep(Duration::MAX)),
            last_motion: None,
            last_completion: None,
            last_cwd: None,
            config,
            auto_pairs,
            exit_code: 0,
            config_events: unbounded_channel(),
            needs_redraw: false,
            handlers,
            mouse_down_range: None,
            cursor_cache: CursorCache::default(),
        }
    }

    pub fn popup_border(&self) -> bool {
        self.config().popup_border == PopupBorderConfig::All
            || self.config().popup_border == PopupBorderConfig::Popup
    }

    pub fn menu_border(&self) -> bool {
        self.config().popup_border == PopupBorderConfig::All
            || self.config().popup_border == PopupBorderConfig::Menu
    }

    pub fn apply_motion<F: Fn(&mut Self) + 'static>(&mut self, motion: F) {
        motion(self);
        self.last_motion = Some(Box::new(motion));
    }

    pub fn repeat_last_motion(&mut self, count: usize) {
        if let Some(motion) = self.last_motion.take() {
            for _ in 0..count {
                motion(self);
            }
            self.last_motion = Some(motion);
        }
    }
    /// Current editing mode for the [`Editor`].
    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn config(&self) -> DynGuard<Config> {
        self.config.load()
    }

    /// Call if the config has changed to let the editor update all
    /// relevant members.
    pub fn refresh_config(&mut self, old_config: &Config) {
        let config = self.config();
        self.auto_pairs = (&config.auto_pairs).into();
        self.reset_idle_timer();
        self._refresh();
        helix_event::dispatch(crate::events::ConfigDidChange {
            editor: self,
            old: old_config,
            new: &config,
        })
    }

    pub fn clear_idle_timer(&mut self) {
        // equivalent to internal Instant::far_future() (30 years)
        self.idle_timer
            .as_mut()
            .reset(Instant::now() + Duration::from_secs(86400 * 365 * 30));
    }

    pub fn reset_idle_timer(&mut self) {
        let config = self.config();
        self.idle_timer
            .as_mut()
            .reset(Instant::now() + config.idle_timeout);
    }

    pub fn clear_status(&mut self) {
        self.status_msg = None;
    }

    #[inline]
    pub fn set_status<T: Into<Cow<'static, str>>>(&mut self, status: T) {
        let status = status.into();
        log::debug!("editor status: {}", status);
        self.status_msg = Some((status, Severity::Info));
    }

    #[inline]
    pub fn set_error<T: Into<Cow<'static, str>>>(&mut self, error: T) {
        let error = error.into();
        log::debug!("editor error: {}", error);
        self.status_msg = Some((error, Severity::Error));
    }

    #[inline]
    pub fn set_warning<T: Into<Cow<'static, str>>>(&mut self, warning: T) {
        let warning = warning.into();
        log::warn!("editor warning: {}", warning);
        self.status_msg = Some((warning, Severity::Warning));
    }

    #[inline]
    pub fn get_status(&self) -> Option<(&Cow<'static, str>, &Severity)> {
        self.status_msg.as_ref().map(|(status, sev)| (status, sev))
    }

    /// Returns true if the current status is an error
    #[inline]
    pub fn is_err(&self) -> bool {
        self.status_msg
            .as_ref()
            .map(|(_, sev)| *sev == Severity::Error)
            .unwrap_or(false)
    }

    pub fn unset_theme_preview(&mut self) {
        if let Some(last_theme) = self.last_theme.take() {
            self.set_theme(last_theme);
        }
        // None likely occurs when the user types ":theme" and then exits before previewing
    }

    pub fn set_theme_preview(&mut self, theme: Theme) {
        self.set_theme_impl(theme, ThemeAction::Preview);
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.set_theme_impl(theme, ThemeAction::Set);
    }

    fn set_theme_impl(&mut self, theme: Theme, preview: ThemeAction) {
        // `ui.selection` is the only scope required to be able to render a theme.
        if theme.find_highlight_exact("ui.selection").is_none() {
            self.set_error("Invalid theme: `ui.selection` required");
            return;
        }

        let scopes = theme.scopes();
        (*self.syn_loader).load().set_scopes(scopes.to_vec());

        match preview {
            ThemeAction::Preview => {
                let last_theme = std::mem::replace(&mut self.theme, theme);
                // only insert on first preview: this will be the last theme the user has saved
                self.last_theme.get_or_insert(last_theme);
            }
            ThemeAction::Set => {
                self.last_theme = None;
                self.theme = theme;
            }
        }

        self._refresh();
    }

    #[inline]
    pub fn language_server_by_id(
        &self,
        language_server_id: LanguageServerId,
    ) -> Option<&helix_lsp::Client> {
        self.language_servers
            .get_by_id(language_server_id)
            .map(|client| &**client)
    }

    /// Refreshes the language server for a given document
    pub fn refresh_language_servers(&mut self, doc_id: DocumentId) {
        self.launch_language_servers(doc_id)
    }

    /// moves/renames a path, invoking any event handlers (currently only lsp)
    /// and calling `set_doc_path` if the file is open in the editor
    pub fn move_path(&mut self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        let new_path = canonicalize(new_path);
        // sanity check
        if old_path == new_path {
            return Ok(());
        }
        let is_dir = old_path.is_dir();
        let language_servers: Vec<_> = self
            .language_servers
            .iter_clients()
            .filter(|client| client.is_initialized())
            .cloned()
            .collect();
        for language_server in language_servers {
            let Some(request) = language_server.will_rename(old_path, &new_path, is_dir) else {
                continue;
            };
            let edit = match helix_lsp::block_on(request) {
                Ok(edit) => edit.unwrap_or_default(),
                Err(err) => {
                    log::error!("invalid willRename response: {err:?}");
                    continue;
                }
            };
            if let Err(err) = self.apply_workspace_edit(language_server.offset_encoding(), &edit) {
                log::error!("failed to apply workspace edit: {err:?}")
            }
        }

        if old_path.exists() {
            fs::rename(old_path, &new_path)?;
        }

        if let Some(doc) = self.document_by_path(old_path) {
            self.set_doc_path(doc.id(), &new_path);
        }
        let is_dir = new_path.is_dir();
        for ls in self.language_servers.iter_clients() {
            // A new language server might have been started in `set_doc_path` and won't
            // be initialized yet. Skip the `did_rename` notification for this server.
            if !ls.is_initialized() {
                continue;
            }
            ls.did_rename(old_path, &new_path, is_dir);
        }
        self.language_servers
            .file_event_handler
            .file_changed(old_path.to_owned());
        self.language_servers
            .file_event_handler
            .file_changed(new_path);
        Ok(())
    }

    pub fn set_doc_path(&mut self, doc_id: DocumentId, path: &Path) {
        let doc = doc_mut!(self, &doc_id);
        let old_path = doc.path();

        if let Some(old_path) = old_path {
            // sanity check, should not occur but some callers (like an LSP) may
            // create bogus calls
            if old_path == path {
                return;
            }
            // if we are open in LSPs send did_close notification
            for language_server in doc.language_servers() {
                language_server.text_document_did_close(doc.identifier());
            }
        }
        // we need to clear the list of language servers here so that
        // refresh_doc_language/refresh_language_servers doesn't resend
        // text_document_did_close. Since we called `text_document_did_close`
        // we have fully unregistered this document from its LS
        doc.language_servers.clear();
        doc.set_path(Some(path));
        doc.detect_editor_config();
        self.refresh_doc_language(doc_id)
    }

    pub fn refresh_doc_language(&mut self, doc_id: DocumentId) {
        let loader = self.syn_loader.load();
        let doc = doc_mut!(self, &doc_id);
        doc.detect_language(&loader);
        doc.detect_editor_config();
        doc.detect_indent_and_line_ending();
        self.refresh_language_servers(doc_id);
        let doc = doc_mut!(self, &doc_id);
        let diagnostics = Editor::doc_diagnostics(&self.language_servers, &self.diagnostics, doc);
        doc.replace_diagnostics(diagnostics, &[], None);
        doc.reset_all_inlay_hints();
    }

    /// Launch a language server for a given document
    fn launch_language_servers(&mut self, doc_id: DocumentId) {
        if !self.config().lsp.enable {
            return;
        }
        // if doc doesn't have a URL it's a scratch buffer, ignore it
        let Some(doc) = self.documents.get_mut(&doc_id) else {
            return;
        };
        let Some(doc_url) = doc.url() else {
            return;
        };
        let (lang, path) = (doc.language.clone(), doc.path().cloned());
        let config = doc.config.load();
        let root_dirs = &config.workspace_lsp_roots;

        // store only successfully started language servers
        let language_servers = lang.as_ref().map_or_else(HashMap::default, |language| {
            self.language_servers
                .get(language, path.as_ref(), root_dirs, config.lsp.snippets)
                .filter_map(|(lang, client)| match client {
                    Ok(client) => Some((lang, client)),
                    Err(err) => {
                        if let helix_lsp::Error::ExecutableNotFound(err) = err {
                            // Silence by default since some language servers might just not be installed
                            log::debug!(
                                "Language server not found for `{}` {} {}", language.scope, lang, err,
                            );
                        } else {
                            log::error!(
                                "Failed to initialize the language servers for `{}` - `{}` {{ {} }}",
                                language.scope,
                                lang,
                                err
                            );
                        }
                        None
                    }
                })
                .collect::<HashMap<_, _>>()
        });

        if language_servers.is_empty() && doc.language_servers.is_empty() {
            return;
        }

        let language_id = doc.language_id().map(ToOwned::to_owned).unwrap_or_default();

        // only spawn new language servers if the servers aren't the same
        let doc_language_servers_not_in_registry =
            doc.language_servers.iter().filter(|(name, doc_ls)| {
                language_servers
                    .get(*name)
                    .is_none_or(|ls| ls.id() != doc_ls.id())
            });

        for (_, language_server) in doc_language_servers_not_in_registry {
            language_server.text_document_did_close(doc.identifier());
        }

        let language_servers_not_in_doc = language_servers.iter().filter(|(name, ls)| {
            doc.language_servers
                .get(*name)
                .is_none_or(|doc_ls| ls.id() != doc_ls.id())
        });

        for (_, language_server) in language_servers_not_in_doc {
            // TODO: this now races with on_init code if the init happens too quickly
            language_server.text_document_did_open(
                doc_url.clone(),
                doc.version(),
                doc.text(),
                language_id.clone(),
            );
        }

        doc.language_servers = language_servers;
    }

    fn _refresh(&mut self) {
        let config = self.config();

        // Reset the inlay hints annotations *before* updating the views, that way we ensure they
        // will disappear during the `.sync_change(doc)` call below.
        //
        // We can't simply check this config when rendering because inlay hints are only parts of
        // the possible annotations, and others could still be active, so we need to selectively
        // drop the inlay hints.
        if !config.lsp.display_inlay_hints {
            for doc in self.documents_mut() {
                doc.reset_all_inlay_hints();
            }
        }

        for (view, _) in self.tree.views_mut() {
            let doc = doc_mut!(self, &view.doc);
            view.sync_changes(doc);
            view.gutters = config.gutters.clone();
            view.ensure_cursor_in_view(doc, config.scrolloff)
        }
    }

    fn replace_document_in_view(&mut self, current_view: ViewId, doc_id: DocumentId) {
        let scrolloff = self.config().scrolloff;
        let view = self.tree.get_mut(current_view);

        view.doc = doc_id;
        let doc = doc_mut!(self, &doc_id);

        doc.ensure_view_init(view.id);
        view.sync_changes(doc);
        doc.mark_as_focused();

        view.ensure_cursor_in_view(doc, scrolloff)
    }

    pub fn switch(&mut self, id: DocumentId, action: Action) {
        use crate::tree::Layout;

        if !self.documents.contains_key(&id) {
            log::error!("cannot switch to document that does not exist (anymore)");
            return;
        }

        if !matches!(action, Action::Load) {
            self.enter_normal_mode();
        }

        let focust_lost = match action {
            Action::Replace => {
                let (view, doc) = current_ref!(self);
                // If the current view is an empty scratch buffer and is not displayed in any other views, delete it.
                // Boolean value is determined before the call to `view_mut` because the operation requires a borrow
                // of `self.tree`, which is mutably borrowed when `view_mut` is called.
                let remove_empty_scratch = !doc.is_modified()
                    // If the buffer has no path and is not modified, it is an empty scratch buffer.
                    && doc.path().is_none()
                    // If the buffer we are changing to is not this buffer
                    && id != doc.id
                    // Ensure the buffer is not displayed in any other splits.
                    && !self
                        .tree
                        .traverse()
                        .any(|(_, v)| v.doc == doc.id && v.id != view.id);

                let (view, doc) = current!(self);
                let view_id = view.id;

                // Append any outstanding changes to history in the old document.
                doc.append_changes_to_history(view);

                if remove_empty_scratch {
                    // Copy `doc.id` into a variable before calling `self.documents.remove`, which requires a mutable
                    // borrow, invalidating direct access to `doc.id`.
                    let id = doc.id;
                    self.documents.remove(&id);

                    // Remove the scratch buffer from any jumplists
                    for (view, _) in self.tree.views_mut() {
                        view.remove_document(&id);
                    }
                } else {
                    let jump = (view.doc, doc.selection(view.id).clone());
                    view.jumps.push(jump);
                    // Set last accessed doc if it is a different document
                    if doc.id != id {
                        view.add_to_history(view.doc);
                        // Set last modified doc if modified and last modified doc is different
                        if std::mem::take(&mut doc.modified_since_accessed)
                            && view.last_modified_docs[0] != Some(view.doc)
                        {
                            view.last_modified_docs = [Some(view.doc), view.last_modified_docs[0]];
                        }
                    }
                }

                self.replace_document_in_view(view_id, id);

                dispatch(DocumentFocusLost {
                    editor: self,
                    doc: id,
                });
                return;
            }
            Action::Load => {
                let view_id = view!(self).id;
                let doc = doc_mut!(self, &id);
                doc.ensure_view_init(view_id);
                doc.mark_as_focused();
                return;
            }
            Action::HorizontalSplit | Action::VerticalSplit => {
                let focus_lost = self.tree.try_get(self.tree.focus).map(|view| view.doc);
                // copy the current view, unless there is no view yet
                let view = self
                    .tree
                    .try_get(self.tree.focus)
                    .filter(|v| id == v.doc) // Different Document
                    .cloned()
                    .unwrap_or_else(|| View::new(id, self.config().gutters.clone()));
                let view_id = self.tree.split(
                    view,
                    match action {
                        Action::HorizontalSplit => Layout::Horizontal,
                        Action::VerticalSplit => Layout::Vertical,
                        _ => unreachable!(),
                    },
                );
                // initialize selection for view
                let doc = doc_mut!(self, &id);
                doc.ensure_view_init(view_id);
                doc.mark_as_focused();
                focus_lost
            }
        };

        self._refresh();
        if let Some(focus_lost) = focust_lost {
            dispatch(DocumentFocusLost {
                editor: self,
                doc: focus_lost,
            });
        }
    }

    /// Generate an id for a new document and register it.
    fn new_document(&mut self, mut doc: Document) -> DocumentId {
        let id = self.next_document_id;
        // Safety: adding 1 from 1 is fine, practically impossible to reach usize max
        self.next_document_id =
            DocumentId(unsafe { NonZeroUsize::new_unchecked(self.next_document_id.0.get() + 1) });
        doc.id = id;
        self.documents.insert(id, doc);

        let (save_sender, save_receiver) = tokio::sync::mpsc::unbounded_channel();
        self.saves.insert(id, save_sender);

        let stream = UnboundedReceiverStream::new(save_receiver).flatten();
        self.save_queue.push(stream);

        id
    }

    fn new_file_from_document(&mut self, action: Action, doc: Document) -> DocumentId {
        let id = self.new_document(doc);
        self.switch(id, action);
        id
    }

    pub fn new_file(&mut self, action: Action) -> DocumentId {
        self.new_file_from_document(
            action,
            Document::default(self.config.clone(), self.syn_loader.clone()),
        )
    }

    pub fn new_file_from_stdin(&mut self, action: Action) -> Result<DocumentId, Error> {
        let (stdin, encoding, has_bom) = crate::document::read_to_string(&mut stdin(), None)?;
        let doc = Document::from(
            helix_core::Rope::default(),
            Some((encoding, has_bom)),
            self.config.clone(),
            self.syn_loader.clone(),
        );
        let doc_id = self.new_file_from_document(action, doc);
        let doc = doc_mut!(self, &doc_id);
        let view = view_mut!(self);
        doc.ensure_view_init(view.id);
        let transaction =
            helix_core::Transaction::insert(doc.text(), doc.selection(view.id), stdin.into())
                .with_selection(Selection::point(0));
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view);
        Ok(doc_id)
    }

    pub fn document_id_by_path(&self, path: &Path) -> Option<DocumentId> {
        self.document_by_path(path).map(|doc| doc.id)
    }

    // ??? possible use for integration tests
    pub fn open(&mut self, path: &Path, action: Action) -> Result<DocumentId, DocumentOpenError> {
        let path = helix_stdx::path::canonicalize(path);
        let id = self.document_id_by_path(&path);

        let id = if let Some(id) = id {
            id
        } else {
            let mut doc = Document::open(
                &path,
                None,
                true,
                self.config.clone(),
                self.syn_loader.clone(),
            )?;

            let diagnostics =
                Editor::doc_diagnostics(&self.language_servers, &self.diagnostics, &doc);
            doc.replace_diagnostics(diagnostics, &[], None);

            if let Some(diff_base) = self.diff_providers.get_diff_base(&path) {
                doc.set_diff_base(diff_base);
            }
            doc.set_version_control_head(self.diff_providers.get_current_head_name(&path));

            let id = self.new_document(doc);
            self.launch_language_servers(id);

            helix_event::dispatch(DocumentDidOpen {
                editor: self,
                doc: id,
            });

            id
        };

        self.switch(id, action);

        Ok(id)
    }

    pub fn close(&mut self, id: ViewId) {
        // Remove selections for the closed view on all documents.
        for doc in self.documents_mut() {
            doc.remove_view(id);
        }
        self.tree.remove(id);
        self._refresh();
    }

    pub fn close_document(&mut self, doc_id: DocumentId, force: bool) -> Result<(), CloseError> {
        let doc = match self.documents.get(&doc_id) {
            Some(doc) => doc,
            None => return Err(CloseError::DoesNotExist),
        };
        if !force && doc.is_modified() {
            return Err(CloseError::BufferModified(doc.display_name().into_owned()));
        }

        // This will also disallow any follow-up writes
        self.saves.remove(&doc_id);

        enum Action {
            Close(ViewId),
            ReplaceDoc(ViewId, DocumentId),
        }

        let actions: Vec<Action> = self
            .tree
            .views_mut()
            .filter_map(|(view, _focus)| {
                view.remove_document(&doc_id);

                if view.doc == doc_id {
                    // something was previously open in the view, switch to previous doc
                    if let Some(prev_doc) = view.docs_access_history.pop() {
                        Some(Action::ReplaceDoc(view.id, prev_doc))
                    } else {
                        // only the document that is being closed was in the view, close it
                        Some(Action::Close(view.id))
                    }
                } else {
                    None
                }
            })
            .collect();

        for action in actions {
            match action {
                Action::Close(view_id) => {
                    self.close(view_id);
                }
                Action::ReplaceDoc(view_id, doc_id) => {
                    self.replace_document_in_view(view_id, doc_id);
                }
            }
        }

        let doc = self.documents.remove(&doc_id).unwrap();

        // If the document we removed was visible in all views, we will have no more views. We don't
        // want to close the editor just for a simple buffer close, so we need to create a new view
        // containing either an existing document, or a brand new document.
        if self.tree.views().next().is_none() {
            let doc_id = self
                .documents
                .iter()
                .map(|(&doc_id, _)| doc_id)
                .next()
                .unwrap_or_else(|| {
                    self.new_document(Document::default(
                        self.config.clone(),
                        self.syn_loader.clone(),
                    ))
                });
            let view = View::new(doc_id, self.config().gutters.clone());
            let view_id = self.tree.insert(view);
            let doc = doc_mut!(self, &doc_id);
            doc.ensure_view_init(view_id);
            doc.mark_as_focused();
        }

        self._refresh();

        helix_event::dispatch(DocumentDidClose { editor: self, doc });

        Ok(())
    }

    pub fn save<P: Into<PathBuf>>(
        &mut self,
        doc_id: DocumentId,
        path: Option<P>,
        force: bool,
    ) -> anyhow::Result<()> {
        // convert a channel of futures to pipe into main queue one by one
        // via stream.then() ? then push into main future

        let path = path.map(|path| path.into());
        let doc = doc_mut!(self, &doc_id);
        let doc_save_future = doc.save(path, force)?;

        // When a file is written to, notify the file event handler.
        // Note: This can be removed once proper file watching is implemented.
        let handler = self.language_servers.file_event_handler.clone();
        let future = async move {
            let res = doc_save_future.await;
            if let Ok(event) = &res {
                handler.file_changed(event.path.clone());
            }
            res
        };

        use futures_util::stream;

        self.saves
            .get(&doc_id)
            .ok_or_else(|| anyhow::format_err!("saves are closed for this document!"))?
            .send(stream::once(Box::pin(future)))
            .map_err(|err| anyhow!("failed to send save event: {}", err))?;

        self.write_count += 1;

        Ok(())
    }

    pub fn resize(&mut self, area: Rect) {
        if self.tree.resize(area) {
            self._refresh();
        };
    }

    pub fn focus(&mut self, view_id: ViewId) {
        if self.tree.focus == view_id {
            return;
        }

        // Reset mode to normal and ensure any pending changes are committed in the old document.
        self.enter_normal_mode();
        let (view, doc) = current!(self);
        doc.append_changes_to_history(view);
        self.ensure_cursor_in_view(view_id);
        // Update jumplist selections with new document changes.
        for (view, _focused) in self.tree.views_mut() {
            let doc = doc_mut!(self, &view.doc);
            view.sync_changes(doc);
        }

        let prev_id = std::mem::replace(&mut self.tree.focus, view_id);
        doc_mut!(self).mark_as_focused();

        let focus_lost = self.tree.get(prev_id).doc;
        dispatch(DocumentFocusLost {
            editor: self,
            doc: focus_lost,
        });
    }

    pub fn focus_next(&mut self) {
        self.focus(self.tree.next());
    }

    pub fn focus_prev(&mut self) {
        self.focus(self.tree.prev());
    }

    pub fn focus_direction(&mut self, direction: tree::Direction) {
        let current_view = self.tree.focus;
        if let Some(id) = self.tree.find_split_in_direction(current_view, direction) {
            self.focus(id)
        }
    }

    pub fn swap_split_in_direction(&mut self, direction: tree::Direction) {
        self.tree.swap_split_in_direction(direction);
    }

    pub fn transpose_view(&mut self) {
        self.tree.transpose();
    }

    pub fn should_close(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn ensure_cursor_in_view(&mut self, id: ViewId) {
        let config = self.config();
        let view = self.tree.get(id);
        let doc = doc_mut!(self, &view.doc);
        view.ensure_cursor_in_view(doc, config.scrolloff)
    }

    #[inline]
    pub fn document(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(&id)
    }

    #[inline]
    pub fn document_mut(&mut self, id: DocumentId) -> Option<&mut Document> {
        self.documents.get_mut(&id)
    }

    #[inline]
    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }

    #[inline]
    pub fn documents_mut(&mut self) -> impl Iterator<Item = &mut Document> {
        self.documents.values_mut()
    }

    pub fn document_by_path<P: AsRef<Path>>(&self, path: P) -> Option<&Document> {
        self.documents()
            .find(|doc| doc.path().map(|p| p == path.as_ref()).unwrap_or(false))
    }

    pub fn document_by_path_mut<P: AsRef<Path>>(&mut self, path: P) -> Option<&mut Document> {
        self.documents_mut()
            .find(|doc| doc.path().map(|p| p == path.as_ref()).unwrap_or(false))
    }

    /// Returns all supported diagnostics for the document
    pub fn doc_diagnostics<'a>(
        language_servers: &'a helix_lsp::Registry,
        diagnostics: &'a Diagnostics,
        document: &Document,
    ) -> impl Iterator<Item = helix_core::Diagnostic> + 'a {
        Editor::doc_diagnostics_with_filter(language_servers, diagnostics, document, |_, _| true)
    }

    /// Returns all supported diagnostics for the document
    /// filtered by `filter` which is invocated with the raw `lsp::Diagnostic` and the language server id it came from
    pub fn doc_diagnostics_with_filter<'a>(
        language_servers: &'a helix_lsp::Registry,
        diagnostics: &'a Diagnostics,
        document: &Document,
        filter: impl Fn(&lsp::Diagnostic, &DiagnosticProvider) -> bool + 'a,
    ) -> impl Iterator<Item = helix_core::Diagnostic> + 'a {
        let text = document.text().clone();
        let language_config = document.language.clone();
        document
            .uri()
            .and_then(|uri| diagnostics.get(&uri))
            .map(|diags| {
                diags.iter().filter_map(move |(diagnostic, provider)| {
                    let server_id = provider.language_server_id()?;
                    let ls = language_servers.get_by_id(server_id)?;
                    language_config
                        .as_ref()
                        .and_then(|c| {
                            c.language_servers.iter().find(|features| {
                                features.name == ls.name()
                                    && features.has_feature(LanguageServerFeature::Diagnostics)
                            })
                        })
                        .and_then(|_| {
                            if filter(diagnostic, provider) {
                                Document::lsp_diagnostic_to_diagnostic(
                                    &text,
                                    language_config.as_deref(),
                                    diagnostic,
                                    provider.clone(),
                                    ls.offset_encoding(),
                                )
                            } else {
                                None
                            }
                        })
                })
            })
            .into_iter()
            .flatten()
    }

    /// Gets the primary cursor position in screen coordinates,
    /// or `None` if the primary cursor is not visible on screen.
    pub fn cursor(&self) -> (Option<Position>, CursorKind) {
        let config = self.config();
        let (view, doc) = current_ref!(self);
        if let Some(mut pos) = self.cursor_cache.get(view, doc) {
            let inner = view.inner_area(doc);
            pos.col += inner.x as usize;
            pos.row += inner.y as usize;
            let cursorkind = config.cursor_shape.from_mode(self.mode);
            (Some(pos), cursorkind)
        } else {
            (None, CursorKind::default())
        }
    }

    /// Closes language servers with timeout. The default timeout is 10000 ms, use
    /// `timeout` parameter to override this.
    pub async fn close_language_servers(
        &self,
        timeout: Option<u64>,
    ) -> Result<(), tokio::time::error::Elapsed> {
        // Remove all language servers from the file event handler.
        // Note: this is non-blocking.
        for client in self.language_servers.iter_clients() {
            self.language_servers
                .file_event_handler
                .remove_client(client.id());
        }

        tokio::time::timeout(
            Duration::from_millis(timeout.unwrap_or(3000)),
            future::join_all(
                self.language_servers
                    .iter_clients()
                    .map(|client| client.force_shutdown()),
            ),
        )
        .await
        .map(|_| ())
    }

    pub async fn wait_event(&mut self) -> EditorEvent {
        // the loop only runs once or twice and would be better implemented with a recursion + const generic
        // however due to limitations with async functions that can not be implemented right now
        loop {
            tokio::select! {
                biased;

                Some(event) = self.save_queue.next() => {
                    self.write_count -= 1;
                    return EditorEvent::DocumentSaved(event)
                }
                Some(config_event) = self.config_events.1.recv() => {
                    return EditorEvent::ConfigEvent(config_event)
                }
                Some(message) = self.language_servers.incoming.next() => {
                    return EditorEvent::LanguageServerMessage(message)
                }
                Some(event) = self.debug_adapters.incoming.next() => {
                    return EditorEvent::DebuggerEvent(event)
                }

                _ = helix_event::redraw_requested() => {
                    if  !self.needs_redraw{
                        self.needs_redraw = true;
                        let timeout = Instant::now() + Duration::from_millis(33);
                        if timeout < self.idle_timer.deadline() && timeout < self.redraw_timer.deadline(){
                            self.redraw_timer.as_mut().reset(timeout)
                        }
                    }
                }

                _ = &mut self.redraw_timer  => {
                    self.redraw_timer.as_mut().reset(Instant::now() + Duration::from_secs(86400 * 365 * 30));
                    return EditorEvent::Redraw
                }
                _ = &mut self.idle_timer  => {
                    return EditorEvent::IdleTimer
                }
            }
        }
    }

    pub async fn flush_writes(&mut self) -> anyhow::Result<()> {
        while self.write_count > 0 {
            if let Some(save_event) = self.save_queue.next().await {
                self.write_count -= 1;

                let save_event = match save_event {
                    Ok(event) => event,
                    Err(err) => {
                        self.set_error(err.to_string());
                        bail!(err);
                    }
                };

                let doc = doc_mut!(self, &save_event.doc_id);
                doc.set_last_saved_revision(save_event.revision, save_event.save_time);
            }
        }

        Ok(())
    }

    /// Switches the editor into normal mode.
    pub fn enter_normal_mode(&mut self) {
        use helix_core::graphemes;

        if self.mode == Mode::Normal {
            return;
        }

        self.mode = Mode::Normal;
        let (view, doc) = current!(self);

        try_restore_indent(doc, view);

        // if leaving append mode, move cursor back by 1
        if doc.restore_cursor {
            let text = doc.text().slice(..);
            let selection = doc.selection(view.id).clone().transform(|range| {
                let mut head = range.to();
                if range.head > range.anchor {
                    head = graphemes::prev_grapheme_boundary(text, head);
                }

                Range::new(range.from(), head)
            });

            doc.set_selection(view.id, selection);
            doc.restore_cursor = false;
        }
    }

    pub fn current_stack_frame(&self) -> Option<&dap::StackFrame> {
        self.debug_adapters.current_stack_frame()
    }

    /// Returns the id of a view that this doc contains a selection for,
    /// making sure it is synced with the current changes
    /// if possible or there are no selections returns current_view
    /// otherwise uses an arbitrary view
    pub fn get_synced_view_id(&mut self, id: DocumentId) -> ViewId {
        let current_view = view_mut!(self);
        let doc = self.documents.get_mut(&id).unwrap();
        if doc.selections().contains_key(&current_view.id) {
            // only need to sync current view if this is not the current doc
            if current_view.doc != id {
                current_view.sync_changes(doc);
            }
            current_view.id
        } else if let Some(view_id) = doc.selections().keys().next() {
            let view_id = *view_id;
            let view = self.tree.get_mut(view_id);
            view.sync_changes(doc);
            view_id
        } else {
            doc.ensure_view_init(current_view.id);
            current_view.id
        }
    }

    pub fn set_cwd(&mut self, path: &Path) -> std::io::Result<()> {
        self.last_cwd = helix_stdx::env::set_current_working_dir(path)?;
        self.clear_doc_relative_paths();
        Ok(())
    }

    pub fn get_last_cwd(&mut self) -> Option<&Path> {
        self.last_cwd.as_deref()
    }
}

fn try_restore_indent(doc: &mut Document, view: &mut View) {
    use helix_core::{
        chars::char_is_whitespace,
        line_ending::{line_end_char_index, str_is_line_ending},
        unicode::segmentation::UnicodeSegmentation,
        Operation, Transaction,
    };

    fn inserted_a_new_blank_line(changes: &[Operation], pos: usize, line_end_pos: usize) -> bool {
        if let [Operation::Retain(move_pos), Operation::Insert(ref inserted_str), Operation::Retain(_)] =
            changes
        {
            let mut graphemes = inserted_str.graphemes(true);
            move_pos + inserted_str.len() == pos
                && graphemes.next().is_some_and(str_is_line_ending)
                && graphemes.all(|g| g.chars().all(char_is_whitespace))
                && pos == line_end_pos // ensure no characters exists after current position
        } else {
            false
        }
    }

    let doc_changes = doc.changes().changes();
    let text = doc.text().slice(..);
    let range = doc.selection(view.id).primary();
    let pos = range.cursor(text);
    let line_end_pos = line_end_char_index(&text, range.cursor_line(text));

    if inserted_a_new_blank_line(doc_changes, pos, line_end_pos) {
        // Removes tailing whitespaces.
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let line_start_pos = text.line_to_char(range.cursor_line(text));
                (line_start_pos, pos, None)
            });
        doc.apply(&transaction, view.id);
    }
}

#[derive(Default)]
pub struct CursorCache(Cell<Option<Option<Position>>>);

impl CursorCache {
    pub fn get(&self, view: &View, doc: &Document) -> Option<Position> {
        if let Some(pos) = self.0.get() {
            return pos;
        }

        let text = doc.text().slice(..);
        let cursor = doc.selection(view.id).primary().cursor(text);
        let res = view.screen_coords_at_pos(doc, text, cursor);
        self.set(res);
        res
    }

    pub fn set(&self, cursor_pos: Option<Position>) {
        self.0.set(Some(cursor_pos))
    }

    pub fn reset(&self) {
        self.0.set(None)
    }
}
