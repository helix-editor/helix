pub use encoding_rs as encoding;

pub mod auto_pairs;
pub mod case_conversion;
pub mod chars;
pub mod command_line;
pub mod comment;
pub mod completion;
pub mod config;
pub mod diagnostic;
pub mod diff;
pub mod doc_formatter;
pub mod editor_config;
pub mod fuzzy;
pub mod graphemes;
pub mod history;
pub mod increment;
pub mod indent;
pub mod line_ending;
pub mod macros;
pub mod match_brackets;
pub mod movement;
pub mod object;
mod position;
pub mod search;
pub mod selection;
pub mod snippets;
pub mod surround;
pub mod syntax;
pub mod test;
pub mod text_annotations;
pub mod textobject;
mod transaction;
pub mod uri;
pub mod wrap;

pub mod unicode {
    pub use unicode_general_category as category;
    pub use unicode_segmentation as segmentation;
    pub use unicode_width as width;
    use unicode_width::UnicodeWidthStr;

    #[inline]
    #[must_use]
    pub fn width(s: &str) -> usize {
        if s.is_empty() {
            return 0;
        }

        let mut width = s.width();
        let chars = s.as_bytes();
        // `UnicodeWidthStr::width` assigns a width of 1 to certain control
        // sequences at the *string* level.
        //
        // Notably, the CRLF sequence (`"\r\n"`) is treated as a single unit
        // and has a total width of 1, even though `'\r'` and `'\n'` each
        // have a character width of 1 when considered individually.
        //
        // This function needs newline and tab characters to contribute zero
        // width. We correct for this by subtracting the count of `'\n'` and
        // `'\t'` characters from the string width.
        //
        // NOTE: Subtracting on `\n` works for `\r\n`, as this grapheme only
        // counts as 1 width, so just subtracting 1 for the `\n` would zero
        // it out, removing its contribution to the width.
        for _ in memchr::memchr2_iter(b'\n', b'\t', chars) {
            if width == 0 {
                break;
            }

            width -= 1;
        }
        width
    }

    #[cfg(test)]
    mod test {
        use super::width;

        #[test]
        fn should_have_expected_unicode_width() {
            assert_eq!(width("\n"), 0);
            assert_eq!(width("\t"), 0);
            assert_eq!(width("\r\n"), 0);
            assert_eq!(width("\r\n\t"), 0);
            assert_eq!(width("\n\t\r\n"), 0);
            assert_eq!(width("\n\tH\r\n"), 1);
            assert_eq!(width("🤦🏼‍♂️"), 2);
            assert_eq!(width("\n🤦🏼‍♂️\n"), 2);
            assert_eq!(width("\r\n🤦🏼‍♂️\r\n"), 2);
            assert_eq!(width("\t🤦🏼‍♂️\t"), 2);
            assert_eq!(width("\n\t🤦🏼‍♂️\t\n"), 2);
            assert_eq!(width("\u{200B}"), 0);
            assert_eq!(width("▲"), 1);
            assert_eq!(width(" ▲ "), 3);
            assert_eq!(width("┌"), 1);
        }
    }
}

pub use helix_loader::find_workspace;

mod rope_reader;

pub use rope_reader::RopeReader;
pub use ropey::{self, str_utils, Rope, RopeBuilder, RopeSlice};

// pub use tendril::StrTendril as Tendril;
pub use smartstring::SmartString;

pub type Tendril = SmartString<smartstring::LazyCompact>;

#[doc(inline)]
pub use {regex, tree_house::tree_sitter};

pub use position::{
    char_idx_at_visual_offset, coords_at_pos, pos_at_coords, softwrapped_dimensions,
    visual_offset_from_anchor, visual_offset_from_block, Position, VisualOffsetError,
};
#[allow(deprecated)]
pub use position::{pos_at_visual_coords, visual_coords_at_pos};

pub use selection::{Range, Selection};
pub use smallvec::{smallvec, SmallVec};
pub use syntax::Syntax;

pub use completion::CompletionItem;
pub use diagnostic::Diagnostic;

pub use line_ending::{LineEnding, NATIVE_LINE_ENDING};
pub use transaction::{Assoc, Change, ChangeSet, Deletion, Operation, Transaction};

pub use uri::Uri;

pub use tree_house::Language;
