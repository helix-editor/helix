#[macro_use]
pub mod macros;

pub mod clipboard;
pub mod document;
pub mod editor;
pub mod env;
pub mod graphics;
pub mod gutter;
pub mod handlers {
    pub mod dap;
    pub mod lsp;
}
pub mod base64;
pub mod info;
pub mod input;
pub mod keyboard;
pub mod register;
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
    let doc_text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(doc_text);
    let viewport = view.inner_area(doc);
    let last_line_height = viewport.height.saturating_sub(1);

    let relative = match align {
        Align::Center => last_line_height / 2,
        Align::Top => 0,
        Align::Bottom => last_line_height,
    };

    let text_fmt = doc.text_format(viewport.width, None);
    let annotations = view.text_annotations(doc, None);
    (view.offset.anchor, view.offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        cursor,
        -(relative as isize),
        0,
        &text_fmt,
        &annotations,
    );
}

pub use document::Document;
pub use editor::Editor;
use helix_core::char_idx_at_visual_offset;
pub use theme::Theme;
pub use view::View;
