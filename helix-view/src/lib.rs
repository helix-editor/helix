pub mod document;
pub mod editor;
pub mod theme;
pub mod tree;
pub mod view;

use slotmap::new_key_type;
new_key_type! { pub struct DocumentId; }
new_key_type! { pub struct ViewId; }

pub use document::Document;
pub use editor::Editor;
pub use theme::Theme;
pub use view::View;
