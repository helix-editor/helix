//! This module contains the various annotations that can be added to a [`super::Document`] when
//! displaying it.
//!
//! Examples: inline diagnostics, inlay hints, git blames.

use std::rc::Rc;

use helix_core::diagnostic::Severity;
use helix_core::text_annotations::LineAnnotation;
use helix_core::Assoc;
use helix_core::ChangeSet;

/// Diagnostics annotations are [`LineAnnotation`]s embed below the first line of the diagnostic
/// they're about.
///
/// Below is an example in plain text of the expect result:
///
/// ```text
/// use std::alloc::{alloc, Layout};
///                       │ └─── unused import: `Layout`
///                       │      `#[warn(unused_imports)]` on by default
///                       └─── remove the unused import
///
/// fn main() {
///     match std::cmp::Ordering::Less {
///     └─── any code following this `match` expression is unreachable, as all arms diverge
///         std::cmp::Ordering::Less => todo!(),
///         std::cmp::Ordering:Equal => todo!(),
///                           │ └─── Syntax Error: expected `,`
///                           ├─── maybe write a path separator here: `::`
///                           ├─── expected one of `!`, `(`, `...`, `..=`, `..`, `::`, `{`, or `|`, found `:`
///                           │    expected one of 8 possible tokens
///                           ├─── Syntax Error: expected expression
///                           └─── Syntax Error: expected FAT_ARROW
///         std::cmp::Ordering::Greater => todo!(),
///     }
///  
///     let layout: Layout = Layou::new::<String>();
///     │                    ├─── a struct with a similar name exists: `Layout`
///     │                    └─── failed to resolve: use of undeclared type `Layou`
///     │                         use of undeclared type `Layou`
///     └─── unreachable statement
///          `#[warn(unreachable_code)]` on by default
/// }
/// ```
pub struct DiagnosticAnnotations {
    /// The `LineAnnotation` don't contain any text, they're simply used to reserve the space for display.
    pub annotations: Rc<[LineAnnotation]>,

    /// The messages are the text linked to the `annotations`.
    ///
    /// To make the work of the renderer less costly, this must maintain a sort order following
    /// [`DiagnosticAnnotationMessage.anchor_char_idx`].
    ///
    /// The function [`diagnostic_inline_messages_from_diagnostics()`] can be used to do this.
    pub messages: Rc<[DiagnosticAnnotationMessage]>,
}

/// A `DiagnosticAnnotationMessage` is a single diagnostic to be displayed inline.
#[derive(Debug)]
pub struct DiagnosticAnnotationMessage {
    /// `line` is used to quickly gather all the diagnostics for a line.
    pub line: usize,
    /// The anchor is where the diagnostic is positioned in the document. This is used to compute
    /// the exact column for rendering after taking virtual text into account.
    pub anchor_char_idx: usize,
    /// The message to display. It can contain line breaks so be careful when displaying them.
    pub message: Rc<String>,
    /// The diagnostic's severity, to get the relevant style at rendering time.
    pub severity: Option<Severity>,
}

impl Default for DiagnosticAnnotations {
    fn default() -> Self {
        Self {
            annotations: Vec::new().into(),
            messages: Vec::new().into(),
        }
    }
}

/// Compute the list of `DiagnosticAnnotationMessage`s from the diagnostics.
pub fn diagnostic_inline_messages_from_diagnostics(
    diagnostics: &[helix_core::Diagnostic],
) -> Rc<[DiagnosticAnnotationMessage]> {
    let mut res = Vec::with_capacity(diagnostics.len());

    for diag in diagnostics {
        res.push(DiagnosticAnnotationMessage {
            line: diag.line,
            anchor_char_idx: diag.range.start,
            message: Rc::clone(&diag.message),
            severity: diag.severity,
        });
    }

    res.sort_unstable_by_key(|a| a.anchor_char_idx);

    res.into()
}

/// Used in [`super::Document::apply_impl()`] to recompute the inline diagnostics after changes have
/// been made to the document.
///
/// **Must be called with sorted diagnostics.**
pub(super) fn apply_changes_to_diagnostic_annotations(
    doc: &mut super::Document,
    changes: &ChangeSet,
) {
    // Only recompute if they're not empty since being empty probably means the annotation are
    // disabled, no need to build them in this case (building them would not display them since the
    // line annotations list is empty too in this case).

    match Rc::get_mut(&mut doc.diagnostic_annotations.messages) {
        // If for some reason we can't update the annotations, just delete them: the document is being saved and they
        // will be updated soon anyway.
        None | Some([]) => {
            doc.diagnostic_annotations = Default::default();
            return;
        }
        Some(messages) => {
            // The diagnostics have been sorted after being updated in `Document::apply_impl()` but nothing got deleted
            // so simply use the same order for the annotation messages.
            for (diag, message) in doc.diagnostics.iter().zip(messages.iter_mut()) {
                let DiagnosticAnnotationMessage {
                    line,
                    anchor_char_idx,
                    message,
                    severity,
                } = message;

                *line = diag.line;
                *anchor_char_idx = diag.range.start;
                *message = Rc::clone(&diag.message);
                *severity = diag.severity;
            }
        }
    }

    match Rc::get_mut(&mut doc.diagnostic_annotations.annotations) {
        // See `None` case above
        None | Some([]) => doc.diagnostic_annotations = Default::default(),
        Some(line_annotations) => {
            let map_pos =
                |annot: &LineAnnotation| changes.map_pos(annot.anchor_char_idx, Assoc::After);

            // The algorithm here does its best to modify in place to avoid reallocations as much as possible
            //
            // 1) We know the line annotations are non-empty because we checked for it in the match above.
            // 2) We update the first line annotation.
            // 3) For each subsequent annotation
            //    1) We compute its new anchor
            //    2) Logically, it cannot move further back than the previous one else the previous one would
            //       also have moved back more
            //    3) IF the new anchor is equal to the new anchor of the previous annotation, add the current one's
            //       height to the previous
            //    4) ELSE update the write position and write the current annotation (with updated anchor) there
            // 4) If the last write position was not the last member of the current lines annotations, it means we
            //    merged some of them together so we update the saved line annotations.

            let new_anchor_char_idx = map_pos(&line_annotations[0]);
            line_annotations[0].anchor_char_idx = new_anchor_char_idx;

            let mut previous_anchor_char_idx = new_anchor_char_idx;

            let mut writing_index = 0;

            for reading_index in 1..line_annotations.len() {
                let annot = &mut line_annotations[reading_index];
                let new_anchor_char_idx = map_pos(annot);

                if new_anchor_char_idx == previous_anchor_char_idx {
                    line_annotations[writing_index].height += annot.height;
                } else {
                    previous_anchor_char_idx = new_anchor_char_idx;

                    writing_index += 1;
                    line_annotations[writing_index].height = annot.height;
                    line_annotations[writing_index].anchor_char_idx = new_anchor_char_idx;
                }
            }

            // If we updated less annotations than there was previously, keep only those.
            if writing_index < line_annotations.len() - 1 {
                doc.diagnostic_annotations.annotations =
                    line_annotations[..=writing_index].to_vec().into();
            }
        }
    }
}
