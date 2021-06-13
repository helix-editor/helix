use crate::{Rope, RopeGraphemes, RopeSlice};

/// Represents one of the valid Unicode line endings.
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum LineEnding {
    Crlf, // CarriageReturn followed by LineFeed
    LF,   // U+000A -- LineFeed
    CR,   // U+000D -- CarriageReturn
    Nel,  // U+0085 -- NextLine
    LS,   // U+2028 -- Line Separator
    VT,   // U+000B -- VerticalTab
    FF,   // U+000C -- FormFeed
    PS,   // U+2029 -- ParagraphSeparator
}

pub fn rope_slice_to_line_ending(g: &RopeSlice) -> Option<LineEnding> {
    if let Some(text) = g.as_str() {
        str_to_line_ending(text)
    } else if g == "\u{000D}\u{000A}" {
        Some(LineEnding::Crlf)
    } else {
        // Not a line ending
        None
    }
}

pub fn str_to_line_ending(g: &str) -> Option<LineEnding> {
    match g {
        "\u{000D}\u{000A}" => Some(LineEnding::Crlf),
        "\u{000A}" => Some(LineEnding::LF),
        "\u{000D}" => Some(LineEnding::CR),
        "\u{0085}" => Some(LineEnding::Nel),
        "\u{2028}" => Some(LineEnding::LS),
        // Not a line ending
        _ => None,
    }
}

pub fn auto_detect_line_ending(doc: &Rope) -> Option<LineEnding> {
    // based on https://github.com/cessen/led/blob/27572c8838a1c664ee378a19358604063881cc1d/src/editor/mod.rs#L88-L162

    let mut ending = None;
    // return first matched line ending. Not all possible line endings are being matched, as they might be special-use only
    for line in doc.lines().take(100) {
        ending = match line.len_chars() {
            1 => {
                let g = RopeGraphemes::new(line.slice((line.len_chars() - 1)..))
                    .last()
                    .unwrap();
                rope_slice_to_line_ending(&g)
            }
            n if n > 1 => {
                let g = RopeGraphemes::new(line.slice((line.len_chars() - 2)..))
                    .last()
                    .unwrap();
                rope_slice_to_line_ending(&g)
            }
            _ => None,
        };
        if ending.is_some() {
            return ending;
        }
    }
    ending
}

#[cfg(target_os = "windows")]
pub const DEFAULT_LINE_ENDING: LineEnding = LineEnding::Crlf;
#[cfg(not(target_os = "windows"))]
pub const DEFAULT_LINE_ENDING: LineEnding = LineEnding::LF;

#[cfg(test)]
mod line_ending_tests {
    use super::*;

    #[test]
    fn test_autodetect() {
        assert_eq!(
            auto_detect_line_ending(&Rope::from_str("\n")),
            Some(LineEnding::LF)
        );
        assert_eq!(
            auto_detect_line_ending(&Rope::from_str("\r\n")),
            Some(LineEnding::Crlf)
        );
        assert_eq!(auto_detect_line_ending(&Rope::from_str("hello")), None);
        assert_eq!(auto_detect_line_ending(&Rope::from_str("")), None);
        assert_eq!(
            auto_detect_line_ending(&Rope::from_str("hello\nhelix\r\n")),
            Some(LineEnding::LF)
        );
        assert_eq!(
            auto_detect_line_ending(&Rope::from_str("a formfeed\u{000C}")),
            None
        );
        assert_eq!(
            auto_detect_line_ending(&Rope::from_str("\n\u{000A}\n \u{000A}")),
            Some(LineEnding::LF)
        );
        assert_eq!(
            auto_detect_line_ending(&Rope::from_str(
                "a formfeed\u{000C} with a\u{000C} linefeed\u{000A}"
            )),
            Some(LineEnding::LF)
        );
        assert_eq!(auto_detect_line_ending(&Rope::from_str("a formfeed\u{000C} with a\u{000C} carriage return linefeed\u{000D}\u{000A} and a linefeed\u{000A}")), Some(LineEnding::Crlf));
    }

    #[test]
    fn test_rope_slice_to_line_ending() {
        let r = Rope::from_str("\r\n");
        assert_eq!(
            rope_slice_to_line_ending(&r.slice(1..2)),
            Some(LineEnding::LF)
        );
        assert_eq!(
            rope_slice_to_line_ending(&r.slice(0..2)),
            Some(LineEnding::Crlf)
        );
    }
}
