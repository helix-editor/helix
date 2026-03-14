//! LSP diagnostic utility types.
use std::{fmt, sync::Arc};

pub use helix_stdx::range::Range;
use serde::{Deserialize, Serialize};

/// Describes the severity level of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Severity {
    #[default]
    Hint,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Eq, Hash, PartialEq, Clone, Deserialize, Serialize)]
pub enum NumberOrString {
    Number(i32),
    String(String),
}

#[derive(Debug, Clone)]
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
}

/// Corresponds to [`lsp_types::Diagnostic`](https://docs.rs/lsp-types/0.94.0/lsp_types/struct.Diagnostic.html)
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    // whether this diagnostic ends at the end of(or inside) a word
    pub ends_at_word: bool,
    pub starts_at_word: bool,
    pub zero_width: bool,
    pub line: usize,
    pub message: String,
    pub severity: Option<Severity>,
    pub code: Option<NumberOrString>,
    pub provider: DiagnosticProvider,
    pub tags: Vec<DiagnosticTag>,
    pub source: Option<String>,
    pub data: Option<serde_json::Value>,
}

/// The source of a diagnostic.
///
/// This type is cheap to clone: all data is either `Copy` or wrapped in an `Arc`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticProvider {
    Lsp {
        /// The ID of the language server which sent the diagnostic.
        server_id: LanguageServerId,
        /// An optional identifier under which diagnostics are managed by the client.
        ///
        /// `identifier` is a field from the LSP "Pull Diagnostics" feature meant to provide an
        /// optional "namespace" for diagnostics: a language server can respond to a diagnostics
        /// pull request with an identifier and these diagnostics should be treated as separate
        /// from push diagnostics. Rust-analyzer uses this feature for example to provide Cargo
        /// diagnostics with push and internal diagnostics with pull. The push diagnostics should
        /// not clear the pull diagnostics and vice-versa.
        identifier: Option<Arc<str>>,
    },
    // Future internal features can go here...
}

impl DiagnosticProvider {
    pub fn language_server_id(&self) -> Option<LanguageServerId> {
        match self {
            Self::Lsp { server_id, .. } => Some(*server_id),
            // _ => None,
        }
    }
}

// while I would prefer having this in helix-lsp that necessitates a bunch of
// conversions I would rather not add. I think its fine since this just a very
// trivial newtype wrapper and we would need something similar once we define
// completions in core
slotmap::new_key_type! {
    pub struct LanguageServerId;
}

impl fmt::Display for LanguageServerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Diagnostic {
    #[inline]
    pub fn severity(&self) -> Severity {
        self.severity.unwrap_or(Severity::Warning)
    }
}
