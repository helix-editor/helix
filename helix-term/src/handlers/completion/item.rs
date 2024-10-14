use helix_lsp::{lsp, LanguageServerId};

#[derive(Debug, PartialEq, Clone)]
pub struct LspCompletionItem {
    pub item: lsp::CompletionItem,
    pub provider: LanguageServerId,
    pub resolved: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompletionItem {
    Lsp(LspCompletionItem),
    Other(helix_core::CompletionItem),
}

impl PartialEq<CompletionItem> for LspCompletionItem {
    fn eq(&self, other: &CompletionItem) -> bool {
        match other {
            CompletionItem::Lsp(other) => self == other,
            _ => false,
        }
    }
}

impl PartialEq<CompletionItem> for helix_core::CompletionItem {
    fn eq(&self, other: &CompletionItem) -> bool {
        match other {
            CompletionItem::Other(other) => self == other,
            _ => false,
        }
    }
}

impl CompletionItem {
    pub fn preselect(&self) -> bool {
        match self {
            CompletionItem::Lsp(LspCompletionItem { item, .. }) => item.preselect.unwrap_or(false),
            CompletionItem::Other(_) => false,
        }
    }
}
