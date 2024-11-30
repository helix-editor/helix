use std::borrow::Cow;

use crate::Transaction;

#[derive(Debug, PartialEq, Clone)]
pub struct CompletionItem {
    pub transaction: Transaction,
    pub label: Cow<'static, str>,
    pub kind: Cow<'static, str>,
    /// Containing Markdown
    pub documentation: String,
}
