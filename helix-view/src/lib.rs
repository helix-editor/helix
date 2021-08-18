#[macro_use]
pub mod macros;

pub mod clipboard;
pub mod compositor;
pub mod commands;
pub mod completers;
pub mod completion;
pub mod job;
pub mod widgets;
pub mod document;
pub mod editor;
pub mod graphics;
pub mod info;
pub mod input;
pub mod keyboard;
pub mod register_selection;
pub mod theme;
pub mod tree;
pub mod view;

slotmap::new_key_type! {
    pub struct DocumentId;
    pub struct ViewId;
}

pub use document::Document;
pub use editor::Editor;
pub use register_selection::RegisterSelection;
pub use theme::Theme;
pub use view::View;

pub trait Frontend {}
