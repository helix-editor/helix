use std::borrow::Cow;

use crate::{graphics::Rect, Document, DocumentId, ViewId};
use helix_core::{
    coords_at_pos,
    graphemes::{grapheme_width, RopeGraphemes},
    Position, RopeSlice, Selection,
};

pub const PADDING: usize = 5;

type Jump = (DocumentId, Selection);

#[derive(Debug)]
pub struct JumpList {
    jumps: Vec<Jump>,
    current: usize,
}

impl JumpList {
    pub fn new(initial: Jump) -> Self {
        Self {
            jumps: vec![initial],
            current: 0,
        }
    }

    pub fn push(&mut self, jump: Jump) {
        self.jumps.truncate(self.current);
        // don't push duplicates
        if self.jumps.last() != Some(&jump) {
            self.jumps.push(jump);
            self.current = self.jumps.len();
        }
    }

    pub fn forward(&mut self, count: usize) -> Option<&Jump> {
        if self.current + count < self.jumps.len() {
            self.current += count;
            self.jumps.get(self.current)
        } else {
            None
        }
    }

    // Taking view and doc to prevent unnecessary cloning when jump is not required.
    pub fn backward(&mut self, view_id: ViewId, doc: &mut Document, count: usize) -> Option<&Jump> {
        if let Some(current) = self.current.checked_sub(count) {
            if self.current == self.jumps.len() {
                let jump = (doc.id(), doc.selection(view_id).clone());
                self.push(jump);
            }
            self.current = current;
            self.jumps.get(self.current)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct View {
    pub id: ViewId,
    pub doc: DocumentId,
    pub first_line: usize,
    pub first_col: usize,
    pub area: Rect,
    pub jumps: JumpList,
    /// the last accessed file before the current one
    pub last_accessed_doc: Option<DocumentId>,
}

impl View {
    pub fn new(doc: DocumentId) -> Self {
        Self {
            id: ViewId::default(),
            doc,
            first_line: 0,
            first_col: 0,
            area: Rect::default(), // will get calculated upon inserting into tree
            jumps: JumpList::new((doc, Selection::point(0))), // TODO: use actual sel
            last_accessed_doc: None,
        }
    }

    pub fn ensure_cursor_in_view(&mut self, doc: &Document) {
        let cursor = doc.selection(self.id).cursor(doc.text().slice(..));
        let pos = coords_at_pos(doc.text().slice(..), cursor);
        let line = pos.row;
        let col = pos.col;
        let height = self.area.height.saturating_sub(1); // - 1 for statusline
        let last_line = self.first_line + height as usize;

        let scrolloff = PADDING.min(self.area.height as usize / 2); // TODO: user pref

        // TODO: not ideal
        const OFFSET: usize = 7; // 1 diagnostic + 5 linenr + 1 gutter
        let last_col = self.first_col + (self.area.width as usize - OFFSET);

        if line > last_line.saturating_sub(scrolloff) {
            // scroll down
            self.first_line += line - (last_line.saturating_sub(scrolloff));
        } else if line < self.first_line + scrolloff {
            // scroll up
            self.first_line = line.saturating_sub(scrolloff);
        }

        if col > last_col.saturating_sub(scrolloff) {
            // scroll right
            self.first_col += col - (last_col.saturating_sub(scrolloff));
        } else if col < self.first_col + scrolloff {
            // scroll left
            self.first_col = col.saturating_sub(scrolloff);
        }
    }

    /// Calculates the last visible line on screen
    #[inline]
    pub fn last_line(&self, doc: &Document) -> usize {
        let height = self.area.height.saturating_sub(1); // - 1 for statusline
        std::cmp::min(
            // Saturating subs to make it inclusive zero indexing.
            (self.first_line + height as usize).saturating_sub(1),
            doc.text().len_lines().saturating_sub(1),
        )
    }

    /// Translates a document position to an absolute position in the terminal.
    /// Returns a (line, col) position if the position is visible on screen.
    // TODO: Could return width as well for the character width at cursor.
    pub fn screen_coords_at_pos(
        &self,
        doc: &Document,
        text: RopeSlice,
        pos: usize,
    ) -> Option<Position> {
        let line = text.char_to_line(pos);

        if line < self.first_line || line > self.last_line(doc) {
            // Line is not visible on screen
            return None;
        }

        let line_start = text.line_to_char(line);
        let line_slice = text.slice(line_start..pos);
        let mut col = 0;
        let tab_width = doc.tab_width();

        for grapheme in RopeGraphemes::new(line_slice) {
            if grapheme == "\t" {
                col += tab_width;
            } else {
                let grapheme = Cow::from(grapheme);
                col += grapheme_width(&grapheme);
            }
        }

        // It is possible for underflow to occur if the buffer length is larger than the terminal width.
        let row = line.saturating_sub(self.first_line);
        let col = col.saturating_sub(self.first_col);

        Some(Position::new(row, col))
    }

    // pub fn traverse<F>(&self, text: RopeSlice, start: usize, end: usize, fun: F)
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
