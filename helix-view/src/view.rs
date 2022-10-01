use crate::{
    graphics::Rect,
    gutter::{self, Gutter},
    Document, DocumentId, ViewId,
};
use helix_core::{pos_at_visual_coords, visual_coords_at_pos, Position, RopeSlice, Selection};

use std::fmt;

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

    pub fn get(&self) -> &[Jump] {
        &self.jumps
    }
}

#[derive(Clone)]
pub struct View {
    pub id: ViewId,
    pub offset: Position,
    pub area: Rect,
    pub doc: DocumentId,
    pub jumps: JumpList,
    // documents accessed from this view from the oldest one to last viewed one
    pub docs_access_history: Vec<DocumentId>,
    /// the last modified files before the current one
    /// ordered from most frequent to least frequent
    // uses two docs because we want to be able to swap between the
    // two last modified docs which we need to manually keep track of
    pub last_modified_docs: [Option<DocumentId>; 2],
    /// used to store previous selections of tree-sitter objects
    pub object_selections: Vec<Selection>,
    /// Gutter (constructor) and width of gutter, used to calculate
    /// `gutter_offset`
    gutters: Vec<(Gutter, usize)>,
    /// cached total width of gutter
    gutter_offset: u16,
}

impl fmt::Debug for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("View")
            .field("id", &self.id)
            .field("area", &self.area)
            .field("doc", &self.doc)
            .finish()
    }
}

impl View {
    pub fn new(doc: DocumentId, gutter_types: Vec<crate::editor::GutterType>) -> Self {
        let mut gutters: Vec<(Gutter, usize)> = vec![];
        let mut gutter_offset = 0;
        use crate::editor::GutterType;
        for gutter_type in &gutter_types {
            let width = match gutter_type {
                GutterType::Diagnostics => 1,
                GutterType::LineNumbers => 5,
                GutterType::Spacer => 1,
            };
            gutter_offset += width;
            gutters.push((
                match gutter_type {
                    GutterType::Diagnostics => gutter::diagnostics_or_breakpoints,
                    GutterType::LineNumbers => gutter::line_numbers,
                    GutterType::Spacer => gutter::padding,
                },
                width as usize,
            ));
        }
        if !gutter_types.is_empty() {
            gutter_offset += 1;
        }
        Self {
            id: ViewId::default(),
            doc,
            offset: Position::new(0, 0),
            area: Rect::default(), // will get calculated upon inserting into tree
            jumps: JumpList::new((doc, Selection::point(0))), // TODO: use actual sel
            docs_access_history: Vec::new(),
            last_modified_docs: [None, None],
            object_selections: Vec::new(),
            gutters,
            gutter_offset,
        }
    }

    pub fn add_to_history(&mut self, id: DocumentId) {
        if let Some(pos) = self.docs_access_history.iter().position(|&doc| doc == id) {
            self.docs_access_history.remove(pos);
        }
        self.docs_access_history.push(id);
    }

    pub fn inner_area(&self) -> Rect {
        // TODO add abilty to not use cached offset for runtime configurable gutter
        self.area.clip_left(self.gutter_offset).clip_bottom(1) // -1 for statusline
    }

    pub fn gutters(&self) -> &[(Gutter, usize)] {
        &self.gutters
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

        let tab_width = doc.tab_width();
        // TODO: visual_coords_at_pos also does char_to_line which we ignore, can we reuse the call?
        let Position { col, .. } = visual_coords_at_pos(text, pos, tab_width);

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

        let text_row = (row - inner.y) as usize + self.offset.row;
        if text_row > text.len_lines() - 1 {
            return Some(text.len_chars());
        }

        let text_col = (column - inner.x) as usize + self.offset.col;

        Some(pos_at_visual_coords(
            *text,
            Position {
                row: text_row,
                col: text_col,
            },
            tab_width,
        ))
    }

    /// Translates a screen position to position in the text document.
    /// Returns a usize typed position in bounds of the text if found in this view, None if out of view.
    pub fn pos_at_screen_coords(&self, doc: &Document, row: u16, column: u16) -> Option<usize> {
        self.text_pos_at_screen_coords(&doc.text().slice(..), row, column, doc.tab_width())
    }

    /// Translates screen coordinates into coordinates on the gutter of the view.
    /// Returns a tuple of usize typed line and column numbers starting with 0.
    /// Returns None if coordinates are not on the gutter.
    pub fn gutter_coords_at_screen_coords(&self, row: u16, column: u16) -> Option<Position> {
        // 1 for status
        if row < self.area.top() || row >= self.area.bottom() {
            return None;
        }

        if column < self.area.left() || column > self.area.right() {
            return None;
        }

        Some(Position::new(
            (row - self.area.top()) as usize,
            (column - self.area.left()) as usize,
        ))
    }

    pub fn remove_document(&mut self, doc_id: &DocumentId) {
        self.jumps.remove(doc_id);
        self.docs_access_history.retain(|doc| doc != doc_id);
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
    const OFFSET_WITHOUT_LINE_NUMBERS: u16 = 2; // 1 diagnostic + 1 gutter
                                                // const OFFSET: u16 = GUTTERS.iter().map(|(_, width)| *width as u16).sum();
    use crate::editor::GutterType;

    #[test]
    fn test_text_pos_at_screen_coords() {
        let mut view = View::new(
            DocumentId::default(),
            vec![GutterType::Diagnostics, GutterType::LineNumbers],
        );
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
            Some(4)
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
    fn test_text_pos_at_screen_coords_without_line_numbers_gutter() {
        let mut view = View::new(DocumentId::default(), vec![GutterType::Diagnostics]);
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let text = rope.slice(..);
        assert_eq!(
            view.text_pos_at_screen_coords(&text, 41, 40 + OFFSET_WITHOUT_LINE_NUMBERS + 1, 4),
            Some(4)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_without_any_gutters() {
        let mut view = View::new(DocumentId::default(), vec![]);
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let text = rope.slice(..);
        assert_eq!(
            view.text_pos_at_screen_coords(&text, 41, 40 + 1, 4),
            Some(4)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_cjk() {
        let mut view = View::new(
            DocumentId::default(),
            vec![GutterType::Diagnostics, GutterType::LineNumbers],
        );
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("Hi! こんにちは皆さん");
        let text = rope.slice(..);

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET, 4),
            Some(0)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 4, 4),
            Some(4)
        );
        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 5, 4),
            Some(4)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 6, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 7, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET + 8, 4),
            Some(6)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_graphemes() {
        let mut view = View::new(
            DocumentId::default(),
            vec![GutterType::Diagnostics, GutterType::LineNumbers],
        );
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("Hèl̀l̀ò world!");
        let text = rope.slice(..);

        assert_eq!(
            view.text_pos_at_screen_coords(&text, 40, 40 + OFFSET, 4),
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
