//! Utility functions to categorize a `char`.

use crate::LineEnding;

#[derive(Debug, Eq, PartialEq)]
pub enum WordCategory {
    Alphanumeric,
    Superscript,
    Subscript,
    Braille,
    Hiragana,
    Katakana,
    HangulSyllable,
    CJKIdeograph,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CharCategory {
    Whitespace,
    Eol,
    Word(WordCategory),
    Punctuation,
    Unknown,
}

#[inline]
pub fn categorize_char(ch: char) -> CharCategory {
    if char_is_line_ending(ch) {
        CharCategory::Eol
    } else if ch.is_whitespace() {
        CharCategory::Whitespace
    } else if let Some(cat) = char_word_category(ch) {
        CharCategory::Word(cat)
    } else if char_is_punctuation(ch) {
        CharCategory::Punctuation
    } else {
        CharCategory::Unknown
    }
}

/// Determine whether a character is a line ending.
#[inline]
pub fn char_is_line_ending(ch: char) -> bool {
    LineEnding::from_char(ch).is_some()
}

/// Determine whether a character qualifies as (non-line-break)
/// whitespace.
#[inline]
pub fn char_is_whitespace(ch: char) -> bool {
    // TODO: this is a naive binary categorization of whitespace
    // characters.  For display, word wrapping, etc. we'll need a better
    // categorization based on e.g. breaking vs non-breaking spaces
    // and whether they're zero-width or not.
    match ch {
        //'\u{1680}' | // Ogham Space Mark (here for completeness, but usually displayed as a dash, not as whitespace)
        '\u{0009}' | // Character Tabulation
        '\u{0020}' | // Space
        '\u{00A0}' | // No-break Space
        '\u{180E}' | // Mongolian Vowel Separator
        '\u{202F}' | // Narrow No-break Space
        '\u{205F}' | // Medium Mathematical Space
        '\u{3000}' | // Ideographic Space
        '\u{FEFF}'   // Zero Width No-break Space
        => true,

        // En Quad, Em Quad, En Space, Em Space, Three-per-em Space,
        // Four-per-em Space, Six-per-em Space, Figure Space,
        // Punctuation Space, Thin Space, Hair Space, Zero Width Space.
        '\u{2000}' ..= '\u{200B}' => true,

        _ => false,
    }
}

#[inline]
pub fn char_is_punctuation(ch: char) -> bool {
    use unicode_general_category::{get_general_category, GeneralCategory};

    matches!(
        get_general_category(ch),
        GeneralCategory::OtherPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::MathSymbol
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ModifierSymbol
    )
}

#[inline]
pub fn char_is_word(ch: char) -> bool {
    char_word_category(ch).is_some()
}

pub fn char_word_category(ch: char) -> Option<WordCategory> {
    use WordCategory::*;

    // Different subcategories so e.g. おはよう世界 is not treated as one block
    let level = match ch {
        '\u{2070}'..='\u{207f}' => Superscript,
        '\u{2080}'..='\u{2094}' => Subscript,
        '\u{2800}'..='\u{28ff}' => Braille,
        '\u{3040}'..='\u{309f}' => Hiragana,
        '\u{30a0}'..='\u{30ff}' => Katakana,
        '\u{ac00}'..='\u{d7a3}' => HangulSyllable,

        '\u{3300}'..='\u{9fff}'
        | '\u{f900}'..='\u{faff}'
        | '\u{20000}'..='\u{2a6df}'
        | '\u{2a700}'..='\u{2b73f}'
        | '\u{2b740}'..='\u{2b81f}'
        | '\u{2f800}'..='\u{2fa1f}' => CJKIdeograph,
        ch if ch.is_alphanumeric() || ch == '_' => Alphanumeric,
        _ => return None,
    };
    Some(level)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_categorize() {
        #[cfg(not(feature = "unicode-lines"))]
        const EOL_TEST_CASE: &str = "\n";
        #[cfg(feature = "unicode-lines")]
        const EOL_TEST_CASE: &str = "\n\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}";
        const WORD_TEST_CASE: &str = "_hello_world_あいうえおー1234567890１２３４５６７８９０";
        const PUNCTUATION_TEST_CASE: &str =
            "!\"#$%&\'()*+,-./:;<=>?@[\\]^`{|}~！”＃＄％＆’（）＊＋、。：；＜＝＞？＠「」＾｀｛｜｝～";
        const WHITESPACE_TEST_CASE: &str = "  　   ";

        for ch in EOL_TEST_CASE.chars() {
            assert_eq!(CharCategory::Eol, categorize_char(ch));
        }

        for ch in WHITESPACE_TEST_CASE.chars() {
            assert_eq!(
                CharCategory::Whitespace,
                categorize_char(ch),
                "Testing '{}', but got `{:?}` instead of `Category::Whitespace`",
                ch,
                categorize_char(ch)
            );
        }

        for ch in WORD_TEST_CASE.chars() {
            assert!(
                matches!(categorize_char(ch), CharCategory::Word(_)),
                "Testing '{}', but got `{:?}` instead of `Category::Word`",
                ch,
                categorize_char(ch)
            );
        }

        for ch in PUNCTUATION_TEST_CASE.chars() {
            assert_eq!(
                CharCategory::Punctuation,
                categorize_char(ch),
                "Testing '{}', but got `{:?}` instead of `Category::Punctuation`",
                ch,
                categorize_char(ch)
            );
        }
    }
}
