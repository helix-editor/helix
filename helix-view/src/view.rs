use std::borrow::Cow;

use crate::{graphics::Rect, Document, DocumentId, ViewId};
use helix_core::{
    coords_at_pos,
    graphemes::{grapheme_width, RopeGraphemes},
    line_ending::line_end_char_index,
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
        let cursor = doc
            .selection(self.id)
            .primary()
            .cursor(doc.text().slice(..));
        let pos = coords_at_pos(doc.text().slice(..), cursor);
        let line = pos.row;
        let col = pos.col;
        let height = self.area.height.saturating_sub(1); // - 1 for statusline
        let last_line = (self.first_line + height as usize).saturating_sub(1);

        let scrolloff = PADDING.min(self.area.height as usize / 2); // TODO: user pref

        // TODO: not ideal
        const OFFSET: usize = 7; // 1 diagnostic + 5 linenr + 1 gutter
        let last_col = (self.first_col + self.area.width as usize).saturating_sub(OFFSET + 1);

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

    pub fn text_pos_at_screen_coords(
        &self,
        text: &RopeSlice,
        row: u16,
        column: u16,
        tab_width: usize,
    ) -> Option<usize> {
        // TODO: not ideal
        const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter

        // 2 for status
        if row < self.area.top() || row > self.area.bottom().saturating_sub(2) {
            return None;
        }

        if column < self.area.left() + OFFSET || column > self.area.right() {
            return None;
        }

        let line_number = (row - self.area.y) as usize + self.first_line;

        if line_number > text.len_lines() - 1 {
            return Some(text.len_chars());
        }

        let mut pos = text.line_to_char(line_number);

        let current_line = text.line(line_number);

        let target = (column - OFFSET - self.area.x) as usize + self.first_col;
        let mut selected = 0;

        for grapheme in RopeGraphemes::new(current_line) {
            if selected >= target {
                break;
            }
            if grapheme == "\t" {
                selected += tab_width;
            } else {
                let width = grapheme_width(&Cow::from(grapheme));
                selected += width;
            }
            pos += grapheme.chars().count();
        }

        Some(pos.min(line_end_char_index(&text.slice(..), line_number)))
    }

    /// Translates a screen position to position in the text document.
    /// Returns a usize typed position in bounds of the text if found in this view, None if out of view.
    pub fn pos_at_screen_coords(&self, doc: &Document, row: u16, column: u16) -> Option<usize> {
        self.text_pos_at_screen_coords(&doc.text().slice(..), row, column, doc.tab_width())
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

#[cfg(test)]
mod tests {
    use super::*;
    use helix_core::Rope;

    #[test]
    fn test_text_pos_at_screen_coords() {
        let mut view = View::new(DocumentId::default());
        view.area = Rect::new(40, 40, 40, 40);
        let text = Rope::from_str("abc\n\tdef");

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 40, 2, 4),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 40, 41, 4),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 0, 2, 4),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 0, 49, 4),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 0, 41, 4),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 40, 81, 4),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 78, 41, 4),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 40, 40 + 7 + 3, 4),
            Some(3)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 40, 80, 4),
            Some(3)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 41, 40 + 7 + 1, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 41, 40 + 7 + 4, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 41, 40 + 7 + 7, 4),
            Some(8)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text.slice(..), 41, 80, 4),
            Some(8)
        );
    }
}
