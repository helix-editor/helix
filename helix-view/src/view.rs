use std::borrow::Cow;

use crate::{graphics::Rect, Document, DocumentId, ViewId};
use helix_core::{
    graphemes::{grapheme_width, RopeGraphemes},
    line_ending::line_end_char_index,
    visual_coords_at_pos, Position, RopeSlice, Selection,
};

type Jump = (DocumentId, Selection);

#[derive(Debug, Clone)]
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

    pub fn remove(&mut self, doc_id: &DocumentId) {
        self.jumps.retain(|(other_id, _)| other_id != doc_id);
    }
}

#[derive(Debug)]
pub struct View {
    pub id: ViewId,
    pub doc: DocumentId,
    pub offset: Position,
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
            offset: Position::new(0, 0),
            area: Rect::default(), // will get calculated upon inserting into tree
            jumps: JumpList::new((doc, Selection::point(0))), // TODO: use actual sel
            last_accessed_doc: None,
        }
    }

    pub fn inner_area(&self) -> Rect {
        // TODO: not ideal
        const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter
        self.area.clip_left(OFFSET).clip_bottom(1) // -1 for statusline
    }

    //
    pub fn offset_coords_to_in_view(
        &self,
        doc: &Document,
        scrolloff: usize,
    ) -> Option<(usize, usize)> {
        let cursor = doc
            .selection(self.id)
            .primary()
            .cursor(doc.text().slice(..));

        let Position { col, row: line } =
            visual_coords_at_pos(doc.text().slice(..), cursor, doc.tab_width());

        let inner_area = self.inner_area();
        let last_line = (self.offset.row + inner_area.height as usize).saturating_sub(1);

        // - 1 so we have at least one gap in the middle.
        // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
        // as we type
        let scrolloff = scrolloff.min(inner_area.height.saturating_sub(1) as usize / 2);

        let last_col = self.offset.col + inner_area.width.saturating_sub(1) as usize;

        let row = if line > last_line.saturating_sub(scrolloff) {
            // scroll down
            self.offset.row + line - (last_line.saturating_sub(scrolloff))
        } else if line < self.offset.row + scrolloff {
            // scroll up
            line.saturating_sub(scrolloff)
        } else {
            self.offset.row
        };

        let col = if col > last_col.saturating_sub(scrolloff) {
            // scroll right
            self.offset.col + col - (last_col.saturating_sub(scrolloff))
        } else if col < self.offset.col + scrolloff {
            // scroll left
            col.saturating_sub(scrolloff)
        } else {
            self.offset.col
        };
        if row == self.offset.row && col == self.offset.col {
            None
        } else {
            Some((row, col))
        }
    }

    pub fn ensure_cursor_in_view(&mut self, doc: &Document, scrolloff: usize) {
        if let Some((row, col)) = self.offset_coords_to_in_view(doc, scrolloff) {
            self.offset.row = row;
            self.offset.col = col;
        }
    }

    pub fn is_cursor_in_view(&mut self, doc: &Document, scrolloff: usize) -> bool {
        self.offset_coords_to_in_view(doc, scrolloff).is_none()
    }

    /// Calculates the last visible line on screen
    #[inline]
    pub fn last_line(&self, doc: &Document) -> usize {
        let height = self.inner_area().height;
        std::cmp::min(
            // Saturating subs to make it inclusive zero indexing.
            (self.offset.row + height as usize).saturating_sub(1),
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

        if line < self.offset.row || line > self.last_line(doc) {
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
        let row = line.saturating_sub(self.offset.row);
        let col = col.saturating_sub(self.offset.col);

        Some(Position::new(row, col))
    }

    pub fn text_pos_at_screen_coords(
        &self,
        text: &RopeSlice,
        row: u16,
        column: u16,
        tab_width: usize,
    ) -> Option<usize> {
        let inner = self.inner_area();
        // 1 for status
        if row < inner.top() || row >= inner.bottom() {
            return None;
        }

        if column < inner.left() || column > inner.right() {
            return None;
        }

        let line_number = (row - inner.y) as usize + self.offset.row;

        if line_number > text.len_lines() - 1 {
            return Some(text.len_chars());
        }

        let mut pos = text.line_to_char(line_number);

        let current_line = text.line(line_number);

        let target = (column - inner.x) as usize + self.offset.col;
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
    const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter

    #[test]
    fn test_text_pos_at_screen_coords() {
        let mut view = View::new(DocumentId::default());
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let text = rope.slice(..);

        assert_eq!(view.text_pos_at_screen_coords(&text, 40, 2, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&text, 40, 41, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&text, 0, 2, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&text, 0, 49, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&text, 0, 41, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&text, 40, 81, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&text, 78, 41, 4), None);

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 3, 4),
            Some(3)
        );

        assert_eq!(view.text_pos_at_screen_coords(&text, 40, 80, 4), Some(3));

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 41, 40 + OFFSET + 1, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 41, 40 + OFFSET + 4, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 41, 40 + OFFSET + 7, 4),
            Some(8)
        );

        assert_eq!(view.text_pos_at_screen_coords(&text, 41, 80, 4), Some(8));
    }

    #[test]
    fn test_text_pos_at_screen_coords_cjk() {
        let mut view = View::new(DocumentId::default());
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("Hi! こんにちは皆さん");
        let text = rope.slice(..);

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 0, 4),
            Some(0)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 5, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 6, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 7, 4),
            Some(6)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 8, 4),
            Some(6)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_graphemes() {
        let mut view = View::new(DocumentId::default());
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("Hèl̀l̀ò world!");
        let text = rope.slice(..);

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 0, 4),
            Some(0)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 1, 4),
            Some(1)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 2, 4),
            Some(3)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 3, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 4, 4),
            Some(7)
        );
    }
}
