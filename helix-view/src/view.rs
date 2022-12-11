use crate::{align_view, editor::GutterType, graphics::Rect, Align, Document, DocumentId, ViewId};
use helix_core::{
    pos_at_visual_coords, visual_coords_at_pos, Position, RopeSlice, Selection, Transaction,
};

use std::{
    collections::{HashMap, VecDeque},
    fmt,
};

const JUMP_LIST_CAPACITY: usize = 30;

type Jump = (DocumentId, Selection);

#[derive(Debug, Clone)]
pub struct JumpList {
    jumps: VecDeque<Jump>,
    current: usize,
}

impl JumpList {
    pub fn new(initial: Jump) -> Self {
        let mut jumps = VecDeque::with_capacity(JUMP_LIST_CAPACITY);
        jumps.push_back(initial);
        Self { jumps, current: 0 }
    }

    pub fn push(&mut self, jump: Jump) {
        self.jumps.truncate(self.current);
        // don't push duplicates
        if self.jumps.back() != Some(&jump) {
            // If the jumplist is full, drop the oldest item.
            while self.jumps.len() >= JUMP_LIST_CAPACITY {
                self.jumps.pop_front();
            }

            self.jumps.push_back(jump);
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

    pub fn iter(&self) -> impl Iterator<Item = &Jump> {
        self.jumps.iter()
    }

    /// Applies a [`Transaction`] of changes to the jumplist.
    /// This is necessary to ensure that changes to documents do not leave jump-list
    /// selections pointing to parts of the text which no longer exist.
    fn apply(&mut self, transaction: &Transaction, doc: &Document) {
        let text = doc.text().slice(..);

        for (doc_id, selection) in &mut self.jumps {
            if doc.id() == *doc_id {
                *selection = selection
                    .clone()
                    .map(transaction.changes())
                    .ensure_invariants(text);
            }
        }
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
    /// GutterTypes used to fetch Gutter (constructor) and width for rendering
    gutters: Vec<GutterType>,
    /// A mapping between documents and the last history revision the view was updated at.
    /// Changes between documents and views are synced lazily when switching windows. This
    /// mapping keeps track of the last applied history revision so that only new changes
    /// are applied.
    doc_revisions: HashMap<DocumentId, usize>,
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
        Self {
            id: ViewId::default(),
            doc,
            offset: Position::new(0, 0),
            area: Rect::default(), // will get calculated upon inserting into tree
            jumps: JumpList::new((doc, Selection::point(0))), // TODO: use actual sel
            docs_access_history: Vec::new(),
            last_modified_docs: [None, None],
            object_selections: Vec::new(),
            gutters: gutter_types,
            doc_revisions: HashMap::new(),
        }
    }

    pub fn add_to_history(&mut self, id: DocumentId) {
        if let Some(pos) = self.docs_access_history.iter().position(|&doc| doc == id) {
            self.docs_access_history.remove(pos);
        }
        self.docs_access_history.push(id);
    }

    pub fn inner_area(&self, doc: &Document) -> Rect {
        self.area.clip_left(self.gutter_offset(doc)).clip_bottom(1) // -1 for statusline
    }

    pub fn inner_height(&self) -> usize {
        self.area.clip_bottom(1).height.into() // -1 for statusline
    }

    pub fn gutters(&self) -> &[GutterType] {
        &self.gutters
    }

    pub fn gutter_offset(&self, doc: &Document) -> u16 {
        self.gutters
            .iter()
            .map(|gutter| gutter.width(self, doc) as u16)
            .sum()
    }

    //
    pub fn offset_coords_to_in_view(
        &self,
        doc: &Document,
        scrolloff: usize,
    ) -> Option<(usize, usize)> {
        self.offset_coords_to_in_view_center(doc, scrolloff, false)
    }

    pub fn offset_coords_to_in_view_center(
        &self,
        doc: &Document,
        scrolloff: usize,
        centering: bool,
    ) -> Option<(usize, usize)> {
        let cursor = doc
            .selection(self.id)
            .primary()
            .cursor(doc.text().slice(..));

        let Position { col, row: line } =
            visual_coords_at_pos(doc.text().slice(..), cursor, doc.tab_width());

        let inner_area = self.inner_area(doc);
        let last_line = (self.offset.row + inner_area.height as usize).saturating_sub(1);
        let last_col = self.offset.col + inner_area.width.saturating_sub(1) as usize;

        let new_offset = |scrolloff: usize| {
            // - 1 so we have at least one gap in the middle.
            // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
            // as we type
            let scrolloff = scrolloff.min(inner_area.height.saturating_sub(1) as usize / 2);

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
            (row, col)
        };
        let current_offset = (self.offset.row, self.offset.col);
        if centering {
            // return None if cursor is out of view
            let offset = new_offset(0);
            (offset == current_offset).then(|| {
                if scrolloff == 0 {
                    offset
                } else {
                    new_offset(scrolloff)
                }
            })
        } else {
            // return None if cursor is in (view - scrolloff)
            let offset = new_offset(scrolloff);
            (offset != current_offset).then(|| offset) // TODO: use 'then_some' when 1.62 <= MSRV
        }
    }

    pub fn ensure_cursor_in_view(&mut self, doc: &Document, scrolloff: usize) {
        if let Some((row, col)) = self.offset_coords_to_in_view_center(doc, scrolloff, false) {
            self.offset.row = row;
            self.offset.col = col;
        }
    }

    pub fn ensure_cursor_in_view_center(&mut self, doc: &Document, scrolloff: usize) {
        if let Some((row, col)) = self.offset_coords_to_in_view_center(doc, scrolloff, true) {
            self.offset.row = row;
            self.offset.col = col;
        } else {
            align_view(doc, self, Align::Center);
        }
    }

    pub fn is_cursor_in_view(&mut self, doc: &Document, scrolloff: usize) -> bool {
        self.offset_coords_to_in_view(doc, scrolloff).is_none()
    }

    /// Calculates the last visible line on screen
    #[inline]
    pub fn last_line(&self, doc: &Document) -> usize {
        std::cmp::min(
            // Saturating subs to make it inclusive zero indexing.
            (self.offset.row + self.inner_height()).saturating_sub(1),
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
        doc: &Document,
        row: u16,
        column: u16,
        tab_width: usize,
    ) -> Option<usize> {
        let text = doc.text().slice(..);
        let inner = self.inner_area(doc);
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
            text,
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
        self.text_pos_at_screen_coords(doc, row, column, doc.tab_width())
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

    /// Applies a [`Transaction`] to the view.
    /// Instead of calling this function directly, use [crate::apply_transaction]
    /// which applies a transaction to the [`Document`] and view together.
    pub fn apply(&mut self, transaction: &Transaction, doc: &mut Document) {
        self.jumps.apply(transaction, doc);
        self.doc_revisions
            .insert(doc.id(), doc.get_current_revision());
    }

    pub fn sync_changes(&mut self, doc: &mut Document) {
        let latest_revision = doc.get_current_revision();
        let current_revision = *self
            .doc_revisions
            .entry(doc.id())
            .or_insert(latest_revision);

        if current_revision == latest_revision {
            return;
        }

        log::debug!(
            "Syncing view {:?} between {} and {}",
            self.id,
            current_revision,
            latest_revision
        );

        if let Some(transaction) = doc.history.get_mut().changes_since(current_revision) {
            self.apply(&transaction, doc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_core::Rope;
    const OFFSET: u16 = 3; // 1 diagnostic + 2 linenr (< 100 lines)
    const OFFSET_WITHOUT_LINE_NUMBERS: u16 = 1; // 1 diagnostic
                                                // const OFFSET: u16 = GUTTERS.iter().map(|(_, width)| *width as u16).sum();
    use crate::document::Document;
    use crate::editor::GutterType;

    #[test]
    fn test_text_pos_at_screen_coords() {
        let mut view = View::new(
            DocumentId::default(),
            vec![GutterType::Diagnostics, GutterType::LineNumbers],
        );
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let doc = Document::from(rope, None);

        assert_eq!(view.text_pos_at_screen_coords(&doc, 40, 2, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&doc, 40, 41, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&doc, 0, 2, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&doc, 0, 49, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&doc, 0, 41, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&doc, 40, 81, 4), None);

        assert_eq!(view.text_pos_at_screen_coords(&doc, 78, 41, 4), None);

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 3, 4),
            Some(3)
        );

        assert_eq!(view.text_pos_at_screen_coords(&doc, 40, 80, 4), Some(3));

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 41, 40 + OFFSET + 1, 4),
            Some(4)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 41, 40 + OFFSET + 4, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 41, 40 + OFFSET + 7, 4),
            Some(8)
        );

        assert_eq!(view.text_pos_at_screen_coords(&doc, 41, 80, 4), Some(8));
    }

    #[test]
    fn test_text_pos_at_screen_coords_without_line_numbers_gutter() {
        let mut view = View::new(DocumentId::default(), vec![GutterType::Diagnostics]);
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let doc = Document::from(rope, None);
        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 41, 40 + OFFSET_WITHOUT_LINE_NUMBERS + 1, 4),
            Some(4)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_without_any_gutters() {
        let mut view = View::new(DocumentId::default(), vec![]);
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let doc = Document::from(rope, None);
        assert_eq!(view.text_pos_at_screen_coords(&doc, 41, 40 + 1, 4), Some(4));
    }

    #[test]
    fn test_text_pos_at_screen_coords_cjk() {
        let mut view = View::new(
            DocumentId::default(),
            vec![GutterType::Diagnostics, GutterType::LineNumbers],
        );
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("Hi! こんにちは皆さん");
        let doc = Document::from(rope, None);

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET, 4),
            Some(0)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 4, 4),
            Some(4)
        );
        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 5, 4),
            Some(4)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 6, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 7, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 8, 4),
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
        let doc = Document::from(rope, None);

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET, 4),
            Some(0)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 1, 4),
            Some(1)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 2, 4),
            Some(3)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 3, 4),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(&doc, 40, 40 + OFFSET + 4, 4),
            Some(7)
        );
    }
}
