use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::Range;
use std::ptr::NonNull;

use crate::doc_formatter::FormattedGrapheme;
use crate::syntax::{Highlight, OverlayHighlights};
use crate::{Position, Tendril};

/// An inline annotation is continuous text shown
/// on the screen before the grapheme that starts at
/// `char_idx`
#[derive(Debug, Clone)]
pub struct InlineAnnotation {
    pub text: Tendril,
    pub char_idx: usize,
}

impl InlineAnnotation {
    pub fn new(char_idx: usize, text: impl Into<Tendril>) -> Self {
        Self {
            char_idx,
            text: text.into(),
        }
    }
}

/// Represents a **single Grapheme** that is part of the document
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
/// Overlay::new(0, "X");
///
/// // replaces X͎̊͢͜͝͡
/// Overlay::new(1, "\t");
///
/// // replaces b
/// Overlay::new(6, "X̢̢̟͖̲͌̋̇͑͝");
/// ```
///
/// The following examples are invalid uses
///
/// ```
/// use helix_core::text_annotations::Overlay;
///
/// // overlay is not aligned at grapheme boundary
/// Overlay::new(3, "x");
///
/// // overlay contains multiple graphemes
/// Overlay::new(0, "xy");
/// ```
#[derive(Debug, Clone)]
pub struct Overlay {
    pub char_idx: usize,
    pub grapheme: Tendril,
}

impl Overlay {
    pub fn new(char_idx: usize, grapheme: impl Into<Tendril>) -> Self {
        Self {
            char_idx,
            grapheme: grapheme.into(),
        }
    }
}

/// Line annotations allow inserting virtual text lines between normal text
/// lines.  These lines can be filled with text in the rendering code as their
/// contents have no effect beyond visual appearance.
///
/// The height of virtual text is usually not known ahead of time as virtual
/// text often requires softwrapping. Furthermore the height of some virtual
/// text like side-by-side diffs depends on the height of the text (again
/// influenced by softwrap) and other virtual text. Therefore line annotations
/// are computed on the fly instead of ahead of time like other annotations.
///
/// The core of this trait `insert_virtual_lines` function. It is called at the
/// end of every  visual line and allows the `LineAnnotation` to insert empty
/// virtual lines. Apart from that the `LineAnnotation` trait has multiple
/// methods that allow it to track anchors in the document.
///
/// When a new traversal of a document starts `reset_pos` is called. Afterwards
/// the other functions are called with indices that are larger then the
/// one passed to `reset_pos`. This allows performing a binary search (use
/// `partition_point`) in `reset_pos` once and then to only look at the next
/// anchor during each method call.
///
/// The `reset_pos`, `skip_conceal` and `process_anchor` functions all return a
/// `char_idx` anchor. This anchor is stored when transversing the document and
/// when the grapheme at the anchor is traversed the `process_anchor` function
/// is called.
///
/// # Note
///
/// All functions only receive immutable references to `self`.
/// `LineAnnotation`s that want to store an internal position or
/// state of some kind should use `Cell`. Using interior mutability for
/// caches is preferable as otherwise a lot of lifetimes become invariant
/// which complicates APIs a lot.
pub trait LineAnnotation {
    /// Resets the internal position to `char_idx`. This function is called
    /// when a new traversal of a document starts.
    ///
    /// All `char_idx` passed to `insert_virtual_lines` are strictly monotonically increasing
    /// with the first `char_idx` greater or equal to the `char_idx`
    /// passed to this function.
    ///
    /// # Returns
    ///
    /// The `char_idx` of the next anchor this `LineAnnotation` is interested in,
    /// replaces the currently registered anchor. Return `usize::MAX` to ignore
    fn reset_pos(&mut self, _char_idx: usize) -> usize {
        usize::MAX
    }

    /// Called when a text is concealed that contains an anchor registered by this `LineAnnotation`.
    /// In this case the line decorations  **must** ensure that virtual text anchored within that
    /// char range is skipped.
    ///
    /// # Returns
    ///
    /// The `char_idx` of the next anchor this `LineAnnotation` is interested in,
    /// **after the end of conceal_end_char_idx**
    /// replaces the currently registered anchor. Return `usize::MAX` to ignore
    fn skip_concealed_anchors(&mut self, conceal_end_char_idx: usize) -> usize {
        self.reset_pos(conceal_end_char_idx)
    }

    /// Process an anchor (horizontal position is provided) and returns the next anchor.
    ///
    /// # Returns
    ///
    /// The `char_idx` of the next anchor this `LineAnnotation` is interested in,
    /// replaces the currently registered anchor. Return `usize::MAX` to ignore
    fn process_anchor(&mut self, _grapheme: &FormattedGrapheme) -> usize {
        usize::MAX
    }

    /// This function is called at the end of a visual line to insert virtual text
    ///
    /// # Returns
    ///
    /// The number of additional virtual lines to reserve
    ///
    /// # Note
    ///
    /// The `line_end_visual_pos` parameter indicates the visual vertical distance
    /// from the start of block where the traversal starts.  This includes the offset
    /// from other `LineAnnotations`. This allows inline annotations to consider
    /// the height of the text and "align" two different documents (like for side
    /// by side diffs).  These annotations that want to "align" two documents should
    /// therefore be added last so that other virtual text is also considered while aligning
    fn insert_virtual_lines(
        &mut self,
        line_end_char_idx: usize,
        line_end_visual_pos: Position,
        doc_line: usize,
    ) -> Position;
}

#[derive(Debug)]
struct Layer<'a, A, M> {
    annotations: &'a [A],
    current_index: Cell<usize>,
    metadata: M,
}

impl<A, M: Clone> Clone for Layer<'_, A, M> {
    fn clone(&self) -> Self {
        Layer {
            annotations: self.annotations,
            current_index: self.current_index.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl<A, M> Layer<'_, A, M> {
    pub fn reset_pos(&self, char_idx: usize, get_char_idx: impl Fn(&A) -> usize) {
        let new_index = self
            .annotations
            .partition_point(|annot| get_char_idx(annot) < char_idx);
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

impl<'a, A, M> From<(&'a [A], M)> for Layer<'a, A, M> {
    fn from((annotations, metadata): (&'a [A], M)) -> Layer<'a, A, M> {
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

/// Safety: We store LineAnnotation in a NonNull pointer. This is necessary to work
/// around an unfortunate inconsistency in rusts variance system that unnnecesarily
/// makes the lifetime invariant if implemented with safe code. This makes the
/// DocFormatter API very cumbersome/basically impossible to work with.
///
/// Normally object types `dyn Foo + 'a` are covariant so if we used `Box<dyn LineAnnotation + 'a>` below
/// everything would be alright. However we want to use `Cell<Box<dyn LineAnnotation + 'a>>`
/// to be able to call the mutable function on `LineAnnotation`. The problem is that
/// some types like `Cell` make all their arguments invariant. This is important for soundness
/// normally for the same reasons that `&'a mut T` is invariant over `T`
/// (see <https://doc.rust-lang.org/nomicon/subtyping.html>). However for `&'a mut` (`dyn Foo + 'b`)
/// there is a specical rule in the language to make `'b` covariant (otherwise trait objects would be
/// super annoying to use). See  <https://users.rust-lang.org/t/solved-variance-of-dyn-trait-a> for
/// why this is sound. Sadly that rule doesn't apply to `Cell<Box<(dyn Foo + 'a)>`
/// (or other invariant types like `UnsafeCell` or `*mut (dyn Foo + 'a)`).
///
/// We sidestep the problem by using `NonNull` which is covariant. In the
/// special case of trait objects this is sound (easily checked by adding a
/// `PhantomData<&'a mut Foo + 'a)>` field). We don't need an explicit `Cell`
/// type here because we never hand out any refereces to the trait objects. That
/// means any reference to the pointer can create a valid multable reference
/// that is covariant over `'a` (or in other words it's a raw pointer, as long as
/// we don't hand out references we are free to do whatever we want).
struct RawBox<T: ?Sized>(NonNull<T>);

impl<T: ?Sized> RawBox<T> {
    /// Safety: Only a single mutable reference
    /// created by this function may exist at a given time.
    #[allow(clippy::mut_from_ref)]
    unsafe fn get(&self) -> &mut T {
        &mut *self.0.as_ptr()
    }
}
impl<T: ?Sized> From<Box<T>> for RawBox<T> {
    fn from(box_: Box<T>) -> Self {
        // obviously safe because Box::into_raw never returns null
        unsafe { Self(NonNull::new_unchecked(Box::into_raw(box_))) }
    }
}

impl<T: ?Sized> Drop for RawBox<T> {
    fn drop(&mut self) {
        unsafe { drop(Box::from_raw(self.0.as_ptr())) }
    }
}

/// Annotations that change that is displayed when the document is render.
/// Also commonly called virtual text.
#[derive(Default)]
pub struct TextAnnotations<'a> {
    inline_annotations: Vec<Layer<'a, InlineAnnotation, Option<Highlight>>>,
    overlays: Vec<Layer<'a, Overlay, Option<Highlight>>>,
    line_annotations: Vec<(Cell<usize>, RawBox<dyn LineAnnotation + 'a>)>,
}

impl Debug for TextAnnotations<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextAnnotations")
            .field("inline_annotations", &self.inline_annotations)
            .field("overlays", &self.overlays)
            .finish_non_exhaustive()
    }
}

impl<'a> TextAnnotations<'a> {
    /// Prepare the TextAnnotations for iteration starting at char_idx
    pub fn reset_pos(&self, char_idx: usize) {
        reset_pos(&self.inline_annotations, char_idx, |annot| annot.char_idx);
        reset_pos(&self.overlays, char_idx, |annot| annot.char_idx);
        for (next_anchor, layer) in &self.line_annotations {
            next_anchor.set(unsafe { layer.get().reset_pos(char_idx) });
        }
    }

    pub fn collect_overlay_highlights(&self, char_range: Range<usize>) -> OverlayHighlights {
        let mut highlights = Vec::new();
        self.reset_pos(char_range.start);
        for char_idx in char_range {
            if let Some((_, Some(highlight))) = self.overlay_at(char_idx) {
                // we don't know the number of chars the original grapheme takes
                // however it doesn't matter as highlight boundaries are automatically
                // aligned to grapheme boundaries in the rendering code
                highlights.push((highlight, char_idx..char_idx + 1));
            }
        }

        OverlayHighlights::Heterogenous { highlights }
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
        layer: &'a [InlineAnnotation],
        highlight: Option<Highlight>,
    ) -> &mut Self {
        if !layer.is_empty() {
            self.inline_annotations.push((layer, highlight).into());
        }
        self
    }

    /// Add new grapheme overlays.
    ///
    /// The overlaid grapheme will be rendered with `highlight`
    /// patched on top of `ui.text`.
    ///
    /// The overlays **must be sorted** by their `char_idx`.
    /// Multiple overlays with the same `char_idx` **are allowed**.
    ///
    /// If multiple layers contain overlay at the same position
    /// the overlay from the layer added last will be show.
    pub fn add_overlay(&mut self, layer: &'a [Overlay], highlight: Option<Highlight>) -> &mut Self {
        if !layer.is_empty() {
            self.overlays.push((layer, highlight).into());
        }
        self
    }

    /// Add new annotation lines.
    ///
    /// The line annotations **must be sorted** by their `char_idx`.
    /// Multiple line annotations with the same `char_idx` **are not allowed**.
    pub fn add_line_annotation(&mut self, layer: Box<dyn LineAnnotation + 'a>) -> &mut Self {
        self.line_annotations
            .push((Cell::new(usize::MAX), layer.into()));
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

    pub(crate) fn process_virtual_text_anchors(&self, grapheme: &FormattedGrapheme) {
        for (next_anchor, layer) in &self.line_annotations {
            loop {
                match next_anchor.get().cmp(&grapheme.char_idx) {
                    Ordering::Less => next_anchor
                        .set(unsafe { layer.get().skip_concealed_anchors(grapheme.char_idx) }),
                    Ordering::Equal => {
                        next_anchor.set(unsafe { layer.get().process_anchor(grapheme) })
                    }
                    Ordering::Greater => break,
                };
            }
        }
    }

    pub(crate) fn virtual_lines_at(
        &self,
        char_idx: usize,
        line_end_visual_pos: Position,
        doc_line: usize,
    ) -> usize {
        let mut virt_off = Position::new(0, 0);
        for (_, layer) in &self.line_annotations {
            virt_off += unsafe {
                layer
                    .get()
                    .insert_virtual_lines(char_idx, line_end_visual_pos + virt_off, doc_line)
            };
        }
        virt_off.row
    }
}
