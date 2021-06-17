/// Determine whether a character is a line break.
pub fn char_is_linebreak(c: char) -> bool {
    matches!(
        c,
        '\u{000A}' | // LineFeed
        '\u{000B}' | // VerticalTab
        '\u{000C}' | // FormFeed
        '\u{000D}' | // CarriageReturn
        '\u{0085}' | // NextLine
        '\u{2028}' | // Line Separator
        '\u{2029}' // ParagraphSeparator
    )
}

/// Determine whether a character qualifies as (non-line-break)
/// whitespace.
pub fn char_is_whitespace(c: char) -> bool {
    // TODO: this is a naive binary categorization of whitespace
    // characters.  For display, word wrapping, etc. we'll need a better
    // categorization based on e.g. breaking vs non-breaking spaces
    // and whether they're zero-width or not.
    match c {
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
        c if ('\u{2000}' ..= '\u{200B}').contains(&c) => true,

        _ => false,
    }
}
