//! This module contains the various annotations that can be added to a [`super::Document`] when
//! displaying it.
//!
//! Examples: inline diagnostics, inlay hints, git blames.

use std::rc::Rc;

use helix_core::diagnostic::Severity;
use helix_core::text_annotations::LineAnnotation;

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
