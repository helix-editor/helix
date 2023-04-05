use crate::{
    align_view,
    document::DocumentInlayHints,
    editor::{GutterConfig, GutterType},
    graphics::Rect,
    Align, Document, DocumentId, Theme, ViewId,
};

use helix_core::{
    char_idx_at_visual_offset,
    doc_formatter::TextFormat,
    syntax::Highlight,
    text_annotations::TextAnnotations,
    visual_offset_from_anchor, visual_offset_from_block, Position, RopeSlice, Selection,
    Transaction,
    VisualOffsetError::{PosAfterMaxRow, PosBeforeAnchorRow},
};

use std::{
    collections::{HashMap, VecDeque},
    fmt,
    rc::Rc,
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

#[derive(Clone, Debug, PartialEq, Eq, Copy, Default)]
pub struct ViewPosition {
    pub anchor: usize,
    pub horizontal_offset: usize,
    pub vertical_offset: usize,
}

#[derive(Clone)]
pub struct View {
    pub id: ViewId,
    pub offset: ViewPosition,
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
    /// all gutter-related configuration settings, used primarily for gutter rendering
    pub gutters: GutterConfig,
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
    pub fn new(doc: DocumentId, gutters: GutterConfig) -> Self {
        Self {
            id: ViewId::default(),
            doc,
            offset: ViewPosition {
                anchor: 0,
                horizontal_offset: 0,
                vertical_offset: 0,
            },
            area: Rect::default(), // will get calculated upon inserting into tree
            jumps: JumpList::new((doc, Selection::point(0))), // TODO: use actual sel
            docs_access_history: Vec::new(),
            last_modified_docs: [None, None],
            object_selections: Vec::new(),
            gutters,
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

    pub fn inner_width(&self, doc: &Document) -> u16 {
        self.area.clip_left(self.gutter_offset(doc)).width
    }

    pub fn gutters(&self) -> &[GutterType] {
        &self.gutters.layout
    }

    pub fn gutter_offset(&self, doc: &Document) -> u16 {
        self.gutters
            .layout
            .iter()
            .map(|gutter| gutter.width(self, doc) as u16)
            .sum()
    }

    //
    pub fn offset_coords_to_in_view(
        &self,
        doc: &Document,
        scrolloff: usize,
    ) -> Option<ViewPosition> {
        self.offset_coords_to_in_view_center::<false>(doc, scrolloff)
    }

    pub fn offset_coords_to_in_view_center<const CENTERING: bool>(
        &self,
        doc: &Document,
        scrolloff: usize,
    ) -> Option<ViewPosition> {
        let doc_text = doc.text().slice(..);
        let viewport = self.inner_area(doc);
        let vertical_viewport_end = self.offset.vertical_offset + viewport.height as usize;
        let text_fmt = doc.text_format(viewport.width, None);
        let annotations = self.text_annotations(doc, None);

        // - 1 so we have at least one gap in the middle.
        // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
        // as we type
        let scrolloff = if CENTERING {
            0
        } else {
            scrolloff.min(viewport.height.saturating_sub(1) as usize / 2)
        };

        let cursor = doc.selection(self.id).primary().cursor(doc_text);
        let mut offset = self.offset;
        let off = visual_offset_from_anchor(
            doc_text,
            offset.anchor,
            cursor,
            &text_fmt,
            &annotations,
            vertical_viewport_end,
        );

        let (new_anchor, at_top) = match off {
            Ok((visual_pos, _)) if visual_pos.row < scrolloff + offset.vertical_offset => {
                if CENTERING {
                    // cursor out of view
                    return None;
                }
                (true, true)
            }
            Ok((visual_pos, _)) if visual_pos.row + scrolloff >= vertical_viewport_end => {
                (true, false)
            }
            Ok((_, _)) => (false, false),
            Err(_) if CENTERING => return None,
            Err(PosBeforeAnchorRow) => (true, true),
            Err(PosAfterMaxRow) => (true, false),
        };

        if new_anchor {
            let v_off = if at_top {
                scrolloff as isize
            } else {
                viewport.height as isize - scrolloff as isize - 1
            };
            (offset.anchor, offset.vertical_offset) =
                char_idx_at_visual_offset(doc_text, cursor, -v_off, 0, &text_fmt, &annotations);
        }

        if text_fmt.soft_wrap {
            offset.horizontal_offset = 0;
        } else {
            // determine the current visual column of the text
            let col = off
                .unwrap_or_else(|_| {
                    visual_offset_from_block(
                        doc_text,
                        offset.anchor,
                        cursor,
                        &text_fmt,
                        &annotations,
                    )
                })
                .0
                .col;

            let last_col = offset.horizontal_offset + viewport.width.saturating_sub(1) as usize;
            if col > last_col.saturating_sub(scrolloff) {
                // scroll right
                offset.horizontal_offset += col - (last_col.saturating_sub(scrolloff))
            } else if col < offset.horizontal_offset + scrolloff {
                // scroll left
                offset.horizontal_offset = col.saturating_sub(scrolloff)
            };
        }

        // if we are not centering return None if view position is unchanged
        if !CENTERING && offset == self.offset {
            return None;
        }

        Some(offset)
    }

    pub fn ensure_cursor_in_view(&mut self, doc: &Document, scrolloff: usize) {
        if let Some(offset) = self.offset_coords_to_in_view_center::<false>(doc, scrolloff) {
            self.offset = offset;
        }
    }

    pub fn ensure_cursor_in_view_center(&mut self, doc: &Document, scrolloff: usize) {
        if let Some(offset) = self.offset_coords_to_in_view_center::<true>(doc, scrolloff) {
            self.offset = offset;
        } else {
            align_view(doc, self, Align::Center);
        }
    }

    pub fn is_cursor_in_view(&mut self, doc: &Document, scrolloff: usize) -> bool {
        self.offset_coords_to_in_view(doc, scrolloff).is_none()
    }

    /// Estimates the last visible document line on screen.
    /// This estimate is an upper bound obtained by calculating the first
    /// visible line and adding the viewport height.
    /// The actual last visible line may be smaller if softwrapping occurs
    /// or virtual text lines are visible
    #[inline]
    pub fn estimate_last_doc_line(&self, doc: &Document) -> usize {
        let doc_text = doc.text().slice(..);
        let line = doc_text.char_to_line(self.offset.anchor.min(doc_text.len_chars()));
        // Saturating subs to make it inclusive zero indexing.
        (line + self.inner_height())
            .min(doc_text.len_lines())
            .saturating_sub(1)
    }

    /// Calculates the last non-empty visual line on screen
    #[inline]
    pub fn last_visual_line(&self, doc: &Document) -> usize {
        let doc_text = doc.text().slice(..);
        let viewport = self.inner_area(doc);
        let text_fmt = doc.text_format(viewport.width, None);
        let annotations = self.text_annotations(doc, None);

        // last visual line in view is trivial to compute
        let visual_height = self.offset.vertical_offset + viewport.height as usize;

        // fast path when the EOF is not visible on the screen,
        if self.estimate_last_doc_line(doc) < doc_text.len_lines() - 1 {
            return visual_height.saturating_sub(1);
        }

        // translate to document line
        let pos = visual_offset_from_anchor(
            doc_text,
            self.offset.anchor,
            usize::MAX,
            &text_fmt,
            &annotations,
            visual_height,
        );

        match pos {
            Ok((Position { row, .. }, _)) => row.saturating_sub(self.offset.vertical_offset),
            Err(PosAfterMaxRow) => visual_height.saturating_sub(1),
            Err(PosBeforeAnchorRow) => 0,
        }
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
        if pos < self.offset.anchor {
            // Line is not visible on screen
            return None;
        }

        let viewport = self.inner_area(doc);
        let text_fmt = doc.text_format(viewport.width, None);
        let annotations = self.text_annotations(doc, None);

        let mut pos = visual_offset_from_anchor(
            text,
            self.offset.anchor,
            pos,
            &text_fmt,
            &annotations,
            viewport.height as usize,
        )
        .ok()?
        .0;
        if pos.row < self.offset.vertical_offset {
            return None;
        }
        pos.row -= self.offset.vertical_offset;
        if pos.row >= viewport.height as usize {
            return None;
        }
        pos.col = pos.col.saturating_sub(self.offset.horizontal_offset);

        Some(pos)
    }

    /// Get the text annotations to display in the current view for the given document and theme.
    pub fn text_annotations(&self, doc: &Document, theme: Option<&Theme>) -> TextAnnotations {
        // TODO custom annotations for custom views like side by side diffs

        let mut text_annotations = doc.text_annotations(theme);

        let DocumentInlayHints {
            id: _,
            type_inlay_hints,
            parameter_inlay_hints,
            other_inlay_hints,
            padding_before_inlay_hints,
            padding_after_inlay_hints,
        } = match doc.inlay_hints.get(&self.id) {
            Some(doc_inlay_hints) => doc_inlay_hints,
            None => return text_annotations,
        };

        let type_style = theme
            .and_then(|t| t.find_scope_index("ui.virtual.inlay-hint.type"))
            .map(Highlight);
        let parameter_style = theme
            .and_then(|t| t.find_scope_index("ui.virtual.inlay-hint.parameter"))
            .map(Highlight);
        let other_style = theme
            .and_then(|t| t.find_scope_index("ui.virtual.inlay-hint"))
            .map(Highlight);

        let mut add_annotations = |annotations: &Rc<[_]>, style| {
            if !annotations.is_empty() {
                text_annotations.add_inline_annotations(Rc::clone(annotations), style);
            }
        };

        // Overlapping annotations are ignored apart from the first so the order here is not random:
        // types -> parameters -> others should hopefully be the "correct" order for most use cases,
        // with the padding coming before and after as expected.
        add_annotations(padding_before_inlay_hints, None);
        add_annotations(type_inlay_hints, type_style);
        add_annotations(parameter_inlay_hints, parameter_style);
        add_annotations(other_inlay_hints, other_style);
        add_annotations(padding_after_inlay_hints, None);

        text_annotations
    }

    pub fn text_pos_at_screen_coords(
        &self,
        doc: &Document,
        row: u16,
        column: u16,
        fmt: TextFormat,
        annotations: &TextAnnotations,
        ignore_virtual_text: bool,
    ) -> Option<usize> {
        let inner = self.inner_area(doc);
        // 1 for status
        if row < inner.top() || row >= inner.bottom() {
            return None;
        }

        if column < inner.left() || column > inner.right() {
            return None;
        }

        self.text_pos_at_visual_coords(
            doc,
            row - inner.y,
            column - inner.x,
            fmt,
            annotations,
            ignore_virtual_text,
        )
    }

    pub fn text_pos_at_visual_coords(
        &self,
        doc: &Document,
        row: u16,
        column: u16,
        text_fmt: TextFormat,
        annotations: &TextAnnotations,
        ignore_virtual_text: bool,
    ) -> Option<usize> {
        let text = doc.text().slice(..);

        let text_row = row as usize + self.offset.vertical_offset;
        let text_col = column as usize + self.offset.horizontal_offset;

        let (char_idx, virt_lines) = char_idx_at_visual_offset(
            text,
            self.offset.anchor,
            text_row as isize,
            text_col,
            &text_fmt,
            annotations,
        );

        // if the cursor is on a line with only virtual text return None
        if virt_lines != 0 && ignore_virtual_text {
            return None;
        }
        Some(char_idx)
    }

    /// Translates a screen position to position in the text document.
    /// Returns a usize typed position in bounds of the text if found in this view, None if out of view.
    pub fn pos_at_screen_coords(
        &self,
        doc: &Document,
        row: u16,
        column: u16,
        ignore_virtual_text: bool,
    ) -> Option<usize> {
        self.text_pos_at_screen_coords(
            doc,
            row,
            column,
            doc.text_format(self.inner_width(doc), None),
            &self.text_annotations(doc, None),
            ignore_virtual_text,
        )
    }

    pub fn pos_at_visual_coords(
        &self,
        doc: &Document,
        row: u16,
        column: u16,
        ignore_virtual_text: bool,
    ) -> Option<usize> {
        self.text_pos_at_visual_coords(
            doc,
            row,
            column,
            doc.text_format(self.inner_width(doc), None),
            &self.text_annotations(doc, None),
            ignore_virtual_text,
        )
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
    use std::sync::Arc;

    use super::*;
    use arc_swap::ArcSwap;
    use helix_core::Rope;

    // 1 diagnostic + 1 spacer + 3 linenr (< 1000 lines) + 1 spacer + 1 diff
    const DEFAULT_GUTTER_OFFSET: u16 = 7;

    // 1 diagnostics + 1 spacer + 1 gutter
    const DEFAULT_GUTTER_OFFSET_ONLY_DIAGNOSTICS: u16 = 3;

    use crate::document::Document;
    use crate::editor::{Config, GutterConfig, GutterLineNumbersConfig, GutterType};

    #[test]
    fn test_text_pos_at_screen_coords() {
        let mut view = View::new(DocumentId::default(), GutterConfig::default());
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let doc = Document::from(
            rope,
            None,
            Arc::new(ArcSwap::new(Arc::new(Config::default()))),
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                2,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                41,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                0,
                2,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                0,
                49,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                0,
                41,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                81,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                78,
                41,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            None
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 3,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(3)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                80,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(3)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                41,
                40 + DEFAULT_GUTTER_OFFSET + 1,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(4)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                41,
                40 + DEFAULT_GUTTER_OFFSET + 4,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                41,
                40 + DEFAULT_GUTTER_OFFSET + 7,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(8)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                41,
                80,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(8)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_without_line_numbers_gutter() {
        let mut view = View::new(
            DocumentId::default(),
            GutterConfig {
                layout: vec![GutterType::Diagnostics],
                line_numbers: GutterLineNumbersConfig::default(),
            },
        );
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let doc = Document::from(
            rope,
            None,
            Arc::new(ArcSwap::new(Arc::new(Config::default()))),
        );
        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                41,
                40 + DEFAULT_GUTTER_OFFSET_ONLY_DIAGNOSTICS + 1,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(4)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_without_any_gutters() {
        let mut view = View::new(
            DocumentId::default(),
            GutterConfig {
                layout: vec![],
                line_numbers: GutterLineNumbersConfig::default(),
            },
        );
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("abc\n\tdef");
        let doc = Document::from(
            rope,
            None,
            Arc::new(ArcSwap::new(Arc::new(Config::default()))),
        );
        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                41,
                40 + 1,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(4)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_cjk() {
        let mut view = View::new(DocumentId::default(), GutterConfig::default());
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("Hi! こんにちは皆さん");
        let doc = Document::from(
            rope,
            None,
            Arc::new(ArcSwap::new(Arc::new(Config::default()))),
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(0)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 4,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(4)
        );
        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 5,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(4)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 6,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 7,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 8,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(6)
        );
    }

    #[test]
    fn test_text_pos_at_screen_coords_graphemes() {
        let mut view = View::new(DocumentId::default(), GutterConfig::default());
        view.area = Rect::new(40, 40, 40, 40);
        let rope = Rope::from_str("Hèl̀l̀ò world!");
        let doc = Document::from(
            rope,
            None,
            Arc::new(ArcSwap::new(Arc::new(Config::default()))),
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(0)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 1,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(1)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 2,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(3)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 3,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(5)
        );

        assert_eq!(
            view.text_pos_at_screen_coords(
                &doc,
                40,
                40 + DEFAULT_GUTTER_OFFSET + 4,
                TextFormat::default(),
                &TextAnnotations::default(),
                true
            ),
            Some(7)
        );
    }
}
