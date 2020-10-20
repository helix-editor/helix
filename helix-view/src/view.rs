use anyhow::Error;

use std::borrow::Cow;

use helix_core::{
    graphemes::{grapheme_width, RopeGraphemes},
    indent::TAB_WIDTH,
    History, Position, RopeSlice, State,
};
use tui::layout::Rect;

pub const PADDING: usize = 5;

// TODO: view should be View { doc: Document(state, history,..) }
// since we can have multiple views into the same file
pub struct View {
    pub state: State,
    pub first_line: usize,
    pub size: (u16, u16),

    // TODO: Doc fields
    pub history: History,
}

impl View {
    pub fn new(state: State, size: (u16, u16)) -> Result<Self, Error> {
        let view = Self {
            state,
            first_line: 0,
            size,
            history: History::default(),
        };

        Ok(view)
    }

    pub fn ensure_cursor_in_view(&mut self) {
        let cursor = self.state.selection().cursor();
        let line = self.state.doc().char_to_line(cursor);
        let document_end = self.first_line + (self.size.1 as usize).saturating_sub(2);

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
        let viewport = Rect::new(6, 0, self.size.0, self.size.1 - 2); // - 2 for statusline and prompt
        std::cmp::min(
            self.first_line + (viewport.height as usize),
            self.state.doc().len_lines() - 1,
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
}
