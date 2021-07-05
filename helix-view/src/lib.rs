#[macro_use]
pub mod macros;

pub mod clipboard;
pub mod decorations;
pub mod document;
pub mod editor;
pub mod graphics;
pub mod info;
pub mod input;
pub mod keyboard;
pub mod theme;
pub mod tree;
pub mod view;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct DocumentId(usize);

slotmap::new_key_type! {
    pub struct ViewId;
}

pub use document::Document;
pub use editor::Editor;
pub use theme::Theme;
pub use view::View;
