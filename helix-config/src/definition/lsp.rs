use std::fmt::{self, Display};
use std::time::Duration;

use helix_core::diagnostic::Severity;
use serde::{Deserialize, Serialize};

use crate::*;

// TODO: move to stdx
/// Helper macro that automatically generates an array
/// that contains all variants of an enum
macro_rules! variant_list {
    (
        $(#[$outer:meta])*
        $vis: vis enum $name: ident {
           $($(#[$inner: meta])* $variant: ident $(= $_: literal)?),*$(,)?
        }
    ) => {
        $(#[$outer])*
        $vis enum $name  {
           $($(#[$inner])* $variant),*
        }
        impl $name {
            $vis const ALL: &[$name] = &[$(Self::$variant),*];
        }
    }
}
variant_list! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub enum LanguageServerFeature {
        Format,
        GotoDeclaration,
        GotoDefinition,
        GotoTypeDefinition,
        GotoReference,
        GotoImplementation,
        // Goto, use bitflags, combining previous Goto members?
        SignatureHelp,
        Hover,
        DocumentHighlight,
        Completion,
        CodeAction,
        WorkspaceCommand,
        DocumentSymbols,
        WorkspaceSymbols,
        // Symbols, use bitflags, see above?
        Diagnostics,
        RenameSymbol,
        InlayHints,
    }
}

impl LanguageServerFeature {
    fn to_str(self) -> &'static str {
        use LanguageServerFeature::*;

        match self {
            Format => "format",
            GotoDeclaration => "goto-declaration",
            GotoDefinition => "goto-definition",
            GotoTypeDefinition => "goto-type-definition",
            GotoReference => "goto-reference",
            GotoImplementation => "goto-implementation",
            SignatureHelp => "signature-help",
            Hover => "hover",
            DocumentHighlight => "document-highlight",
            Completion => "completion",
            CodeAction => "code-action",
            WorkspaceCommand => "workspace-command",
            DocumentSymbols => "document-symbols",
            WorkspaceSymbols => "workspace-symbols",
            Diagnostics => "diagnostics",
            RenameSymbol => "rename-symbol",
            InlayHints => "inlay-hints",
        }
    }
    fn description(self) -> &'static str {
        use LanguageServerFeature::*;

        match self {
            Format => "Use this language server for autoformatting.",
            GotoDeclaration => "Use this language server for the goto_declaration command.",
            GotoDefinition => "Use this language server for the goto_definition command.",
            GotoTypeDefinition => "Use this language server for the goto_type_definition command.",
            GotoReference => "Use this language server for the goto_reference command.",
            GotoImplementation => "Use this language server for the goto_implementation command.",
            SignatureHelp => "Use this language server to display signature help.",
            Hover => "Use this language server to display hover information.",
            DocumentHighlight => {
                "Use this language server for the select_references_to_symbol_under_cursor command."
            }
            Completion => "Request completion items from this language server.",
            CodeAction => "Use this language server for the code_action command.",
            WorkspaceCommand => "Use this language server for :lsp-workspace-command.",
            DocumentSymbols => "Use this language server for the symbol_picker command.",
            WorkspaceSymbols => "Use this language server for the workspace_symbol_picker command.",
            Diagnostics => "Display diagnostics emitted by this language server.",
            RenameSymbol => "Use this language server for the rename_symbol command.",
            InlayHints => "Display inlay hints form this language server.",
        }
    }
}

impl Display for LanguageServerFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let feature = self.to_str();
        write!(f, "{feature}",)
    }
}

impl Debug for LanguageServerFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl Ty for LanguageServerFeature {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        let val: String = val.typed()?;
        use LanguageServerFeature::*;

        match &*val {
            "format" => Ok(Format),
            "goto-declaration" => Ok(GotoDeclaration),
            "goto-definition" => Ok(GotoDefinition),
            "goto-type-definition" => Ok(GotoTypeDefinition),
            "goto-reference" => Ok(GotoReference),
            "goto-implementation" => Ok(GotoImplementation),
            "signature-help" => Ok(SignatureHelp),
            "hover" => Ok(Hover),
            "document-highlight" => Ok(DocumentHighlight),
            "completion" => Ok(Completion),
            "code-action" => Ok(CodeAction),
            "workspace-command" => Ok(WorkspaceCommand),
            "document-symbols" => Ok(DocumentSymbols),
            "workspace-symbols" => Ok(WorkspaceSymbols),
            "diagnostics" => Ok(Diagnostics),
            "rename-symbol" => Ok(RenameSymbol),
            "inlay-hints" => Ok(InlayHints),
            _ => bail!("invalid language server feature {val}"),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_str().into())
    }
}

/// Describes the diagnostic filter level for inline diagnostics.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum DiagnosticFilter {
    Disable,
    Enable(Severity),
}

impl<'de> Deserialize<'de> for DiagnosticFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match &*std::string::String::deserialize(deserializer)? {
            "disable" => Ok(DiagnosticFilter::Disable),
            "hint" => Ok(DiagnosticFilter::Enable(Severity::Hint)),
            "info" => Ok(DiagnosticFilter::Enable(Severity::Info)),
            "warning" => Ok(DiagnosticFilter::Enable(Severity::Warning)),
            "error" => Ok(DiagnosticFilter::Enable(Severity::Error)),
            variant => Err(serde::de::Error::unknown_variant(
                variant,
                &["disable", "hint", "info", "warning", "error"],
            )),
        }
    }
}

impl Serialize for DiagnosticFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let filter = match self {
            DiagnosticFilter::Disable => "disable",
            DiagnosticFilter::Enable(Severity::Hint) => "hint",
            DiagnosticFilter::Enable(Severity::Info) => "info",
            DiagnosticFilter::Enable(Severity::Warning) => "warning",
            DiagnosticFilter::Enable(Severity::Error) => "error",
        };
        filter.serialize(serializer)
    }
}

impl Ty for DiagnosticFilter {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        let val: String = val.typed()?;
        match &*val {
            "disable" => Ok(DiagnosticFilter::Disable),
            "hint" => Ok(DiagnosticFilter::Enable(Severity::Hint)),
            "info" => Ok(DiagnosticFilter::Enable(Severity::Info)),
            "warning" => Ok(DiagnosticFilter::Enable(Severity::Warning)),
            "error" => Ok(DiagnosticFilter::Enable(Severity::Error)),
            _ => bail!("expected one of 'disable', 'hint', 'info', 'warning' or 'error' (got {val:?})"),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            DiagnosticFilter::Disable => "disable",
            DiagnosticFilter::Enable(Severity::Hint) => "hint",
            DiagnosticFilter::Enable(Severity::Info) => "info",
            DiagnosticFilter::Enable(Severity::Warning) => "warning",
            DiagnosticFilter::Enable(Severity::Error) => "error",
        }
        .into()
    }
}

pub fn init_language_server_config(registry: &mut OptionRegistry, language_server: &str) {
    registry.register(
        &format!("language-servers.{language_server}.active"),
        "Whether this language server is used for a buffer",
        false,
    );
    for &feature in LanguageServerFeature::ALL {
        registry.register(
            &format!("language-servers.{language_server}.{feature}"),
            feature.description(),
            true,
        );
    }
}

options! {
    struct LspConfig {
        /// Enables LSP integration. Setting to false will completely disable language servers.
        #[name = "lsp.enable"]
        #[read = copy]
        enable: bool = true,
        /// Display LSP messages in the status line
        #[name = "lsp.display-messages"]
        #[read = copy]
        display_messages: bool = true,
        /// Display LSP progress messages below statusline
        #[name = "lsp.display-progress-messages"]
        #[read = copy]
        display_progress_messages: bool = false,
        /// Enable automatic popup of signature help (parameter hints)
        #[name = "lsp.auto-signature-help"]
        #[read = copy]
        auto_signature_help: bool = true,
        /// Enable automatic popup of signature help (parameter hints)
        #[name = "lsp.display-inlay-hints"]
        #[read = copy]
        display_inlay_hints: bool = false,
        /// Maximum length of inlay hints. Inlay hints that exceed this length will be truncated.
        #[name = "lsp.inlay-hints-length-limit"]
        #[read = copy]
        inlay_hints_length_limit: Option<usize> = None,
        /// Display color swatches for color values in the editor
        #[name = "lsp.display-color-swatches"]
        #[read = copy]
        display_color_swatches: bool = false,
        /// Display docs under signature help popup
        #[name = "lsp.display-signature-help-docs"]
        #[read = copy]
        display_signature_help_docs: bool = true,
        /// Enables snippet completions. Requires a server restart
        /// (`:lsp-restart`) to take effect after `:config-reload`/`:set`.
        #[name = "lsp.snippets"]
        #[read = copy]
        snippets: bool = true,
        /// Include declaration in the goto references popup.
        #[name = "lsp.goto-reference-include-declaration"]
        #[read = copy]
        goto_reference_include_declaration: bool = true,
        // TODO(breaking): prefix all options below with `lsp.`
        /// The language-id for language servers, checkout the
        /// table at [TextDocumentItem](https://microsoft.github.io/
        /// language-server-protocol/specifications/lsp/3.17/specification/
        /// #textDocumentItem) for the right id
        #[name = "language-id"]
        language_server_id: Option<String> = None,
        // TODO(breaking): rename to root-markers to differentiate from workspace-roots
        // TODO: also makes this settable on the language server
        /// A set of marker files to look for when trying to find the workspace
        /// root. For example `Cargo.lock`, `yarn.lock`
        roots: List<String> = List::default(),
        // TODO: also makes this settable on the language server
        /// Directories relative to the workspace root that are treated as LSP
        /// roots. The search for root markers (starting at the path of the
        /// file) will stop at these paths.
        #[name = "workspace-lsp-roots"]
        workspace_roots: List<std::path::PathBuf> = List::default(),
        /// An array of LSP diagnostic sources assumed unchanged when the
        /// language server resends the same set of diagnostics. Helix can track
        /// the position for these diagnostics internally instead. Useful for
        /// diagnostics that are recomputed on save.
        persistent_diagnostic_sources: List<String> = List::default(),
        /// Minimal severity of diagnostic for it to be displayed. (Allowed
        /// values: `error`, `warning`, `info`, `hint`)
        diagnostic_severity: Severity = Severity::Hint,
    }

    struct CompletionConfig {
        /// Automatic auto-completion, automatically pop up without user trigger.
        #[read = copy]
        auto_completion: bool = true,
        /// Whether to apply completion item instantly when selected
        #[read = copy]
        preview_completion_insert: bool = true,
        /// Whether to apply completion item instantly when selected
        #[read = copy]
        completion_replace: bool = false,
        /// Whether to apply completion item instantly when selected
        #[read = copy]
        completion_trigger_len: u8 = 2,
        /// Enable filepath completion. Shows files and directories if an
        /// existing path at the cursor was recognized.
        #[read = copy]
        path_completion: bool = true,
        /// Time in milliseconds after typing a word character before auto
        /// completions are shown, set to 5 for instant. Defaults to 250ms.
        #[read = copy]
        completion_timeout: Duration = Duration::from_millis(250),
    }

    struct WordCompletionConfig {
        /// Enable word-based completion (completes words from open buffers)
        #[name = "word-completion.enable"]
        #[read = copy]
        enable: bool = true,
        /// Minimum word length to trigger word completion
        #[name = "word-completion.trigger-length"]
        #[read = copy]
        trigger_length: u8 = 7,
    }

    struct InlineDiagnosticsConfig {
        /// Minimum severity for diagnostics to be displayed inline on the cursor line
        #[name = "inline-diagnostics.cursor-line"]
        #[read = copy]
        cursor_line: DiagnosticFilter = DiagnosticFilter::Enable(Severity::Warning),
        /// Minimum severity for diagnostics to be displayed inline on lines other than the cursor line
        #[name = "inline-diagnostics.other-lines"]
        #[read = copy]
        other_lines: DiagnosticFilter = DiagnosticFilter::Disable,
        /// Minimum width for inline diagnostics to be displayed (space on line required)
        #[name = "inline-diagnostics.min-diagnostic-width"]
        #[read = copy]
        min_diagnostic_width: u16 = 40,
        /// Length of diagnostic prefix (spacing before diagnostic text)
        #[name = "inline-diagnostics.prefix-len"]
        #[read = copy]
        prefix_len: u16 = 1,
        /// Maximum number of lines to wrap inline diagnostic messages
        #[name = "inline-diagnostics.max-wrap"]
        #[read = copy]
        max_wrap: u16 = 20,
        /// Maximum number of diagnostics to display inline per line
        #[name = "inline-diagnostics.max-diagnostics"]
        #[read = copy]
        max_diagnostics: usize = 10,
    }

    struct EndOfLineDiagnosticsConfig {
        /// Minimum severity for diagnostics to be displayed at the end of lines
        #[name = "end-of-line-diagnostics"]
        #[read = copy]
        end_of_line_diagnostics: DiagnosticFilter = DiagnosticFilter::Enable(Severity::Hint),
    }
}
