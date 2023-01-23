use std::cell::Cell;
use std::convert::identity;
use std::ops::Range;
use std::rc::Rc;

use crate::syntax::Highlight;
use crate::Tendril;

/// An inline annotation is continuos text show
/// on the screen before the grapheme that starts at
/// `char_idx`
#[derive(Debug, Clone)]
pub struct InlineAnnotation {
    pub text: Tendril,
    pub char_idx: usize,
}

/// Represents a *single Grapheme** that is part of the document
/// that start at `char_idx` that will be replaced with
/// a different `grapheme`.
/// If `grapheme` contains multiple graphemes the text
/// will render incorrectly.
/// If you want to overlay multiple graphemes simply
/// use multiple `Overlays`.
///
/// # Examples
///
/// The following examples are valid overlays for the following text:
///
/// `aX͎̊͢͜͝͡bc`
///
/// ```
/// use helix_core::text_annotations::Overlay;
///
/// // replaces a
/// Overlay {
///   char_idx: 0,
///   grapheme: "X".into(),
/// };
///
/// // replaces X͎̊͢͜͝͡
/// Overlay{
///   char_idx: 1,
///   grapheme: "\t".into(),
/// };
///
/// // replaces b
/// Overlay{
///   char_idx: 6,
///   grapheme: "X̢̢̟͖̲͌̋̇͑͝".into(),
/// };
/// ```
///
/// The following examples are invalid uses
///
/// ```
/// use helix_core::text_annotations::Overlay;
///
/// // overlay is not aligned at grapheme boundary
/// Overlay{
///   char_idx: 3,
///   grapheme: "x".into(),
/// };
///
/// // overlay contains multiple graphemes
/// Overlay{
///   char_idx: 0,
///   grapheme: "xy".into(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Overlay {
    pub char_idx: usize,
    pub grapheme: Tendril,
}

/// Line annotations allow for virtual text between normal
/// text lines. They cause `height` empty lines to be inserted
/// below the document line that contains `anchor_char_idx`.
///
/// These lines can be filled with text in the rendering code.
/// as their contents have no effect beyond visual appearance.
///
/// To insert a line after a documet line simply set
/// `anchor_char_idx` to `doc.line_to_char(line_idx)`
#[derive(Debug, Clone)]
pub struct LineAnnotation {
    pub anchor_char_idx: usize,
    pub height: usize,
}

#[derive(Debug)]
struct Layer<A, M> {
    annotations: Rc<[A]>,
    current_index: Cell<usize>,
    metadata: M,
}

impl<'a, A, M: Clone> Clone for Layer<A, M> {
    fn clone(&self) -> Self {
        Layer {
            annotations: self.annotations.clone(),
            current_index: self.current_index.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl<A, M> Layer<A, M> {
    pub fn reset_pos(&self, char_idx: usize, get_char_idx: impl Fn(&A) -> usize) {
        let new_index = self
            .annotations
            .binary_search_by_key(&char_idx, get_char_idx)
            .unwrap_or_else(identity);

        self.current_index.set(new_index);
    }

    pub fn consume(&self, char_idx: usize, get_char_idx: impl Fn(&A) -> usize) -> Option<&A> {
        let annot = self.annotations.get(self.current_index.get())?;
        debug_assert!(get_char_idx(annot) >= char_idx);
        if get_char_idx(annot) == char_idx {
            self.current_index.set(self.current_index.get() + 1);
            Some(annot)
        } else {
            None
        }
    }
}

impl<A, M> From<(Rc<[A]>, M)> for Layer<A, M> {
    fn from((annotations, metadata): (Rc<[A]>, M)) -> Layer<A, M> {
        Layer {
            annotations,
            current_index: Cell::new(0),
            metadata,
        }
    }
}

fn reset_pos<A, M>(layers: &[Layer<A, M>], pos: usize, get_pos: impl Fn(&A) -> usize) {
    for layer in layers {
        layer.reset_pos(pos, &get_pos)
    }
}

/// Annotations that change that is displayed when the document is render.
/// Also commonly called virtual text.
#[derive(Default, Debug, Clone)]
pub struct TextAnnotations {
    inline_annotations: Vec<Layer<InlineAnnotation, Option<Highlight>>>,
    overlays: Vec<Layer<Overlay, Option<Highlight>>>,
    line_annotations: Vec<Layer<LineAnnotation, ()>>,
}

impl TextAnnotations {
    /// Prepare the TextAnnotations for iteration starting at char_idx
    pub fn reset_pos(&self, char_idx: usize) {
        reset_pos(&self.inline_annotations, char_idx, |annot| annot.char_idx);
        reset_pos(&self.overlays, char_idx, |annot| annot.char_idx);
        reset_pos(&self.line_annotations, char_idx, |annot| {
            annot.anchor_char_idx
        });
    }

    pub fn collect_overlay_highlights(
        &self,
        char_range: Range<usize>,
    ) -> Vec<(usize, Range<usize>)> {
        let mut highlights = Vec::new();
        self.reset_pos(char_range.start);
        for char_idx in char_range {
            if let Some((_, Some(highlight))) = self.overlay_at(char_idx) {
                // we don't know the number of chars the original grapheme takes
                // however it doesn't matter as highlight bounderies are automatically
                // aligned to grapheme boundaries in the rendering code
                highlights.push((highlight.0, char_idx..char_idx + 1))
            }
        }

        highlights
    }

    /// Add new inline annotations.
    ///
    /// The annotations grapheme will be rendered with `highlight`
    /// patched on top of `ui.text`.
    ///
    /// The annotations **must be sorted** by their `char_idx`.
    /// Multiple annotations with the same `char_idx` are allowed,
    /// they will be display in the order that they are present in the layer.
    ///
    /// If multiple layers contain annotations at the same position
    /// the annotations that belong to the layers added first will be shown first.
    pub fn add_inline_annotations(
        &mut self,
        layer: Rc<[InlineAnnotation]>,
        highlight: Option<Highlight>,
    ) -> &mut Self {
        self.inline_annotations.push((layer, highlight).into());
        self
    }

    /// Add new grapheme overlays.
    ///
    /// The overlayed grapheme will be rendered with `highlight`
    /// patched on top of `ui.text`.
    ///
    /// The overlays **must be sorted** by their `char_idx`.
    /// Multiple overlays with the same `char_idx` **are allowed**.
    ///
    /// If multiple layers contain overlay at the same position
    /// the overlay from the layer added last will be show.
    pub fn add_overlay(&mut self, layer: Rc<[Overlay]>, highlight: Option<Highlight>) -> &mut Self {
        self.overlays.push((layer, highlight).into());
        self
    }

    /// Add new annotation lines.
    ///
    /// The line annotations **must be sorted** by their `char_idx`.
    /// Multiple line annotations with the smame `char_idx` **are not allowed**.
    pub fn add_line_annotation(&mut self, layer: Rc<[LineAnnotation]>) -> &mut Self {
        self.line_annotations.push((layer, ()).into());
        self
    }

    /// Removes all line annotations, useful for vertical motions
    /// so that virtual text lines are automatically skipped.
    pub fn clear_line_annotations(&mut self) {
        self.line_annotations.clear();
    }

    pub(crate) fn next_inline_annotation_at(
        &self,
        char_idx: usize,
    ) -> Option<(&InlineAnnotation, Option<Highlight>)> {
        self.inline_annotations.iter().find_map(|layer| {
            let annotation = layer.consume(char_idx, |annot| annot.char_idx)?;
            Some((annotation, layer.metadata))
        })
    }

    pub(crate) fn overlay_at(&self, char_idx: usize) -> Option<(&Overlay, Option<Highlight>)> {
        let mut overlay = None;
        for layer in &self.overlays {
            while let Some(new_overlay) = layer.consume(char_idx, |annot| annot.char_idx) {
                overlay = Some((new_overlay, layer.metadata));
            }
        }
        overlay
    }

    pub(crate) fn annotation_lines_at(&self, char_idx: usize) -> usize {
        self.line_annotations
            .iter()
            .map(|layer| {
                let mut lines = 0;
                while let Some(annot) = layer.annotations.get(layer.current_index.get()) {
                    if annot.anchor_char_idx == char_idx {
                        layer.current_index.set(layer.current_index.get() + 1);
                        lines += annot.height
                    } else {
                        break;
                    }
                }
                lines
            })
            .sum()
    }
}
