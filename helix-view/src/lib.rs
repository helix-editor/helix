#[macro_use]
pub mod macros;

pub mod annotations;
pub mod base64;
pub mod clipboard;
pub mod document;
pub mod editor;
pub mod events;
pub mod expansion;
pub mod graphics;
pub mod gutter;
pub mod handlers;
pub mod info;
pub mod input;
pub mod keyboard;
pub mod register;
pub mod theme;
pub mod tree;
pub mod view;

use std::{borrow::Cow, num::NonZeroUsize, path::Path};

// uses NonZeroUsize so Option<DocumentId> use a byte rather than two
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DocumentId(NonZeroUsize);

impl Default for DocumentId {
    fn default() -> DocumentId {
        // Safety: 1 is non-zero
        DocumentId(unsafe { NonZeroUsize::new_unchecked(1) })
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

slotmap::new_key_type! {
    pub struct ViewId;
}

pub enum Align {
    Top,
    Center,
    Bottom,
}

pub fn align_view(doc: &mut Document, view: &View, align: Align) {
    let doc_text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(doc_text);
    let viewport = view.inner_area(doc);
    let last_line_height = viewport.height.saturating_sub(1);
    let mut view_offset = doc.view_offset(view.id);

    let relative = match align {
        Align::Center => last_line_height / 2,
        Align::Top => 0,
        Align::Bottom => last_line_height,
    };

    let text_fmt = doc.text_format(viewport.width, None);
    (view_offset.anchor, view_offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        cursor,
        -(relative as isize),
        0,
        &text_fmt,
        &view.text_annotations(doc, None),
    );
    doc.set_view_offset(view.id, view_offset);
}

/// Returns the left-side position of the primary selection.
pub fn primary_cursor(view: &View, doc: &Document) -> usize {
    doc.selection(view.id)
        .primary()
        .cursor(doc.text().slice(..))
}

/// Returns the next diagnostic in the document if any.
///
/// This does not wrap-around.
pub fn next_diagnostic_in_doc<'d>(
    view: &View,
    doc: &'d Document,
    severity_filter: Option<helix_core::diagnostic::Severity>,
) -> Option<&'d Diagnostic> {
    let cursor = primary_cursor(view, doc);
    doc.diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity >= severity_filter)
        .find(|diag| diag.range.start > cursor)
}

/// Returns the previous diagnostic in the document if any.
///
/// This does not wrap-around.
pub fn prev_diagnostic_in_doc<'d>(
    view: &View,
    doc: &'d Document,
    severity_filter: Option<helix_core::diagnostic::Severity>,
) -> Option<&'d Diagnostic> {
    let cursor = primary_cursor(view, doc);
    doc.diagnostics()
        .iter()
        .rev()
        .filter(|diagnostic| diagnostic.severity >= severity_filter)
        .find(|diag| diag.range.start < cursor)
}

pub struct WorkspaceDiagnostic<'e> {
    pub path: Cow<'e, Path>,
    pub diagnostic: Cow<'e, helix_lsp::lsp::Diagnostic>,
    pub offset_encoding: OffsetEncoding,
}
impl<'e> WorkspaceDiagnostic<'e> {
    pub fn into_owned(self) -> WorkspaceDiagnostic<'static> {
        WorkspaceDiagnostic {
            path: Cow::Owned(self.path.into_owned()),
            diagnostic: Cow::Owned(self.diagnostic.into_owned()),
            offset_encoding: self.offset_encoding,
        }
    }
}

fn workspace_diagnostics(
    editor: &Editor,
    severity_filter: Option<helix_core::diagnostic::Severity>,
) -> impl Iterator<Item = WorkspaceDiagnostic<'_>> {
    editor
        .diagnostics
        .iter()
        .filter_map(|(uri, diagnostics)| {
            // Extract Path from diagnostic Uri, skipping diagnostics that don't have a path.
            uri.as_path().map(|p| (p, diagnostics))
        })
        .flat_map(|(path, diagnostics)| {
            let mut diagnostics = diagnostics.iter().collect::<Vec<_>>();
            diagnostics.sort_by_key(|(diagnostic, _)| diagnostic.range.start);

            diagnostics
                .into_iter()
                .map(move |(diagnostic, diagnostic_provider)| {
                    (path, diagnostic, diagnostic_provider)
                })
        })
        .filter(move |(_, diagnostic, _)| {
            // Filter by severity
            let severity = diagnostic
                .severity
                .and_then(Document::lsp_severity_to_severity);
            severity >= severity_filter
        })
        .map(|(path, diag, diagnostic_provider)| {
            match diagnostic_provider {
                DiagnosticProvider::Lsp { server_id, .. } => {
                    // Map language server ID to offset encoding
                    let offset_encoding = editor
                        .language_server_by_id(*server_id)
                        .map(|client| client.offset_encoding())
                        .unwrap_or_default();
                    (path, diag, offset_encoding)
                }
            }
        })
        .map(|(path, diagnostic, offset_encoding)| WorkspaceDiagnostic {
            path: Cow::Borrowed(path),
            diagnostic: Cow::Borrowed(diagnostic),
            offset_encoding,
        })
}

pub fn first_diagnostic_in_workspace(
    editor: &Editor,
    severity_filter: Option<helix_core::diagnostic::Severity>,
) -> Option<WorkspaceDiagnostic> {
    workspace_diagnostics(editor, severity_filter).next()
}

pub fn next_diagnostic_in_workspace(
    editor: &Editor,
    severity_filter: Option<helix_core::diagnostic::Severity>,
) -> Option<WorkspaceDiagnostic> {
    let (view, doc) = current_ref!(editor);

    let Some(current_doc_path) = doc.path() else {
        return first_diagnostic_in_workspace(editor, severity_filter);
    };

    let cursor = primary_cursor(view, doc);

    #[allow(clippy::filter_next)]
    workspace_diagnostics(editor, severity_filter)
        .filter(|d| {
            // Skip diagnostics before the current document
            d.path >= current_doc_path.as_path()
        })
        .filter(|d| {
            // Skip diagnostics before the primary cursor in the current document
            if d.path == current_doc_path.as_path() {
                let Some(start) = helix_lsp::util::lsp_pos_to_pos(
                    doc.text(),
                    d.diagnostic.range.start,
                    d.offset_encoding,
                ) else {
                    return false;
                };
                if start <= cursor {
                    return false;
                }
            }
            true
        })
        .next()
}

pub fn ensure_selections_forward(view: &View, doc: &mut Document) {
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|r| r.with_direction(Direction::Forward));

    doc.set_selection(view.id, selection);
}

pub use document::Document;
pub use editor::Editor;
use helix_core::{
    char_idx_at_visual_offset, diagnostic::DiagnosticProvider, movement::Direction, Diagnostic,
};
use helix_lsp::OffsetEncoding;
pub use theme::Theme;
pub use view::View;
