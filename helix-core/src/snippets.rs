mod active;
mod elaborate;
mod parser;
mod render;

#[derive(PartialEq, Eq, Hash, Debug, PartialOrd, Ord, Clone, Copy)]
pub struct TabstopIdx(usize);
pub const LAST_TABSTOP_IDX: TabstopIdx = TabstopIdx(usize::MAX);

pub use active::ActiveSnippet;
pub use elaborate::{Snippet, SnippetElement, Transform};
pub use render::RenderedSnippet;
pub use render::SnippetRenderCtx;
