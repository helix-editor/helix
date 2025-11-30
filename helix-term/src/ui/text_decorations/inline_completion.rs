use helix_core::unicode::width::UnicodeWidthStr;
use helix_core::Position;
use helix_view::theme::Style;

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

/// Decoration for rendering inline completion ghost text.
/// Handles both EOL first-line ghost text and multi-line additional lines.
pub struct InlineCompletionDecoration<'a> {
    /// Document line where the cursor is (where the completion starts).
    cursor_doc_line: usize,
    /// First line ghost text when at EOL (rendered at end of line).
    eol_ghost_text: Option<&'a str>,
    /// Additional lines to render (after the first line).
    additional_lines: &'a [String],
    /// Style for ghost text.
    style: Style,
    /// Track if we've already rendered for this line.
    rendered: bool,
}

impl<'a> InlineCompletionDecoration<'a> {
    pub fn new(
        cursor_doc_line: usize,
        eol_ghost_text: Option<&'a str>,
        additional_lines: &'a [String],
        style: Style,
    ) -> Self {
        Self {
            cursor_doc_line,
            eol_ghost_text,
            additional_lines,
            style,
            rendered: false,
        }
    }
}

impl Decoration for InlineCompletionDecoration<'_> {
    fn reset_pos(&mut self, _pos: usize) -> usize {
        self.rendered = false;
        usize::MAX
    }

    fn render_virt_lines(
        &mut self,
        renderer: &mut TextRenderer,
        pos: LinePos,
        virt_off: Position,
    ) -> Position {
        // Only render on the cursor's document line
        if pos.doc_line != self.cursor_doc_line || self.rendered {
            return Position::new(0, 0);
        }

        self.rendered = true;

        let mut col_offset = 0;

        // Render EOL first-line ghost text at end of current line
        if let Some(eol_text) = self.eol_ghost_text {
            let col = renderer.viewport.x + virt_off.col as u16;
            renderer.set_string_truncated(
                col,
                pos.visual_line,
                eol_text,
                renderer.viewport.width.saturating_sub(virt_off.col as u16) as usize,
                |_| self.style,
                true,
                false,
            );
            // Return ghost text width so diagnostics shift
            col_offset = eol_text.width();
        }

        // Render each additional line in the virtual line space
        for (i, line) in self.additional_lines.iter().enumerate() {
            let row = pos.visual_line + virt_off.row as u16 + i as u16;
            renderer.set_string_truncated(
                renderer.viewport.x,
                row,
                line,
                renderer.viewport.width as usize,
                |_| self.style,
                true,
                false,
            );
        }

        Position::new(self.additional_lines.len(), col_offset)
    }
}
