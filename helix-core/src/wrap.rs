use smartstring::{LazyCompact, SmartString};
use textwrap::{Options, WordSplitter::NoHyphenation};

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(text: &str, text_width: usize) -> SmartString<LazyCompact> {
    let options = Options::new(text_width).word_splitter(NoHyphenation);
    textwrap::refill(text, options).into()
}
