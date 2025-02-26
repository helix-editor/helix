use std::mem;

use helix_core::completion::CompletionProvider;
use helix_lsp::{lsp, LanguageServerId};
use helix_view::handlers::completion::ResponseContext;

pub struct CompletionResponse {
    pub items: CompletionItems,
    pub provider: CompletionProvider,
    pub context: ResponseContext,
}

pub enum CompletionItems {
    Lsp(Vec<lsp::CompletionItem>),
    Other(Vec<CompletionItem>),
}

impl CompletionItems {
    pub fn is_empty(&self) -> bool {
        match self {
            CompletionItems::Lsp(items) => items.is_empty(),
            CompletionItems::Other(items) => items.is_empty(),
        }
    }
}

impl CompletionResponse {
    pub fn take_items(&mut self, dst: &mut Vec<CompletionItem>) {
        match &mut self.items {
            CompletionItems::Lsp(items) => dst.extend(items.drain(..).map(|item| {
                CompletionItem::Lsp(LspCompletionItem {
                    item,
                    provider: match self.provider {
                        CompletionProvider::Lsp(provider) => provider,
                        _ => unreachable!(),
                    },
                    resolved: false,
                    provider_priority: self.context.priority,
                })
            })),
            CompletionItems::Other(items) if dst.is_empty() => mem::swap(dst, items),
            CompletionItems::Other(items) => dst.append(items),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LspCompletionItem {
    pub item: lsp::CompletionItem,
    pub provider: LanguageServerId,
    pub resolved: bool,
    // TODO: we should not be filtering and sorting incomplete completion list
    // according to the spec but vscode does that anyway and most servers (
    // including rust-analyzer) rely on that.. so we can't do that without
    // breaking completions.
    pub provider_priority: i8,
}

impl LspCompletionItem {
    #[inline]
    pub fn filter_text(&self) -> &str {
        self.item
            .filter_text
            .as_ref()
            .unwrap_or(&self.item.label)
            .as_str()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompletionItem {
    Lsp(LspCompletionItem),
    Other(helix_core::CompletionItem),
}

impl CompletionItem {
    #[inline]
    pub fn filter_text(&self) -> &str {
        match self {
            CompletionItem::Lsp(item) => item.filter_text(),
            CompletionItem::Other(item) => &item.label,
        }
    }
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
    pub fn provider_priority(&self) -> i8 {
        match self {
            CompletionItem::Lsp(item) => item.provider_priority,
            // sorting path completions after LSP for now
            CompletionItem::Other(_) => 1,
        }
    }

    pub fn provider(&self) -> CompletionProvider {
        match self {
            CompletionItem::Lsp(item) => CompletionProvider::Lsp(item.provider),
            CompletionItem::Other(item) => item.provider,
        }
    }

    pub fn preselect(&self) -> bool {
        match self {
            CompletionItem::Lsp(LspCompletionItem { item, .. }) => item.preselect.unwrap_or(false),
            CompletionItem::Other(_) => false,
        }
    }
}
