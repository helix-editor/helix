use std::borrow::Cow;

use crate::graphics::Style;

#[derive(Clone, Copy, PartialEq)]
pub enum TextAnnotationKind {
    /// Add to end of line
    Eol,
    /// Replace actual text or arbitary cells with annotations.
    /// Specifies an offset from the 0th column.
    Overlay(usize),
}

impl TextAnnotationKind {
    pub fn is_eol(&self) -> bool {
        *self == Self::Eol
    }

    pub fn is_overlay(&self) -> bool {
        matches!(*self, Self::Overlay(_))
    }
}

/// Namespaces and identifes similar annotations
pub type TextAnnotationGroup = &'static str;

pub struct TextAnnotation {
    pub text: Cow<'static, str>,
    pub style: Style,
    pub line: usize,
    pub kind: TextAnnotationKind,
}
