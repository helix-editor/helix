use smartstring::{LazyCompact, SmartString};
use textwrap::{word_splitters::WordSplitter, Options};

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(
    text: &str,
    text_width: usize,
    no_break_on_hyphen: bool,
) -> SmartString<LazyCompact> {
    let mut options = Options::new(text_width);
    if no_break_on_hyphen {
        options.word_splitter = WordSplitter::NoHyphenation;
    }
    textwrap::refill(text, options).into()
}
