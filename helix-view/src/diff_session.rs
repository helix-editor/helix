use std::ops::Range;
use std::sync::Arc;

use helix_core::doc_formatter::TextFormat;
use helix_core::text_annotations::{LineAnnotation, TextAnnotations};
use helix_core::{visual_offset_from_block, Position, Rope, RopeSlice, Tendril, Transaction};
use helix_vcs::Hunk;
use imara_diff::{Algorithm, Diff, InternedInput, TokenSource};

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

/// Iterator that yields word-level tokens from a string slice.
/// Alphanumeric + underscore runs are emitted as a single token; all other chars
/// (whitespace, punctuation, newlines) are emitted individually. This gives
/// coarser intra-line diffs than char-level: only whole words are marked as changed.
struct WordTokenIter<'a> {
    text: &'a str,
    byte_pos: usize,
}

impl<'a> Iterator for WordTokenIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.byte_pos >= self.text.len() {
            return None;
        }
        let rest = &self.text[self.byte_pos..];
        let first = rest.chars().next()?;
        let end_byte = if first.is_alphanumeric() || first == '_' {
            rest.char_indices()
                .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(first.len_utf8())
        } else {
            first.len_utf8()
        };
        let token = &rest[..end_byte];
        self.byte_pos += end_byte;
        Some(token)
    }
}

/// A `TokenSource` that yields word-level tokens from a `&str`.
/// Used by `intra_line_changes` so the Myers diff operates on whole words
/// rather than individual characters.
struct WordSlice<'a>(&'a str);

impl<'a> TokenSource for WordSlice<'a> {
    type Token = &'a str;
    type Tokenizer = WordTokenIter<'a>;

    fn tokenize(&self) -> Self::Tokenizer {
        WordTokenIter {
            text: self.0,
            byte_pos: 0,
        }
    }

    fn estimate_tokens(&self) -> u32 {
        (self.0.len() / 4).max(1) as u32
    }
}

/// Returns the char-index (within `text`) at which each word token starts.
/// `result[i]` is the char start of token `i`; `result.len()` equals the token count.
/// If `tok < result.len()`, the token starts at `result[tok]` and ends at
/// `result[tok + 1]` (or at `text.chars().count()` for the last token).
fn word_token_char_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut char_pos = 0usize;
    let mut byte_pos = 0usize;
    while byte_pos < text.len() {
        let rest = &text[byte_pos..];
        let first = rest.chars().next().unwrap();
        starts.push(char_pos);
        let end_byte = if first.is_alphanumeric() || first == '_' {
            rest.char_indices()
                .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(first.len_utf8())
        } else {
            first.len_utf8()
        };
        char_pos += rest[..end_byte].chars().count();
        byte_pos += end_byte;
    }
    starts
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
    /// Shared hunk list. Stored as Arc so callers can take a cheap reference-counted
    /// snapshot for closures and annotations without cloning the full Vec each frame.
    hunks: Arc<Vec<Hunk>>,
    /// Word-level intra-line diff results cached per hunk, parallel to `hunks`.
    /// Stored as Arc so render closures can take a cheap snapshot without cloning.
    /// Populated in `compute_hunks` and replaced on each recomputation.
    /// Pure insertions/deletions store empty vecs (no character diff needed).
    intra_line_cache: Arc<Vec<(Vec<InlineChange>, Vec<InlineChange>)>>,
    version_a: Option<i32>,
    version_b: Option<i32>,
}

impl DiffSession {
    pub fn new(view_a: ViewId, view_b: ViewId, doc_a: DocumentId, doc_b: DocumentId) -> Self {
        Self {
            view_a,
            view_b,
            doc_a,
            doc_b,
            hunks: Arc::new(Vec::new()),
            intra_line_cache: Arc::new(Vec::new()),
            version_a: None,
            version_b: None,
        }
    }

    /// Returns true if the stored versions differ from the given ones, meaning
    /// `update_if_changed` would recompute. Lets callers defer expensive Rope
    /// clones until they are actually needed.
    pub fn needs_update(&self, version_a: i32, version_b: i32) -> bool {
        self.version_a != Some(version_a) || self.version_b != Some(version_b)
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
        if !self.needs_update(version_a, version_b) {
            return false;
        }
        self.version_a = Some(version_a);
        self.version_b = Some(version_b);
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

    /// Returns a cheap reference-counted snapshot of the hunk list.
    /// Callers that need to hold hunks across a borrow boundary (e.g. render closures)
    /// should use this instead of `hunks().to_vec()`.
    pub fn hunks_arc(&self) -> Arc<Vec<Hunk>> {
        Arc::clone(&self.hunks)
    }

    /// Returns a cheap reference-counted snapshot of the intra-line cache.
    /// Render closures should use this to capture the cache without cloning it,
    /// mirroring the `hunks_arc` pattern.
    pub fn intra_line_cache_arc(&self) -> Arc<Vec<(Vec<InlineChange>, Vec<InlineChange>)>> {
        Arc::clone(&self.intra_line_cache)
    }

    /// Returns true if the given view is part of this diff session.
    pub fn contains_view(&self, view_id: ViewId) -> bool {
        self.view_a == view_id || self.view_b == view_id
    }

    /// Returns true if the given document is part of this diff session.
    pub fn contains_doc(&self, doc_id: DocumentId) -> bool {
        self.doc_a == doc_id || self.doc_b == doc_id
    }

    /// Computes line-level hunks between two Ropes using the Histogram diff algorithm.
    /// `rope_a` corresponds to the left/before side, `rope_b` to the right/after side.
    /// Also rebuilds the intra-line character diff cache, indexed parallel to `hunks`.
    pub fn compute_hunks(&mut self, rope_a: &Rope, rope_b: &Rope) {
        let input = InternedInput::new(RopeLines(rope_a.slice(..)), RopeLines(rope_b.slice(..)));
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_with(
            &input.before,
            &input.after,
            imara_diff::IndentHeuristic::new(|token| {
                imara_diff::IndentLevel::for_ascii_line(input.interner[token].bytes(), 4)
            }),
        );
        self.hunks = Arc::new(diff.hunks().collect());
        // Rebuild cache parallel to hunks. Pure insertions/deletions get empty entries
        // (nothing to diff at the character level when one side is empty).
        self.intra_line_cache = Arc::new(
            self.hunks
                .iter()
                .map(|hunk| {
                    if hunk.before.is_empty() || hunk.after.is_empty() {
                        (Vec::new(), Vec::new())
                    } else {
                        intra_line_changes(rope_a, rope_b, hunk)
                    }
                })
                .collect(),
        );
    }

    /// Returns the cached intra-line character diff for the hunk at `index`,
    /// or `None` if the index is out of range.
    /// The tuple is `(changes_a, changes_b)` as produced by `intra_line_changes`.
    pub fn intra_line_changes_for(
        &self,
        index: usize,
    ) -> Option<&(Vec<InlineChange>, Vec<InlineChange>)> {
        self.intra_line_cache.get(index)
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

    /// Returns which side of the diff session the given view is on.
    pub fn side_for_view(&self, view_id: ViewId) -> Option<DiffSide> {
        if view_id == self.view_a {
            Some(DiffSide::A)
        } else if view_id == self.view_b {
            Some(DiffSide::B)
        } else {
            None
        }
    }

    /// Returns the partner document ID for the given document.
    pub fn partner_doc(&self, doc_id: DocumentId) -> Option<DocumentId> {
        if doc_id == self.doc_a {
            Some(self.doc_b)
        } else if doc_id == self.doc_b {
            Some(self.doc_a)
        } else {
            None
        }
    }

    /// Map a line on `from_side` to its corresponding position on the partner side.
    ///
    /// Walks the hunk list once with a running offset (sum of `other_len - my_len`
    /// for hunks fully above the input line):
    ///
    /// - Outside any hunk: 1:1 with the running offset applied.
    /// - Inside a hunk where the partner has lines: clamp the within-hunk offset
    ///   to `other_range.end - 1` so an asymmetric replace doesn't run past the
    ///   partner's last line.
    /// - Inside a hunk where the partner is empty (deletion-on-partner): return
    ///   [`MappedLine::Filler`] anchored at `other_range.start - 1`. The partner
    ///   has no real line for this position.
    ///
    /// Saturating arithmetic throughout so `u32::MAX` inputs do not panic.
    pub fn map_line(&self, from_side: DiffSide, line: u32) -> MappedLine {
        let mut offset: i64 = 0;
        for hunk in self.hunks.iter() {
            let (my_range, other_range) = match from_side {
                DiffSide::A => (hunk.before.clone(), hunk.after.clone()),
                DiffSide::B => (hunk.after.clone(), hunk.before.clone()),
            };

            if line < my_range.start {
                return MappedLine::Real(apply_offset(line, offset));
            }

            // `line < my_range.end` only when my_range is non-empty (else
            // start == end and the previous branch already handled it).
            if line < my_range.end {
                if other_range.is_empty() {
                    return MappedLine::Filler {
                        after: other_range.start.saturating_sub(1),
                    };
                }
                let within_offset = line.saturating_sub(my_range.start);
                let other_last = other_range
                    .end
                    .saturating_sub(other_range.start)
                    .saturating_sub(1);
                let clamped = within_offset.min(other_last);
                return MappedLine::Real(other_range.start.saturating_add(clamped));
            }

            let my_len = my_range.end as i64 - my_range.start as i64;
            let other_len = other_range.end as i64 - other_range.start as i64;
            offset += other_len - my_len;
        }

        MappedLine::Real(apply_offset(line, offset))
    }

    /// Map a line and clamp the result to the partner document's line count.
    ///
    /// Filler positions collapse to the line they sit after. Useful for cursor
    /// placement, where a real partner line is always required.
    pub fn map_to_real_line(&self, from_side: DiffSide, line: u32, partner_line_count: u32) -> u32 {
        let last = partner_line_count.saturating_sub(1);
        let mapped = match self.map_line(from_side, line) {
            MappedLine::Real(n) => n,
            MappedLine::Filler { after } => after,
        };
        mapped.min(last)
    }
}

/// Result of [`DiffSession::map_line`]. The partner side may have a real line at
/// the mapped position, or only a filler if the input lands inside a deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedLine {
    /// The partner has a real line at this index.
    Real(u32),
    /// The partner has no line for the input position; conceptually the input
    /// sits in a filler row just after partner line `after`.
    Filler { after: u32 },
}

/// Apply a signed offset to a line index, saturating at the `u32` bounds.
fn apply_offset(line: u32, offset: i64) -> u32 {
    let result = line as i64 + offset;
    result.clamp(0, u32::MAX as i64) as u32
}

/// Side of the session that holds `source`, but only when `source` and `dest`
/// are paired partners in the same `DiffSession`. Returns `None` when no
/// session contains `source`, or when `source`'s partner is some other view.
///
/// This is the gating predicate for cursor-sync writes: returning a side
/// authorizes the caller to write to `dest`'s selection. Returning `None`
/// means the caller must skip - either no session pairs them, or the session
/// has changed shape since the focus event arrived.
pub fn paired_side(sessions: &[DiffSession], source: ViewId, dest: ViewId) -> Option<DiffSide> {
    sessions
        .iter()
        .find(|s| s.contains_view(source))
        .and_then(|s| {
            if s.partner_view(source) != Some(dest) {
                return None;
            }
            s.side_for_view(source)
        })
}

/// Returns an iterator over hunks whose range on `side` intersects any of the given line
/// ranges. Line ranges are (start_line, end_line) pairs (inclusive, 0-indexed).
///
/// For empty ranges (pure deletions on this side), a hunk matches if its start position
/// is at or before the end of the selection range, mirroring VCS `hunks_intersecting_line_ranges`.
fn hunks_intersecting<'a>(
    hunks: &'a [Hunk],
    line_ranges: &'a [(usize, usize)],
    side: DiffSide,
) -> impl Iterator<Item = &'a Hunk> {
    hunks.iter().filter(move |h| {
        let curr_range = match side {
            DiffSide::A => &h.before,
            DiffSide::B => &h.after,
        };
        line_ranges.iter().any(|(start, end)| {
            if curr_range.is_empty() {
                curr_range.start as usize <= *end
            } else {
                curr_range.start as usize <= *end && curr_range.end as usize > *start
            }
        })
    })
}

/// Index of the first hunk strictly after `cursor_line` on `side`.
///
/// "Strictly after" means: if the cursor sits inside or at the start of a
/// hunk, that hunk is skipped. This matches `helix-vcs`'s `next_hunk` and is
/// the predicate behind `]g` when a `DiffSession` is active. Returns `None`
/// when no later hunk exists.
///
/// Empty ranges (deletions on this side) are handled by the same predicate:
/// the deletion's filler renders between line `start - 1` and `start`, so a
/// cursor at line `start` is already past the filler and the deletion should
/// not match.
pub fn find_next_hunk(hunks: &[Hunk], side: DiffSide, cursor_line: u32) -> Option<usize> {
    hunks.iter().position(|h| {
        let r = match side {
            DiffSide::A => &h.before,
            DiffSide::B => &h.after,
        };
        r.start > cursor_line
    })
}

/// Index of the last hunk strictly before `cursor_line` on `side`.
///
/// Symmetric to [`find_next_hunk`]: a cursor inside or at the end of a hunk
/// skips that hunk. Used by `[g` when a `DiffSession` is active. Returns
/// `None` when no earlier hunk exists.
pub fn find_prev_hunk(hunks: &[Hunk], side: DiffSide, cursor_line: u32) -> Option<usize> {
    hunks.iter().rposition(|h| {
        let r = match side {
            DiffSide::A => &h.before,
            DiffSide::B => &h.after,
        };
        if r.is_empty() {
            r.start < cursor_line
        } else {
            r.end <= cursor_line
        }
    })
}

/// Build a transaction on `curr_text` that replaces content at hunks under `line_ranges`
/// with content from `partner_text`. This is the `:diffget` operation: pull from partner.
/// Returns `(transaction, number_of_changes)`.
pub fn build_get_transaction(
    hunks: &[Hunk],
    side: DiffSide,
    line_ranges: &[(usize, usize)],
    curr_text: &Rope,
    partner_text: &Rope,
) -> (Transaction, usize) {
    let mut changes = 0usize;
    let transaction = Transaction::change(
        curr_text,
        hunks_intersecting(hunks, line_ranges, side).map(|h| {
            let (curr_range, partner_range) = match side {
                DiffSide::A => (&h.before, &h.after),
                DiffSide::B => (&h.after, &h.before),
            };
            changes += 1;
            let curr_start = curr_text.line_to_char(curr_range.start as usize);
            let curr_end = curr_text.line_to_char(curr_range.end as usize);
            let p_start = partner_text.line_to_char(partner_range.start as usize);
            let p_end = partner_text.line_to_char(partner_range.end as usize);
            let text: Tendril = partner_text.slice(p_start..p_end).chunks().collect();
            (curr_start, curr_end, (!text.is_empty()).then_some(text))
        }),
    );
    (transaction, changes)
}

/// Build a transaction on `partner_text` that replaces content at hunks under `line_ranges`
/// with content from `curr_text`. This is the `:diffput` operation: push to partner.
/// Returns `(transaction, number_of_changes)`.
pub fn build_put_transaction(
    hunks: &[Hunk],
    side: DiffSide,
    line_ranges: &[(usize, usize)],
    curr_text: &Rope,
    partner_text: &Rope,
) -> (Transaction, usize) {
    let mut changes = 0usize;
    let transaction = Transaction::change(
        partner_text,
        hunks_intersecting(hunks, line_ranges, side).map(|h| {
            let (curr_range, partner_range) = match side {
                DiffSide::A => (&h.before, &h.after),
                DiffSide::B => (&h.after, &h.before),
            };
            changes += 1;
            let p_start = partner_text.line_to_char(partner_range.start as usize);
            let p_end = partner_text.line_to_char(partner_range.end as usize);
            let c_start = curr_text.line_to_char(curr_range.start as usize);
            let c_end = curr_text.line_to_char(curr_range.end as usize);
            let text: Tendril = curr_text.slice(c_start..c_end).chunks().collect();
            (p_start, p_end, (!text.is_empty()).then_some(text))
        }),
    );
    (transaction, changes)
}

/// A per-line column range that should be highlighted for intra-line changes.
#[derive(Debug, Clone, PartialEq)]
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

/// Compute word-level intra-line diff for a single hunk.
/// Returns per-line column ranges for each side indicating which words (or
/// non-word chars) changed. Whole alphanumeric+underscore runs are treated as
/// one token, so only the changed words are highlighted rather than individual chars.
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

    // Precompute char-start of each word token so token indices can be mapped back
    // to char-column ranges after the Myers diff produces token-level hunks.
    let tok_starts_a = word_token_char_starts(&text_a);
    let tok_starts_b = word_token_char_starts(&text_b);
    let total_chars_a = text_a.chars().count();
    let total_chars_b = text_b.chars().count();

    let input = InternedInput::new(WordSlice(&text_a), WordSlice(&text_b));
    let diff = Diff::compute(Algorithm::Myers, &input);

    // Convert a token index into a char offset within the hunk text.
    // `tok` == number of tokens means "end of text".
    let tok_to_char = |starts: &[usize], total: usize, tok: u32| -> usize {
        *starts.get(tok as usize).unwrap_or(&total)
    };

    let char_to_line_col = |base_char: usize, char_offset: usize, rope: &Rope| -> (usize, usize) {
        let char_idx = base_char + char_offset;
        let line = rope.char_to_line(char_idx);
        let line_start = rope.line_to_char(line);
        (line, char_idx - line_start)
    };

    let mut changes_a = Vec::new();
    let mut changes_b = Vec::new();

    for tok_hunk in diff.hunks() {
        if !tok_hunk.before.is_empty() {
            let sc = tok_to_char(&tok_starts_a, total_chars_a, tok_hunk.before.start);
            let ec = tok_to_char(&tok_starts_a, total_chars_a, tok_hunk.before.end);
            let (start_line, start_col) = char_to_line_col(a_start, sc, rope_a);
            let (end_line, end_col) = char_to_line_col(a_start, ec, rope_a);
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
        if !tok_hunk.after.is_empty() {
            let sc = tok_to_char(&tok_starts_b, total_chars_b, tok_hunk.after.start);
            let ec = tok_to_char(&tok_starts_b, total_chars_b, tok_hunk.after.end);
            let (start_line, start_col) = char_to_line_col(b_start, sc, rope_b);
            let (end_line, end_col) = char_to_line_col(b_start, ec, rope_b);
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

/// Doc-line at which a hunk should emit its filler lines.
///
/// For non-empty `my_range` the trigger is the hunk's last line so the filler
/// renders below the hunk on this side. For an empty `my_range` (the partner
/// has lines but we don't), the trigger is the line just before the insertion
/// point so the filler renders in the gap.
fn trigger_for(my_range: &Range<u32>) -> usize {
    if my_range.is_empty() {
        my_range.start.saturating_sub(1) as usize
    } else {
        (my_range.end - 1) as usize
    }
}

/// Visual height of `range` in `rope` under `text_fmt`. Counts wrapped rows,
/// not just doc lines, so a single long line that soft-wraps to four rows
/// reports four. Empty ranges report zero.
pub fn visual_height(rope: &Rope, range: &Range<u32>, text_fmt: &TextFormat) -> u32 {
    if range.is_empty() {
        return 0;
    }
    let start_char = rope.line_to_char(range.start as usize);
    let end_char = rope.line_to_char(range.end as usize);
    let (pos, _) = visual_offset_from_block(
        rope.slice(..),
        start_char,
        end_char,
        text_fmt,
        &TextAnnotations::default(),
    );
    pos.row as u32
}

/// Per-hunk filler counts on `my_side`, in visual rows.
///
/// Each entry is `max(0, partner_visual_height - my_visual_height)`. Visual
/// heights are computed with each side's own `TextFormat`, so wrap widths
/// and tab settings can differ between panes. The result is parallel to
/// `hunks`.
pub fn compute_visual_fillers(
    hunks: &[Hunk],
    my_side: DiffSide,
    my_rope: &Rope,
    partner_rope: &Rope,
    my_text_fmt: &TextFormat,
    partner_text_fmt: &TextFormat,
) -> Vec<u32> {
    hunks
        .iter()
        .map(|hunk| {
            let (my_range, other_range) = match my_side {
                DiffSide::A => (&hunk.before, &hunk.after),
                DiffSide::B => (&hunk.after, &hunk.before),
            };
            let my_height = visual_height(my_rope, my_range, my_text_fmt);
            let partner_height = visual_height(partner_rope, other_range, partner_text_fmt);
            partner_height.saturating_sub(my_height)
        })
        .collect()
}

/// `LineAnnotation` that inserts virtual filler lines to keep two
/// side-by-side diff panes visually aligned at hunk boundaries.
///
/// Each side of the diff has its own `DiffAlignment`. The filler count for
/// each hunk is precomputed (usually by [`compute_visual_fillers`]) and
/// passed in at construction; `filler_lines_at` looks it up as the renderer
/// walks doc lines.
pub struct DiffAlignment {
    hunks: Arc<Vec<Hunk>>,
    side: DiffSide,
    /// Per-hunk filler row counts on this side, parallel to `hunks`.
    fillers: Arc<Vec<u32>>,
    cursor: usize,
}

impl DiffAlignment {
    pub fn new(hunks: Arc<Vec<Hunk>>, side: DiffSide, fillers: Arc<Vec<u32>>) -> Self {
        debug_assert_eq!(
            hunks.len(),
            fillers.len(),
            "fillers must be parallel to hunks",
        );
        Self {
            hunks,
            side,
            fillers,
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

    /// How many filler rows to insert after `doc_line`. Returns 0 if no
    /// filler is needed at this line.
    ///
    /// Multiple hunks can share a trigger position. A non-empty hunk ending
    /// at line X and a pure insertion at `[X..X)` both trigger at `X - 1`,
    /// for example. We accumulate fillers from every hunk at the current
    /// trigger in a single call. Otherwise the cursor advances past them on
    /// the next call and their fillers go missing.
    pub fn filler_lines_at(&mut self, doc_line: usize) -> usize {
        // Advance cursor past hunks that ended before this line.
        while self.cursor < self.hunks.len() {
            let (my_range, _) = self.ranges_for(&self.hunks[self.cursor]);
            if trigger_for(&my_range) < doc_line {
                self.cursor += 1;
            } else {
                break;
            }
        }

        // Sum precomputed fillers from every hunk whose trigger matches doc_line.
        let mut total = 0usize;
        while self.cursor < self.hunks.len() {
            let (my_range, _) = self.ranges_for(&self.hunks[self.cursor]);
            if trigger_for(&my_range) != doc_line {
                break;
            }
            total += self.fillers[self.cursor] as usize;
            self.cursor += 1;
        }
        total
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

    /// Build a DiffAlignment whose fillers come from doc-line differences,
    /// matching the original (pre-soft-wrap-aware) behavior. Render code uses
    /// compute_visual_fillers instead, which is exercised in helix-term tests.
    fn alignment_with_doc_line_fillers(hunks: Arc<Vec<Hunk>>, side: DiffSide) -> DiffAlignment {
        let fillers: Vec<u32> = hunks
            .iter()
            .map(|h| {
                let (my, other) = match side {
                    DiffSide::A => (&h.before, &h.after),
                    DiffSide::B => (&h.after, &h.before),
                };
                (other.end - other.start).saturating_sub(my.end - my.start)
            })
            .collect();
        DiffAlignment::new(hunks, side, Arc::new(fillers))
    }

    #[test]
    fn alignment_identical_files_no_fillers() {
        let hunks = make_hunks(&[]);
        let mut align = alignment_with_doc_line_fillers(hunks, DiffSide::A);

        for line in 0..5 {
            assert_eq!(align.filler_lines_at(line), 0);
        }
    }

    #[test]
    fn alignment_addition_pads_side_a() {
        // before: 5 lines, after: 8 lines (3 added at lines 2..5 on after)
        // Hunk: before 2..2 (pure insertion), after 2..5
        let hunks = make_hunks(&[(2..2, 2..5)]);
        let mut align = alignment_with_doc_line_fillers(hunks, DiffSide::A);

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
        let mut align = alignment_with_doc_line_fillers(hunks, DiffSide::B);

        // Side B has 3 lines, other side has 0. No filler needed.
        for line in 0..8 {
            assert_eq!(align.filler_lines_at(line), 0);
        }
    }

    #[test]
    fn alignment_deletion_pads_side_b() {
        // before has 3 lines (1..4), after has 0 lines (1..1)
        let hunks = make_hunks(&[(1..4, 1..1)]);
        let mut align = alignment_with_doc_line_fillers(hunks, DiffSide::B);

        // Side B (after) has 0 lines, other has 3. Trigger at 1-1=0
        assert_eq!(align.filler_lines_at(0), 3);
        assert_eq!(align.filler_lines_at(1), 0);
    }

    #[test]
    fn alignment_modification_pads_shorter_side() {
        // before: 2 lines (1..3), after: 5 lines (1..6)
        let hunks = make_hunks(&[(1..3, 1..6)]);
        let mut align_a = alignment_with_doc_line_fillers(hunks.clone(), DiffSide::A);

        // Side A has 2 lines, other has 5. Need 3 fillers after last line (doc_line 2)
        assert_eq!(align_a.filler_lines_at(0), 0);
        assert_eq!(align_a.filler_lines_at(1), 0);
        assert_eq!(align_a.filler_lines_at(2), 3);
        assert_eq!(align_a.filler_lines_at(3), 0);

        // Side B has 5 lines, other has 2. No filler needed.
        let mut align_b = alignment_with_doc_line_fillers(hunks, DiffSide::B);
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
        let mut align = alignment_with_doc_line_fillers(hunks, DiffSide::A);

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
        let mut align_a = alignment_with_doc_line_fillers(hunks.clone(), DiffSide::A);
        let mut align_b = alignment_with_doc_line_fillers(hunks, DiffSide::B);

        for line in 0..8 {
            assert_eq!(align_a.filler_lines_at(line), 0);
            assert_eq!(align_b.filler_lines_at(line), 0);
        }
    }

    // --- visual_height / compute_visual_fillers tests (soft-wrap awareness) ---

    /// Build a TextFormat that wraps at `viewport_width` columns.
    fn wrap_text_fmt(viewport_width: u16) -> TextFormat {
        TextFormat {
            soft_wrap: true,
            tab_width: 4,
            max_wrap: 1,
            max_indent_retain: 0,
            wrap_indicator: "".into(),
            wrap_indicator_highlight: None,
            viewport_width,
            soft_wrap_at_text_width: false,
        }
    }

    #[test]
    fn visual_height_no_wrap_matches_doc_line_count() {
        // 3 lines, all short. With wide viewport no wrap kicks in.
        let rope = Rope::from("aaa\nbbb\nccc\n");
        let fmt = wrap_text_fmt(80);
        assert_eq!(visual_height(&rope, &(0..3), &fmt), 3);
        assert_eq!(visual_height(&rope, &(1..3), &fmt), 2);
    }

    #[test]
    fn visual_height_counts_wrapped_rows() {
        // One long line, narrow viewport.
        let rope = Rope::from("aaaaaaaaaaaaaaaaaaaa\nbbb\n");
        let fmt = wrap_text_fmt(5);
        // First line (20 chars) wraps at width 5: at least 4 visual rows.
        // Second line (3 chars) takes 1 row.
        let h = visual_height(&rope, &(0..2), &fmt);
        assert!(
            h > 2,
            "wrapped first line should produce more than 2 rows, got {h}"
        );
    }

    #[test]
    fn visual_height_empty_range_is_zero() {
        let rope = Rope::from("aaa\nbbb\n");
        let fmt = wrap_text_fmt(80);
        assert_eq!(visual_height(&rope, &(2..2), &fmt), 0);
    }

    #[test]
    fn compute_visual_fillers_matches_doc_line_diff_when_no_wrap() {
        // Without wrap, visual fillers should equal what the old doc-line
        // subtraction produced. This is the regression guarantee.
        let hunks = make_hunks(&[(5..7, 5..10), (12..15, 15..15)]);
        let rope_a = Rope::from("a\n".repeat(20));
        let rope_b = Rope::from("b\n".repeat(20));
        let fmt = wrap_text_fmt(80);

        let fillers_a = compute_visual_fillers(&hunks, DiffSide::A, &rope_a, &rope_b, &fmt, &fmt);
        // Hunk 0: A has 2 lines, B has 5 -> A needs 3 fillers.
        // Hunk 1: A has 3 lines, B has 0 -> A needs 0 fillers (saturating).
        assert_eq!(fillers_a, vec![3, 0]);

        let fillers_b = compute_visual_fillers(&hunks, DiffSide::B, &rope_b, &rope_a, &fmt, &fmt);
        assert_eq!(fillers_b, vec![0, 3]);
    }

    #[test]
    fn compute_visual_fillers_inflates_when_partner_wraps() {
        // Side A has 1 short line, side B has 1 very long line that wraps.
        // From A's perspective, A's range visually consumes 1 row but B's range
        // consumes several. A needs extra fillers to keep aligned.
        let hunks = make_hunks(&[(0..1, 0..1)]);
        let rope_a = Rope::from("short\n");
        let rope_b = Rope::from("aaaaaaaaaaaaaaaaaaaaaaaaa\n"); // 25 chars
        let fmt = wrap_text_fmt(5);

        let fillers_a = compute_visual_fillers(&hunks, DiffSide::A, &rope_a, &rope_b, &fmt, &fmt);
        // Doc-line subtraction would say 0 (both ranges are 1 line). With wrap
        // awareness it must be > 0 because B's content wraps.
        assert!(
            fillers_a[0] > 0,
            "wrapped partner line should require fillers on the unwrapped side, got {}",
            fillers_a[0]
        );
    }

    // --- alignment-invariant tests (issue C: drift on archseer-shaped diffs) ---

    /// Sum filler counts produced for `total_lines` consecutive doc lines.
    fn total_fillers(hunks: Arc<Vec<Hunk>>, side: DiffSide, total_lines: u32) -> u32 {
        let mut align = alignment_with_doc_line_fillers(hunks, side);
        let mut sum = 0u32;
        for line in 0..total_lines {
            sum += align.filler_lines_at(line as usize) as u32;
        }
        sum
    }

    /// `len_a + fillers_a == len_b + fillers_b` must hold for alignment to work.
    fn assert_aligned(hunks: &[(Range<u32>, Range<u32>)], len_a: u32, len_b: u32) {
        let arc = make_hunks(hunks);
        let fa = total_fillers(Arc::clone(&arc), DiffSide::A, len_a);
        let fb = total_fillers(arc, DiffSide::B, len_b);
        assert_eq!(
            len_a + fa,
            len_b + fb,
            "alignment broken: A has {len_a} lines + {fa} fillers, B has {len_b} lines + {fb} fillers",
        );
    }

    #[test]
    fn alignment_holds_with_three_replace_hunks() {
        // Three pure replacements at increasing positions; symmetric sizes -> no fillers needed.
        assert_aligned(&[(2..3, 2..3), (10..11, 10..11), (20..21, 20..21)], 30, 30);
    }

    #[test]
    fn alignment_holds_with_three_pure_deletions() {
        // archseer-like: deletions only. A has 30 lines, B has 25 (lost 5 across 3 hunks).
        assert_aligned(&[(3..5, 3..3), (10..12, 8..8), (20..21, 16..16)], 30, 25);
    }

    #[test]
    fn alignment_holds_with_mixed_asymmetric_hunks() {
        // Mix of replace, pure-insert, pure-delete at increasing positions.
        // A: 30 lines.
        // Hunk 1 at A[5..7) -> B[5..10): replace 2 with 5  -> B gains 3
        // Hunk 2 at A[12..15) -> B[15..15): delete 3 lines -> B loses 3
        // Hunk 3 at A[22..22) -> B[22..27): pure insert 5  -> B gains 5
        // Net: B has 30 + 3 - 3 + 5 = 35 lines.
        assert_aligned(&[(5..7, 5..10), (12..15, 15..15), (22..22, 22..27)], 30, 35);
    }

    #[test]
    fn alignment_holds_when_replace_is_adjacent_to_pure_insert() {
        // The shape that produces colliding triggers on side A:
        // A: replace at [10..15) and immediately a pure insert at [15..15).
        // Both empty's trigger = start-1 = 14, AND the replace's trigger = end-1 = 14.
        // Side A's filler_lines_at(14) must emit fillers for BOTH hunks.
        assert_aligned(
            &[(10..15, 10..15), (15..15, 15..18)],
            // A: 15 lines through hunk 1 + 15 + 0 (hunk 2 empty on A) + ... = arbitrary; use 30.
            30,
            // B: hunk 1 same length, hunk 2 inserts 3. So 30 + 3 = 33.
            33,
        );
    }

    #[test]
    fn alignment_holds_when_pure_insert_is_adjacent_to_replace() {
        // Symmetric flavor on side B: a pure-insert on A immediately before a replace.
        assert_aligned(
            &[(10..10, 10..13), (10..15, 13..18)],
            30,
            // A inserts 3 (B gains 3) + replace same length = 33.
            33,
        );
    }

    #[test]
    fn alignment_holds_with_consecutive_deletions_on_one_side() {
        // Two pure deletions on side B (same as deletions on A's perspective: empty B ranges).
        // The B-side empty ranges have triggers at start-1; consecutive deletions can
        // share the same B-side trigger when their A-side spans differ but B-side
        // accumulator doesn't advance.
        assert_aligned(
            &[(5..7, 5..5), (10..13, 5..5)],
            30,
            // B loses (2 + 3) = 5 lines.
            25,
        );
    }

    // --- paired_side gating tests (D's cursor-sync predicate) ---

    #[test]
    fn paired_side_returns_source_side_when_views_are_partners() {
        let (view_a, view_b) = make_view_ids();
        let session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));
        let sessions = vec![session];

        // Source on side A, partner on side B: returns A.
        assert_eq!(paired_side(&sessions, view_a, view_b), Some(DiffSide::A));
        // Reverse direction also resolves.
        assert_eq!(paired_side(&sessions, view_b, view_a), Some(DiffSide::B));
    }

    #[test]
    fn paired_side_returns_none_when_dest_is_not_partner() {
        // AB4: two sessions exist, but source and dest are in different ones.
        let mut sm: slotmap::SlotMap<ViewId, ()> = slotmap::SlotMap::with_key();
        let view_a = sm.insert(());
        let view_b = sm.insert(());
        let view_c = sm.insert(());
        let view_d = sm.insert(());

        let sessions = vec![
            DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2)),
            DiffSession::new(view_c, view_d, make_doc_id(3), make_doc_id(4)),
        ];

        // view_a's partner is view_b, not view_c. Cross-session sync is denied.
        assert_eq!(paired_side(&sessions, view_a, view_c), None);
        assert_eq!(paired_side(&sessions, view_a, view_d), None);
        assert_eq!(paired_side(&sessions, view_c, view_a), None);
    }

    #[test]
    fn paired_side_returns_none_when_source_is_not_in_any_session() {
        // Source view exists but is not a member of any session. No write.
        // All three IDs come from the same slotmap so they really are
        // distinct: keys from different slotmaps can collide on the integer.
        let mut sm: slotmap::SlotMap<ViewId, ()> = slotmap::SlotMap::with_key();
        let view_a = sm.insert(());
        let view_b = sm.insert(());
        let outsider = sm.insert(());

        let sessions = vec![DiffSession::new(
            view_a,
            view_b,
            make_doc_id(1),
            make_doc_id(2),
        )];
        assert_eq!(paired_side(&sessions, outsider, view_a), None);
        assert_eq!(paired_side(&sessions, outsider, view_b), None);
    }

    #[test]
    fn paired_side_returns_none_when_no_sessions_exist() {
        let (view_a, view_b) = make_view_ids();
        let sessions: Vec<DiffSession> = vec![];
        assert_eq!(paired_side(&sessions, view_a, view_b), None);
    }

    #[test]
    fn paired_side_returns_none_when_source_equals_dest() {
        // Defensive: a focus event from a view to itself shouldn't sync.
        // partner_view returns the other view, never self, so source==dest
        // always falls into the "not partner" branch.
        let (view_a, view_b) = make_view_ids();
        let sessions = vec![DiffSession::new(
            view_a,
            view_b,
            make_doc_id(1),
            make_doc_id(2),
        )];
        assert_eq!(paired_side(&sessions, view_a, view_a), None);
        assert_eq!(paired_side(&sessions, view_b, view_b), None);
    }

    // --- map_line / map_to_real_line tests ---

    fn session_with_hunks(pairs: &[(Range<u32>, Range<u32>)]) -> DiffSession {
        let mut session = make_session();
        session.hunks = make_hunks(pairs);
        session
    }

    #[test]
    fn map_line_identical_files_is_identity() {
        // M1: With no hunks the mapping is 1:1 in both directions.
        let session = session_with_hunks(&[]);
        for line in [0u32, 5, 100] {
            assert_eq!(session.map_line(DiffSide::A, line), MappedLine::Real(line));
            assert_eq!(session.map_line(DiffSide::B, line), MappedLine::Real(line));
        }
    }

    #[test]
    fn map_line_pure_insert_on_partner_shifts_lines_after_hunk() {
        // M2: side A has no lines for the hunk, side B inserts 3 lines at row 5.
        // Lines on A before row 5 are unchanged on B; lines at/after row 5 shift +3.
        let session = session_with_hunks(&[(5..5, 5..8)]);

        // Before the insert: 1:1.
        assert_eq!(session.map_line(DiffSide::A, 0), MappedLine::Real(0));
        assert_eq!(session.map_line(DiffSide::A, 4), MappedLine::Real(4));

        // After the insert: shift by other_len - my_len = 3.
        assert_eq!(session.map_line(DiffSide::A, 5), MappedLine::Real(8));
        assert_eq!(session.map_line(DiffSide::A, 10), MappedLine::Real(13));
    }

    #[test]
    fn map_line_pure_delete_on_partner_yields_filler() {
        // M3: side A has 2 lines (3..5) that side B doesn't have. From A's perspective
        // those lines have no real partner row, so map_line returns Filler.
        let session = session_with_hunks(&[(3..5, 3..3)]);

        // Before the delete: 1:1.
        assert_eq!(session.map_line(DiffSide::A, 0), MappedLine::Real(0));
        assert_eq!(session.map_line(DiffSide::A, 2), MappedLine::Real(2));

        // Inside the deleted range (no partner row): Filler anchored at start - 1.
        assert_eq!(
            session.map_line(DiffSide::A, 3),
            MappedLine::Filler { after: 2 }
        );
        assert_eq!(
            session.map_line(DiffSide::A, 4),
            MappedLine::Filler { after: 2 }
        );

        // After the delete: shift by -2 (other_len 0 - my_len 2).
        assert_eq!(session.map_line(DiffSide::A, 5), MappedLine::Real(3));
        assert_eq!(session.map_line(DiffSide::A, 10), MappedLine::Real(8));
    }

    #[test]
    fn map_line_pure_delete_filler_anchors_at_zero_when_at_file_start() {
        // Edge: deletion at line 0; saturating_sub keeps `after` at 0.
        let session = session_with_hunks(&[(0..2, 0..0)]);
        assert_eq!(
            session.map_line(DiffSide::A, 0),
            MappedLine::Filler { after: 0 }
        );
        assert_eq!(
            session.map_line(DiffSide::A, 1),
            MappedLine::Filler { after: 0 }
        );
    }

    #[test]
    fn map_line_multiple_delete_only_hunks_accumulate_offset() {
        // M4: archseer's repro shape - several deletion-only hunks at increasing
        // positions. After each, the running offset shrinks by the deletion size.
        let session = session_with_hunks(&[
            (3..5, 3..3),     // delete 2 lines on B
            (10..12, 8..8),   // delete 2 more
            (20..21, 16..16), // delete 1 more
        ]);

        // After the third hunk: cumulative offset is -5 (lost 5 lines on B).
        assert_eq!(session.map_line(DiffSide::A, 30), MappedLine::Real(25));

        // Between hunks the offset reflects only the previous deletions.
        assert_eq!(session.map_line(DiffSide::A, 6), MappedLine::Real(4));
        assert_eq!(session.map_line(DiffSide::A, 13), MappedLine::Real(9));
    }

    #[test]
    fn map_line_asymmetric_replace_clamps_within_hunk_offset() {
        // M5: side A has 3 lines (5..8), side B has 5 lines (5..10).
        // Within the hunk, A row 5 -> B row 5, A row 6 -> B row 6, A row 7 -> B row 7
        // (clamped to other_range.end - 1 = 9, but 7 is below that anyway).
        let session = session_with_hunks(&[(5..8, 5..10)]);
        assert_eq!(session.map_line(DiffSide::A, 5), MappedLine::Real(5));
        assert_eq!(session.map_line(DiffSide::A, 6), MappedLine::Real(6));
        assert_eq!(session.map_line(DiffSide::A, 7), MappedLine::Real(7));

        // Reverse: B has more lines; B row 5..10 maps onto A row 5..8 with clamp.
        assert_eq!(session.map_line(DiffSide::B, 5), MappedLine::Real(5));
        assert_eq!(session.map_line(DiffSide::B, 6), MappedLine::Real(6));
        assert_eq!(session.map_line(DiffSide::B, 7), MappedLine::Real(7));
        assert_eq!(session.map_line(DiffSide::B, 8), MappedLine::Real(7)); // clamped
        assert_eq!(session.map_line(DiffSide::B, 9), MappedLine::Real(7)); // clamped
    }

    #[test]
    fn map_to_real_line_clamps_to_partner_line_count() {
        // M6: a row that maps past the partner doc's last line is clamped, no panic.
        let session = session_with_hunks(&[]);
        // Partner has 3 lines (indices 0..2). Asking for line 100 returns last line.
        assert_eq!(session.map_to_real_line(DiffSide::A, 100, 3), 2);

        // Even with partner_line_count = 0, we return 0 instead of underflowing.
        assert_eq!(session.map_to_real_line(DiffSide::A, 5, 0), 0);

        // Filler positions also clamp.
        let session = session_with_hunks(&[(3..5, 3..3)]);
        assert_eq!(session.map_to_real_line(DiffSide::A, 3, 1), 0);
    }

    #[test]
    fn map_line_does_not_panic_on_extreme_inputs() {
        // M7: u32::MAX as an input must not panic.
        let session = session_with_hunks(&[(3..5, 3..3)]);
        let result = session.map_line(DiffSide::A, u32::MAX);
        // Past all hunks, offset is -2; result is u32::MAX - 2.
        assert_eq!(result, MappedLine::Real(u32::MAX - 2));

        // map_to_real_line clamps to the partner's last line.
        assert_eq!(session.map_to_real_line(DiffSide::A, u32::MAX, 10), 9);
    }

    // --- find_next_hunk / find_prev_hunk tests ---

    #[test]
    fn find_next_hunk_advances_when_cursor_inside_hunk() {
        // Hunk spans lines 5..8 (lines 5, 6, 7) on side A; another at 12..14.
        // Cursor on line 6 (middle of first hunk) should advance past it.
        let hunks = make_hunks(&[(5..8, 5..8), (12..14, 12..14)]);
        let next = find_next_hunk(&hunks, DiffSide::A, 6);
        assert_eq!(next, Some(1));
    }

    #[test]
    fn find_next_hunk_advances_when_cursor_at_hunk_start() {
        // Cursor exactly on the first line of a hunk: still considered "inside",
        // so the predicate should advance to the next hunk.
        let hunks = make_hunks(&[(5..8, 5..8), (12..14, 12..14)]);
        let next = find_next_hunk(&hunks, DiffSide::A, 5);
        assert_eq!(next, Some(1));
    }

    #[test]
    fn find_next_hunk_returns_first_hunk_when_cursor_before_all() {
        let hunks = make_hunks(&[(5..8, 5..8), (12..14, 12..14)]);
        let next = find_next_hunk(&hunks, DiffSide::A, 0);
        assert_eq!(next, Some(0));
    }

    #[test]
    fn find_next_hunk_returns_none_when_cursor_past_all_hunks() {
        let hunks = make_hunks(&[(5..8, 5..8), (12..14, 12..14)]);
        let next = find_next_hunk(&hunks, DiffSide::A, 20);
        assert_eq!(next, None);
    }

    #[test]
    fn find_next_hunk_handles_empty_range_deletion() {
        // Deletion on side A: empty range at start=5. Cursor at 4 should match,
        // cursor at 5 should NOT match (already at/past the filler position).
        let hunks = make_hunks(&[(5..5, 5..7)]);
        assert_eq!(find_next_hunk(&hunks, DiffSide::A, 4), Some(0));
        assert_eq!(find_next_hunk(&hunks, DiffSide::A, 5), None);
    }

    #[test]
    fn find_prev_hunk_retreats_when_cursor_inside_hunk() {
        // Cursor on line 6 (middle of second hunk 5..8); should retreat to first hunk.
        let hunks = make_hunks(&[(0..2, 0..2), (5..8, 5..8)]);
        let prev = find_prev_hunk(&hunks, DiffSide::A, 6);
        assert_eq!(prev, Some(0));
    }

    #[test]
    fn find_prev_hunk_retreats_when_cursor_at_hunk_end_minus_one() {
        // Cursor on the last line of a hunk (5..8 means lines 5, 6, 7). Cursor on 7
        // is still inside; should retreat.
        let hunks = make_hunks(&[(0..2, 0..2), (5..8, 5..8)]);
        let prev = find_prev_hunk(&hunks, DiffSide::A, 7);
        assert_eq!(prev, Some(0));
    }

    #[test]
    fn find_prev_hunk_includes_hunk_just_above_cursor() {
        // Cursor at line 8: a hunk ending at 8 (exclusive) covers lines 5..7.
        // The hunk is fully above the cursor, so it should match.
        let hunks = make_hunks(&[(5..8, 5..8)]);
        let prev = find_prev_hunk(&hunks, DiffSide::A, 8);
        assert_eq!(prev, Some(0));
    }

    #[test]
    fn find_prev_hunk_returns_none_when_cursor_before_all() {
        let hunks = make_hunks(&[(5..8, 5..8)]);
        let prev = find_prev_hunk(&hunks, DiffSide::A, 0);
        assert_eq!(prev, None);
    }

    #[test]
    fn find_prev_hunk_handles_empty_range_deletion() {
        // Deletion on side A at line 5 (empty range). Cursor at 5 should retreat past it
        // (we're at or past the filler), cursor at 6 should match (filler is above us).
        let hunks = make_hunks(&[(0..1, 0..1), (5..5, 5..7)]);
        assert_eq!(find_prev_hunk(&hunks, DiffSide::A, 5), Some(0));
        assert_eq!(find_prev_hunk(&hunks, DiffSide::A, 6), Some(1));
    }

    #[test]
    fn find_next_and_prev_use_correct_side_range() {
        // Hunk where before and after differ in placement:
        // before: lines 5..6, after: lines 10..11.
        let hunks = make_hunks(&[(5..6, 10..11)]);

        // Side A: cursor at 4 sees hunk at line 5.
        assert_eq!(find_next_hunk(&hunks, DiffSide::A, 4), Some(0));
        // Side B: same cursor (4) sees hunk at line 10.
        assert_eq!(find_next_hunk(&hunks, DiffSide::B, 4), Some(0));

        // Side A: cursor at 7 has hunk behind it at line 5.
        assert_eq!(find_prev_hunk(&hunks, DiffSide::A, 7), Some(0));
        // Side B: cursor at 7 has hunk ahead of it at line 10, none behind.
        assert_eq!(find_prev_hunk(&hunks, DiffSide::B, 7), None);
    }

    // --- build_get_transaction / build_put_transaction tests ---

    fn apply_tx(rope: &Rope, tx: Transaction) -> Rope {
        let mut result = rope.clone();
        tx.apply(&mut result);
        result
    }

    #[test]
    fn diffget_replaces_hunk_with_partner_content() {
        let mut session = make_session();
        let rope_a = Rope::from("line1\nold_content\nline3\n");
        let rope_b = Rope::from("line1\nnew_content\nline3\n");
        session.compute_hunks(&rope_a, &rope_b);

        let line_ranges = vec![(1usize, 1usize)];
        let (tx, changes) =
            build_get_transaction(session.hunks(), DiffSide::A, &line_ranges, &rope_a, &rope_b);

        assert_eq!(changes, 1);
        assert_eq!(apply_tx(&rope_a, tx), rope_b);
    }

    #[test]
    fn diffput_replaces_partner_hunk_with_current_content() {
        let mut session = make_session();
        let rope_a = Rope::from("line1\nold_content\nline3\n");
        let rope_b = Rope::from("line1\nnew_content\nline3\n");
        session.compute_hunks(&rope_a, &rope_b);

        let line_ranges = vec![(1usize, 1usize)];
        let (tx, changes) =
            build_put_transaction(session.hunks(), DiffSide::A, &line_ranges, &rope_a, &rope_b);

        assert_eq!(changes, 1);
        assert_eq!(apply_tx(&rope_b, tx), rope_a);
    }

    #[test]
    fn diffget_no_change_when_no_intersecting_hunks() {
        let mut session = make_session();
        let rope_a = Rope::from("aaa\nbbb\nccc\nddd\n");
        let rope_b = Rope::from("aaa\nBBB\nccc\nDDD\n");
        session.compute_hunks(&rope_a, &rope_b);

        let line_ranges = vec![(2usize, 2usize)];
        let (_, changes) =
            build_get_transaction(session.hunks(), DiffSide::A, &line_ranges, &rope_a, &rope_b);

        assert_eq!(changes, 0);
    }

    #[test]
    fn diffget_handles_pure_deletion_from_side_b() {
        // doc_a has line2 that doc_b is missing; hunk: before=1..2, after=1..1 (empty in B)
        let mut session = make_session();
        let rope_a = Rope::from("line1\nline2\nline3\n");
        let rope_b = Rope::from("line1\nline3\n");
        session.compute_hunks(&rope_a, &rope_b);

        // On side B, the deletion hunk has after.start=1 (between line1 and line3).
        // Cursor on line 1 (line3) in doc_b: after.start=1 <= end_line=1, so it matches.
        let line_ranges = vec![(1usize, 1usize)];
        let (tx, changes) =
            build_get_transaction(session.hunks(), DiffSide::B, &line_ranges, &rope_b, &rope_a);

        assert_eq!(changes, 1);
        // After diffget on side B: doc_b gains line2 back from doc_a.
        assert_eq!(apply_tx(&rope_b, tx), rope_a);
    }

    #[test]
    fn contains_doc_returns_true_for_session_members() {
        let (view_a, view_b) = make_view_ids();
        let doc_a = make_doc_id(1);
        let doc_b = make_doc_id(2);
        let session = DiffSession::new(view_a, view_b, doc_a, doc_b);

        assert!(session.contains_doc(doc_a));
        assert!(session.contains_doc(doc_b));
    }

    #[test]
    fn contains_doc_returns_false_for_non_members() {
        let (view_a, view_b) = make_view_ids();
        let doc_a = make_doc_id(1);
        let doc_b = make_doc_id(2);
        let unrelated = make_doc_id(3);
        let session = DiffSession::new(view_a, view_b, doc_a, doc_b);

        assert!(!session.contains_doc(unrelated));
    }

    #[test]
    fn intra_line_cache_is_populated_after_compute_hunks() {
        let (view_a, view_b) = make_view_ids();
        let mut session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));
        let rope_a = Rope::from("line1\nold\nline3\n");
        let rope_b = Rope::from("line1\nnew\nline3\n");

        session.compute_hunks(&rope_a, &rope_b);

        // One modified hunk; cache entry must be present.
        assert_eq!(session.hunks().len(), 1);
        assert!(session.intra_line_changes_for(0).is_some());
    }

    #[test]
    fn intra_line_cache_matches_free_function() {
        let (view_a, view_b) = make_view_ids();
        let mut session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));
        let rope_a = Rope::from("line1\nold\nline3\n");
        let rope_b = Rope::from("line1\nnew\nline3\n");

        session.compute_hunks(&rope_a, &rope_b);

        let hunk = &session.hunks()[0];
        let expected = intra_line_changes(&rope_a, &rope_b, hunk);
        let cached = session.intra_line_changes_for(0).unwrap();

        assert_eq!(cached.0, expected.0);
        assert_eq!(cached.1, expected.1);
    }

    #[test]
    fn intra_line_cache_is_empty_for_pure_addition() {
        let (view_a, view_b) = make_view_ids();
        let mut session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));
        let rope_a = Rope::from("line1\nline3\n");
        let rope_b = Rope::from("line1\nnew\nline3\n");

        session.compute_hunks(&rope_a, &rope_b);

        // Pure insertion: before is empty, no character diff to compute.
        assert_eq!(session.hunks().len(), 1);
        let cached = session.intra_line_changes_for(0).unwrap();
        assert!(cached.0.is_empty());
        assert!(cached.1.is_empty());
    }

    #[test]
    fn intra_line_cache_refreshes_on_update_if_changed() {
        let (view_a, view_b) = make_view_ids();
        let mut session = DiffSession::new(view_a, view_b, make_doc_id(1), make_doc_id(2));
        let rope_a = Rope::from("line1\nold\nline3\n");
        let rope_b_v1 = Rope::from("line1\nnew\nline3\n");
        let rope_b_v2 = Rope::from("line1\ncompletely different\nline3\n");

        session.compute_hunks(&rope_a, &rope_b_v1);
        let cached_v1 = session.intra_line_changes_for(0).unwrap().clone();

        // Simulate a document edit: version changes, triggering recomputation.
        session.update_if_changed(1, 1, &rope_a, &rope_b_v2);
        let cached_v2 = session.intra_line_changes_for(0).unwrap();

        assert_ne!(
            cached_v1.1, cached_v2.1,
            "cache must reflect updated content"
        );
    }

    #[test]
    fn intra_line_diff_uses_word_granularity() {
        // "hello world\n" vs "hello earth\n": only the word "world"/"earth" differs.
        // Word-level diff should highlight exactly col 6..11 on each side,
        // not multiple sub-word spans as character-level diff would produce.
        let rope_a = Rope::from("hello world\n");
        let rope_b = Rope::from("hello earth\n");
        let hunk = helix_vcs::Hunk {
            before: 0..1,
            after: 0..1,
        };
        let (changes_a, changes_b) = intra_line_changes(&rope_a, &rope_b, &hunk);

        assert_eq!(
            changes_a.len(),
            1,
            "expected one word-level change on A side"
        );
        assert_eq!(
            changes_b.len(),
            1,
            "expected one word-level change on B side"
        );

        assert_eq!(changes_a[0].doc_line, 0);
        assert_eq!(
            changes_a[0].col_start, 6,
            "change should start at 'w' of 'world'"
        );
        assert_eq!(
            changes_a[0].col_end, 11,
            "change should end after 'd' of 'world'"
        );

        assert_eq!(changes_b[0].doc_line, 0);
        assert_eq!(
            changes_b[0].col_start, 6,
            "change should start at 'e' of 'earth'"
        );
        assert_eq!(
            changes_b[0].col_end, 11,
            "change should end after 'h' of 'earth'"
        );
    }
}
