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
use std::collections::VecDeque;
use std::fmt::Debug;

#[cfg(test)]
mod test;

use unicode_segmentation::{Graphemes, UnicodeSegmentation};

use helix_stdx::rope::{RopeGraphemes, RopeSliceExt};

use crate::graphemes::{Grapheme, GraphemeStr};
use crate::syntax::Highlight;
use crate::text_annotations::TextAnnotations;
use crate::{Position, RopeSlice};

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

impl GraphemeSource {
    /// Returns whether this grapheme is virtual inline text
    pub fn is_virtual(self) -> bool {
        matches!(self, GraphemeSource::VirtualText { .. })
    }

    pub fn is_eof(self) -> bool {
        // all doc chars except the EOF char have non-zero codepoints
        matches!(self, GraphemeSource::Document { codepoints: 0 })
    }

    pub fn doc_chars(self) -> usize {
        match self {
            GraphemeSource::Document { codepoints } => codepoints as usize,
            GraphemeSource::VirtualText { .. } => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormattedGrapheme<'a> {
    pub raw: Grapheme<'a>,
    pub source: GraphemeSource,
    pub visual_pos: Position,
    /// Document line at the start of the grapheme
    pub line_idx: usize,
    /// Document char position at the start of the grapheme
    pub char_idx: usize,
}

impl FormattedGrapheme<'_> {
    pub fn is_virtual(&self) -> bool {
        self.source.is_virtual()
    }

    pub fn doc_chars(&self) -> usize {
        self.source.doc_chars()
    }

    pub fn is_whitespace(&self) -> bool {
        self.raw.is_whitespace()
    }

    pub fn width(&self) -> usize {
        self.raw.width()
    }

    pub fn is_word_boundary(&self) -> bool {
        self.raw.is_word_boundary()
    }
}

#[derive(Debug, Clone)]
struct GraphemeWithSource<'a> {
    grapheme: Grapheme<'a>,
    source: GraphemeSource,
}

impl<'a> GraphemeWithSource<'a> {
    fn new(
        g: GraphemeStr<'a>,
        visual_x: usize,
        tab_width: u16,
        source: GraphemeSource,
    ) -> GraphemeWithSource<'a> {
        GraphemeWithSource {
            grapheme: Grapheme::new(g, visual_x, tab_width),
            source,
        }
    }

    fn doc_chars(&self) -> usize {
        self.source.doc_chars()
    }

    fn is_whitespace(&self) -> bool {
        self.grapheme.is_whitespace()
    }

    fn is_newline(&self) -> bool {
        matches!(self.grapheme, Grapheme::Newline)
    }

    fn is_eof(&self) -> bool {
        self.source.is_eof()
    }

    fn width(&self) -> usize {
        self.grapheme.width()
    }
}

/// split_priority returns how good it would be to split between g1 and g2.
/// Lower is better.
fn split_priority(g1: Option<&Grapheme>, g2: &Grapheme) -> i32 {
    // prefer splitting after whitespace
    if g1.is_some_and(|g| g.is_whitespace()) && !g2.is_whitespace() {
        return 1;
    }
    // but before whitespace is ok too
    if g2.is_whitespace() {
        return 2;
    }
    // otherwise try to split at punctuation
    if g1.is_some_and(|g| g.is_word_boundary()) && !g2.is_word_boundary() {
        return 3;
    }
    if g2.is_word_boundary() {
        return 4;
    }
    5
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
    pub soft_wrap_at_text_width: bool,
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
            soft_wrap_at_text_width: false,
        }
    }
}

#[derive(Debug)]
struct GraphemeWithSplit<'t> {
    grapheme: GraphemeWithSource<'t>,
    split_priority: i32,
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

    inline_annotation_graphemes: Option<(Graphemes<'t>, Option<Highlight>)>,

    // softwrap specific
    /// The indentation of the current line
    /// Is set to `None` if the indentation level is not yet known
    /// because no non-whitespace graphemes have been encountered yet
    indent_level: Option<usize>,

    /// Buffer of graphemes that could be wrapped
    line_buf: VecDeque<GraphemeWithSplit<'t>>,
    /// Number of chars in line_buf
    line_buf_chars: usize,
    /// Width of line_buf. line_buf_width is at most text_fmt.maxwrap
    line_buf_width: usize,

    /// Buffer of graphemes ready to be yielded from next()
    out: VecDeque<FormattedGrapheme<'t>>,
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
    ) -> Self {
        // TODO divide long lines into blocks to avoid bad performance for long lines
        let block_line_idx = text.char_to_line(char_idx.min(text.len_chars()));
        let block_char_idx = text.line_to_char(block_line_idx);
        annotations.reset_pos(block_char_idx);

        DocumentFormatter {
            text_fmt,
            annotations,
            visual_pos: Position { row: 0, col: 0 },
            graphemes: text.slice(block_char_idx..).graphemes(),
            char_pos: block_char_idx,
            exhausted: false,
            indent_level: None,
            line_pos: block_line_idx,
            inline_annotation_graphemes: None,
            line_buf: VecDeque::new(),
            line_buf_chars: 0,
            line_buf_width: 0,
            out: VecDeque::new(),
        }
    }

    fn next_inline_annotation_grapheme(
        &mut self,
        char_pos: usize,
    ) -> Option<(&'t str, Option<Highlight>)> {
        loop {
            if let Some(&mut (ref mut annotation, highlight)) =
                self.inline_annotation_graphemes.as_mut()
            {
                if let Some(grapheme) = annotation.next() {
                    return Some((grapheme, highlight));
                }
            }

            if let Some((annotation, highlight)) =
                self.annotations.next_inline_annotation_at(char_pos)
            {
                self.inline_annotation_graphemes = Some((
                    UnicodeSegmentation::graphemes(&*annotation.text, true),
                    highlight,
                ))
            } else {
                return None;
            }
        }
    }

    fn advance_grapheme(&mut self, col: usize, char_pos: usize) -> Option<GraphemeWithSource<'t>> {
        let (grapheme, source) =
            if let Some((grapheme, highlight)) = self.next_inline_annotation_grapheme(char_pos) {
                (grapheme.into(), GraphemeSource::VirtualText { highlight })
            } else if let Some(grapheme) = self.graphemes.next() {
                let codepoints = grapheme.len_chars() as u32;

                let overlay = self.annotations.overlay_at(char_pos);
                let grapheme = match overlay {
                    Some((overlay, _)) => overlay.grapheme.as_str().into(),
                    None => Cow::from(grapheme).into(),
                };

                (grapheme, GraphemeSource::Document { codepoints })
            } else {
                if self.exhausted {
                    return None;
                }
                self.exhausted = true;
                // EOF grapheme is required for rendering
                // and correct position computations
                return Some(GraphemeWithSource {
                    grapheme: Grapheme::Other { g: " ".into() },
                    source: GraphemeSource::Document { codepoints: 0 },
                });
            };

        let grapheme = GraphemeWithSource::new(grapheme, col, self.text_fmt.tab_width, source);

        Some(grapheme)
    }

    /// Move to the next visual line
    fn wrap(&mut self) {
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

        let virtual_lines =
            self.annotations
                .virtual_lines_at(self.char_pos, self.visual_pos, self.line_pos);
        self.visual_pos.col = indent_carry_over as usize;
        self.visual_pos.row += 1 + virtual_lines;
        let mut word_width = 0;
        for g in UnicodeSegmentation::graphemes(&*self.text_fmt.wrap_indicator, true) {
            let grapheme = GraphemeWithSource::new(
                g.into(),
                self.visual_pos.col + word_width,
                self.text_fmt.tab_width,
                GraphemeSource::VirtualText {
                    highlight: self.text_fmt.wrap_indicator_highlight,
                },
            );
            word_width += grapheme.width();
            let grapheme = self.format_grapheme(grapheme);
            self.out.push_back(grapheme);
        }
        let mut visual_x = self.visual_pos.col;
        for grapheme in &mut self.line_buf {
            grapheme
                .grapheme
                .grapheme
                .change_position(visual_x, self.text_fmt.tab_width);
            visual_x += grapheme.grapheme.width();
        }
    }

    fn push_line_buf(&mut self, grapheme: GraphemeWithSplit<'t>) {
        self.line_buf_chars += grapheme.grapheme.doc_chars();
        self.line_buf_width += grapheme.grapheme.width();
        self.line_buf.push_back(grapheme);
    }

    fn pop_line_buf(&mut self) -> Option<GraphemeWithSplit<'t>> {
        let Some(grapheme) = self.line_buf.pop_front() else {
            return None;
        };
        self.line_buf_chars -= grapheme.grapheme.doc_chars();
        self.line_buf_width -= grapheme.grapheme.width();
        Some(grapheme)
    }

    fn advance_grapheme_with_soft_wrap(&mut self) {
        let Some(grapheme) =
            self.advance_grapheme(self.visual_pos.col, self.char_pos + self.line_buf_chars)
        else {
            return;
        };
        if !grapheme.is_whitespace() && self.indent_level.is_none() {
            self.indent_level = Some(self.visual_pos.col + self.line_buf_width);
        } else if grapheme.grapheme == Grapheme::Newline {
            self.indent_level = None;
        }
        let col = self.visual_pos.col + self.line_buf_width + grapheme.width();
        if col <= usize::from(self.text_fmt.viewport_width)
            || self.line_buf.is_empty()
            // The EOF char and newline chars are always selectable in helix. That means
            // that wrapping happens "too-early" if a word fits a line perfectly. This
            // is intentional so that all selectable graphemes are always visible (and
            // therefore the cursor never disappears). However if the user manually set a
            // lower softwrap width then this is undesirable. Just increasing the viewport-
            // width by one doesn't work because if a line is wrapped multiple times then
            // some words may extend past the specified width.
            //
            // So we special case a word that ends exactly at line bounds and is followed
            // by a newline/eof character here.
            || self.text_fmt.soft_wrap_at_text_width
                && (grapheme.is_newline() || grapheme.is_eof())
                && col == usize::from(self.text_fmt.viewport_width) + 1
        {
            if grapheme.grapheme == Grapheme::Newline {
                while let Some(g) = self.pop_line_buf() {
                    let g = self.format_grapheme(g.grapheme);
                    self.out.push_back(g);
                }
                let g = self.format_grapheme(grapheme);
                self.out.push_back(g);
                return;
            }
            // make space in line_buf for the new grapheme
            while self.line_buf_width + grapheme.width() > usize::from(self.text_fmt.max_wrap) {
                let Some(g) = self.pop_line_buf() else {
                    break;
                };
                let g = self.format_grapheme(g.grapheme);
                self.out.push_back(g);
            }
            let last_char = if self.line_buf.is_empty() {
                None
            } else {
                Some(&self.line_buf[self.line_buf.len() - 1].grapheme.grapheme)
            };
            let split_priority = split_priority(last_char, &grapheme.grapheme);
            self.push_line_buf(GraphemeWithSplit {
                grapheme,
                split_priority,
            });
            return;
        }
        let new_split_priority = split_priority(
            Some(&self.line_buf[self.line_buf.len() - 1].grapheme.grapheme),
            &grapheme.grapheme,
        );
        let mut best_split = new_split_priority;
        let mut best_split_idx = self.line_buf.len();
        for i in (0..self.line_buf.len()).rev() {
            if self.line_buf[i].split_priority < best_split {
                best_split = self.line_buf[i].split_priority;
                best_split_idx = i;
            }
        }
        for _ in 0..best_split_idx {
            let g = self.pop_line_buf().unwrap().grapheme;
            let g = self.format_grapheme(g);
            self.out.push_back(g);
        }
        self.wrap();

        let priority = split_priority(None, &grapheme.grapheme);
        self.push_line_buf(GraphemeWithSplit {
            grapheme,
            split_priority: priority,
        });
    }

    /// returns the char index at the end of the last yielded grapheme
    pub fn next_char_pos(&self) -> usize {
        self.char_pos
    }
    /// returns the visual position at the end of the last yielded grapheme
    pub fn next_visual_pos(&self) -> Position {
        self.visual_pos
    }

    fn format_grapheme(&mut self, grapheme: GraphemeWithSource<'t>) -> FormattedGrapheme<'t> {
        let grapheme = FormattedGrapheme {
            raw: grapheme.grapheme,
            source: grapheme.source,
            visual_pos: self.visual_pos,
            line_idx: self.line_pos,
            char_idx: self.char_pos,
        };

        self.char_pos += grapheme.doc_chars();
        if !grapheme.is_virtual() {
            self.annotations.process_virtual_text_anchors(&grapheme);
        }
        if grapheme.raw == Grapheme::Newline {
            // move to end of newline char
            self.visual_pos.col += 1;
            let virtual_lines =
                self.annotations
                    .virtual_lines_at(self.char_pos, self.visual_pos, self.line_pos);
            self.visual_pos.row += 1 + virtual_lines;
            self.visual_pos.col = 0;
            if !grapheme.is_virtual() {
                self.line_pos += 1;
            }
        } else {
            self.visual_pos.col += grapheme.width();
        }
        grapheme
    }
}

impl<'t> Iterator for DocumentFormatter<'t> {
    type Item = FormattedGrapheme<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.text_fmt.soft_wrap {
            while !self.exhausted && self.out.is_empty() {
                self.advance_grapheme_with_soft_wrap();
            }
            if let Some(g) = self.out.pop_front() {
                return Some(g);
            }
            let Some(g) = self.pop_line_buf() else {
                return None;
            };
            Some(self.format_grapheme(g.grapheme))
        } else {
            let g = self.advance_grapheme(self.visual_pos.col, self.char_pos)?;
            Some(self.format_grapheme(g))
        }
    }
}
