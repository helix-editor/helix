use std::ops::Range;
use std::sync::Arc;

use helix_core::text_annotations::LineAnnotation;
use helix_core::{Position, Rope, RopeSlice};
use helix_vcs::Hunk;
use imara_diff::{Algorithm, Diff, InternedInput};

use crate::{DocumentId, ViewId};

struct RopeLines<'a>(RopeSlice<'a>);

impl<'a> imara_diff::TokenSource for RopeLines<'a> {
    type Token = RopeSlice<'a>;
    type Tokenizer = helix_core::ropey::iter::Lines<'a>;

    fn tokenize(&self) -> Self::Tokenizer {
        self.0.lines()
    }

    fn estimate_tokens(&self) -> u32 {
        self.0.len_lines() as u32
    }
}

/// A diff session pairs two views for side-by-side diff comparison.
/// It holds the computed hunks between their documents and coordinates
/// scroll synchronization and alignment.
#[derive(Debug)]
pub struct DiffSession {
    view_a: ViewId,
    view_b: ViewId,
    doc_a: DocumentId,
    doc_b: DocumentId,
    hunks: Vec<Hunk>,
    /// Tracked document versions for change detection
    version_a: i32,
    version_b: i32,
}

impl DiffSession {
    pub fn new(view_a: ViewId, view_b: ViewId, doc_a: DocumentId, doc_b: DocumentId) -> Self {
        Self {
            view_a,
            view_b,
            doc_a,
            doc_b,
            hunks: Vec::new(),
            version_a: -1,
            version_b: -1,
        }
    }

    /// Recompute hunks if either document has changed since the last computation.
    /// Returns true if hunks were recomputed.
    pub fn update_if_changed(
        &mut self,
        version_a: i32,
        version_b: i32,
        rope_a: &Rope,
        rope_b: &Rope,
    ) -> bool {
        if version_a == self.version_a && version_b == self.version_b {
            return false;
        }
        self.version_a = version_a;
        self.version_b = version_b;
        self.compute_hunks(rope_a, rope_b);
        true
    }

    pub fn view_a(&self) -> ViewId {
        self.view_a
    }

    pub fn view_b(&self) -> ViewId {
        self.view_b
    }

    pub fn doc_a(&self) -> DocumentId {
        self.doc_a
    }

    pub fn doc_b(&self) -> DocumentId {
        self.doc_b
    }

    pub fn hunks(&self) -> &[Hunk] {
        &self.hunks
    }

    /// Returns true if the given view is part of this diff session.
    pub fn contains_view(&self, view_id: ViewId) -> bool {
        self.view_a == view_id || self.view_b == view_id
    }

    /// Computes line-level hunks between two Ropes using the Histogram diff algorithm.
    /// `rope_a` corresponds to the left/before side, `rope_b` to the right/after side.
    pub fn compute_hunks(&mut self, rope_a: &Rope, rope_b: &Rope) {
        self.hunks.clear();
        let input = InternedInput::new(RopeLines(rope_a.slice(..)), RopeLines(rope_b.slice(..)));
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_with(
            &input.before,
            &input.after,
            imara_diff::IndentHeuristic::new(|token| {
                imara_diff::IndentLevel::for_ascii_line(input.interner[token].bytes(), 4)
            }),
        );
        self.hunks.extend(diff.hunks());
    }

    /// Returns the documents in this session.
    pub fn doc_ids(&self) -> (DocumentId, DocumentId) {
        (self.doc_a, self.doc_b)
    }

    /// Returns the partner view ID for the given view, if it belongs to this session.
    pub fn partner_view(&self, view_id: ViewId) -> Option<ViewId> {
        if view_id == self.view_a {
            Some(self.view_b)
        } else if view_id == self.view_b {
            Some(self.view_a)
        } else {
            None
        }
    }
}

/// A per-line column range that should be highlighted for intra-line changes.
#[derive(Debug, Clone)]
pub struct InlineChange {
    pub doc_line: usize,
    pub col_start: usize,
    pub col_end: usize,
}

impl InlineChange {
    /// Convert to an absolute char range in the given rope.
    pub fn to_char_range(&self, rope: &Rope) -> std::ops::Range<usize> {
        let line_start = rope.line_to_char(self.doc_line);
        (line_start + self.col_start)..(line_start + self.col_end)
    }
}

/// Compute character-level diff for a single hunk.
/// Returns per-line column ranges for each side indicating which characters changed.
pub fn intra_line_changes(
    rope_a: &Rope,
    rope_b: &Rope,
    hunk: &Hunk,
) -> (Vec<InlineChange>, Vec<InlineChange>) {
    let a_start = rope_a.line_to_char(hunk.before.start as usize);
    let a_end = rope_a.line_to_char(hunk.before.end as usize);
    let b_start = rope_b.line_to_char(hunk.after.start as usize);
    let b_end = rope_b.line_to_char(hunk.after.end as usize);

    let text_a: String = rope_a.slice(a_start..a_end).into();
    let text_b: String = rope_b.slice(b_start..b_end).into();

    let input = InternedInput::new(text_a.as_str(), text_b.as_str());
    let diff = Diff::compute(Algorithm::Myers, &input);

    let char_to_line_col = |base_char: usize, offset: u32, rope: &Rope| -> (usize, usize) {
        let char_idx = base_char + offset as usize;
        let line = rope.char_to_line(char_idx);
        let line_start = rope.line_to_char(line);
        (line, char_idx - line_start)
    };

    let mut changes_a = Vec::new();
    let mut changes_b = Vec::new();

    for char_hunk in diff.hunks() {
        if !char_hunk.before.is_empty() {
            let (start_line, start_col) = char_to_line_col(a_start, char_hunk.before.start, rope_a);
            let (end_line, end_col) = char_to_line_col(a_start, char_hunk.before.end, rope_a);
            // Split across lines if the change spans multiple lines
            for line in start_line..=end_line {
                let cs = if line == start_line { start_col } else { 0 };
                let ce = if line == end_line {
                    end_col
                } else {
                    rope_a.line(line).len_chars()
                };
                changes_a.push(InlineChange {
                    doc_line: line,
                    col_start: cs,
                    col_end: ce,
                });
            }
        }
        if !char_hunk.after.is_empty() {
            let (start_line, start_col) = char_to_line_col(b_start, char_hunk.after.start, rope_b);
            let (end_line, end_col) = char_to_line_col(b_start, char_hunk.after.end, rope_b);
            for line in start_line..=end_line {
                let cs = if line == start_line { start_col } else { 0 };
                let ce = if line == end_line {
                    end_col
                } else {
                    rope_b.line(line).len_chars()
                };
                changes_b.push(InlineChange {
                    doc_line: line,
                    col_start: cs,
                    col_end: ce,
                });
            }
        }
    }

    (changes_a, changes_b)
}

/// Which side of a diff session this alignment annotation is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    /// The left/before side (doc_a)
    A,
    /// The right/after side (doc_b)
    B,
}

/// A `LineAnnotation` that inserts virtual filler lines to keep two
/// side-by-side diff panes visually aligned at hunk boundaries.
///
/// Each side of the diff gets its own `DiffAlignment`. When the "other" side
/// has more lines in a hunk, this annotation pads with filler lines so both
/// panes show the same visual height for every hunk.
pub struct DiffAlignment {
    hunks: Arc<Vec<Hunk>>,
    side: DiffSide,
    cursor: usize,
}

impl DiffAlignment {
    pub fn new(hunks: Arc<Vec<Hunk>>, side: DiffSide) -> Self {
        Self {
            hunks,
            side,
            cursor: 0,
        }
    }

    /// Returns (my_range, other_range) for the hunk based on our side.
    fn ranges_for(&self, hunk: &Hunk) -> (Range<u32>, Range<u32>) {
        match self.side {
            DiffSide::A => (hunk.before.clone(), hunk.after.clone()),
            DiffSide::B => (hunk.after.clone(), hunk.before.clone()),
        }
    }

    /// Compute how many filler lines to insert after `doc_line`.
    /// Returns 0 if no filler is needed at this line.
    pub fn filler_lines_at(&mut self, doc_line: usize) -> usize {
        // Advance cursor past hunks that ended before this line
        while self.cursor < self.hunks.len() {
            let (my_range, _) = self.ranges_for(&self.hunks[self.cursor]);
            let trigger = if my_range.is_empty() {
                // Pure insertion/deletion on our side: trigger at the line
                // just before the insertion point (or 0 if at start)
                my_range.start.saturating_sub(1) as usize
            } else {
                (my_range.end - 1) as usize
            };
            if trigger < doc_line {
                self.cursor += 1;
            } else {
                break;
            }
        }

        if self.cursor >= self.hunks.len() {
            return 0;
        }

        let (my_range, other_range) = self.ranges_for(&self.hunks[self.cursor]);

        let trigger = if my_range.is_empty() {
            my_range.start.saturating_sub(1) as usize
        } else {
            (my_range.end - 1) as usize
        };

        if doc_line == trigger {
            self.cursor += 1;
            let filler =
                (other_range.end - other_range.start).saturating_sub(my_range.end - my_range.start);
            filler as usize
        } else {
            0
        }
    }
}

impl LineAnnotation for DiffAlignment {
    fn reset_pos(&mut self, _char_idx: usize) -> usize {
        self.cursor = 0;
        usize::MAX
    }

    fn insert_virtual_lines(
        &mut self,
        _line_end_char_idx: usize,
        _line_end_visual_pos: Position,
        doc_line: usize,
    ) -> Position {
        let filler = self.filler_lines_at(doc_line);
        Position::new(filler, 0)
    }
}

impl std::fmt::Debug for DiffAlignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffAlignment")
            .field("side", &self.side)
            .field("cursor", &self.cursor)
            .field("num_hunks", &self.hunks.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    fn make_view_ids() -> (ViewId, ViewId) {
        let mut sm: slotmap::SlotMap<ViewId, ()> = slotmap::SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        (a, b)
    }

    fn make_doc_id(n: usize) -> DocumentId {
        DocumentId(NonZeroUsize::new(n).unwrap())
    }

    #[test]
    fn stores_paired_view_ids() {
        let (view_a, view_b) = make_view_ids();
        let doc_a = make_doc_id(1);
        let doc_b = make_doc_id(2);

        let session = DiffSession::new(view_a, view_b, doc_a, doc_b);

        assert_eq!(session.view_a(), view_a);
        assert_eq!(session.view_b(), view_b);
        assert_eq!(session.doc_a(), doc_a);
        assert_eq!(session.doc_b(), doc_b);
    }

    #[test]
    fn contains_view_returns_true_for_session_members() {
        let (view_a, view_b) = make_view_ids();
        let session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));

        assert!(session.contains_view(view_a));
        assert!(session.contains_view(view_b));
    }

    #[test]
    fn contains_view_returns_false_for_non_members() {
        let mut sm: slotmap::SlotMap<ViewId, ()> = slotmap::SlotMap::with_key();
        let view_a = sm.insert(());
        let view_b = sm.insert(());
        let view_c = sm.insert(());

        let session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));
        assert!(!session.contains_view(view_c));
    }

    #[test]
    fn partner_view_returns_other_side() {
        let (view_a, view_b) = make_view_ids();
        let session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));

        assert_eq!(session.partner_view(view_a), Some(view_b));
        assert_eq!(session.partner_view(view_b), Some(view_a));
    }

    #[test]
    fn partner_view_returns_none_for_non_member() {
        let mut sm: slotmap::SlotMap<ViewId, ()> = slotmap::SlotMap::with_key();
        let view_a = sm.insert(());
        let view_b = sm.insert(());
        let view_c = sm.insert(());

        let session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));
        assert_eq!(session.partner_view(view_c), None);
    }

    #[test]
    fn new_session_has_no_hunks() {
        let (view_a, view_b) = make_view_ids();
        let session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));

        assert!(session.hunks().is_empty());
    }

    fn make_session() -> DiffSession {
        let (view_a, view_b) = make_view_ids();
        DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2))
    }

    #[test]
    fn compute_hunks_detects_addition() {
        let mut session = make_session();
        let rope_a = Rope::from("line1\nline2\nline3\n");
        let rope_b = Rope::from("line1\nline2\ninserted\nline3\n");

        session.compute_hunks(&rope_a, &rope_b);

        assert_eq!(session.hunks().len(), 1);
        let hunk = &session.hunks()[0];
        // "inserted" is added after line2 (line index 2) in rope_b
        assert_eq!(hunk.before.start, 2);
        assert_eq!(hunk.before.end, 2); // pure insertion: empty range in before
        assert_eq!(hunk.after.start, 2);
        assert_eq!(hunk.after.end, 3); // one line added
    }

    #[test]
    fn compute_hunks_detects_deletion() {
        let mut session = make_session();
        let rope_a = Rope::from("line1\nline2\nline3\nline4\n");
        let rope_b = Rope::from("line1\nline4\n");

        session.compute_hunks(&rope_a, &rope_b);

        assert_eq!(session.hunks().len(), 1);
        let hunk = &session.hunks()[0];
        assert_eq!(hunk.before.start, 1);
        assert_eq!(hunk.before.end, 3); // lines 2 and 3 removed
        assert_eq!(hunk.after.start, 1);
        assert_eq!(hunk.after.end, 1); // pure deletion: empty range in after
    }

    #[test]
    fn compute_hunks_detects_modification() {
        let mut session = make_session();
        let rope_a = Rope::from("line1\nold\nline3\n");
        let rope_b = Rope::from("line1\nnew\nline3\n");

        session.compute_hunks(&rope_a, &rope_b);

        assert_eq!(session.hunks().len(), 1);
        let hunk = &session.hunks()[0];
        assert_eq!(hunk.before.start, 1);
        assert_eq!(hunk.before.end, 2);
        assert_eq!(hunk.after.start, 1);
        assert_eq!(hunk.after.end, 2);
    }

    #[test]
    fn compute_hunks_identical_files_produces_no_hunks() {
        let mut session = make_session();
        let text = "line1\nline2\nline3\n";
        let rope_a = Rope::from(text);
        let rope_b = Rope::from(text);

        session.compute_hunks(&rope_a, &rope_b);

        assert!(session.hunks().is_empty());
    }

    #[test]
    fn compute_hunks_multiple_changes() {
        let mut session = make_session();
        let rope_a = Rope::from("aaa\nbbb\nccc\nddd\neee\n");
        let rope_b = Rope::from("aaa\nBBB\nccc\nddd\nEEE\n");

        session.compute_hunks(&rope_a, &rope_b);

        assert_eq!(session.hunks().len(), 2);
        // First hunk: bbb -> BBB
        assert_eq!(session.hunks()[0].before, 1..2);
        assert_eq!(session.hunks()[0].after, 1..2);
        // Second hunk: eee -> EEE
        assert_eq!(session.hunks()[1].before, 4..5);
        assert_eq!(session.hunks()[1].after, 4..5);
    }

    #[test]
    fn compute_hunks_recomputes_on_new_input() {
        let mut session = make_session();

        let rope_a = Rope::from("aaa\nbbb\n");
        let rope_b = Rope::from("aaa\nccc\n");
        session.compute_hunks(&rope_a, &rope_b);
        assert_eq!(session.hunks().len(), 1);

        // Recompute with identical content: hunks should clear
        session.compute_hunks(&rope_a, &rope_a);
        assert!(session.hunks().is_empty());
    }

    // --- DiffAlignment tests ---

    fn make_hunks(pairs: &[(Range<u32>, Range<u32>)]) -> Arc<Vec<Hunk>> {
        Arc::new(
            pairs
                .iter()
                .map(|(b, a)| Hunk {
                    before: b.clone(),
                    after: a.clone(),
                })
                .collect(),
        )
    }

    #[test]
    fn alignment_identical_files_no_fillers() {
        let hunks = make_hunks(&[]);
        let mut align = DiffAlignment::new(hunks, DiffSide::A);

        for line in 0..5 {
            assert_eq!(align.filler_lines_at(line), 0);
        }
    }

    #[test]
    fn alignment_addition_pads_side_a() {
        // before: 5 lines, after: 8 lines (3 added at lines 2..5 on after)
        // Hunk: before 2..2 (pure insertion), after 2..5
        let hunks = make_hunks(&[(2..2, 2..5)]);
        let mut align = DiffAlignment::new(hunks, DiffSide::A);

        // Side A has 0 lines in hunk, other side has 3
        // Trigger at doc_line = max(0, 2-1) = 1
        assert_eq!(align.filler_lines_at(0), 0);
        assert_eq!(align.filler_lines_at(1), 3); // 3 fillers for side A
        assert_eq!(align.filler_lines_at(2), 0);
    }

    #[test]
    fn alignment_addition_no_pad_on_longer_side() {
        // Same hunk but from side B's perspective: B has the 3 added lines
        let hunks = make_hunks(&[(2..2, 2..5)]);
        let mut align = DiffAlignment::new(hunks, DiffSide::B);

        // Side B has 3 lines, other side has 0. No filler needed.
        for line in 0..8 {
            assert_eq!(align.filler_lines_at(line), 0);
        }
    }

    #[test]
    fn alignment_deletion_pads_side_b() {
        // before has 3 lines (1..4), after has 0 lines (1..1)
        let hunks = make_hunks(&[(1..4, 1..1)]);
        let mut align = DiffAlignment::new(hunks, DiffSide::B);

        // Side B (after) has 0 lines, other has 3. Trigger at 1-1=0
        assert_eq!(align.filler_lines_at(0), 3);
        assert_eq!(align.filler_lines_at(1), 0);
    }

    #[test]
    fn alignment_modification_pads_shorter_side() {
        // before: 2 lines (1..3), after: 5 lines (1..6)
        let hunks = make_hunks(&[(1..3, 1..6)]);
        let mut align_a = DiffAlignment::new(hunks.clone(), DiffSide::A);

        // Side A has 2 lines, other has 5. Need 3 fillers after last line (doc_line 2)
        assert_eq!(align_a.filler_lines_at(0), 0);
        assert_eq!(align_a.filler_lines_at(1), 0);
        assert_eq!(align_a.filler_lines_at(2), 3);
        assert_eq!(align_a.filler_lines_at(3), 0);

        // Side B has 5 lines, other has 2. No filler needed.
        let mut align_b = DiffAlignment::new(hunks, DiffSide::B);
        for line in 0..8 {
            assert_eq!(align_b.filler_lines_at(line), 0);
        }
    }

    #[test]
    fn alignment_multiple_hunks() {
        // Two hunks: modification at lines 1 and addition at lines 4
        let hunks = make_hunks(&[
            (1..2, 1..4), // 1 line before, 3 after: side A needs 2 fillers
            (4..4, 6..8), // pure insertion: 0 before, 2 after: side A needs 2 fillers
        ]);
        let mut align = DiffAlignment::new(hunks, DiffSide::A);

        assert_eq!(align.filler_lines_at(0), 0);
        assert_eq!(align.filler_lines_at(1), 2); // end of first hunk
        assert_eq!(align.filler_lines_at(2), 0);
        assert_eq!(align.filler_lines_at(3), 2); // trigger for second hunk (4-1=3)
        assert_eq!(align.filler_lines_at(4), 0);
    }

    #[test]
    fn alignment_equal_length_modification_no_fillers() {
        // Both sides have same number of lines
        let hunks = make_hunks(&[(2..5, 2..5)]);
        let mut align_a = DiffAlignment::new(hunks.clone(), DiffSide::A);
        let mut align_b = DiffAlignment::new(hunks, DiffSide::B);

        for line in 0..8 {
            assert_eq!(align_a.filler_lines_at(line), 0);
            assert_eq!(align_b.filler_lines_at(line), 0);
        }
    }
}
