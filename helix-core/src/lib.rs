use std::fmt;

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

/// A spelling dictionary identifier, such as `en_US`.
///
/// This names the dictionary files (`dictionaries/<id>/<id>.{aff,dic}`) under the runtime
/// directories; it is not otherwise interpreted, so it can be any identifier a dictionary is
/// distributed under (`en_US`, `de_DE_frami`, `ca`, ...).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpellingLanguage(SmartString<smartstring::LazyCompact>);

impl SpellingLanguage {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SpellingLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug)]
pub struct ParseSpellingLanguageError(String);

impl std::str::FromStr for SpellingLanguage {
    type Err = ParseSpellingLanguageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The identifier is interpolated into a dictionary file path, so restrict it to a single
        // safe path component: non-empty ASCII alphanumerics plus `_` and `-`.
        if !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            Ok(Self(s.into()))
        } else {
            Err(ParseSpellingLanguageError(s.to_owned()))
        }
    }
}

impl fmt::Display for ParseSpellingLanguageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid spelling language '{}': expected a dictionary name of ASCII letters, digits, '_' or '-'",
            self.0
        )
    }
}

impl std::error::Error for ParseSpellingLanguageError {}
