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

    // * If the leading graphemes are whitespace, we want to keep track of how many there
    // are. Once a non-whitespace grapheme or a virtual linebreak is returned, we want to
    // delete those whitespace graphemes and reset the formatter's width back to 0.
    // * If the trailing graphemes are whitespace, we want to count how many there are.
    // If no non-whitespace graphemes are returned at a virtual line break, we delete them
    // and reset the formatter's width back to 0.
    // * All other whitespace should be preserved.
    let mut line_width: usize = 0;
    let mut whitespace_count: usize = 0;
    let mut whitespace_width: usize = 0;
    while let Some(event) = formatter.next() {
        match event {
            TextFormatEvent::Backtrack(count, width) if whitespace_count > 0 && count == 0 => {
                whitespace_count += count;
                whitespace_width += width;
                formatter.offset(line_width);
                formatter.delete_grapheme(count, whitespace_width, false);
                changes.push((
                    formatter.index() - whitespace_count,
                    formatter.index(),
                    None,
                ));
                whitespace_count = 0;
                whitespace_width = 0;
            }
            TextFormatEvent::Grapheme(GraphemeKind::LineBreak(line_ending), width)
                if whitespace_count > 0 =>
            {
                whitespace_count += line_ending.len_chars();
                whitespace_width += width;
                formatter.offset(line_width);
                formatter.delete_width(whitespace_width, false);
                changes.push((
                    formatter.index() - whitespace_count,
                    formatter.index(),
                    None,
                ));
                whitespace_count = 0;
                whitespace_width = 0;
            }
            // Insert line breaks and reset calculated line width.
            TextFormatEvent::Backtrack(count, width) if count == 0 => {
                line_width -= width;
                if let Some((from, to, _)) = changes.last_mut() {
                    *from -= count;
                    *to -= count;
                }
            }
            TextFormatEvent::Backtrack(_, width) => {
                line_width -= width;
                changes.push((
                    formatter.index(),
                    formatter.index(),
                    Some(line_ending.as_str().into()),
                ))
            }
            TextFormatEvent::ForceBreak
            | TextFormatEvent::Grapheme(GraphemeKind::LineBreak(_), _) => {
                line_width = 0;
                changes.push((
                    formatter.index(),
                    formatter.index(),
                    Some(line_ending.as_str().into()),
                ))
            }
            // Line with leading whitespace graphemes.
            // Reset width.
            TextFormatEvent::Grapheme(grapheme, width)
                if formatter.width == width && grapheme.is_whitespace() =>
            {
                whitespace_count += grapheme.len_chars();
                whitespace_width += width;
                formatter.delete_width(width, false);
            }
            // Track whitespace.
            TextFormatEvent::Grapheme(grapheme, width) if grapheme.is_whitespace() => {
                whitespace_count += grapheme.len_chars();
                whitespace_width += width;
            }
            // Trim leading whitespace (width == 0) if a non-whitespace grapheme is returned.
            TextFormatEvent::Grapheme(grapheme, width)
                if formatter.width() == 0 && whitespace_count > 0 =>
            {
                formatter.delete_grapheme(grapheme.len_chars(), width, true);
                changes.push((
                    formatter.index() - whitespace_count,
                    formatter.index(),
                    None,
                ));
                whitespace_count = 0;
                whitespace_width = 0;
            }
            // Reset whitespace counter if a non-whitespace grapheme is returned.
            TextFormatEvent::Grapheme(_, width) => {
                line_width += width;
                whitespace_count = 0;
                whitespace_width = 0;
            }
        }
    }
    Transaction::change(text, changes.into_iter())
}

#[derive(Debug, PartialEq)]
pub enum GraphemeKind<'a> {
    Tab,
    Space,
    NbSpace,
    LineBreak(LineEnding),
    Other(RopeSlice<'a>),
}

impl<'a> GraphemeKind<'a> {
    pub fn is_whitespace(&'a self) -> bool {
        matches!(
            self,
            GraphemeKind::Tab
                | GraphemeKind::Space
                | GraphemeKind::NbSpace
                | GraphemeKind::LineBreak(_)
        )
    }

    pub fn len_chars(&self) -> usize {
        match self {
            Self::Tab | Self::Space | Self::NbSpace => 1,
            Self::LineBreak(line_ending) => line_ending.len_chars(),
            Self::Other(grapheme) => grapheme.len_bytes(),
        }
    }
}

/// An event created by [TextFormatter].
#[derive(Debug, PartialEq)]
pub enum TextFormatEvent<'a> {
    /// Grapheme and its width.
    Grapheme(GraphemeKind<'a>, usize),
    /// The (width, len_chars) to backtrack. To be interpreted as going to the next virtual line.
    Backtrack(usize, usize),
    ForceBreak,
}

/// Iterates over the text's graphemes yielding [TextFormatEvent]s.
pub struct TextFormatter<'a> {
    text: RopeSlice<'a>,
    graphemes: RopeGraphemes<'a>,
    max_width: usize,
    tab_width: usize,
    index: usize,
    width: usize,
    backtrack: usize,
    backtrack_width: usize,
}

impl<'a> TextFormatter<'a> {
    /// Create a new [TextFormatter] instance.
    // If you want to offset the text, you can have `max_width = offset + max_width`
    // and ignore any grapheme events yielded before the offset.
    pub fn new(text: RopeSlice<'a>, max_width: usize, tab_width: usize) -> Self {
        Self {
            text,
            graphemes: RopeGraphemes::new(text),
            max_width,
            tab_width,
            index: 0,
            width: 0,
            backtrack: 0,
            backtrack_width: 0,
        }
    }

    /// Offset the calculated width by `n` characters.
    // TODO: To be used in the editor to indent virtual lines.
    #[inline]
    pub fn offset(&mut self, n: usize) {
        self.width += n;
        self.backtrack = 0;
        self.backtrack_width = 0;
    }

    // // TODO: to be used if soft-wrapping is disabled
    // #[inline]
    // pub fn next_line(&mut self) {
    //     let len = self.text.line_to_char(1);
    //     self.text = self.text.slice(len..);
    //     self.graphemes = RopeGraphemes::new(self.text);
    // }

    /// Get the current char index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the current calculated width.
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn delete_grapheme(&mut self, count: usize, width: usize, delete_backtrack: bool) {
        self.index -= count;
        self.width -= width;
        self.graphemes = RopeGraphemes::new(self.text.slice(self.index..));
        if delete_backtrack {
            self.backtrack -= count;
            self.backtrack_width -= width;
        } else {
            self.backtrack = 0;
            self.backtrack_width = 0;
        }
    }

    pub fn delete_width(&mut self, width: usize, delete_backtrack: bool) {
        self.width -= width;
        if delete_backtrack {
            self.backtrack_width -= width;
        }
    }
}

impl<'a> Iterator for TextFormatter<'a> {
    type Item = TextFormatEvent<'a>;

    // Maybe virtual text could be inserted through this?
    // Always inlined because it will be in the rendering hot path.
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.graphemes.next().map(|grapheme| {
            debug_assert!(self.index >= self.backtrack);
            debug_assert!(self.width >= self.backtrack_width);

            let (display_grapheme, width) = if grapheme == "\t" {
                (GraphemeKind::Tab, self.tab_width)
            } else if grapheme == " " {
                (GraphemeKind::Space, 1)
            } else if grapheme == "\u{00A0}" {
                (GraphemeKind::NbSpace, 1)
            } else if let Some(line_ending) = LineEnding::from_rope_slice(&grapheme) {
                (GraphemeKind::LineBreak(line_ending), 1)
            } else {
                // Cow will prevent allocations if span contained in a single slice
                // which should really be the majority case
                let width = crate::graphemes::grapheme_width(&Cow::from(grapheme));
                (GraphemeKind::Other(grapheme), width)
            };
            // We've read one grapheme.
            self.index += grapheme.len_chars();
            self.backtrack += grapheme.len_chars();
            self.width += width;
            self.backtrack_width += width;

            // * If the final character is whitespace and fits within the width,
            // then it'll be rendered and we'll go to the next line.
            // * If the final character isn't whitespace and/or exceeds the width,
            // then a virtual line break event is sent and we go to the next line.
            // * If the character fits, then we sent the grapheme and increment the
            // internal width calculations.
            // * If the character is whitespace and fits, then we send the grapheme
            // and reset the backtrack counters.
            // * If we've already backtracked once for this word, then give up on
            // placing the word on one line and just insert a virtual line break.
            if self.width <= self.max_width && display_grapheme.is_whitespace() {
                self.backtrack = 0;
                self.backtrack_width = 0;
                TextFormatEvent::Grapheme(display_grapheme, width)
            } else if self.width < self.max_width {
                TextFormatEvent::Grapheme(display_grapheme, width)
            } else {
                let event = if self.backtrack_width >= self.max_width {
                    self.index -= grapheme.len_chars();
                    TextFormatEvent::ForceBreak
                } else {
                    self.index -= self.backtrack;
                    TextFormatEvent::Backtrack(
                        self.backtrack - grapheme.len_chars(),
                        self.backtrack_width - width,
                    )
                };
                self.graphemes = RopeGraphemes::new(self.text.slice(self.index..));
                self.backtrack = 0;
                self.backtrack_width = 0;
                self.width = 0;
                event
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflow() {
        let mut text = Rope::from("Hello world! How are you doing?");
        let a = reflow_hard_wrap(&text.to_string(), 6);
        new_reflow_hard_wrap(&text, LineEnding::LF, 7, 0).apply(&mut text);
        assert_eq!(a.to_string(), text.to_string())
    }

    // #[test]
    // fn test_text_formatter() {
    //     let graphemes = |text: &str| {
    //         text.chars().map(|char| {
    //             TextFormatEvent::Grapheme(
    //                 GraphemeKind::Other(Rope::from(char.to_string()).slice(..)),
    //                 1,
    //             )
    //         })
    //     };

    //     let text = Rope::from("Hello world! How are you doing?");
    //     let formatter_events = TextFormatter::new(text.slice(..), 6, 0).collect::<Vec<_>>();
    //     let mut expected = Vec::new();
    //     expected.extend(graphemes("Hello"));
    //     expected.push(TextFormatEvent::Grapheme(GraphemeKind::Space, 1));
    //     expected.push(TextFormatEvent::Backtrack(0, 0));
    //     expected.extend(graphemes("world"));
    //     expected.push(TextFormatEvent::Backtrack(5, 5));
    //     expected.extend(graphemes("world"));
    //     expected.push(TextFormatEvent::Backtrack(0, 0));
    //     expected.push(TextFormatEvent::Grapheme(GraphemeKind::Space, 1));
    //     expected.extend(graphemes("How"));
    //     expected.push(TextFormatEvent::Grapheme(GraphemeKind::Space, 1));
    //     expected.push(TextFormatEvent::Backtrack(0, 0));
    //     expected.extend(graphemes("are"));
    //     expected.push(TextFormatEvent::Grapheme(GraphemeKind::Space, 1));
    //     expected.extend(graphemes("yo"));
    //     expected.push(TextFormatEvent::Backtrack(2, 2));
    //     expected.extend(graphemes("you"));
    //     expected.push(TextFormatEvent::Grapheme(GraphemeKind::Space, 1));
    //     expected.extend(graphemes("do"));
    //     expected.push(TextFormatEvent::Backtrack(2, 2));
    //     expected.extend(graphemes("doing"));
    //     expected.push(TextFormatEvent::Backtrack(5, 5));
    //     expected.extend(graphemes("doing"));
    //     expected.push(TextFormatEvent::Backtrack(0, 0));
    //     expected.extend(graphemes("?"));

    //     assert_eq!(expected, formatter_events)
    // }
}
