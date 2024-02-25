use crate::Transaction;

#[derive(Debug, PartialEq, Clone)]
pub struct CompletionItem {
    pub transaction: Transaction,
    pub label: String,
    /// Containing Markdown
    pub documentation: String,
}
