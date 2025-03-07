//! LSP diagnostic utility types.
use std::fmt;

pub use helix_stdx::range::Range;
use serde::{Deserialize, Serialize};

/// Describes the severity level of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Hint,
    Info,
    Warning,
    Error,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Hint
    }
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

// TODO turn this into a feature flag when lsp becomes optional
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticProvider {
    PublishDiagnosticProvider(LanguageServerId),
    PullDiagnosticProvider(LanguageServerId),
}

impl From<DiagnosticProvider> for LanguageServerId {
    fn from(value: DiagnosticProvider) -> Self {
        match value {
            DiagnosticProvider::PublishDiagnosticProvider(id) => id,
            DiagnosticProvider::PullDiagnosticProvider(id) => id,
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

impl DiagnosticProvider {
    pub fn server_id(&self) -> &LanguageServerId {
        match self {
            DiagnosticProvider::PublishDiagnosticProvider(language_server_id) => language_server_id,
            DiagnosticProvider::PullDiagnosticProvider(language_server_id) => language_server_id,
        }
    }

    pub fn has_server_id(&self, server_id: &LanguageServerId) -> bool {
        match self {
            DiagnosticProvider::PublishDiagnosticProvider(language_server_id) => {
                language_server_id == server_id
            }
            DiagnosticProvider::PullDiagnosticProvider(language_server_id) => {
                language_server_id == server_id
            }
        }
    }

    pub fn equals(&self, diagnostic_provider: &DiagnosticProvider) -> bool {
        let server_id = self.server_id();
        let other_server_id = diagnostic_provider.server_id();

        server_id == other_server_id
    }
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

#[cfg(test)]
mod tests {
    use slotmap::KeyData;

    use super::DiagnosticProvider;
    use crate::diagnostic::LanguageServerId;

    #[test]
    fn can_compare_equal_diagnostic_provider() {
        let first_provider =
            DiagnosticProvider::PublishDiagnosticProvider(LanguageServerId(KeyData::from_ffi(1)));
        let second_provider =
            DiagnosticProvider::PublishDiagnosticProvider(LanguageServerId(KeyData::from_ffi(1)));

        assert!(first_provider.equals(&second_provider));
    }

    #[test]
    fn can_distinguish_diagnostic_provider() {
        let first_provider =
            DiagnosticProvider::PublishDiagnosticProvider(LanguageServerId(KeyData::from_ffi(1)));
        let second_provider =
            DiagnosticProvider::PullDiagnosticProvider(LanguageServerId(KeyData::from_ffi(1)));

        assert!(first_provider.equals(&second_provider));
    }

    #[test]
    fn can_compare_language_server_id() {
        let provider =
            DiagnosticProvider::PublishDiagnosticProvider(LanguageServerId(KeyData::from_ffi(1)));

        let language_server_id = LanguageServerId(KeyData::from_ffi(1));

        assert!(provider.has_server_id(&language_server_id));
    }
}
