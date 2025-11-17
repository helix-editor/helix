pub use helix_stdx::range::Range;

/// Corresponds to [`lsp_types::Diagnostic`](https://docs.rs/lsp-types/0.94.0/lsp_types/struct.Diagnostic.html)
#[derive(Debug, Clone)]
pub struct Lens {
    pub range: Range,
    pub line: usize,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
