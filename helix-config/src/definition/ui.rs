use helix_core::diagnostic::Severity;
use serde::{Deserialize, Serialize};

use crate::*;

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

config_serde_adapter!(GutterType);

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

config_serde_adapter!(LineNumber);

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
    /// The absolute file path
    FileAbsolutePath,
    // The file modification indicator
    FileModificationIndicator,
    /// An indicator that shows `"[readonly]"` when a file cannot be written
    ReadOnlyIndicator,
    /// The file encoding
    FileEncoding,
    /// The file line endings (CRLF or LF)
    FileLineEnding,
    /// The file type (language ID or "text")
    FileType,
    /// The file indent style (tabs or spaces and width)
    FileIndentStyle,
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
    /// Current working directory
    CurrentWorkingDirectory,
}

config_serde_adapter!(StatusLineElement);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
/// UNSTABLE
pub enum CursorKind {
    /// █
    Block,
    /// |
    Bar,
    /// _
    Underline,
    /// Hidden cursor, can set cursor position with this to let IME have correct cursor position.
    Hidden,
}

impl Default for CursorKind {
    fn default() -> Self {
        Self::Block
    }
}

config_serde_adapter!(CursorKind);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WhitespaceRenderValue {
    None,
    // TODO
    // Selection,
    All,
}

config_serde_adapter!(WhitespaceRenderValue);

/// bufferline render modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BufferLine {
    /// Don't render bufferline
    Never,
    /// Always render
    Always,
    /// Only if multiple buffers are open
    Multiple,
}

config_serde_adapter!(BufferLine);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PopupBorderConfig {
    None,
    All,
    Popup,
    Menu,
}

config_serde_adapter!(PopupBorderConfig);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PickerStartPosition {
    Current,
    Previous,
}

impl Default for PickerStartPosition {
    fn default() -> Self {
        Self::Current
    }
}

impl PickerStartPosition {
    pub fn is_previous(self) -> bool {
        matches!(self, Self::Previous)
    }
}

config_serde_adapter!(PickerStartPosition);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileExplorerPosition {
    Left,
    Right,
}

impl Default for FileExplorerPosition {
    fn default() -> Self {
        Self::Left
    }
}

config_serde_adapter!(FileExplorerPosition);

options! {
    struct UiConfig {
        /// Whether to display info boxes
        #[read = copy]
        auto_info: bool = true,
        /// Renders a line at the top of the editor displaying open buffers.
        /// Can be `always`, `never` or `multiple` (only shown if more than one
        /// buffer is in use)
        #[read = copy]
        bufferline: BufferLine = BufferLine::Never,
        /// Highlight all lines with a cursor
        #[read = copy]
        cursorline: bool = false,
        /// Highlight all columns with a cursor
        #[read = copy]
        cursorcolumn: bool = false,
        /// List of column positions at which to display the rulers.
        #[read = deref]
        rulers: List<u16> = List::default(),
        /// Draw border around popup, menu, all, or none
        #[read = copy]
        popup_border: PopupBorderConfig = PopupBorderConfig::None,
        /// Whether to color the mode indicator with different colors depending on the mode itself
        #[read = copy]
        color_modes: bool = false,
        /// Line number display mode (absolute or relative)
        #[name = "line-number"]
        #[read = copy]
        line_number: LineNumber = LineNumber::Absolute,
        /// Whether to render rainbow colors for matching brackets
        #[name = "rainbow-brackets"]
        #[read = copy]
        rainbow_brackets: bool = false,
        /// Characters to use for jump labels
        #[name = "jump-label-alphabet"]
        #[read = deref]
        jump_label_alphabet: String = "abcdefghijklmnopqrstuvwxyz",
    }

    struct WhiteSpaceRenderConfig {
        #[name = "whitespace.characters.space"]
        #[read = copy]
        space_char: char =  '·',   // U+00B7
        #[name = "whitespace.characters.nbsp"]
        #[read = copy]
        nbsp_char: char =  '⍽',    // U+237D
        #[name = "whitespace.characters.tab"]
        #[read = copy]
        tab_char: char =  '→',     // U+2192
        #[name = "whitespace.characters.tabpad"]
        #[read = copy]
        tabpad_char: char =  ' ', // Space character for tab padding
        #[name = "whitespace.characters.newline"]
        #[read = copy]
        newline_char: char =  '⏎', // U+23CE
        #[name = "whitespace.render.default"]
        #[read = copy]
        render: WhitespaceRenderValue = WhitespaceRenderValue::None,
        #[name = "whitespace.render.space"]
        #[read = copy]
        render_space: Option<WhitespaceRenderValue> = None,
        #[name = "whitespace.render.nbsp"]
        #[read = copy]
        render_nbsp: Option<WhitespaceRenderValue> = None,
        #[name = "whitespace.render.tab"]
        #[read = copy]
        render_tab: Option<WhitespaceRenderValue> = None,
        #[name = "whitespace.render.newline"]
        #[read = copy]
        render_newline: Option<WhitespaceRenderValue> = None,
    }

    struct TerminfoConfig {
        /// Set to `true` to override automatic detection of terminal truecolor
        /// support in the event of a false negative
        #[name = "true-color"]
        #[read = copy]
        force_true_color: bool = false,
        /// Set to `true` to override automatic detection of terminal undercurl
        /// support in the event of a false negative
        #[name = "undercurl"]
        #[read = copy]
        force_undercurl: bool = false,
    }

    struct IndentGuidesConfig {
        /// Whether to render indent guides
        #[read = copy]
        render: bool = false,
        /// Character to use for rendering indent guides
        #[read = copy]
        character: char = '│',
        /// Number of indent levels to skip
        #[read = copy]
        skip_levels: u8 = 0,
    }

    struct CursorShapeConfig {
        /// Cursor shape in normal mode
        #[name = "cursor-shape.normal"]
        #[read = copy]
        normal_mode_cursor: CursorKind = CursorKind::Block,
        /// Cursor shape in select mode
        #[name = "cursor-shape.select"]
        #[read = copy]
        select_mode_cursor: CursorKind = CursorKind::Block,
        /// Cursor shape in insert mode
        #[name = "cursor-shape.insert"]
        #[read = copy]
        insert_mode_cursor: CursorKind = CursorKind::Block,
    }

    struct FilePickerConfig {
        /// Whether to exclude hidden files from any file pickers.
        #[name = "file-picker.hidden"]
        #[read = copy]
        hidden: bool = true,
        /// Follow symlinks instead of ignoring them
        #[name = "file-picker.follow-symlinks"]
        #[read = copy]
        follow_symlinks: bool = true,
        /// Ignore symlinks that point at files already shown in the picker
        #[name = "file-picker.deduplicate-links"]
        #[read = copy]
        deduplicate_links: bool = true,
        /// Enables reading ignore files from parent directories.
        #[name = "file-picker.parents"]
        #[read = copy]
        parents: bool = true,
        /// Enables reading `.ignore` files.
        #[name = "file-picker.ignore"]
        #[read = copy]
        ignore: bool = true,
        /// Enables reading `.gitignore` files.
        #[name = "file-picker.git-ignore"]
        #[read = copy]
        git_ignore: bool = true,
        /// Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option.
        #[name = "file-picker.git-global"]
        #[read = copy]
        git_global: bool = true,
        /// Enables reading `.git/info/exclude` files.
        #[name = "file-picker.git-exclude"]
        #[read = copy]
        git_exclude: bool = true,
        /// Maximum Depth to recurse directories in file picker and global search.
        #[name = "file-picker.max-depth"]
        #[read = copy]
        max_depth: Option<usize> = None,
    }

    struct GutterConfig {
        /// Gutter Layout - list of gutter components to display
        #[name = "gutters.layout"]
        #[read = deref]
        layout: List<GutterType> = &[
            GutterType::Diagnostics,
            GutterType::Spacer,
            GutterType::LineNumbers,
            GutterType::Spacer,
            GutterType::Diff,
        ],
        /// Minimum number of characters to use for line number gutter
        #[name = "gutters.line-numbers.min-width"]
        #[read = copy]
        line_numbers_min_width: usize = 3,
    }

    struct StatusLineConfig{
        /// A list of elements aligned to the left of the statusline
        #[name = "statusline.left"]
        #[read = deref]
        left: List<StatusLineElement> =  &[
            StatusLineElement::Mode,
            StatusLineElement::Spinner,
            StatusLineElement::FileName,
            StatusLineElement::ReadOnlyIndicator,
            StatusLineElement::FileModificationIndicator,
        ],
        /// A list of elements aligned to the middle of the statusline
        #[name = "statusline.center"]
        #[read = deref]
        center: List<StatusLineElement> =  List::default(),
        /// A list of elements aligned to the right of the statusline
        #[name = "statusline.right"]
        #[read = deref]
        right: List<StatusLineElement> =  &[
            StatusLineElement::Diagnostics,
            StatusLineElement::Selections,
            StatusLineElement::Register,
            StatusLineElement::Position,
            StatusLineElement::FileEncoding,
        ],
        /// The character used to separate elements in the statusline
        #[name = "statusline.separator"]
        #[read = deref]
        separator: String = "│",
        /// The text shown in the `mode` element for normal mode
        #[name = "statusline.mode.normal"]
        #[read = deref]
        mode_indicator_normal: String = "NOR",
        /// The text shown in the `mode` element for insert mode
        #[name = "statusline.mode.insert"]
        #[read = deref]
        mode_indicator_insert: String = "INS",
        /// The text shown in the `mode` element for select mode
        #[name = "statusline.mode.select"]
        #[read = deref]
        mode_indicator_select: String = "SEL",
        /// Which diagnostic severity levels to display in the statusline
        #[name = "statusline.diagnostics"]
        #[read = deref]
        diagnostics: List<Severity> = &[Severity::Warning, Severity::Error],
        /// Which diagnostic severity levels to display for workspace diagnostics
        #[name = "statusline.workspace-diagnostics"]
        #[read = deref]
        workspace_diagnostics: List<Severity> = &[Severity::Warning, Severity::Error],
    }

    struct BufferPickerConfig {
        /// The initial position of the cursor in the buffer picker
        #[name = "buffer-picker.start-position"]
        #[read = copy]
        start_position: PickerStartPosition = PickerStartPosition::Current,
    }

    struct FileExplorerConfig {
        /// Position of file explorer (left or right)
        #[name = "explorer.position"]
        #[read = copy]
        position: FileExplorerPosition = FileExplorerPosition::Left,
        /// Column width of file explorer
        #[name = "explorer.column-width"]
        #[read = copy]
        column_width: Option<usize> = None,
    }
}
