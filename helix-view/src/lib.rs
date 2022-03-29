#[macro_use]
pub mod macros;

pub mod backend {
    #[cfg(feature = "term")]
    pub mod term;
}
pub mod clipboard;
pub mod document;
pub mod editor;
pub mod graphics;
pub mod gutter;
pub mod handlers {
    #[cfg(feature = "dap")]
    pub mod dap;
    pub mod lsp;
}
pub mod info;
pub mod input;
pub mod keyboard;
pub mod theme;
pub mod tree;
pub mod view;

use std::num::NonZeroUsize;

// uses NonZeroUsize so Option<DocumentId> use a byte rather than two
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DocumentId(NonZeroUsize);

impl Default for DocumentId {
    fn default() -> DocumentId {
        // Safety: 1 is non-zero
        DocumentId(unsafe { NonZeroUsize::new_unchecked(1) })
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

slotmap::new_key_type! {
    pub struct ViewId;
}

pub enum Align {
    Top,
    Center,
    Bottom,
}

pub fn align_view(doc: &Document, view: &mut View, align: Align) {
    let pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));
    let line = doc.text().char_to_line(pos);

    let height = view.inner_area().height as usize;

    let relative = match align {
        Align::Center => height / 2,
        Align::Top => 0,
        Align::Bottom => height,
    };

    view.offset.row = line.saturating_sub(relative);
}

pub use document::Document;
pub use editor::Editor;
pub use theme::Theme;
pub use view::View;
