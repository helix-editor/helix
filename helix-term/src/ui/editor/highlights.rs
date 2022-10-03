use std::ops::Range;
use std::{slice, vec};

use helix_core::diagnostic::Severity;
use helix_core::graphemes::{next_grapheme_boundary, prev_grapheme_boundary};
use helix_core::syntax::{
    self, monotonic_overlay, overlapping_overlay, Highlight, HighlightEvent, MonotonicOverlay,
};
use helix_core::Diagnostic;
use helix_view::document::Mode;
use helix_view::graphics::CursorKind;
use helix_view::{Document, Editor, Theme, View};

pub trait HighlightOverlay<In: Iterator<Item = HighlightEvent>> {
    type Out: Iterator<Item = HighlightEvent>;
    fn apply(self, highlights: In) -> Self::Out;
}

macro_rules! impl_composition {
    ($first: ident, $last: ident $(,$names: ident)*) => {
        impl_composition!(@impl [$first, $($names,)* $last]  [$first $(,$names)*] [$($names,)* $last] $first $last);
    };

    ($first: ident) => {};

    (@impl [$($names: ident),+] [$($head: ident),*] [$($tail: ident),*] $first: ident $last: ident) => {
    #[allow(non_snake_case)]
     impl<In, $($names),*> HighlightOverlay<In> for ($($names),+)
        where
            In: Iterator<Item = HighlightEvent>,
            $first: HighlightOverlay<In>,
            $($tail:  HighlightOverlay<$head::Out>),+
        {
            type Out = $last::Out;

            fn apply(self, highlights: In) -> Self::Out {
                let ($($names),+) = self;
                $(let highlights = $names.apply(highlights);)+
                highlights
            }
        }
        impl_composition!($($head),*);
    };
    (@impl [$name: ident] [$first: ident]  [$last: ident]) => {};
}

impl_composition!(A, B, C, D, E, F, G, H, I, J, K);

pub struct DiagnosticsOverlay<'a> {
    pub theme: &'a Theme,
    pub doc: &'a Document,
    pub severity: Option<Severity>,
}

pub struct DiagnosticIter<'doc> {
    diagnostics: slice::Iter<'doc, Diagnostic>,
    severity: Option<Severity>,
}

impl<'doc> Iterator for DiagnosticIter<'doc> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.diagnostics.find_map(|diagnostic| {
            if diagnostic.severity != self.severity {
                return None;
            }
            Some(diagnostic.range.start..diagnostic.range.end)
        })
    }
}

impl<'a, In> HighlightOverlay<In> for DiagnosticsOverlay<'a>
where
    In: Iterator<Item = HighlightEvent>,
{
    type Out = syntax::OverlappingOverlay<In, DiagnosticIter<'a>>;
    fn apply(self, highlights: In) -> Self::Out {
        let get_scope_of = |scope| {
            self.theme
            .find_scope_index(scope)
            // get one of the themes below as fallback values
            .or_else(|| self.theme.find_scope_index("diagnostic"))
            .or_else(|| self.theme.find_scope_index("ui.cursor"))
            .or_else(|| self.theme.find_scope_index("ui.selection"))
            .expect(
                "at least one of the following scopes must be defined in the theme: `diagnostic`, `ui.cursor`, or `ui.selection`",
            )
        };

        let highlight = match self.severity {
            Some(Severity::Error) => get_scope_of("diagnostic.error"),
            Some(Severity::Warning) => get_scope_of("diagnostic.warning"),
            Some(Severity::Info) => get_scope_of("diagnostic.info"),
            Some(Severity::Hint) => get_scope_of("diagnostic.hint"),
            None => get_scope_of("diagnostic"),
        };

        overlapping_overlay(
            highlights,
            DiagnosticIter {
                diagnostics: self.doc.diagnostics().iter(),
                severity: self.severity,
            },
            Highlight(highlight),
        )
    }
}

pub type AllDiagnosticsOverlay<'a> = (
    DiagnosticsOverlay<'a>,
    DiagnosticsOverlay<'a>,
    DiagnosticsOverlay<'a>,
    DiagnosticsOverlay<'a>,
    DiagnosticsOverlay<'a>,
);

pub fn all_diganostic_overlays<'a>(
    doc: &'a Document,
    theme: &'a Theme,
) -> AllDiagnosticsOverlay<'a> {
    (
        DiagnosticsOverlay {
            theme,
            doc,
            severity: Some(Severity::Hint),
        },
        DiagnosticsOverlay {
            theme,
            doc,
            severity: Some(Severity::Info),
        },
        DiagnosticsOverlay {
            theme,
            doc,
            severity: None,
        },
        DiagnosticsOverlay {
            theme,
            doc,
            severity: Some(Severity::Warning),
        },
        DiagnosticsOverlay {
            theme,
            doc,
            severity: Some(Severity::Error),
        },
    )
}

pub struct SelectionOverlay<'a> {
    pub focused: bool,
    pub editor: &'a Editor,
    pub doc: &'a Document,
    pub theme: &'a Theme,
    pub view: &'a View,
}

impl<'a, In> HighlightOverlay<In> for SelectionOverlay<'a>
where
    In: Iterator<Item = HighlightEvent>,
{
    type Out = MonotonicOverlay<In, vec::IntoIter<syntax::Span>>;

    fn apply(self, highlights: In) -> Self::Out {
        let SelectionOverlay {
            focused,
            editor,
            doc,
            theme,
            view,
        } = self;

        if !focused {
            return monotonic_overlay(highlights, Vec::new().into_iter());
        }

        // TODO avoid collecting into a vec
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);
        let primary_idx = selection.primary_index();
        let mode = editor.mode();

        let cursorkind = editor.config().cursor_shape.from_mode(mode);
        let cursor_is_block = cursorkind == CursorKind::Block;

        let selection_scope = theme
            .find_scope_index("ui.selection")
            .expect("could not find `ui.selection` scope in the theme!");
        let base_cursor_scope = theme
            .find_scope_index("ui.cursor")
            .unwrap_or(selection_scope);

        let cursor_scope = match mode {
            Mode::Insert => theme.find_scope_index("ui.cursor.insert"),
            Mode::Select => theme.find_scope_index("ui.cursor.select"),
            Mode::Normal => Some(base_cursor_scope),
        }
        .unwrap_or(base_cursor_scope);

        let primary_cursor_scope = theme
            .find_scope_index("ui.cursor.primary")
            .unwrap_or(cursor_scope);
        let primary_selection_scope = theme
            .find_scope_index("ui.selection.primary")
            .unwrap_or(selection_scope);

        let mut spans: Vec<syntax::Span> = Vec::new();
        for (i, range) in selection.iter().enumerate() {
            let selection_is_primary = i == primary_idx;
            let (cursor_scope, selection_scope) = if selection_is_primary {
                (primary_cursor_scope, primary_selection_scope)
            } else {
                (cursor_scope, selection_scope)
            };

            // Special-case: cursor at end of the rope.
            if range.head == range.anchor && range.head == text.len_chars() {
                if !selection_is_primary || cursor_is_block {
                    // Bar and underline cursors are drawn by the terminal
                    // BUG: If the editor area loses focus while having a bar or
                    // underline cursor (eg. when a regex prompt has focus) then
                    // the primary cursor will be invisible. This doesn't happen
                    // with block cursors since we manually draw *all* cursors.
                    spans.push(syntax::Span {
                        scope: Highlight(cursor_scope),
                        start: range.head,
                        end: range.head + 1,
                    });
                }
                continue;
            }

            let range = range.min_width_1(text);
            if range.head > range.anchor {
                // Standard case.
                let cursor_start = prev_grapheme_boundary(text, range.head);
                spans.push(syntax::Span {
                    scope: Highlight(selection_scope),
                    start: range.anchor,
                    end: cursor_start,
                });
                if !selection_is_primary || cursor_is_block {
                    spans.push(syntax::Span {
                        scope: Highlight(cursor_scope),
                        start: cursor_start,
                        end: range.head,
                    });
                }
            } else {
                // Reverse case.
                let cursor_end = next_grapheme_boundary(text, range.head);
                if !selection_is_primary || cursor_is_block {
                    spans.push(syntax::Span {
                        scope: Highlight(cursor_scope),
                        start: range.head,
                        end: cursor_end,
                    });
                }
                spans.push(syntax::Span {
                    scope: Highlight(selection_scope),
                    start: cursor_end,
                    end: range.anchor,
                });
            }
        }

        monotonic_overlay(highlights, spans.into_iter())
    }
}

impl<'a, In> HighlightOverlay<In> for ()
where
    In: Iterator<Item = HighlightEvent>,
{
    type Out = In;

    fn apply(self, highlights: In) -> Self::Out {
        highlights
    }
}
