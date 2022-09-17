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

impl From<LineEnding> for textwrap::LineEnding {
    fn from(le: LineEnding) -> Self {
        match le {
            LineEnding::Crlf => textwrap::LineEnding::CRLF,
            // Best effort, match what's supported by textwrap
            _ => textwrap::LineEnding::LF,
        }
    }
}
