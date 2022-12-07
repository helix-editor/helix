//! The `DocumentCursor` forms the bridge between the raw document text
//! and onscreen rendering. It behaves similar to an iterator
//! and transverses (part) of the document text. During that transversal it
//! handles grapheme detection, softwrapping and annotation.
//! The result are [`Word`]s which are chunks of graphemes placed at **visual**
//! coordinates.
//!
//! The document cursor very flexible and be used to efficently map char positions in the document
//! to visual coordinates (and back).

use std::borrow::Cow;
use std::mem::take;
use std::vec;

#[cfg(test)]
mod test;

use crate::graphemes::Grapheme;
use crate::{LineEnding, Position, RopeGraphemes, RopeSlice};

/// A preprossed Grapheme that is ready for rendering
/// with attachted styling data
#[derive(Debug)]
pub struct StyledGraphemes<'a, S> {
    pub grapheme: Grapheme<'a>,
    pub style: S,
    // the number of chars in the document required by this grapheme
    pub doc_chars: u16,
}

impl<'a, S: Default> StyledGraphemes<'a, S> {
    pub fn placeholder() -> Self {
        StyledGraphemes {
            grapheme: Grapheme::Space,
            style: S::default(),
            doc_chars: 0,
        }
    }

    pub fn new(
        raw: Cow<'a, str>,
        style: S,
        visual_x: usize,
        tab_width: u16,
        chars: u16,
    ) -> StyledGraphemes<'a, S> {
        StyledGraphemes {
            grapheme: Grapheme::new(raw, visual_x, tab_width),
            style,
            doc_chars: chars,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        self.grapheme.is_whitespace()
    }

    pub fn is_breaking_space(&self) -> bool {
        self.grapheme.is_breaking_space()
    }

    /// Returns the approximate visual width of this grapheme,
    pub fn width(&self) -> u16 {
        self.grapheme.width()
    }
}

/// An annotation source allows inserting virtual text during rendering
/// that is correctly considered by the positioning and rendering code
/// The AnnotiationSource essentially fonctions as a cursor over annotations.
/// To facilitate efficent implementation it is garunteed that all
/// functions except `set_pos` are only called with increasing `char_pos`.
///
/// Further only `char_pos` correspoding to grapehme boundries are passed to `AnnotationSource`
pub trait AnnotationSource<'t, S> {
    /// Yield a grapeheme to insert at the current `char_pos`.
    /// `char_pos` will not increase as long as this funciton yields `Some` grapheme.
    fn next_annotation_grapheme(&mut self, char_pos: usize) -> Option<(Cow<'t, str>, S)>;
    /// This function is usuully only called when a [`DocumentCursor`] is created.
    /// It moves the annotation source to a random `char_pos` that might be before
    /// other char_pos previously passed to this annotation source
    fn set_pos(&mut self, char_pos: usize);
}

impl<'a, 't, S: Default, A> AnnotationSource<'t, S> for &'a mut A
where
    A: AnnotationSource<'t, S>,
{
    fn next_annotation_grapheme(&mut self, char_pos: usize) -> Option<(Cow<'t, str>, S)> {
        A::next_annotation_grapheme(self, char_pos)
    }

    fn set_pos(&mut self, char_pos: usize) {
        A::set_pos(self, char_pos);
    }
}

impl<'t, S: Default> AnnotationSource<'t, S> for () {
    fn next_annotation_grapheme(&mut self, _char_pos: usize) -> Option<(Cow<'t, str>, S)> {
        None
    }

    fn set_pos(&mut self, _char_pos: usize) {}
}

#[derive(Debug, Clone, Copy)]
pub struct CursorConfig {
    pub soft_wrap: bool,
    pub tab_width: u16,
    pub max_wrap: u16,
    pub max_indent_retain: u16,
    pub wrap_indent: u16,
    pub viewport_width: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct LineBreak {
    /// wether this linebreak corresponds to a softwrap
    pub is_softwrap: bool,
    /// Amount of additional indentation to insert before this line
    pub indent: u16,
}

enum WordBoundary {
    /// Any line break
    LineBreak,
    /// a breaking space (' ' or \t)
    Space,
}

#[derive(Debug, Clone)]
pub struct Word {
    pub visual_width: usize,
    pub doc_char_width: usize,
    pub terminating_linebreak: Option<LineBreak>,
    /// The graphemes in this words
    graphemes: Option<usize>,
}

impl Word {
    pub fn consume_graphemes<'d, 't, S: Default, A: AnnotationSource<'t, S>>(
        &mut self,
        cursor: &'d mut DocumentCursor<'t, S, A>,
    ) -> vec::Drain<'d, StyledGraphemes<'t, S>> {
        let num_graphemes = self
            .graphemes
            .take()
            .expect("finish_word can only be called once for a word");
        cursor.word_buf.drain(..num_graphemes)
    }
}

impl Drop for Word {
    fn drop(&mut self) {
        if self.graphemes.is_some() {
            unreachable!("A words graphemes must be consumed with `Word::consume_graphemes`")
        }
    }
}

#[derive(Debug)]
pub struct DocumentCursor<'t, S: Default, A: AnnotationSource<'t, S>> {
    pub config: CursorConfig,

    indent_level: Option<usize>,
    /// The char index of the last yielded word boundary
    doc_char_idx: usize,
    /// The current line index inside the documnet
    doc_line_idx: usize,
    /// The visual position at the end of the last yielded word boundary
    visual_pos: Position,

    graphemes: RopeGraphemes<'t>,
    annotation_source: A,

    /// The visual width of the word
    word_width: usize,
    /// The number of codepoints (chars) in the current word
    word_doc_chars: usize,
    word_buf: Vec<StyledGraphemes<'t, S>>,
}

impl<'t, S: Default + Copy, A: AnnotationSource<'t, S>> DocumentCursor<'t, S, A> {
    /// Create a new `DocumentCursor` that transveres `text`.
    /// The char/document idx is offset by `doc_char_offset`
    /// and `doc_line_offset`.
    /// This has no effect on the cursor itself and only affects the char
    /// indecies passed to the `annotation_source` and used to determine when the end of the `highlight_scope`
    /// is reached
    pub fn new(
        text: RopeSlice<'t>,
        config: CursorConfig,
        doc_char_offset: usize,
        doc_line_offset: usize,
        annotation_source: A,
    ) -> Self {
        // // TODO implement blocks that force hardwraps at specific positions to avoid backtracing huge distances for large files here
        // let doc_line = text.char_to_line(char_pos);
        // let doc_line_start = text.line_to_char(doc_line);
        DocumentCursor {
            config,
            indent_level: None,
            doc_char_idx: doc_char_offset,
            doc_line_idx: doc_line_offset,
            visual_pos: Position { row: 0, col: 0 },
            graphemes: RopeGraphemes::new(text),
            annotation_source,
            word_width: 0,
            word_doc_chars: 0,
            word_buf: Vec::with_capacity(64),
        }
    }

    /// Returns the last checkpoint as (char_idx, line_idx) from which the `DocumentCursor` must be started
    /// to find the first visual line.
    ///
    /// Right now only document lines are used as checkpoints
    /// which leads to inefficent rendering for extremly large wrapped lines
    /// In the future we want to mimic led and implement blocks to chunk extermly long lines
    fn prev_checkpoint(text: RopeSlice, doc_char_idx: usize) -> (usize, usize) {
        let line = text.char_to_line(doc_char_idx);
        let line_start = text.line_to_char(line);
        (line_start, line)
    }

    /// Creates a new cursor at the visual line start that is closest to
    pub fn new_at_prev_line(
        text: RopeSlice<'t>,
        config: CursorConfig,
        char_idx: usize,
        mut annotation_source: A,
    ) -> Self {
        let (mut checkpoint_char_idx, checkpoint_line_idx) = Self::prev_checkpoint(text, char_idx);
        let mut line_off = 0;
        let mut indent_level = None;

        if config.soft_wrap {
            annotation_source.set_pos(checkpoint_char_idx);
            let mut cursor = DocumentCursor::new(
                text.slice(checkpoint_char_idx..),
                config,
                checkpoint_char_idx,
                checkpoint_line_idx,
                &mut annotation_source,
            );

            while let Some(mut word) = cursor.advance() {
                word.consume_graphemes(&mut cursor);
                if cursor.doc_char_idx > char_idx {
                    break;
                }
                if let Some(line_break) = word.terminating_linebreak {
                    line_off = line_break.indent;
                    checkpoint_char_idx = cursor.doc_char_idx;
                    indent_level = Some((line_off - config.wrap_indent) as usize);
                }
            }
        }

        annotation_source.set_pos(checkpoint_char_idx);
        let mut cursor = DocumentCursor::new(
            text.slice(checkpoint_char_idx..),
            config,
            checkpoint_char_idx,
            checkpoint_line_idx,
            annotation_source,
        );

        cursor.indent_level = indent_level;
        cursor.visual_pos.col = line_off as usize;
        cursor
    }

    pub fn doc_line_idx(&self) -> usize {
        self.doc_line_idx
    }

    pub fn doc_char_idx(&self) -> usize {
        self.doc_char_idx
    }

    pub fn visual_pos(&self) -> Position {
        self.visual_pos
    }

    pub fn advance(&mut self) -> Option<Word> {
        self.advance_with_highlight((usize::MAX, S::default()))
    }

    pub fn advance_with_highlight(&mut self, highlight_scope: (usize, S)) -> Option<Word> {
        loop {
            if self.doc_char_idx + self.word_doc_chars >= highlight_scope.0 {
                debug_assert_eq!(
                    self.doc_char_idx + self.word_doc_chars,
                    highlight_scope.0,
                    "Highlight scope must be aligned to grapheme boundary"
                );
                return None;
            }

            if self.word_width + self.visual_pos.col >= self.config.viewport_width as usize {
                break;
            }

            let (grapheme, style, doc_chars) = if let Some(annotation) = self
                .annotation_source
                .next_annotation_grapheme(self.doc_char_idx + self.word_doc_chars)
            {
                (annotation.0, annotation.1, 0)
            } else if let Some(grapheme) = self.graphemes.next() {
                let codepoints = grapheme.len_chars();
                self.word_doc_chars += codepoints;
                (Cow::from(grapheme), highlight_scope.1, codepoints as u16)
            } else {
                return None;
            };

            match self.push_grapheme(grapheme, style, doc_chars) {
                Some(WordBoundary::LineBreak) => {
                    self.indent_level = None;
                    let word = self.take_word(Some(LineBreak {
                        is_softwrap: false,
                        indent: 0,
                    }));
                    return Some(word);
                }
                Some(WordBoundary::Space) => {
                    return Some(self.take_word(None));
                }
                _ => (),
            }
        }

        if self.config.soft_wrap {
            let indent_carry_over = if let Some(indent) = self.indent_level {
                if indent as u16 <= self.config.max_indent_retain {
                    indent as u16
                } else {
                    0
                }
            } else {
                0
            };
            let line_indent = indent_carry_over + self.config.wrap_indent;

            let mut num_graphemes = 0;
            let mut visual_width = 0;
            let mut doc_chars = 0;
            if self.word_width > self.config.max_wrap as usize {
                num_graphemes = self.word_buf.len();
                visual_width = take(&mut self.word_width);
                doc_chars = take(&mut self.word_doc_chars);

                // Usually we stop accomulating graphemes as soon as softwrapping becomes necessary.
                // However if the last grapheme is multiple columns wide it might extend beyond the EOL.
                // The condition below ensures that this grapheme is not yielded yet and instead wrapped to the next line
                if self.word_buf.last().map_or(false, |last| last.width() != 1) {
                    num_graphemes -= 1;
                    let wrapped_grapheme = self.word_buf.last_mut().unwrap();

                    wrapped_grapheme
                        .grapheme
                        .change_position(line_indent as usize, self.config.tab_width);
                    let wrapped_grapheme_width = wrapped_grapheme.width() as usize;
                    visual_width -= wrapped_grapheme_width;
                    self.word_width = wrapped_grapheme_width as usize;
                    let wrapped_grapheme_chars = wrapped_grapheme.doc_chars as usize;
                    self.word_doc_chars = wrapped_grapheme_chars;
                    doc_chars -= wrapped_grapheme_chars;
                }
            }

            let word = Word {
                visual_width,
                graphemes: Some(num_graphemes),
                terminating_linebreak: Some(LineBreak {
                    is_softwrap: true,
                    indent: line_indent,
                }),
                doc_char_width: doc_chars,
            };
            self.visual_pos.row += 1;
            self.visual_pos.col = line_indent as usize;
            self.doc_char_idx += doc_chars;

            Some(word)
        } else {
            Some(self.take_word(None))
        }
    }

    pub fn finish(&mut self) -> Word {
        self.take_word(None)
    }

    fn push_grapheme(
        &mut self,
        grapheme: Cow<'t, str>,
        style: S,
        doc_chars: u16,
    ) -> Option<WordBoundary> {
        if LineEnding::from_str(&grapheme).is_some() {
            // we reached EOL reset column and advance the row
            // do not push a grapheme for the line end, instead let the caller handle decide that
            self.word_buf.push(StyledGraphemes {
                grapheme: Grapheme::Newline,
                style,
                doc_chars,
            });
            self.word_width += 1;
            return Some(WordBoundary::LineBreak);
        }

        let grapheme = StyledGraphemes::new(
            grapheme,
            style,
            self.visual_pos.col + self.word_width,
            self.config.tab_width,
            doc_chars,
        );

        if self.indent_level.is_none() && !grapheme.is_whitespace() {
            self.indent_level = Some(self.visual_pos.col);
        }

        self.word_width += grapheme.width() as usize;
        let word_end = if grapheme.is_breaking_space() {
            Some(WordBoundary::Space)
        } else {
            None
        };

        self.word_buf.push(grapheme);
        word_end
    }

    fn take_word(&mut self, terminating_linebreak: Option<LineBreak>) -> Word {
        if let Some(line_break) = terminating_linebreak {
            debug_assert!(
                !line_break.is_softwrap,
                "Softwrapped words are handeled seperatly"
            );
            self.doc_line_idx += 1;
            self.visual_pos.row += 1;
            self.visual_pos.col = 0;
        } else {
            self.visual_pos.col += self.word_width;
        }
        let doc_char_width = take(&mut self.word_doc_chars);
        self.doc_char_idx += doc_char_width;
        Word {
            visual_width: take(&mut self.word_width),
            graphemes: Some(self.word_buf.len()),
            terminating_linebreak,
            doc_char_width,
        }
    }
}
