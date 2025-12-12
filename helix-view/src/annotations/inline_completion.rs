use std::cell::Cell;

use helix_core::text_annotations::LineAnnotation;
use helix_core::Position;

/// LineAnnotation implementation for multi-line inline completion ghost text.
/// Reserves virtual lines after the cursor line for additional ghost text lines.
pub struct InlineCompletionLines {
    /// Document line where the cursor is (where the completion starts).
    cursor_doc_line: usize,
    /// Number of additional lines to render after the cursor line.
    additional_lines_count: usize,
    /// Track if we've already reserved lines for this render pass.
    reserved: Cell<bool>,
}

impl InlineCompletionLines {
    pub fn new(cursor_doc_line: usize, additional_lines: &[String]) -> Box<dyn LineAnnotation> {
        Box::new(Self {
            cursor_doc_line,
            additional_lines_count: additional_lines.len(),
            reserved: Cell::new(false),
        })
    }
}

impl LineAnnotation for InlineCompletionLines {
    fn reset_pos(&mut self, _char_idx: usize) -> usize {
        self.reserved.set(false);
        usize::MAX
    }

    fn insert_virtual_lines(
        &mut self,
        _line_end_char_idx: usize,
        _line_end_visual_pos: Position,
        doc_line: usize,
    ) -> Position {
        // Only reserve lines on the cursor's document line
        if doc_line == self.cursor_doc_line && !self.reserved.get() {
            self.reserved.set(true);
            return Position::new(self.additional_lines_count, 0);
        }
        Position::new(0, 0)
    }
}
