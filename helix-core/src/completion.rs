use std::borrow::Cow;

use crate::diagnostic::LanguageServerId;
use crate::Transaction;

#[derive(Debug, PartialEq, Clone)]
pub struct CompletionItem {
    pub transaction: Transaction,
    pub label: Cow<'static, str>,
    pub kind: Cow<'static, str>,
    /// Containing Markdown
    pub documentation: String,
    pub provider: CompletionProvider,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CompletionProvider {
    Lsp(LanguageServerId),
    PathCompletions,
}

impl From<LanguageServerId> for CompletionProvider {
    fn from(id: LanguageServerId) -> Self {
        CompletionProvider::Lsp(id)
    }
}
