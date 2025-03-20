#[macro_use]
pub mod macros;

mod action;
pub mod annotations;
pub mod base64;
pub mod clipboard;
pub mod diagnostic;
pub mod document;
pub mod editor;
pub mod events;
pub mod expansion;
pub mod graphics;
pub mod gutter;
pub mod handlers;
pub mod info;
pub mod input;
pub mod keyboard;
pub mod register;
pub mod theme;
pub mod tree;
pub mod view;

slotmap::new_key_type! {
    pub struct ViewId;
}

pub enum Align {
    Top,
    Center,
    Bottom,
}

pub fn align_view(doc: &mut Document, view: &View, align: Align) {
    let doc_text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(doc_text);
    let viewport = view.inner_area(doc);
    let last_line_height = viewport.height.saturating_sub(1);
    let mut view_offset = doc.view_offset(view.id);

    let relative = match align {
        Align::Center => last_line_height / 2,
        Align::Top => 0,
        Align::Bottom => last_line_height,
    };

    let text_fmt = doc.text_format(viewport.width, None);
    (view_offset.anchor, view_offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        cursor,
        -(relative as isize),
        0,
        &text_fmt,
        &view.text_annotations(doc, None),
    );
    doc.set_view_offset(view.id, view_offset);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Range {
    Document(helix_stdx::Range),
    Lsp {
        range: helix_lsp::lsp::Range,
        offset_encoding: helix_lsp::OffsetEncoding,
    },
}

pub use action::Action;
pub use diagnostic::Diagnostic;
pub use document::Document;
pub use editor::Editor;
use helix_core::char_idx_at_visual_offset;
pub use helix_core::uri::DocumentId;
pub use theme::Theme;
pub use view::View;
