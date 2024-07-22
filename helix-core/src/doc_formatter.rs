//! The `DocumentFormatter` forms the bridge between the raw document text
//! and onscreen positioning. It yields the text graphemes as an iterator
//! and traverses (part) of the document text. During that traversal it
//! handles grapheme detection, softwrapping and annotations.
//! It yields `FormattedGrapheme`s and their corresponding visual coordinates.
//!
//! As both virtual text and softwrapping can insert additional lines into the document
//! it is generally not possible to find the start of the previous visual line.
//! Instead the `DocumentFormatter` starts at the last "checkpoint" (usually a linebreak)
//! called a "block" and the caller must advance it as needed.

use std::borrow::Cow;
use std::fmt::Debug;
use std::mem::{replace, take};

#[cfg(test)]
mod test;

use unicode_segmentation::{Graphemes, UnicodeSegmentation};

use crate::graphemes::{Grapheme, GraphemeStr};
use crate::syntax::Highlight;
use crate::text_annotations::TextAnnotations;
use crate::{Position, RopeGraphemes, RopeSlice};

/// TODO make Highlight a u32 to reduce the size of this enum to a single word.
#[derive(Debug, Clone, Copy)]
pub enum GraphemeSource {
    Document {
        codepoints: u32,
    },
    /// Inline virtual text can not be highlighted with a `Highlight` iterator
    /// because it's not part of the document. Instead the `Highlight`
    /// is emitted right by the document formatter
    VirtualText {
        highlight: Option<Highlight>,
    },
}

#[derive(Debug, Clone)]
pub struct FormattedGrapheme<'a> {
    pub grapheme: Grapheme<'a>,
    pub source: GraphemeSource,
}

impl<'a> FormattedGrapheme<'a> {
    pub fn new(
        g: GraphemeStr<'a>,
        visual_x: usize,
        tab_width: u16,
        source: GraphemeSource,
    ) -> FormattedGrapheme<'a> {
        FormattedGrapheme {
            grapheme: Grapheme::new(g, visual_x, tab_width),
            source,
        }
    }
    /// Returns whether this grapheme is virtual inline text
    pub fn is_virtual(&self) -> bool {
        matches!(self.source, GraphemeSource::VirtualText { .. })
    }

    pub fn placeholder() -> Self {
        FormattedGrapheme {
            grapheme: Grapheme::Other { g: " ".into() },
            source: GraphemeSource::Document { codepoints: 0 },
        }
    }

    pub fn doc_chars(&self) -> usize {
        match self.source {
            GraphemeSource::Document { codepoints } => codepoints as usize,
            GraphemeSource::VirtualText { .. } => 0,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        self.grapheme.is_whitespace()
    }

    pub fn width(&self) -> usize {
        self.grapheme.width()
    }

    pub fn is_word_boundary(&self) -> bool {
        self.grapheme.is_word_boundary()
    }
}

#[derive(Debug, Clone)]
pub struct TextFormat {
    pub soft_wrap: bool,
    pub tab_width: u16,
    pub max_wrap: u16,
    pub max_indent_retain: u16,
    pub wrap_indicator: Box<str>,
    pub wrap_indicator_highlight: Option<Highlight>,
    pub viewport_width: u16,
}

// test implementation is basically only used for testing or when softwrap is always disabled
impl Default for TextFormat {
    fn default() -> Self {
        TextFormat {
            soft_wrap: false,
            tab_width: 4,
            max_wrap: 3,
            max_indent_retain: 4,
            wrap_indicator: Box::from(" "),
            viewport_width: 17,
            wrap_indicator_highlight: None,
        }
    }
}

#[derive(Debug)]
pub struct DocumentFormatter<'t> {
    text_fmt: &'t TextFormat,
    annotations: &'t TextAnnotations<'t>,

    /// The visual position at the end of the last yielded word boundary
    visual_pos: Position,
    graphemes: RopeGraphemes<'t>,
    /// The character pos of the `graphemes` iter used for inserting annotations
    char_pos: usize,
    /// The line pos of the `graphemes` iter used for inserting annotations
    line_pos: usize,
    exhausted: bool,

    /// Line breaks to be reserved for virtual text
    /// at the next line break
    virtual_lines: usize,
    inline_anntoation_graphemes: Option<(Graphemes<'t>, Option<Highlight>)>,

    // softwrap specific
    /// The indentation of the current line
    /// Is set to `None` if the indentation level is not yet known
    /// because no non-whitespace graphemes have been encountered yet
    indent_level: Option<usize>,
    /// In case a long word needs to be split a single grapheme might need to be wrapped
    /// while the rest of the word stays on the same line
    peeked_grapheme: Option<(FormattedGrapheme<'t>, usize)>,
    /// A first-in first-out (fifo) buffer for the Graphemes of any given word
    word_buf: Vec<FormattedGrapheme<'t>>,
    /// The index of the next grapheme that will be yielded from the `word_buf`
    word_i: usize,
}

impl<'t> DocumentFormatter<'t> {
    /// Creates a new formatter at the last block before `char_idx`.
    /// A block is a chunk which always ends with a linebreak.
    /// This is usually just a normal line break.
    /// However very long lines are always wrapped at constant intervals that can be cheaply calculated
    /// to avoid pathological behaviour.
    pub fn new_at_prev_checkpoint(
        text: RopeSlice<'t>,
        text_fmt: &'t TextFormat,
        annotations: &'t TextAnnotations,
        char_idx: usize,
    ) -> (Self, usize) {
        // TODO divide long lines into blocks to avoid bad performance for long lines
        let block_line_idx = text.char_to_line(char_idx.min(text.len_chars()));
        let block_char_idx = text.line_to_char(block_line_idx);
        annotations.reset_pos(block_char_idx);
        (
            DocumentFormatter {
                text_fmt,
                annotations,
                visual_pos: Position { row: 0, col: 0 },
                graphemes: RopeGraphemes::new(text.slice(block_char_idx..)),
                char_pos: block_char_idx,
                exhausted: false,
                virtual_lines: 0,
                indent_level: None,
                peeked_grapheme: None,
                word_buf: Vec::with_capacity(64),
                word_i: 0,
                line_pos: block_line_idx,
                inline_anntoation_graphemes: None,
            },
            block_char_idx,
        )
    }

    fn next_inline_annotation_grapheme(&mut self) -> Option<(&'t str, Option<Highlight>)> {
        loop {
            if let Some(&mut (ref mut annotation, highlight)) =
                self.inline_anntoation_graphemes.as_mut()
            {
                if let Some(grapheme) = annotation.next() {
                    return Some((grapheme, highlight));
                }
            }

            if let Some((annotation, highlight)) =
                self.annotations.next_inline_annotation_at(self.char_pos)
            {
                self.inline_anntoation_graphemes = Some((
                    UnicodeSegmentation::graphemes(&*annotation.text, true),
                    highlight,
                ))
            } else {
                return None;
            }
        }
    }

    fn advance_grapheme(&mut self, col: usize) -> Option<FormattedGrapheme<'t>> {
        let (grapheme, source) =
            if let Some((grapheme, highlight)) = self.next_inline_annotation_grapheme() {
                (grapheme.into(), GraphemeSource::VirtualText { highlight })
            } else if let Some(grapheme) = self.graphemes.next() {
                self.virtual_lines += self.annotations.annotation_lines_at(self.char_pos);
                let codepoints = grapheme.len_chars() as u32;

                let overlay = self.annotations.overlay_at(self.char_pos);
                let grapheme = match overlay {
                    Some((overlay, _)) => overlay.grapheme.as_str().into(),
                    None => Cow::from(grapheme).into(),
                };

                self.char_pos += codepoints as usize;
                (grapheme, GraphemeSource::Document { codepoints })
            } else {
                if self.exhausted {
                    return None;
                }
                self.exhausted = true;
                // EOF grapheme is required for rendering
                // and correct position computations
                return Some(FormattedGrapheme {
                    grapheme: Grapheme::Other { g: " ".into() },
                    source: GraphemeSource::Document { codepoints: 0 },
                });
            };

        let grapheme = FormattedGrapheme::new(grapheme, col, self.text_fmt.tab_width, source);

        Some(grapheme)
    }

    /// Move a word to the next visual line
    fn wrap_word(&mut self, virtual_lines_before_word: usize) -> usize {
        // softwrap this word to the next line
        let indent_carry_over = if let Some(indent) = self.indent_level {
            if indent as u16 <= self.text_fmt.max_indent_retain {
                indent as u16
            } else {
                0
            }
        } else {
            // ensure the indent stays 0
            self.indent_level = Some(0);
            0
        };

        self.visual_pos.col = indent_carry_over as usize;
        self.virtual_lines -= virtual_lines_before_word;
        self.visual_pos.row += 1 + virtual_lines_before_word;
        let mut i = 0;
        let mut word_width = 0;
        let wrap_indicator = UnicodeSegmentation::graphemes(&*self.text_fmt.wrap_indicator, true)
            .map(|g| {
                i += 1;
                let grapheme = FormattedGrapheme::new(
                    g.into(),
                    self.visual_pos.col + word_width,
                    self.text_fmt.tab_width,
                    GraphemeSource::VirtualText {
                        highlight: self.text_fmt.wrap_indicator_highlight,
                    },
                );
                word_width += grapheme.width();
                grapheme
            });
        self.word_buf.splice(0..0, wrap_indicator);

        for grapheme in &mut self.word_buf[i..] {
            let visual_x = self.visual_pos.col + word_width;
            grapheme
                .grapheme
                .change_position(visual_x, self.text_fmt.tab_width);
            word_width += grapheme.width();
        }
        word_width
    }

    fn advance_to_next_word(&mut self) {
        self.word_buf.clear();
        let mut word_width = 0;
        let virtual_lines_before_word = self.virtual_lines;
        let mut virtual_lines_before_grapheme = self.virtual_lines;

        loop {
            // softwrap word if necessary
            if word_width + self.visual_pos.col >= self.text_fmt.viewport_width as usize {
                // wrapping this word would move too much text to the next line
                // split the word at the line end instead
                if word_width > self.text_fmt.max_wrap as usize {
                    // Usually we stop accomulating graphemes as soon as softwrapping becomes necessary.
                    // However if the last grapheme is multiple columns wide it might extend beyond the EOL.
                    // The condition below ensures that this grapheme is not cutoff and instead wrapped to the next line
                    if word_width + self.visual_pos.col > self.text_fmt.viewport_width as usize {
                        self.peeked_grapheme = self.word_buf.pop().map(|grapheme| {
                            (grapheme, self.virtual_lines - virtual_lines_before_grapheme)
                        });
                        self.virtual_lines = virtual_lines_before_grapheme;
                    }
                    return;
                }

                word_width = self.wrap_word(virtual_lines_before_word);
            }

            virtual_lines_before_grapheme = self.virtual_lines;

            let grapheme = if let Some((grapheme, virtual_lines)) = self.peeked_grapheme.take() {
                self.virtual_lines += virtual_lines;
                grapheme
            } else if let Some(grapheme) = self.advance_grapheme(self.visual_pos.col + word_width) {
                grapheme
            } else {
                return;
            };

            // Track indentation
            if !grapheme.is_whitespace() && self.indent_level.is_none() {
                self.indent_level = Some(self.visual_pos.col);
            } else if grapheme.grapheme == Grapheme::Newline {
                self.indent_level = None;
            }

            let is_word_boundary = grapheme.is_word_boundary();
            word_width += grapheme.width();
            self.word_buf.push(grapheme);

            if is_word_boundary {
                return;
            }
        }
    }

    /// returns the document line pos of the **next** grapheme that will be yielded
    pub fn line_pos(&self) -> usize {
        self.line_pos
    }

    /// returns the visual pos of the **next** grapheme that will be yielded
    pub fn visual_pos(&self) -> Position {
        self.visual_pos
    }
}

impl<'t> Iterator for DocumentFormatter<'t> {
    type Item = (FormattedGrapheme<'t>, Position);

    fn next(&mut self) -> Option<Self::Item> {
        let grapheme = if self.text_fmt.soft_wrap {
            if self.word_i >= self.word_buf.len() {
                self.advance_to_next_word();
                self.word_i = 0;
            }
            let grapheme = replace(
                self.word_buf.get_mut(self.word_i)?,
                FormattedGrapheme::placeholder(),
            );
            self.word_i += 1;
            grapheme
        } else {
            self.advance_grapheme(self.visual_pos.col)?
        };

        let pos = self.visual_pos;
        if grapheme.grapheme == Grapheme::Newline {
            self.visual_pos.row += 1;
            self.visual_pos.row += take(&mut self.virtual_lines);
            self.visual_pos.col = 0;
            self.line_pos += 1;
        } else {
            self.visual_pos.col += grapheme.width();
        }
        Some((grapheme, pos))
    }
}
