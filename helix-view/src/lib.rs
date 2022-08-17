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

const UNICODE_LOWER_CASE_US_START: u32 = 97;
const UNICODE_LOWER_CASE_US_END: u32 = 122;
/// Returns a character 0-9 a-z based on the window index, or 0 if out of bounds
pub fn view_index_to_identifier(view_index: usize) -> Option<char> {
    match view_index {
        0..=8 => Some(char::from_digit((view_index as u32) + 1, 10).unwrap()),
        9..=34 => {
            Some(char::from_u32(UNICODE_LOWER_CASE_US_START - 9 + (view_index as u32)).unwrap())
        }
        _ => None,
    }
}

/// Returns a character 0-9 a-z based on the window index, or 0 if out of bounds
pub fn view_identifier_to_index(view_index: char) -> Option<usize> {
    let view_index_lowercase = view_index.to_ascii_lowercase();
    match view_index_lowercase as u32 {
        49..=57 => Some(char::to_digit(view_index_lowercase, 10).unwrap() as usize - 1),
        UNICODE_LOWER_CASE_US_START..=UNICODE_LOWER_CASE_US_END => {
            Some((view_index_lowercase as u32 + 9 - UNICODE_LOWER_CASE_US_START) as usize)
        }
        _ => None,
    }
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

    let last_line_height = view.inner_height().saturating_sub(1);

    let relative = match align {
        Align::Center => last_line_height / 2,
        Align::Top => 0,
        Align::Bottom => last_line_height,
    };

    view.offset.row = line.saturating_sub(relative);
}

/// Applies a [`helix_core::Transaction`] to the given [`Document`]
/// and [`View`].
pub fn apply_transaction(
    transaction: &helix_core::Transaction,
    doc: &mut Document,
    view: &View,
) -> bool {
    // TODO remove this helper function. Just call Document::apply everywhere directly.
    doc.apply(transaction, view.id)
}

pub use document::Document;
pub use editor::Editor;
pub use theme::Theme;
pub use view::View;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_view_index_to_identifier_and_back() {
        assert_eq!(view_index_to_identifier(0), Some('1'));
        assert_eq!(view_index_to_identifier(8), Some('9'));
        assert_eq!(view_index_to_identifier(9), Some('a'));
        assert_eq!(view_index_to_identifier(34), Some('z'));
        assert_eq!(view_index_to_identifier(35), None);
        assert_eq!(view_identifier_to_index('0'), None);
        assert_eq!(view_identifier_to_index('1'), Some(0));
        assert_eq!(view_identifier_to_index('9'), Some(8));
        assert_eq!(view_identifier_to_index('a'), Some(9));
        assert_eq!(view_identifier_to_index('z'), Some(34));
        assert_eq!(view_identifier_to_index('A'), Some(9));
        assert_eq!(view_identifier_to_index('Z'), Some(34));
        assert_eq!(view_identifier_to_index('{'), None);
        assert_eq!(view_identifier_to_index('@'), None);
        assert_eq!(view_identifier_to_index('/'), None);
    }
}
