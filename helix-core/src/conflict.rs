//! Parsing and navigation of VCS merge conflict markers.
//!
//! Supports three conflict formats:
//!
//! **git 2-way:**
//! ```text
//! <<<<<<< current
//! ... current text ...
//! =======
//! ... incoming text ...
//! >>>>>>> incoming
//! ```
//!
//! **git diff3 (3-way):**
//! ```text
//! <<<<<<< current
//! ... current text ...
//! ||||||| base
//! ... base text ...
//! =======
//! ... incoming text ...
//! >>>>>>> incoming
//! ```
//!
//! **jj snapshot (N-way):**
//! ```text
//! <<<<<<< conflict
//! +++++++ side #1
//! ... side 1 text ...
//! ------- base
//! ... base text ...
//! +++++++ side #2
//! ... side 2 text ...
//! >>>>>>> conflict ends
//! ```
//!
//! All three formats are unified into [`ConflictRegion`] with a `Vec<Section>`.

use std::collections::HashMap;
use std::ops::Range;

use imara_diff::{Algorithm, Diff, InternedInput};

use crate::Rope;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Pair of removed/added word-diff ranges for a conflict refine pair.
type RefineDiffs = (Vec<Range<usize>>, Vec<Range<usize>>);

/// Per-conflict word-level diff cache entry, keyed by conflict start position.
#[derive(Debug, Clone, Default)]
pub struct ConflictRefineEntry {
    /// Current refine pair index (see [`ConflictRegion::refine_pair_indices`]).
    /// For a 3-section diff3 conflict: 0 = current↔base, 1 = base↔incoming, 2 = current↔incoming.
    /// For N-way jj conflicts the ordering prioritises each side vs base,
    /// then side–side comparisons; remaining base-involving pairs are excluded.
    pub pair: usize,
    /// Cached word-diff results for the current `pair`:
    /// `(removed_ranges, added_ranges)`.
    pub diffs: Option<RefineDiffs>,
}

/// Per-conflict word-diff refine state, keyed by [`ConflictRegion::start`].
///
/// Cleared on every edit; the active pair setting is preserved when the cursor
/// was inside a conflict before the edit.
pub type ConflictCache = HashMap<usize, ConflictRefineEntry>;

/// Whether a section holds one side of a conflict or the common base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    /// A `+++++++` side (jj format) or the current/incoming sides (git format).
    Side,
    /// A `-------` base (jj format) or the `|||||||` base (git diff3 format).
    Base,
}

/// One section within a conflict region.
///
/// For git format the first section's `marker_start` equals the `<<<<<<<` line;
/// for jj format every section starts with its own `+++++++` / `-------` marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub kind: SectionKind,
    /// Char index of the first character of this section's marker line.
    ///
    /// For the first side of a git-format conflict this is the `<<<<<<<` line.
    /// For jj-format sections this is the `+++++++` or `-------` line.
    /// For the last side of a git-format conflict this is the `=======` line.
    pub marker_start: usize,
    /// Char index of the first content character (the line *after* the marker).
    pub content_start: usize,
    /// Exclusive end of the content (= `marker_start` of next section, or `end`
    /// of the whole conflict for the final section).
    pub content_end: usize,
}

/// A single VCS merge conflict region found in a document.
///
/// All positions are **character indices** into the document rope.
/// `start` points to the first character of the `<<<<<<<` marker line.
/// `end` points to the character just past the last character of the `>>>>>>>`
/// line (i.e. after the trailing newline, if present).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRegion {
    /// Start of the `<<<<<<< ...` line.
    pub start: usize,
    /// Ordered list of conflict sections.  There is always at least one `Side`.
    pub sections: Vec<Section>,
    /// Exclusive end past the `>>>>>>> ...` line.
    pub end: usize,
}

impl ConflictRegion {
    /// Number of word-diff pairs available via refine.
    ///
    /// Only phase-1 (each side vs adjacent base) and phase-2 (side–side) pairs
    /// are counted; remaining pairs involving bases are never shown.
    pub fn num_refine_pairs(&self) -> usize {
        let n = self.sections.len();
        let mut count = 0;

        // Phase 1
        for i in 1..n {
            if i == 1 || self.sections[i - 1].kind == SectionKind::Base {
                count += 1;
            }
        }
        // Phase 2
        for skip in 1..n {
            for i in 0..(n - skip) {
                let (a, b) = (i, i + skip);
                if self.sections[a].kind == SectionKind::Side
                    && self.sections[b].kind == SectionKind::Side
                    && !((a == 0 || self.sections[a].kind == SectionKind::Base) && b == a + 1)
                {
                    count += 1;
                }
            }
        }
        count
    }

    /// Return the pair of section indices for refine `pair` index.
    ///
    /// Pairs are ordered so comparisons relevant to N-way conflicts come first:
    ///   1. Each Side compared against its adjacent Base — `(0,1)` first, then
    ///      all `(i,i+1)` where section `i` is a **Base**.
    ///   2. All Side–Side pairs (both sections are `Side`), with the left side
    ///      fixed as long as possible before advancing.
    ///
    /// For a standard git diff3 3-section conflict (Side,Base,Side) this still
    /// produces (0,1)=current↔base, (1,2)=base↔incoming, (0,2)=current↔incoming.
    /// Returns `None` if `pair >= num_refine_pairs()`.
    pub fn refine_pair_indices(&self, pair: usize) -> Option<(usize, usize)> {
        let n = self.sections.len();
        if pair >= self.num_refine_pairs() {
            return None;
        }
        let mut idx = 0;

        // Phase 1: (0,1) first, then all (i,i+1) where sections[i] is Base.
        for i in 1..n {
            let in_phase1 = i == 1 || self.sections[i - 1].kind == SectionKind::Base;
            if in_phase1 {
                if idx == pair {
                    return Some((i - 1, i));
                }
                idx += 1;
            }
        }

        // Phase 2: Side–Side pairs, left side fixed as long as possible.
        let side_indices: Vec<usize> = (0..n)
            .filter(|i| self.sections[*i].kind == SectionKind::Side)
            .collect();
        for a_idx in 0..side_indices.len() {
            for b_idx in (a_idx + 1)..side_indices.len() {
                let (a, b) = (side_indices[a_idx], side_indices[b_idx]);
                let already = (a == 0 || self.sections[a].kind == SectionKind::Base) && b == a + 1;
                if !already {
                    if idx == pair {
                        return Some((a, b));
                    }
                    idx += 1;
                }
            }
        }

        None
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse all conflict regions in `text`, in document order.
///
/// Supports git 2-way, git diff3 3-way, and jj snapshot N-way formats.
/// Also recognises "bare" conflicts — a `<<<<<<<` / `>>>>>>>` pair with no
/// inner markers — which arise after manually editing out unwanted sections.
/// Incomplete conflict blocks (no closing `>>>>>>>`) are silently ignored.
pub fn find_conflicts(text: &Rope) -> Vec<ConflictRegion> {
    /// A section whose `content_end` is not yet known.
    #[derive(Debug, Clone)]
    struct PartialSection {
        kind: SectionKind,
        marker_start: usize,
        content_start: usize,
    }

    impl PartialSection {
        fn finish(self, content_end: usize) -> Section {
            Section {
                kind: self.kind,
                marker_start: self.marker_start,
                content_start: self.content_start,
                content_end,
            }
        }
    }

    #[derive(Debug, Default)]
    enum State {
        #[default]
        Idle,
        /// Inside a conflict region (git or jj format).
        InConflict {
            start: usize,
            sections: Vec<Section>,
            current: PartialSection,
            format: ConflictFormat,
        },
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum ConflictFormat {
        Git,
        Jj,
    }

    let mut conflicts: Vec<ConflictRegion> = Vec::new();
    let mut state = State::Idle;
    let mut line_char_start: usize = 0;

    for line in text.lines() {
        let line_len = line.len_chars();
        let next_line_start = line_char_start + line_len;

        let is_marker = |c: char| -> bool {
            let mut chars = line.chars();
            (&mut chars).take(7).all(|ch| ch == c)
                && matches!(chars.next(), None | Some(' ') | Some('\r') | Some('\n'))
        };

        state = match std::mem::take(&mut state) {
            State::Idle => {
                if is_marker('<') {
                    State::InConflict {
                        start: line_char_start,
                        sections: Vec::new(),
                        current: PartialSection {
                            kind: SectionKind::Side,
                            marker_start: line_char_start,
                            content_start: next_line_start,
                        },
                        format: ConflictFormat::Git,
                    }
                } else {
                    State::Idle
                }
            }

            State::InConflict {
                start,
                mut sections,
                current,
                format,
            } => {
                if is_marker('|') {
                    sections.push(current.finish(line_char_start));
                    State::InConflict {
                        start,
                        sections,
                        current: PartialSection {
                            kind: SectionKind::Base,
                            marker_start: line_char_start,
                            content_start: next_line_start,
                        },
                        format,
                    }
                } else if is_marker('=') {
                    sections.push(current.finish(line_char_start));
                    State::InConflict {
                        start,
                        sections,
                        current: PartialSection {
                            kind: SectionKind::Side,
                            marker_start: line_char_start,
                            content_start: next_line_start,
                        },
                        format,
                    }
                } else if is_marker('>') {
                    let end = next_line_start;
                    sections.push(current.finish(line_char_start));
                    if !sections.is_empty() {
                        conflicts.push(ConflictRegion {
                            start,
                            sections,
                            end,
                        });
                    }
                    State::Idle
                } else if is_marker('<') {
                    State::InConflict {
                        start: line_char_start,
                        sections: Vec::new(),
                        current: PartialSection {
                            kind: SectionKind::Side,
                            marker_start: line_char_start,
                            content_start: next_line_start,
                        },
                        format: ConflictFormat::Git,
                    }
                } else if is_marker('+') || is_marker('-') {
                    if format == ConflictFormat::Git {
                        if current.content_start < line_char_start {
                            sections.push(current.finish(line_char_start));
                        }
                    } else {
                        sections.push(current.finish(line_char_start));
                    }
                    let kind = if is_marker('+') {
                        SectionKind::Side
                    } else {
                        SectionKind::Base
                    };
                    State::InConflict {
                        start,
                        sections,
                        current: PartialSection {
                            kind,
                            marker_start: line_char_start,
                            content_start: next_line_start,
                        },
                        format: ConflictFormat::Jj,
                    }
                } else {
                    State::InConflict {
                        start,
                        sections,
                        current,
                        format,
                    }
                }
            }
        };

        line_char_start = next_line_start;
    }

    conflicts
}

// ── Cursor position helpers ───────────────────────────────────────────────────

/// Return the index of the conflict in `conflicts` that contains `char_pos`,
/// or `None` if the position is not inside any conflict region.
pub fn conflict_at(conflicts: &[ConflictRegion], char_pos: usize) -> Option<usize> {
    conflicts
        .iter()
        .position(|c| char_pos >= c.start && char_pos < c.end)
}

/// Return the index of the next conflict after `char_pos` (exclusive).
pub fn next_conflict(conflicts: &[ConflictRegion], char_pos: usize) -> Option<usize> {
    conflicts.iter().position(|c| c.start > char_pos)
}

/// Return the index of the previous conflict before `char_pos` (exclusive).
pub fn prev_conflict(conflicts: &[ConflictRegion], char_pos: usize) -> Option<usize> {
    conflicts.iter().rposition(|c| c.start < char_pos)
}

/// Return which section index of `region` the character position `char_pos` is in,
/// or `None` if it is on a pure separator line (the `=======` line in git format)
/// or outside `region`.
///
/// Marker lines are attributed to their own section (the `<<<<<<<` line is part
/// of the first section, `+++++++`/`-------`/`|||||||` lines are part of their
/// section, `>>>>>>>` is part of the last section).  Only the `=======`
/// separator is a no-op since it doesn't logically belong to either side.
pub fn conflict_section_at(region: &ConflictRegion, text: &Rope, char_pos: usize) -> Option<usize> {
    // For git-format conflicts, the last section's marker is `=======`.
    // A cursor sitting on that marker line belongs to no section (no-op).
    // For jj-format, there is no `=======` — every marker line belongs to its section.
    let sep_line_end = git_sep_line_end(region, text);

    for (i, section) in region.sections.iter().enumerate() {
        let section_end = if i == region.sections.len() - 1 {
            region.end
        } else {
            region.sections[i + 1].marker_start
        };

        if char_pos >= section.marker_start && char_pos < section_end {
            // If this is the last section in git format, the marker line is `=======`
            // and should be a no-op zone.
            if i == region.sections.len() - 1 {
                if let Some(sep_end) = sep_line_end {
                    let sep_start = section.marker_start;
                    if char_pos >= sep_start && char_pos < sep_end {
                        return None; // on the ======= line
                    }
                }
            }
            return Some(i);
        }
    }

    // Shouldn't reach here if char_pos is inside the region.
    None
}

/// For a git-format conflict, return the exclusive end of the `=======` line.
/// Returns `None` for jj-format conflicts (which have no `=======`).
fn git_sep_line_end(region: &ConflictRegion, text: &Rope) -> Option<usize> {
    // In git format, the last section's marker is always `=======`.
    // In jj format, the last section's marker is `+++++++` or `-------`.
    let last = region.sections.last()?;
    // A `=======` line starts with exactly 7 `=` and nothing else (or a space/newline)
    let marker_char: char = text.char(last.marker_start);
    if marker_char == '=' {
        let sep_line = text.char_to_line(last.marker_start);
        Some(text.line_to_char(sep_line + 1))
    } else {
        None
    }
}

/// Returns the document line numbers of every conflict marker line in `text`:
/// the opening `<<<<<<<`, every section marker (`|||||||`, `=======`, `+++++++`,
/// `-------`), and the closing `>>>>>>>`.
///
/// The returned `Vec` is sorted and deduplicated.
pub fn conflict_marker_lines(text: &Rope) -> Vec<usize> {
    let mut lines: Vec<usize> = find_conflicts(text)
        .iter()
        .flat_map(|region| {
            // Section marker lines (includes the opening <<<<<<< via sections[0].marker_start).
            let section_lines = region
                .sections
                .iter()
                .map(|s| text.char_to_line(s.marker_start));
            // Closing >>>>>>> line — `end` is exclusive and points past the trailing
            // newline, so subtract 1 to land somewhere on the last line.
            let end_line = std::iter::once(text.char_to_line(region.end.saturating_sub(1)));
            section_lines.chain(end_line)
        })
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}

// ── Content helpers ───────────────────────────────────────────────────────────

/// Return the char range of the first `Side` section ("current" change).
pub fn current_content(region: &ConflictRegion) -> (usize, usize) {
    let s = region
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::Side)
        .expect("conflict must have at least one Side section");
    (s.content_start, s.content_end)
}

/// Return the char range of the last `Side` section ("incoming" change).
pub fn incoming_content(region: &ConflictRegion) -> (usize, usize) {
    let s = region
        .sections
        .iter()
        .rev()
        .find(|s| s.kind == SectionKind::Side)
        .expect("conflict must have at least one Side section");
    (s.content_start, s.content_end)
}

/// Return the char range of the first `Base` section, or `None` if absent.
pub fn base_content(region: &ConflictRegion) -> Option<(usize, usize)> {
    let s = region
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::Base)?;
    Some((s.content_start, s.content_end))
}

/// Return the content of all `Side` sections concatenated (for accept-all).
pub fn all_sides_content(text: &Rope, region: &ConflictRegion) -> String {
    region
        .sections
        .iter()
        .filter(|s| s.kind == SectionKind::Side)
        .map(|s| text.slice(s.content_start..s.content_end).to_string())
        .collect()
}

// ── Refine (word-level diff) ──────────────────────────────────────────────────

/// Returns the content ranges `(left, right)` for refine pair `pair`.
///
/// Pairs are enumerated in (i, j) order with i < j over all section indices.
/// Returns `None` if `pair >= region.num_refine_pairs()`.
pub fn conflict_pair_sections(
    region: &ConflictRegion,
    pair: usize,
) -> Option<((usize, usize), (usize, usize))> {
    let (i, j) = region.refine_pair_indices(pair)?;
    // When one section is Base, put it on the left (before) so that
    // removed=red highlights what the base had, added=green highlights
    // what the Side added on top.  Side↔Side pairs keep their order.
    let (i, j) = match (region.sections[i].kind, region.sections[j].kind) {
        (SectionKind::Side, SectionKind::Base) => (j, i),
        _ => (i, j),
    };
    let left = (
        region.sections[i].content_start,
        region.sections[i].content_end,
    );
    let right = (
        region.sections[j].content_start,
        region.sections[j].content_end,
    );
    Some((left, right))
}

/// Look up the active refine pair for a conflict.
///
/// Defaults to `0` if no entry is present.
pub fn conflict_refine_pair(state: &HashMap<usize, usize>, region: &ConflictRegion) -> usize {
    let pair = state.get(&region.start).copied().unwrap_or(0);
    let max = region.num_refine_pairs().saturating_sub(1);
    pair.min(max)
}

// ── Word-level diff ───────────────────────────────────────────────────────────

/// A word-token source backed by a contiguous substring of a `Rope`.
///
/// Tokens are non-whitespace runs of characters.
/// To avoid double allocation, token strings are built on-demand during
/// `tokenize()` rather than pre-collecting them. This amortizes the cost
/// across the single `InternedInput::new()` call.
struct WordTokens<'a> {
    text: &'a Rope,
    positions: Vec<(usize, usize)>,
}

impl<'a> WordTokens<'a> {
    fn new(text: &'a Rope, start: usize, end: usize) -> Self {
        let mut positions = Vec::new();
        let mut tok_start: Option<usize> = None;
        for (i, ch) in text.slice(start..end).chars().enumerate() {
            let pos = start + i;
            if ch.is_whitespace() {
                if let Some(s) = tok_start.take() {
                    positions.push((s, pos));
                }
            } else if tok_start.is_none() {
                tok_start = Some(pos);
            }
        }
        if let Some(s) = tok_start {
            positions.push((s, end));
        }
        Self { text, positions }
    }
}

impl<'a> imara_diff::TokenSource for WordTokens<'a> {
    type Token = String;
    type Tokenizer = Box<dyn Iterator<Item = String> + 'a>;

    fn tokenize(&self) -> Self::Tokenizer {
        let text = self.text;
        let positions = self.positions.clone();
        Box::new(
            positions
                .into_iter()
                .map(move |(s, e)| text.slice(s..e).chars().collect()),
        )
    }

    fn estimate_tokens(&self) -> u32 {
        self.positions.len() as u32
    }
}

/// Compute word-level diff between two sections of `text`.
///
/// Returns `(removed_ranges, added_ranges)` as half-open char-index intervals.
pub fn refine_diff(
    text: &Rope,
    left: (usize, usize),
    right: (usize, usize),
) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let left_tokens = WordTokens::new(text, left.0, left.1);
    let right_tokens = WordTokens::new(text, right.0, right.1);

    let left_positions = left_tokens.positions.clone();
    let right_positions = right_tokens.positions.clone();

    let input = InternedInput::new(left_tokens, right_tokens);
    let diff = Diff::compute(Algorithm::Histogram, &input);

    let mut removed: Vec<Range<usize>> = Vec::new();
    let mut added: Vec<Range<usize>> = Vec::new();

    for hunk in diff.hunks() {
        for &(s, e) in &left_positions[hunk.before.start as usize..hunk.before.end as usize] {
            match removed.last_mut() {
                Some(r) if r.end == s => r.end = e,
                _ => removed.push(s..e),
            }
        }
        for &(s, e) in &right_positions[hunk.after.start as usize..hunk.after.end as usize] {
            match added.last_mut() {
                Some(r) if r.end == s => r.end = e,
                _ => added.push(s..e),
            }
        }
    }

    (removed, added)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    fn rope(s: &str) -> Rope {
        Rope::from(s)
    }

    fn parse_one(s: &str) -> (Rope, ConflictRegion) {
        let r = Rope::from(s);
        let mut c = find_conflicts(&r);
        assert_eq!(c.len(), 1, "expected exactly one conflict");
        (r, c.swap_remove(0))
    }

    // ── find_conflicts: git format ────────────────────────────────────────────

    #[test]
    fn no_conflicts() {
        assert!(find_conflicts(&rope("hello\nworld\n")).is_empty());
    }

    #[test]
    fn bare_conflict() {
        // A manually-edited conflict with no inner markers — just <<<<<<< / content / >>>>>>>
        let (r, c) = parse_one("<<<<<<< HEAD\nhand-picked\n>>>>>>> branch\n");
        assert_eq!(c.sections.len(), 1);
        assert_eq!(c.sections[0].kind, SectionKind::Side);
        let (s, e) = current_content(&c);
        assert_eq!(r.slice(s..e).to_string(), "hand-picked\n");
        let (s, e) = incoming_content(&c);
        assert_eq!(r.slice(s..e).to_string(), "hand-picked\n");
        assert!(base_content(&c).is_none());
    }

    #[test]
    fn two_way_conflict_basic() {
        let (r, c) = parse_one("<<<<<<< HEAD\ncurrent\n=======\nincoming\n>>>>>>> branch\n");
        assert_eq!(c.sections.len(), 2);
        assert_eq!(c.sections[0].kind, SectionKind::Side);
        assert_eq!(c.sections[1].kind, SectionKind::Side);
        assert_eq!(c.start, 0);
        assert_eq!(c.end, r.len_chars());
    }

    #[test]
    fn two_way_conflict_content() {
        let (r, c) =
            parse_one("<<<<<<< HEAD\ncurrent line\n=======\nincoming line\n>>>>>>> branch\n");
        let (s, e) = current_content(&c);
        assert_eq!(r.slice(s..e).to_string(), "current line\n");
        let (s, e) = incoming_content(&c);
        assert_eq!(r.slice(s..e).to_string(), "incoming line\n");
    }

    #[test]
    fn three_way_conflict_basic() {
        let (_, c) = parse_one(
            "<<<<<<< HEAD\ncurrent\n||||||| base\noriginal\n=======\nincoming\n>>>>>>> branch\n",
        );
        assert_eq!(c.sections.len(), 3);
        assert_eq!(c.sections[0].kind, SectionKind::Side);
        assert_eq!(c.sections[1].kind, SectionKind::Base);
        assert_eq!(c.sections[2].kind, SectionKind::Side);
    }

    #[test]
    fn three_way_content() {
        let text =
            "<<<<<<< HEAD\ncurrent\n||||||| base\nbase line\n=======\nincoming\n>>>>>>> branch\n";
        let r = rope(text);
        let c = &find_conflicts(&r)[0];
        let (s, e) = current_content(c);
        assert_eq!(r.slice(s..e).to_string(), "current\n");
        let (s, e) = base_content(c).unwrap();
        assert_eq!(r.slice(s..e).to_string(), "base line\n");
        let (s, e) = incoming_content(c);
        assert_eq!(r.slice(s..e).to_string(), "incoming\n");
    }

    #[test]
    fn multiple_conflicts() {
        let text = concat!(
            "before\n",
            "<<<<<<< HEAD\na\n=======\nb\n>>>>>>> b\n",
            "between\n",
            "<<<<<<< HEAD\nc\n=======\nd\n>>>>>>> b\n",
            "after\n",
        );
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts[1].start > conflicts[0].end);
    }

    #[test]
    fn conflict_no_trailing_newline() {
        let text = "<<<<<<< HEAD\ncurrent\n=======\nincoming\n>>>>>>> branch";
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].end, text.chars().count());
    }

    #[test]
    fn incomplete_no_close_ignored() {
        assert!(find_conflicts(&rope("<<<<<<< HEAD\ncurrent\n=======\nincoming\n")).is_empty());
    }

    #[test]
    fn restarted_on_nested_open_marker() {
        let text = concat!(
            "<<<<<<< outer\n",
            "<<<<<<< inner\n",
            "current\n",
            "=======\n",
            "incoming\n",
            ">>>>>>> inner\n",
        );
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].start, "<<<<<<< outer\n".chars().count());
    }

    #[test]
    fn jj_style_git_diff3_labels() {
        // jj produces diff3 markers with change IDs
        let text = concat!(
            "<<<<<<< ouyysnvk c9a24f82 \"first version\"\n",
            "1st version\n",
            "||||||| zxwrknxy 62f152a0 \"base\"\n",
            "original\n",
            "=======\n",
            "2nd version\n",
            ">>>>>>> kyqztmxm cf165681 \"second version\"\n",
        );
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 1);
        let c = &conflicts[0];
        assert_eq!(c.sections.len(), 3);
        let (s, e) = current_content(c);
        assert_eq!(r.slice(s..e).to_string(), "1st version\n");
        let (s, e) = base_content(c).unwrap();
        assert_eq!(r.slice(s..e).to_string(), "original\n");
        let (s, e) = incoming_content(c);
        assert_eq!(r.slice(s..e).to_string(), "2nd version\n");
    }

    #[test]
    fn jj_snapshot_two_sides() {
        // jj snapshot format: 2 sides + 1 base
        let (r, c) = parse_one(concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ side #1\n",
            "alpha\n",
            "------- base\n",
            "beta\n",
            "+++++++ side #2\n",
            "gamma\n",
            ">>>>>>> Conflict 1 of 1 ends\n",
        ));
        assert_eq!(c.sections.len(), 3);
        assert_eq!(c.sections[0].kind, SectionKind::Side);
        assert_eq!(c.sections[1].kind, SectionKind::Base);
        assert_eq!(c.sections[2].kind, SectionKind::Side);
        let (s, e) = current_content(&c);
        assert_eq!(r.slice(s..e).to_string(), "alpha\n");
        let (s, e) = base_content(&c).unwrap();
        assert_eq!(r.slice(s..e).to_string(), "beta\n");
        let (s, e) = incoming_content(&c);
        assert_eq!(r.slice(s..e).to_string(), "gamma\n");
    }

    #[test]
    fn jj_snapshot_three_sides() {
        // jj snapshot format: 3 sides + 1 base (4 sections)
        let text = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ side #1\n",
            "s1\n",
            "------- base\n",
            "base\n",
            "+++++++ side #2\n",
            "s2\n",
            "------- base\n",
            "base\n",
            "+++++++ side #3\n",
            "s3\n",
            ">>>>>>> Conflict 1 of 1 ends\n",
        );
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 1);
        let c = &conflicts[0];
        assert_eq!(c.sections.len(), 5);
        assert_eq!(c.sections[0].kind, SectionKind::Side);
        assert_eq!(c.sections[1].kind, SectionKind::Base);
        assert_eq!(c.sections[2].kind, SectionKind::Side);
        assert_eq!(c.sections[3].kind, SectionKind::Base);
        assert_eq!(c.sections[4].kind, SectionKind::Side);
    }

    // ── conflict_at ───────────────────────────────────────────────────────────

    #[test]
    fn conflict_at_inside() {
        let text = "before\n<<<<<<< HEAD\ncurrent\n=======\nincoming\n>>>>>>> b\nafter\n";
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        let before_len = "before\n".chars().count();
        assert_eq!(conflict_at(&conflicts, before_len), Some(0));
        assert_eq!(conflict_at(&conflicts, 0), None);
        assert_eq!(conflict_at(&conflicts, conflicts[0].end + 1), None);

        // Boundary: at start (inclusive) and at end (exclusive)
        assert_eq!(conflict_at(&conflicts, conflicts[0].start), Some(0));
        assert_eq!(conflict_at(&conflicts, conflicts[0].end), None);
    }

    // ── next/prev_conflict ────────────────────────────────────────────────────

    #[test]
    fn next_conflict_from_before() {
        let text = concat!(
            "before\n",
            "<<<<<<< HEAD\na\n=======\nb\n>>>>>>> b\n",
            "between\n",
            "<<<<<<< HEAD\nc\n=======\nd\n>>>>>>> b\n",
        );
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(next_conflict(&conflicts, 0), Some(0));
        assert_eq!(next_conflict(&conflicts, conflicts[0].start + 1), Some(1));
        assert_eq!(next_conflict(&conflicts, conflicts[1].end), None);
    }

    #[test]
    fn prev_conflict_from_after() {
        let text = concat!(
            "<<<<<<< HEAD\na\n=======\nb\n>>>>>>> b\n",
            "between\n",
            "<<<<<<< HEAD\nc\n=======\nd\n>>>>>>> b\n",
            "after\n",
        );
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        let after_last = conflicts[1].end + 1;
        assert_eq!(prev_conflict(&conflicts, after_last), Some(1));
        let inside_second = conflicts[1].start + 1;
        assert_eq!(prev_conflict(&conflicts, inside_second), Some(1));
        assert_eq!(prev_conflict(&conflicts, conflicts[1].start), Some(0));
        assert_eq!(prev_conflict(&conflicts, 0), None);
    }

    // ── conflict_section_at ───────────────────────────────────────────────────

    #[test]
    fn section_at_git_two_way() {
        let text = "<<<<<<< HEAD\ncurrent\n=======\nincoming\n>>>>>>> b\n";
        let r = rope(text);
        let c = &find_conflicts(&r)[0];
        // On <<<<<<< line → section 0
        assert_eq!(conflict_section_at(c, &r, 0), Some(0));
        // In current content → section 0
        let current_pos = "<<<<<<< HEAD\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, current_pos), Some(0));
        // On ======= line → None
        let sep_pos = "<<<<<<< HEAD\ncurrent\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, sep_pos), None);
        // In incoming content → section 1
        let incoming_pos = "<<<<<<< HEAD\ncurrent\n=======\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, incoming_pos), Some(1));
        // On >>>>>>> line → section 1
        let close_pos = "<<<<<<< HEAD\ncurrent\n=======\nincoming\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, close_pos), Some(1));
    }

    #[test]
    fn section_at_git_three_way() {
        let text = "<<<<<<< HEAD\ncurrent\n||||||| base\nbase\n=======\nincoming\n>>>>>>> b\n";
        let r = rope(text);
        let c = &find_conflicts(&r)[0];
        // On ||||||| line → section 1
        let base_marker = "<<<<<<< HEAD\ncurrent\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, base_marker), Some(1));
        // In base content → section 1
        let base_pos = "<<<<<<< HEAD\ncurrent\n||||||| base\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, base_pos), Some(1));
        // On ======= → None
        let sep_pos = "<<<<<<< HEAD\ncurrent\n||||||| base\nbase\n"
            .chars()
            .count();
        assert_eq!(conflict_section_at(c, &r, sep_pos), None);
    }

    #[test]
    fn section_at_jj_snapshot() {
        let text = concat!(
            "<<<<<<< Conflict\n",
            "+++++++ s1\n",
            "hello\n",
            "------- base\n",
            "world\n",
            "+++++++ s2\n",
            "rust\n",
            ">>>>>>> Conflict ends\n",
        );
        let r = rope(text);
        let c = &find_conflicts(&r)[0];
        // On +++++++ s1 line → section 0
        let s1_marker = "<<<<<<< Conflict\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, s1_marker), Some(0));
        // On ------- line → section 1
        let base_marker = "<<<<<<< Conflict\n+++++++ s1\nhello\n".chars().count();
        assert_eq!(conflict_section_at(c, &r, base_marker), Some(1));
        // On +++++++ s2 line → section 2
        let s2_marker = "<<<<<<< Conflict\n+++++++ s1\nhello\n------- base\nworld\n"
            .chars()
            .count();
        assert_eq!(conflict_section_at(c, &r, s2_marker), Some(2));
        // On >>>>>>> line → section 2 (last)
        let close = "<<<<<<< Conflict\n+++++++ s1\nhello\n------- base\nworld\n+++++++ s2\nrust\n"
            .chars()
            .count();
        assert_eq!(conflict_section_at(c, &r, close), Some(2));
    }

    // ── refine pairs ──────────────────────────────────────────────────────────

    type RefinePairCase<'a> = (&'a str, usize, &'a [(usize, usize)]);

    #[test]
    fn refine_pairs_cases() {
        let cases: &[RefinePairCase<'_>] = &[
            (
                "<<<<<<< HEAD\ncurrent\n=======\nincoming\n>>>>>>> b\n",
                1,
                &[(0, 1)],
            ),
            (
                "<<<<<<< HEAD\ncurrent\n||||||| base\nbase\n=======\nincoming\n>>>>>>> b\n",
                3,
                &[(0, 1), (1, 2), (0, 2)],
            ),
        ];
        for &(text, num_pairs, pairs) in cases {
            let c = &find_conflicts(&rope(text))[0];
            assert_eq!(c.num_refine_pairs(), num_pairs);
            for (i, &expected) in pairs.iter().enumerate() {
                assert_eq!(c.refine_pair_indices(i), Some(expected));
            }
            assert_eq!(c.refine_pair_indices(pairs.len()), None);
        }
    }

    #[test]
    fn refine_pairs_four_sections() {
        // 4 sections [S0,B1,S2,S3], pairs only from phases 1+2:
        //   phase 1: (0,1),(1,2) = 2
        //   phase 2 (Side–Side, fixed left): (0,2),(0,3),(2,3) = 3
        //   total: 5  (pair (1,3)=B1,S3 is excluded)
        let text = concat!(
            "<<<<<<< Conflict\n",
            "+++++++ s1\nA\n",
            "------- base\nB\n",
            "+++++++ s2\nC\n",
            "+++++++ s3\nD\n",
            ">>>>>>> Conflict ends\n",
        );
        let c = &find_conflicts(&rope(text))[0];
        assert_eq!(c.sections.len(), 4);
        assert_eq!(c.num_refine_pairs(), 5);
        assert_eq!(c.refine_pair_indices(0), Some((0, 1)));
        assert_eq!(c.refine_pair_indices(1), Some((1, 2)));
        assert_eq!(c.refine_pair_indices(2), Some((0, 2)));
        assert_eq!(c.refine_pair_indices(3), Some((0, 3)));
        assert_eq!(c.refine_pair_indices(4), Some((2, 3)));
        assert_eq!(c.refine_pair_indices(5), None);
    }

    #[test]
    fn pair_sections_put_base_on_left() {
        // diff3: sections = [Side(current), Base, Side(incoming)]
        // Skip-distance ordering: (0,1), (1,2), (0,2)
        // Pair 0 = (Side, Base) → MUST swap → left=base, right=current
        // Pair 1 = (Base, Side) → already base-left, NO swap
        // Pair 2 = (Side, Side) → order preserved
        let text = "<<<<<<< HEAD\ncurrent\n||||||| base\nbase\n=======\nincoming\n>>>>>>> b\n";
        let r = rope(text);
        let c = &find_conflicts(&r)[0];

        // pair 0 => indices (0,1) = (Side, Base) -> MUST swap to (Base, Side)
        let (left, right) = conflict_pair_sections(c, 0).unwrap();
        assert_eq!(r.slice(left.0..left.1).to_string(), "base\n"); // base on left (red)
        assert_eq!(r.slice(right.0..right.1).to_string(), "current\n"); // side on right (green)

        // pair 1 => indices (1,2) = (Base, Side) -> already base-left, NO swap
        let (left, right) = conflict_pair_sections(c, 1).unwrap();
        assert_eq!(r.slice(left.0..left.1).to_string(), "base\n"); // base on left
        assert_eq!(r.slice(right.0..right.1).to_string(), "incoming\n"); // side on right

        // pair 2 => indices (0,2) = (Side, Side) -> order preserved
        let (left, right) = conflict_pair_sections(c, 2).unwrap();
        assert_eq!(r.slice(left.0..left.1).to_string(), "current\n");
        assert_eq!(r.slice(right.0..right.1).to_string(), "incoming\n");

        assert!(conflict_pair_sections(c, 3).is_none());
    }

    #[test]
    fn refine_pair_clamped_and_default() {
        let text = "<<<<<<< HEAD\ncurrent\n=======\nincoming\n>>>>>>> b\n"; // 1 pair, max idx 0
        let r = rope(text);
        let c = &find_conflicts(&r)[0];

        // Stale/out-of-range selection gets clamped to valid range
        let mut state = HashMap::new();
        state.insert(c.start, 99);
        assert_eq!(conflict_refine_pair(&state, c), 0);

        // Empty state defaults to 0
        assert_eq!(conflict_refine_pair(&HashMap::new(), c), 0);

        // Valid index passes through
        state.clear();
        state.insert(c.start, 0);
        assert_eq!(conflict_refine_pair(&state, c), 0);
    }
    #[test]
    fn conflict_marker_lines_returns_sorted() {
        let text = concat!(
            "before\n",
            "<<<<<<< HEAD\nours\n||||||| base\nbase\n=======\ntheirs\n>>>>>>> b\n",
            "after\n",
        );
        let r = rope(text);
        let lines = conflict_marker_lines(&r);
        assert!(!lines.is_empty());
        for i in 0..lines.len() - 1 {
            assert!(lines[i] < lines[i + 1]);
        }
        let dedup_check: Vec<_> = lines.iter().collect();
        assert_eq!(dedup_check.len(), lines.len());
    }

    #[test]
    fn all_sides_content_concatenates() {
        let text = concat!(
            "<<<<<<< Conflict\n",
            "+++++++ s1\nfirst\n",
            "------- base\nbase\n",
            "+++++++ s2\nsecond\n",
            "+++++++ s3\nthird\n",
            ">>>>>>> Conflict ends\n",
        );
        let r = rope(text);
        let c = &find_conflicts(&r)[0];
        let all = all_sides_content(&r, c);
        assert!(all.contains("first"));
        assert!(all.contains("second"));
        assert!(all.contains("third"));
        // Base should NOT be included
        assert!(!all.contains("base\n"));
    }

    #[test]
    fn empty_side_conflict() {
        let text = "<<<<<<<\n=======\n>>>>>>>\n";
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 1);
        let c = &conflicts[0];
        assert_eq!(c.sections.len(), 2);
        let (s, e) = current_content(c);
        assert_eq!(r.slice(s..e).to_string(), "");
        let (s, e) = incoming_content(c);
        assert_eq!(r.slice(s..e).to_string(), "");
    }

    #[test]
    fn conflict_with_crlf() {
        let text = "<<<<<<< HEAD\r\ncurrent\r\n=======\r\nincoming\r\n>>>>>>> b\r\n";
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn transition_to_jj_with_content_section() {
        // Git→Jj transition where the implicit git side has content
        let text = concat!(
            "<<<<<<< Conflict\n",
            "shared content\n",
            "+++++++ side #1\n",
            "more\n",
            ">>>>>>> Conflict ends\n",
        );
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        assert_eq!(conflicts.len(), 1);
        let c = &conflicts[0];
        // The git side with "shared content" should be preserved
        assert_eq!(c.sections.len(), 2);
        let (s, e) = current_content(c);
        assert_eq!(r.slice(s..e).to_string(), "shared content\n");
        let (s, e) = incoming_content(c);
        assert_eq!(r.slice(s..e).to_string(), "more\n");
    }

    // ── refine_diff ───────────────────────────────────────────────────────────

    #[test]
    fn refine_diff_cases() {
        for &(current, incoming) in &[
            ("hello world", "hello world"),        // identical
            ("hello world", "hello rust"),         // single word
            ("foo bar", "baz qux"),                // completely different
            ("hello world test", "hello foo bar"), // adjacent words
        ] {
            let text = format!(
                "<<<<<<< HEAD\n{}\n=======\n{}\n>>>>>>> b\n",
                current, incoming
            );
            let r = Rope::from(text.as_str());
            let c = &find_conflicts(&r)[0];
            let (removed, added) = refine_diff(&r, current_content(c), incoming_content(c));

            if current == incoming {
                assert!(removed.is_empty());
                assert!(added.is_empty());
                continue;
            }

            assert!(
                !removed.is_empty(),
                "expected removed for '{}' vs '{}'",
                current,
                incoming
            );
            assert!(
                !added.is_empty(),
                "expected added for '{}' vs '{}'",
                current,
                incoming
            );

            let removed_text: String = removed
                .iter()
                .map(|range| r.slice(range.clone()).to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let added_text: String = added
                .iter()
                .map(|range| r.slice(range.clone()).to_string())
                .collect::<Vec<_>>()
                .join(" ");

            for word in current.split_whitespace() {
                if !incoming.contains(word) {
                    assert!(
                        removed_text.contains(word),
                        "'{word}' missing from removed: '{removed_text}'"
                    );
                }
            }
            for word in incoming.split_whitespace() {
                if !current.contains(word) {
                    assert!(
                        added_text.contains(word),
                        "'{word}' missing from added: '{added_text}'"
                    );
                }
            }
        }
    }

    #[test]
    fn refine_diff_identical_sections() {
        // When both sides are identical, refine_diff should return no ranges.
        let text = "<<<<<<< HEAD\nhello world\n=======\nhello world\n>>>>>>> branch\n";
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        let left = current_content(&conflicts[0]);
        let right = incoming_content(&conflicts[0]);
        let (removed, added) = refine_diff(&r, left, right);
        assert!(
            removed.is_empty(),
            "expected no removed ranges, got {:?}",
            removed
        );
        assert!(
            added.is_empty(),
            "expected no added ranges, got {:?}",
            added
        );
    }

    #[test]
    fn refine_diff_single_word_change() {
        // "hello world" vs "hello rust" — only "world"/"rust" differ.
        let text = "<<<<<<< HEAD\nhello world\n=======\nhello rust\n>>>>>>> branch\n";
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        let left = current_content(&conflicts[0]);
        let right = incoming_content(&conflicts[0]);
        let (removed, added) = refine_diff(&r, left, right);

        assert_eq!(
            removed.len(),
            1,
            "expected 1 removed range, got {:?}",
            removed
        );
        assert_eq!(added.len(), 1, "expected 1 added range, got {:?}", added);

        let removed_word: String = r.slice(removed[0].clone()).chars().collect();
        let added_word: String = r.slice(added[0].clone()).chars().collect();
        assert_eq!(removed_word, "world");
        assert_eq!(added_word, "rust");
    }

    #[test]
    fn refine_diff_completely_different() {
        // No words in common — all tokens should be flagged.
        let text = "<<<<<<< HEAD\nfoo bar\n=======\nbaz qux\n>>>>>>> branch\n";
        let r = rope(text);
        let conflicts = find_conflicts(&r);
        let left = current_content(&conflicts[0]);
        let right = incoming_content(&conflicts[0]);
        let (removed, added) = refine_diff(&r, left, right);

        // Both "foo" and "bar" removed; "baz" and "qux" added (may be merged).
        assert!(!removed.is_empty(), "expected removed ranges");
        assert!(!added.is_empty(), "expected added ranges");

        let removed_text: String = removed
            .iter()
            .map(|range| r.slice(range.clone()).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let added_text: String = added
            .iter()
            .map(|range| r.slice(range.clone()).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            removed_text.contains("foo"),
            "expected 'foo' in removed: {removed_text}"
        );
        assert!(
            removed_text.contains("bar"),
            "expected 'bar' in removed: {removed_text}"
        );
        assert!(
            added_text.contains("baz"),
            "expected 'baz' in added: {added_text}"
        );
        assert!(
            added_text.contains("qux"),
            "expected 'qux' in added: {added_text}"
        );
    }
}
