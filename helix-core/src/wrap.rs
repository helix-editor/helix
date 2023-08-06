use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use smartstring::{LazyCompact, SmartString};

use crate::indent::IndentStyle;

static LEADING_TABS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\t+").unwrap());
static LEADING_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^ +").unwrap());

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(
    text: &str,
    text_width: usize,
    indent_style: IndentStyle,
    tab_width: usize,
) -> SmartString<LazyCompact> {
    if indent_style == IndentStyle::Tabs {
        // textwrap doesn't handle tabs correctly (see
        // <https://github.com/helix-editor/helix/issues/3622>). So as a
        // workaround, expand leading tabs before wrapping and change them back
        // afterwards.
        //
        // If/when <https://github.com/mgeisler/textwrap/pull/490> is merged
        // upstream we can remove this workaround.
        let text = LEADING_TABS.replace_all(text, |captures: &Captures| {
            " ".repeat(captures[0].len() * tab_width)
        });
        let text = textwrap::refill(&text, text_width);
        LEADING_SPACES
            .replace_all(&text, |captures: &Captures| {
                "\t".repeat(captures[0].len() / tab_width)
            })
            .into()
    } else {
        textwrap::refill(text, text_width).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflow_hard_wrap_tabs() {
        let text = "\t\t// The quick brown fox jumps over\n\t\t// the lazy dog.\n";
        let expected = "\t\t// The quick brown fox jumps\n\t\t// over the lazy dog.\n";
        assert_eq!(reflow_hard_wrap(text, 40, IndentStyle::Tabs, 4), expected);
    }
}
