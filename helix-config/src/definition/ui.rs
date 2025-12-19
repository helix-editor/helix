use serde::{Deserialize, Serialize};

use crate::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PopupBorderConfig {
    None,
    All,
    Popup,
    Menu,
}

config_serde_adapter!(PopupBorderConfig);

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
        /// Whether to color the mode indicator with different colors depending on the mode itself
        #[read = copy]
        popup_border: bool = false,
        /// Whether to color the mode indicator with different colors depending on the mode itself
        #[read = copy]
        color_modes: bool = false,
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
        tabpad_char: char =  '⏎', // U+23CE
        #[name = "whitespace.characters.newline"]
        #[read = copy]
        newline_char: char =  ' ',
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
    }
}
