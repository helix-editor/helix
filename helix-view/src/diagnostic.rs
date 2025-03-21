use helix_core::{diagnostic::Severity, Rope};
use helix_lsp::{lsp, LanguageServerId, OffsetEncoding};

use std::{borrow::Cow, fmt, sync::Arc};

use crate::Range;

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum NumberOrString {
    Number(i32),
    String(String),
}

impl NumberOrString {
    pub fn as_string(&self) -> Cow<'_, str> {
        match self {
            Self::Number(n) => Cow::Owned(n.to_string()),
            Self::String(s) => Cow::Borrowed(s.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
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
    Spelling,
}

impl DiagnosticProvider {
    pub fn language_server_id(&self) -> Option<LanguageServerId> {
        match self {
            Self::Lsp { server_id, .. } => Some(*server_id),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Option<Severity>,
    pub code: Option<NumberOrString>,
    pub tags: Vec<DiagnosticTag>,
    pub source: Option<String>,
    pub range: Range,
    pub provider: DiagnosticProvider,
    pub data: Option<serde_json::Value>,
}

impl fmt::Debug for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Diagnostic")
            .field("message", &self.message)
            .field("severity", &self.severity)
            .field("code", &self.code)
            .field("tags", &self.tags)
            .field("source", &self.source)
            .field("range", &self.range)
            .field("provider", &self.provider)
            .finish_non_exhaustive()
    }
}

impl Diagnostic {
    pub fn lsp(
        provider: DiagnosticProvider,
        offset_encoding: OffsetEncoding,
        diagnostic: lsp::Diagnostic,
    ) -> Self {
        let severity = diagnostic.severity.and_then(|severity| match severity {
            lsp::DiagnosticSeverity::ERROR => Some(Severity::Error),
            lsp::DiagnosticSeverity::WARNING => Some(Severity::Warning),
            lsp::DiagnosticSeverity::INFORMATION => Some(Severity::Info),
            lsp::DiagnosticSeverity::HINT => Some(Severity::Hint),
            severity => {
                log::error!("unrecognized diagnostic severity: {:?}", severity);
                None
            }
        });
        let code = match diagnostic.code {
            Some(x) => match x {
                lsp::NumberOrString::Number(x) => Some(NumberOrString::Number(x)),
                lsp::NumberOrString::String(x) => Some(NumberOrString::String(x)),
            },
            None => None,
        };
        let tags = if let Some(tags) = diagnostic.tags {
            tags.into_iter()
                .filter_map(|tag| match tag {
                    lsp::DiagnosticTag::DEPRECATED => Some(DiagnosticTag::Deprecated),
                    lsp::DiagnosticTag::UNNECESSARY => Some(DiagnosticTag::Unnecessary),
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };

        Self {
            message: diagnostic.message,
            severity,
            code,
            tags,
            source: diagnostic.source,
            range: Range::Lsp {
                range: diagnostic.range,
                offset_encoding,
            },
            provider,
            data: diagnostic.data,
        }
    }

    /// Converts the diagnostic to a [lsp::Diagnostic].
    pub fn to_lsp_diagnostic(
        &self,
        text: &Rope,
        offset_encoding: OffsetEncoding,
    ) -> lsp::Diagnostic {
        let range = match self.range {
            Range::Document(range) => helix_lsp::util::range_to_lsp_range(
                text,
                helix_core::Range::new(range.start, range.end),
                offset_encoding,
            ),
            Range::Lsp { range, .. } => range,
        };
        let severity = self.severity.map(|severity| match severity {
            Severity::Hint => lsp::DiagnosticSeverity::HINT,
            Severity::Info => lsp::DiagnosticSeverity::INFORMATION,
            Severity::Warning => lsp::DiagnosticSeverity::WARNING,
            Severity::Error => lsp::DiagnosticSeverity::ERROR,
        });
        let code = match self.code.clone() {
            Some(x) => match x {
                NumberOrString::Number(x) => Some(lsp::NumberOrString::Number(x)),
                NumberOrString::String(x) => Some(lsp::NumberOrString::String(x)),
            },
            None => None,
        };
        let new_tags: Vec<_> = self
            .tags
            .iter()
            .map(|tag| match tag {
                DiagnosticTag::Unnecessary => lsp::DiagnosticTag::UNNECESSARY,
                DiagnosticTag::Deprecated => lsp::DiagnosticTag::DEPRECATED,
            })
            .collect();
        let tags = if !new_tags.is_empty() {
            Some(new_tags)
        } else {
            None
        };

        lsp::Diagnostic {
            range,
            severity,
            code,
            source: self.source.clone(),
            message: self.message.clone(),
            tags,
            data: self.data.clone(),
            ..Default::default()
        }
    }
}

impl PartialEq for Diagnostic {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
            && self.severity == other.severity
            && self.code == other.code
            && self.tags == other.tags
            && self.source == other.source
            && self.range == other.range
            && self.provider == other.provider
            && self.data == other.data
    }
}

impl Eq for Diagnostic {}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.range, self.severity, self.provider.clone()).cmp(&(
            other.range,
            other.severity,
            other.provider.clone(),
        ))
    }
}
