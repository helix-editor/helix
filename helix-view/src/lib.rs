pub mod document;
pub mod editor;
pub mod register_selection;
pub mod theme;
pub mod tree;
pub mod view;

use slotmap::new_key_type;
new_key_type! { pub struct DocumentId; }
new_key_type! { pub struct ViewId; }

pub use document::Document;
pub use editor::Editor;
pub use register_selection::RegisterSelection;
pub use theme::Theme;
pub use view::View;
