use anyhow::Error;

use std::borrow::Cow;

use crate::Document;
use helix_core::{
    graphemes::{grapheme_width, RopeGraphemes},
    indent::TAB_WIDTH,
    Position, RopeSlice,
};
use tui::layout::Rect;

pub const PADDING: usize = 5;

// TODO: view should be View { doc: Document(state, history,..) }
// since we can have multiple views into the same file
pub struct View {
    pub doc: Document,
    pub first_line: usize,
    pub area: Rect,
}
// TODO: popups should be a thing on the view with a rect + text

impl View {
    pub fn new(doc: Document) -> Result<Self, Error> {
        let view = Self {
            doc,
            first_line: 0,
            area: Rect::default(), // will get calculated upon inserting into tree
        };

        Ok(view)
    }

    pub fn ensure_cursor_in_view(&mut self) {
        let cursor = self.doc.state.selection().cursor();
        let line = self.doc.text().char_to_line(cursor);
        let document_end = self.first_line + (self.area.height as usize).saturating_sub(2);

        // TODO: side scroll

        if line > document_end.saturating_sub(PADDING) {
            // scroll down
            self.first_line += line - (document_end.saturating_sub(PADDING));
        } else if line < self.first_line + PADDING {
            // scroll up
            self.first_line = line.saturating_sub(PADDING);
        }
    }

    /// Calculates the last visible line on screen
    #[inline]
    pub fn last_line(&self) -> usize {
        let viewport = Rect::new(6, 0, self.area.width, self.area.height - 1); // - 1 for statusline
        std::cmp::min(
            self.first_line + (viewport.height as usize),
            self.doc.text().len_lines() - 1,
        )
    }

    /// Translates a document position to an absolute position in the terminal.
    /// Returns a (line, col) position if the position is visible on screen.
    // TODO: Could return width as well for the character width at cursor.
    pub fn screen_coords_at_pos(&self, text: &RopeSlice, pos: usize) -> Option<Position> {
        let line = text.char_to_line(pos);

        if line < self.first_line as usize || line > self.last_line() {
            // Line is not visible on screen
            return None;
        }

        let line_start = text.line_to_char(line);
        let line_slice = text.slice(line_start..pos);
        let mut col = 0;

        for grapheme in RopeGraphemes::new(&line_slice) {
            if grapheme == "\t" {
                col += TAB_WIDTH;
            } else {
                let grapheme = Cow::from(grapheme);
                col += grapheme_width(&grapheme);
            }
        }

        let row = line - self.first_line as usize;

        Some(Position::new(row, col))
    }

    // pub fn traverse<F>(&self, text: &RopeSlice, start: usize, end: usize, fun: F)
    // where
    //     F: Fn(usize, usize),
    // {
    //     let start = self.screen_coords_at_pos(text, start);
    //     let end = self.screen_coords_at_pos(text, end);

    //     match (start, end) {
    //         // fully on screen
    //         (Some(start), Some(end)) => {
    //             // we want to calculate ends of lines for each char..
    //         }
    //         // from start to end of screen
    //         (Some(start), None) => {}
    //         // from start of screen to end
    //         (None, Some(end)) => {}
    //         // not on screen
    //         (None, None) => return,
    //     }
    // }
}
