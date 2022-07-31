use std::borrow::Cow;

use ropey::{Rope, RopeSlice};
use smartstring::{LazyCompact, SmartString};

use crate::{LineEnding, RopeGraphemes, Transaction};

/// Given a slice of text, return the text re-wrapped to fit it
/// within the given width.
pub fn reflow_hard_wrap(text: &str, max_line_len: usize) -> SmartString<LazyCompact> {
    textwrap::refill(text, max_line_len).into()
}

pub fn new_reflow_hard_wrap(
    text: &Rope,
    line_ending: LineEnding,
    max_width: usize,
    tab_width: usize,
) -> Transaction {
    let mut changes = Vec::new();
    let mut formatter = TextFormatter::new(text.slice(..), max_width, tab_width);
    while let Some(event) = formatter.next() {
        match event {
            TextFormatEvent::Backtrack(_, _backtrack @ 0) => changes.push((
                formatter.index(),
                formatter.index(),
                Some(SmartString::from(line_ending.as_str())),
            )),
            TextFormatEvent::Backtrack(_, backtrack) => {
                let mut change = changes.last_mut().unwrap();
                change.0 -= backtrack;
                change.1 -= backtrack;
            }
            _ => {}
        }
    }
    Transaction::change(text, changes.into_iter())
}

#[derive(Debug)]
pub enum GraphemeKind<'a> {
    Tab,
    Space,
    NbSpace,
    LineBreak,
    Other(RopeSlice<'a>),
}

/// An event created by [TextWrap].
#[derive(Debug)]
pub enum TextFormatEvent<'a> {
    /// Grapheme and its width.
    Grapheme(GraphemeKind<'a>, usize),
    /// The (width, len_chars) to backtrack. To be interpreted as going to the next virtual line.
    Backtrack(usize, usize),
}

/// Iterates over the text's graphemes yielding [TextWrapEvent]s.
pub struct TextFormatter<'a> {
    text: RopeSlice<'a>,
    graphemes: RopeGraphemes<'a>,
    max_width: usize,
    tab_width: usize,
    width: usize,
    idx: usize,
    backtrack: usize,
    backtrack_width: usize,
}

impl<'a> TextFormatter<'a> {
    /// Create a new [TextWrap] instance.
    // If you want to offset the text, you can have `max_width = offset + max_width`
    // and ignore any grapheme events yielded before the offset.
    pub fn new(text: RopeSlice<'a>, max_width: usize, tab_width: usize) -> Self {
        Self {
            text,
            graphemes: RopeGraphemes::new(text),
            max_width,
            tab_width,
            width: 0,
            idx: 0,
            backtrack: 0,
            backtrack_width: 0,
        }
    }

    /// Offset the internal calculated width by n characters.
    // TODO: To be used in the editor to indent virtual lines.
    pub fn offset(&mut self, offset: usize) {
        self.width += offset;
        self.backtrack = 0;
        self.backtrack_width = 0;
    }

    pub fn index(&self) -> usize {
        self.idx
    }
}

impl<'a> Iterator for TextFormatter<'a> {
    type Item = TextFormatEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Maybe virtual text could be inserted through this function?
        self.graphemes.next().map(|grapheme| {
            let display_grapheme = Cow::from(grapheme);
            let (display_grapheme, width) = if display_grapheme == "\t" {
                (GraphemeKind::Tab, self.tab_width)
            } else if display_grapheme == " " {
                (GraphemeKind::Space, 1)
            } else if display_grapheme == "\u{00A0}" {
                (GraphemeKind::NbSpace, 1)
            } else {
                // Cow will prevent allocations if span contained in a single slice
                // which should really be the majority case
                let width = crate::graphemes::grapheme_width(&display_grapheme);
                (GraphemeKind::Other(grapheme), width)
            };
            self.idx += 1;
            self.backtrack += 1;

            // Check if the total width of the line exceeds the max width. If so, then
            // a backtrack is yielded.
            if self.width + width >= self.max_width {
                // If the backtrack width is greater than 80 chars (TODO: configurable).
                // then it won't try to fit the entire word.
                let event = if self.backtrack_width + width < 80 {
                    self.graphemes =
                        RopeGraphemes::new(self.text.slice(self.idx - self.backtrack..));
                    TextFormatEvent::Backtrack(self.backtrack_width, self.backtrack)
                } else {
                    TextFormatEvent::Backtrack(0, 0)
                };
                self.backtrack = 0;
                self.backtrack_width = 0;

                event
            } else {
                self.width += width;
                self.backtrack_width += width;

                TextFormatEvent::Grapheme(display_grapheme, width)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_reflow() {}

    #[test]
    fn test_text_formatter() {}
}
