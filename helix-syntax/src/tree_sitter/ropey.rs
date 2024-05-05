use regex_cursor::{Cursor, RopeyCursor};
use ropey::RopeSlice;

use crate::tree_sitter::parser::{IntoParserInput, ParserInput};

pub struct RopeParserInput<'a> {
    src: RopeSlice<'a>,
    cursor: regex_cursor::RopeyCursor<'a>,
}

impl<'a> IntoParserInput for RopeSlice<'a> {
    type ParserInput = RopeParserInput<'a>;

    fn into_parser_input(self) -> Self::ParserInput {
        RopeParserInput {
            src: self,
            cursor: RopeyCursor::new(self),
        }
    }
}

impl ParserInput for RopeParserInput<'_> {
    fn read(&mut self, offset: usize) -> &[u8] {
        // this cursor is optimized for contigous reads which are by far the most common during parsing
        // very far jumps (like injections at the other end of the document) are handelde
        // by restarting a new cursor (new chunks iterator)
        if offset < self.cursor.offset() && self.cursor.offset() - offset > 4906 {
            self.cursor = regex_cursor::RopeyCursor::at(self.src, offset);
        } else {
            while self.cursor.offset() + self.cursor.chunk().len() >= offset {
                if !self.cursor.advance() {
                    return &[];
                }
            }
        }
        self.cursor.chunk()
    }
}
