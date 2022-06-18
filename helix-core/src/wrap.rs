use smartstring::{LazyCompact, SmartString};

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(text: &str, max_line_len: usize) -> SmartString<LazyCompact> {
    textwrap::refill(text, max_line_len).into()
}
