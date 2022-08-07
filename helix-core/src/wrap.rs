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
    let mut whitespace_count: usize = 0;
    let mut trim = true;
    while let Some(event) = formatter.next() {
        match event {
            TextFormatEvent::Grapheme(grapheme, _)
                if matches!(grapheme, GraphemeKind::LineBreak) && whitespace_count > 0 =>
            {
                // -1 to not delete line break
                changes.push((
                    formatter.index - whitespace_count - 1,
                    formatter.index - 1,
                    None,
                ));
                whitespace_count = 0;
                trim = true;
            }
            TextFormatEvent::Backtrack(_, count) => {
                // -1 to not delete line break
                changes.push((
                    formatter.index - whitespace_count - 1,
                    formatter.index - 1,
                    None,
                ));
                whitespace_count = 0;
                trim = true;

                if count == 0 {
                    if let Some((from, to, _)) = changes.last_mut() {
                        *from -= count;
                        *to -= count;
                    }
                } else {
                    changes.push((
                        formatter.index,
                        formatter.index,
                        Some(SmartString::from(line_ending.as_str())),
                    ))
                }
            }
            TextFormatEvent::Grapheme(grapheme, _) if grapheme.is_whitespace() => {
                whitespace_count += 1;
            }
            TextFormatEvent::Grapheme(_, _) => {
                if trim {
                    // -1 to not delete grapheme
                    formatter.index -= 1;
                    changes.push((formatter.index - whitespace_count, formatter.index, None));
                    formatter.width = 0;
                    trim = false;
                }
                whitespace_count = 0;
            }
            TextFormatEvent::ForceBreak => {
                changes.push((
                    formatter.index,
                    formatter.index,
                    Some(SmartString::from(line_ending.as_str())),
                ));
                whitespace_count = 0;
                trim = true;
            }
        }
    }

    while let Some(event) = formatter.next() {
        match event {
            // Insert a newline if it's a virtual line break.
            TextFormatEvent::Backtrack(_, _backtrack @ 0) => changes.push((
                formatter.index(),
                formatter.index(),
                Some(SmartString::from(line_ending.as_str())),
            )),
            // Update the location of the last inserted line break if we're backtracking.
            TextFormatEvent::Backtrack(_, backtrack) => {
                if let Some((from, to, _)) = changes.last_mut() {
                    *from -= backtrack;
                    *to -= backtrack;
                }
            }
            _ => {}
        }
    }
    Transaction::change(text, changes.into_iter())
}

#[derive(Debug, PartialEq)]
pub enum GraphemeKind<'a> {
    Tab,
    Space,
    NbSpace,
    LineBreak,
    Other(Cow<'a, str>),
}

impl<'a> GraphemeKind<'a> {
    pub fn is_whitespace(&'a self) -> bool {
        matches!(
            self,
            GraphemeKind::Tab
                | GraphemeKind::Space
                | GraphemeKind::NbSpace
                | GraphemeKind::LineBreak
        )
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
    pub text: RopeSlice<'a>,
    pub graphemes: RopeGraphemes<'a>,
    pub max_width: usize,
    pub tab_width: usize,
    pub width: usize,
    pub index: usize,
    pub backtrack: usize,
    pub backtrack_width: usize,
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
            width: 0,
            index: 0,
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

    // TODO: to be used if soft-wrapping is disabled
    #[inline]
    pub fn next_line(&mut self) {
        let len = self.text.line_to_char(1);
        self.text = self.text.slice(len..);
        self.graphemes = RopeGraphemes::new(self.text);
    }

    /// Get the current char index.
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the current calculated width.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }
}

impl<'a> Iterator for TextFormatter<'a> {
    type Item = TextFormatEvent<'a>;

    // Maybe virtual text could be inserted through this?
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.graphemes.next().map(|grapheme| {
            debug_assert!(self.index >= self.backtrack);
            debug_assert!(self.width >= self.backtrack_width);

            let grapheme = Cow::from(grapheme);
            let (display_grapheme, width) = if grapheme == "\t" {
                (GraphemeKind::Tab, self.tab_width)
            } else if grapheme == " " {
                (GraphemeKind::Space, 1)
            } else if grapheme == "\u{00A0}" {
                (GraphemeKind::NbSpace, 1)
            } else if LineEnding::from_str(&grapheme).is_some() {
                (GraphemeKind::LineBreak, 1)
            } else {
                // Cow will prevent allocations if span contained in a single slice
                // which should really be the majority case
                let width = crate::graphemes::grapheme_width(&grapheme);
                (GraphemeKind::Other(grapheme), width)
            };
            // We've read one character.
            self.index += 1;
            self.backtrack += 1;
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
                    self.index -= 1;
                    TextFormatEvent::ForceBreak
                } else {
                    self.index -= self.backtrack;
                    TextFormatEvent::Backtrack(self.backtrack_width - width, self.backtrack - 1)
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
        let b = new_reflow_hard_wrap(&text, LineEnding::LF, 6, 0).apply(&mut text);
        assert_eq!(a.to_string(), b.to_string())
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
