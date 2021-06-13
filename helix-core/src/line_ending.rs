use crate::{Rope, RopeGraphemes, RopeSlice};

/// Represents one of the valid Unicode line endings.
/// VT, FF and PS are excluded here, as we don't expect them to show up as a default line break
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum LineEnding {
    Crlf, // CarriageReturn followed by LineFeed
    LF,   // U+000A -- LineFeed
    CR,   // U+000D -- CarriageReturn
    Nel,  // U+0085 -- NextLine
    LS,   // U+2028 -- Line Separator
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
    for line in doc.lines().take(1) {
        // check first line only - unsure how sound this is
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
        }
    }
    ending
}

#[cfg(target_os = "windows")]
pub const DEFAULT_LINE_ENDING: LineEnding = LineEnding::Crlf;
#[cfg(not(target_os = "windows"))]
pub const DEFAULT_LINE_ENDING: LineEnding = LineEnding::LF;