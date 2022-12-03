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

use crate::graphemes::{Grapheme, StyledGrapheme};
use crate::{LineEnding, Position, RopeGraphemes, RopeSlice};

pub trait AnnotationSource<'a> {
    fn next_annotation_grapheme(&mut self, char_pos: usize) -> Option<Cow<'a, str>>;
}

impl<'a> AnnotationSource<'a> for () {
    fn next_annotation_grapheme(&mut self, _char_pos: usize) -> Option<Cow<'a, str>> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorConfig {
    pub tab_width: u16,
    pub max_wrap: usize,
    pub max_indent_retain: usize,
    pub wrap_indent: usize,
    pub viewport_width: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct LineBreak {
    pub is_softwrap: bool,
}

enum WordBoundary {
    /// Any line break
    LineBreak,
    /// a breaking space (' ' or \t)
    Space,
}

#[derive(Debug)]
pub struct Word<'d, 'a, S> {
    pub visual_position: Position,
    pub visual_width: usize,

    pub terminating_linebreak: Option<LineBreak>,
    pub graphmes: vec::Drain<'d, StyledGrapheme<'a, S>>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndentLevel {
    /// Indentation is disabled for this line because it wrapped for too long
    None,
    /// Indentation level is not yet known for this line because no non-whitespace char has been reached
    /// The previous indentation level is kept so that indentation guides are not interrupted by empty lines
    Unkown,
    /// Identation level is known for this line
    Known(usize),
}

#[derive(Debug)]
pub struct DocumentCursor<'a, S: Default, A: AnnotationSource<'a>> {
    pub config: CursorConfig,

    indent_level: IndentLevel,
    char_pos: usize,
    visual_pos: Position,
    doc_line: usize,

    graphemes: RopeGraphemes<'a>,
    annotation_source: &'a mut A,

    word_width: usize,
    word_buf: Vec<StyledGrapheme<'a, S>>,
}

impl<'a, S: Default + Copy, A: AnnotationSource<'a>> DocumentCursor<'a, S, A> {
    pub fn new(
        text: RopeSlice<'a>,
        config: CursorConfig,
        char_off: usize,
        doc_line_off: usize,
        annotation_source: &'a mut A,
    ) -> Self {
        DocumentCursor {
            config,
            indent_level: IndentLevel::Unkown,
            char_pos: char_off,
            visual_pos: Position { row: 0, col: 0 },
            doc_line: doc_line_off,
            word_width: 0,
            graphemes: RopeGraphemes::new(text),
            word_buf: Vec::with_capacity(64),
            annotation_source,
        }
    }

    /// Byte offset from the start of the document (annotations are not counted)
    pub fn byte_pos(&self) -> usize {
        self.graphemes.byte_pos()
    }

    /// Byte offset from the start of the document (annotations are not counted)
    pub fn char_pos(&self) -> usize {
        self.char_pos
    }

    /// line and (char) column in the document (annotations and softwrap are not counted)
    pub fn visual_pos(&self) -> Position {
        self.visual_pos
    }

    pub fn doc_line(&self) -> usize {
        self.doc_line
    }

    pub fn advance<const SOFTWRAP: bool>(
        &mut self,
        highlight_scope: (usize, S),
    ) -> Option<Word<'_, 'a, S>> {
        loop {
            if self.char_pos >= highlight_scope.0 {
                debug_assert_eq!(
                    self.char_pos, highlight_scope.0,
                    "Highlight scope must be aligned to grapheme boundary"
                );
                return None;
            }

            if self.word_width + self.visual_pos.col >= self.config.viewport_width as usize {
                break;
            }

            let grapheme = if let Some(annotation) = self
                .annotation_source
                .next_annotation_grapheme(self.char_pos)
            {
                annotation
            } else if let Some(grapheme) = self.graphemes.next() {
                let codepoints = grapheme.len_chars();
                self.char_pos += codepoints;
                Cow::from(grapheme)
            } else {
                return None;
            };

            match self.push_grapheme(grapheme, highlight_scope.1) {
                Some(WordBoundary::LineBreak) => {
                    self.indent_level = IndentLevel::Unkown;
                    let word = self.take_word(Some(LineBreak { is_softwrap: false }));
                    return Some(word);
                }
                Some(WordBoundary::Space) => {
                    return Some(self.take_word(None));
                }
                _ => (),
            }
        }

        if SOFTWRAP {
            let indent_carry_over = if let IndentLevel::Known(indent) = self.indent_level {
                if indent <= self.config.max_indent_retain {
                    indent
                } else {
                    self.indent_level = IndentLevel::None;
                    0
                }
            } else {
                0
            };
            let new_visual_col = self.config.wrap_indent + indent_carry_over;

            let mut taken_graphemes = 0;
            let mut visual_width = 0;
            if self.word_width > self.config.max_wrap {
                taken_graphemes = self.word_buf.len();
                visual_width = take(&mut self.word_width);

                // Usually we stop accomulating graphemes as soon as softwrapping becomes necessary.
                // However if the last grapheme is multiple columns wide it might extend beyond the EOL.
                // The condition below ensures that this grapheme is not yielded yet and instead wrapped to the next line
                if self.word_width + self.visual_pos.col != self.config.viewport_width as usize {
                    taken_graphemes -= 1;
                    let wrapped_grapheme = self.word_buf.last_mut().unwrap();

                    wrapped_grapheme
                        .grapheme
                        .change_position(new_visual_col, self.config.tab_width);
                    let wrapped_grapheme_width = wrapped_grapheme.width() as usize;
                    visual_width -= wrapped_grapheme_width;
                    self.word_width = wrapped_grapheme_width as usize;
                }
            }

            let word = Word {
                visual_width,
                graphmes: self.word_buf.drain(..taken_graphemes),
                terminating_linebreak: Some(LineBreak { is_softwrap: true }),
                visual_position: self.visual_pos,
            };
            self.visual_pos.row += 1;
            self.visual_pos.col = new_visual_col;

            Some(word)
        } else {
            Some(self.take_word(None))
        }
    }

    pub fn finish(&mut self) -> impl Iterator<Item = StyledGrapheme<'a, S>> + '_ {
        self.word_buf.drain(..)
    }

    fn push_grapheme(&mut self, grapheme: Cow<'a, str>, style: S) -> Option<WordBoundary> {
        if LineEnding::from_str(&grapheme).is_some() {
            // we reached EOL reset column and advance the row
            // do not push a grapheme for the line end, instead let the caller handle decide that
            self.word_buf.push(StyledGrapheme {
                grapheme: Grapheme::Newline,
                style,
            });
            return Some(WordBoundary::LineBreak);
        }

        let grapheme = StyledGrapheme::new(
            grapheme,
            style,
            self.visual_pos.col + self.word_width,
            self.config.tab_width,
        );

        if self.indent_level == IndentLevel::Unkown && !grapheme.is_whitespace() {
            self.indent_level = IndentLevel::Known(self.visual_pos.col);
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

    fn take_word(&mut self, terminating_linebreak: Option<LineBreak>) -> Word<'_, 'a, S> {
        let visual_position = self.visual_pos;
        if let Some(line_break) = terminating_linebreak {
            debug_assert!(
                !line_break.is_softwrap,
                "Softwrapped words are handeled seperatly"
            );
            self.doc_line += 1;
            self.visual_pos.row += 1;
            self.visual_pos.col = 0;
        } else {
            self.visual_pos.col += self.word_width;
        }
        Word {
            visual_width: take(&mut self.word_width),
            graphmes: self.word_buf.drain(..),
            terminating_linebreak,
            visual_position,
        }
    }
}
