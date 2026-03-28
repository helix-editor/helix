//! Quicklist state and traversal helpers.
//!
//! The quicklist is an editor-global list of locations collected from pickers.
//! It is intended for workflows where a picker already shows a useful set of
//! files or file locations and the user wants to keep traversing that set after
//! the picker is closed.
//!
//! Typical usage looks like this:
//! - A picker is opened and filtered down to a useful result set.
//! - The picker copies its current matched items into the quicklist.
//! - The user navigates those stored entries forward or backward, globally.
//!
//! Quicklist entries can target either paths on disk or open in-memory
//! documents. Each entry also carries a position payload describing how that
//! target should be restored:
//! - `QuicklistPosition::Selection` preserves an exact selection.
//! - `QuicklistPosition::LineRange` stores a coarse zero-based line span.
//! - `QuicklistPosition::LineColRange` stores a zero-based line/column range.
//! - `QuicklistPosition::LspRange` stores an LSP-native range with its offset
//!   encoding for exact restoration.
//! - `QuicklistPosition::None` reopens the target without overriding the
//!   document's existing restored cursor/selection.
//!
//! The stored location data is still mixed-fidelity today. Some pickers can
//! now provide exact selections directly, while many others still only expose
//! preview-oriented file locations such as path + line span. That is good
//! enough for basic quicklist traversal, but it can still lose picker-specific
//! precision such as offset-encoding-aware LSP ranges or other activation
//! metadata.
//!
//! TODO: extend quicklist entries to store richer picker-specific positions so
//! quicklist activation can reproduce the original picker jump exactly.

use crate::DocumentId;
use helix_core::{movement::Direction, RopeSlice, Selection};
use helix_lsp::{lsp, OffsetEncoding};
use std::path::{Path, PathBuf};

/// The destination referenced by a quicklist entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuicklistTarget {
    /// A file on disk.
    Path(PathBuf),
    /// An open in-memory document, including unsaved buffers.
    Document(DocumentId),
}

/// The position to restore when activating a quicklist entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuicklistPosition {
    /// Restore an exact selection.
    Selection(Selection),
    /// Restore a coarse zero-based line range.
    LineRange { start: usize, end: usize },
    /// Restore a zero-based line/column range.
    LineColRange {
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    },
    /// Restore an exact LSP range using the original offset encoding.
    LspRange {
        range: lsp::Range,
        offset_encoding: OffsetEncoding,
    },
    /// Do not override the document's existing restored selection.
    None,
}

impl QuicklistPosition {
    /// Returns a best-effort zero-based line range for preview and display.
    pub fn line_range(&self, text: Option<RopeSlice<'_>>) -> Option<(usize, usize)> {
        match self {
            Self::Selection(selection) => text.map(|text| selection.primary().line_range(text)),
            Self::LineRange { start, end } => Some((*start, *end)),
            Self::LineColRange {
                start_line,
                end_line,
                ..
            } => Some((*start_line, *end_line)),
            Self::LspRange { range, .. } => {
                Some((range.start.line as usize, range.end.line as usize))
            }
            Self::None => None,
        }
    }
}

/// A stored location in the quicklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuicklistEntry {
    /// The file or document to visit.
    pub target: QuicklistTarget,
    /// The position to restore within the target.
    ///
    /// TODO: extend this to store richer picker-specific jump metadata such as
    /// LSP ranges with offset encodings and other activation-only payloads.
    pub position: QuicklistPosition,
}

/// An editor-global list of locations collected from pickers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Quicklist {
    entries: Vec<QuicklistEntry>,
    current: Option<usize>,
}

impl Quicklist {
    /// Replaces the quicklist contents and clears the current position.
    pub fn replace(&mut self, entries: Vec<QuicklistEntry>) {
        self.entries = entries;
        self.current = None;
    }

    /// Returns the stored quicklist entries.
    pub fn entries(&self) -> &[QuicklistEntry] {
        &self.entries
    }

    /// Returns whether the quicklist has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns the current quicklist position, if one has been visited.
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    /// Sets the current quicklist position.
    pub fn set_current(&mut self, current: Option<usize>) {
        self.current = current;
    }

    pub(crate) fn next_index(
        &self,
        count: usize,
        current_doc_id: DocumentId,
        current_path: Option<&Path>,
        same_file: bool,
    ) -> Option<usize> {
        self.find_index(
            Direction::Forward,
            count,
            current_doc_id,
            current_path,
            same_file,
        )
    }

    pub(crate) fn prev_index(
        &self,
        count: usize,
        current_doc_id: DocumentId,
        current_path: Option<&Path>,
        same_file: bool,
    ) -> Option<usize> {
        self.find_index(
            Direction::Backward,
            count,
            current_doc_id,
            current_path,
            same_file,
        )
    }

    fn find_index(
        &self,
        direction: Direction,
        count: usize,
        current_doc_id: DocumentId,
        current_path: Option<&Path>,
        same_file: bool,
    ) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }

        let matches = |entry: &QuicklistEntry| {
            !same_file
                || match &entry.target {
                    QuicklistTarget::Document(doc_id) => *doc_id == current_doc_id,
                    QuicklistTarget::Path(path) => {
                        current_path.is_some_and(|current| path == current)
                    }
                }
        };

        if !self.entries.iter().any(matches) {
            return None;
        }

        let len = self.entries.len();
        let mut index = match (direction, self.current) {
            (Direction::Forward, Some(current)) => current,
            (Direction::Forward, None) => len - 1,
            (Direction::Backward, Some(current)) => current,
            (Direction::Backward, None) => 0,
        };

        let mut remaining = count.max(1);
        while remaining > 0 {
            for _ in 0..len {
                index = match direction {
                    Direction::Forward => (index + 1) % len,
                    Direction::Backward => (index + len - 1) % len,
                };
                if matches(&self.entries[index]) {
                    remaining -= 1;
                    break;
                }
            }
        }

        Some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::{Quicklist, QuicklistEntry, QuicklistPosition, QuicklistTarget};
    use crate::DocumentId;
    use helix_core::{Rope, Selection};
    use std::path::PathBuf;

    fn entry(path: &str, line: usize) -> QuicklistEntry {
        QuicklistEntry {
            target: QuicklistTarget::Path(PathBuf::from(path)),
            position: QuicklistPosition::LineRange {
                start: line,
                end: line,
            },
        }
    }

    fn unsaved_entry(id: DocumentId, line: usize) -> QuicklistEntry {
        QuicklistEntry {
            target: QuicklistTarget::Document(id),
            position: QuicklistPosition::LineRange {
                start: line,
                end: line,
            },
        }
    }

    #[test]
    fn quicklist_selection_reports_line_range() {
        let text = Rope::from("alpha\nbeta\ngamma\n");
        let position = QuicklistPosition::Selection(Selection::single(6, 10));

        assert_eq!(position.line_range(Some(text.slice(..))), Some((1, 1)));
    }

    #[test]
    fn quicklist_wraps_globally() {
        let mut quicklist = Quicklist::default();
        quicklist.replace(vec![entry("a.rs", 0), entry("b.rs", 1), entry("c.rs", 2)]);

        assert_eq!(
            quicklist.next_index(1, DocumentId::default(), None, false),
            Some(0)
        );
        quicklist.set_current(Some(0));
        assert_eq!(
            quicklist.next_index(1, DocumentId::default(), None, false),
            Some(1)
        );
        assert_eq!(
            quicklist.prev_index(1, DocumentId::default(), None, false),
            Some(2)
        );
    }

    #[test]
    fn quicklist_filters_to_current_file() {
        let mut quicklist = Quicklist::default();
        quicklist.replace(vec![
            entry("a.rs", 0),
            entry("b.rs", 1),
            entry("a.rs", 2),
            entry("c.rs", 3),
        ]);

        let path = PathBuf::from("a.rs");
        assert_eq!(
            quicklist.next_index(1, DocumentId::default(), Some(&path), true),
            Some(0)
        );
        quicklist.set_current(Some(0));
        assert_eq!(
            quicklist.next_index(1, DocumentId::default(), Some(&path), true),
            Some(2)
        );
        assert_eq!(
            quicklist.prev_index(1, DocumentId::default(), Some(&path), true),
            Some(2)
        );
    }

    #[test]
    fn quicklist_filters_to_unsaved_document() {
        let mut quicklist = Quicklist::default();
        let doc_id = DocumentId::default();
        quicklist.replace(vec![
            entry("a.rs", 0),
            unsaved_entry(doc_id, 1),
            entry("b.rs", 2),
        ]);

        assert_eq!(quicklist.next_index(1, doc_id, None, true), Some(1));
        quicklist.set_current(Some(1));
        assert_eq!(quicklist.prev_index(1, doc_id, None, true), Some(1));
    }
}
