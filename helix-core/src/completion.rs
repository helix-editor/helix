use std::borrow::Cow;

use crate::{diagnostic::LanguageServerId, Transaction};

#[derive(Debug, PartialEq, Clone)]
pub struct CompletionItem {
    pub transaction: Transaction,
    pub label: Cow<'static, str>,
    pub kind: Cow<'static, str>,
    /// Containing Markdown
    pub documentation: Option<String>,
    pub provider: CompletionProvider,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CompletionProvider {
    Lsp(LanguageServerId),
    Path,
    Word,
}

impl From<LanguageServerId> for CompletionProvider {
    fn from(id: LanguageServerId) -> Self {
        CompletionProvider::Lsp(id)
    }
}
