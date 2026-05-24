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

use crate::Rope;

// ── Types ─────────────────────────────────────────────────────────────────────

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

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse all conflict regions in `text`, in document order.
///
/// Supports git 2-way, git diff3 3-way, and jj snapshot N-way formats.
/// Incomplete conflict blocks are silently ignored.
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
                    if sections.len() >= 2 {
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
    fn incomplete_no_separator_ignored() {
        assert!(find_conflicts(&rope("<<<<<<< HEAD\ncurrent\n>>>>>>> branch\n")).is_empty());
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
}
