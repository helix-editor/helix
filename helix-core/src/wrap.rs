use smartstring::{LazyCompact, SmartString};

use crate::LineEnding;

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(
    text: &str,
    max_line_len: usize,
    line_ending: LineEnding,
) -> SmartString<LazyCompact> {
    let options = textwrap::Options::new(max_line_len).line_ending(line_ending.into());
    textwrap::refill(text, options).into()
}

impl Into<textwrap::LineEnding> for LineEnding {
    fn into(self) -> textwrap::LineEnding {
        match self {
            LineEnding::Crlf => textwrap::LineEnding::CRLF,
            // Best effort, match what's supported by textwrap
            _ => textwrap::LineEnding::LF,
        }
    }
}
