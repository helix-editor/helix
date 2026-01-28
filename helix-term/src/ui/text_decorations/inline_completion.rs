use helix_core::unicode::segmentation::UnicodeSegmentation;
use helix_core::unicode::width::UnicodeWidthStr;
use helix_core::Position;
use helix_view::theme::Style;

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

/// Decoration for rendering inline completion ghost text.
/// Handles EOL first-line ghost text, mid-line overflow, and multi-line additional lines.
pub struct InlineCompletionDecoration<'a> {
    /// Document line where the cursor is (where the completion starts).
    cursor_doc_line: usize,
    /// First line ghost text when at EOL (rendered at end of line).
    eol_ghost_text: Option<&'a str>,
    /// Overflow text for mid-line (preview chars beyond original line length).
    overflow_text: Option<&'a str>,
    /// Additional lines to render (after the first line).
    additional_lines: &'a [String],
    /// Style for ghost text.
    style: Style,
    /// Cursor style for first grapheme at EOL (to appear "on" block cursor).
    cursor_style: Option<Style>,
    /// Track if we've already rendered for this line.
    rendered: bool,
}

impl<'a> InlineCompletionDecoration<'a> {
    pub fn new(
        cursor_doc_line: usize,
        eol_ghost_text: Option<&'a str>,
        overflow_text: Option<&'a str>,
        additional_lines: &'a [String],
        style: Style,
        cursor_style: Option<Style>,
    ) -> Self {
        Self {
            cursor_doc_line,
            eol_ghost_text,
            overflow_text,
            additional_lines,
            style,
            cursor_style,
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
            // Subtract 1 because virt_off.col includes newline width, but cursor is ON the newline cell
            let col_pos = virt_off.col.saturating_sub(1);
            let mut col = renderer.viewport.x + col_pos as u16;
            let max_width = renderer.viewport.width.saturating_sub(col_pos as u16) as usize;

            let mut graphemes = eol_text.graphemes(true);

            // First grapheme with cursor style (appears "on" block cursor)
            if let Some(first_g) = graphemes.next() {
                let first_style = self.cursor_style.unwrap_or(self.style);
                let first_width = first_g.width();
                renderer.set_string_truncated(
                    col,
                    pos.visual_line,
                    first_g,
                    max_width,
                    |_| first_style,
                    true,
                    false,
                );
                col += first_width as u16;
                col_offset += first_width;
            }

            // Rest of ghost text with normal ghost style
            let rest: String = graphemes.collect();
            if !rest.is_empty() {
                let remaining_width = max_width.saturating_sub(col_offset);
                renderer.set_string_truncated(
                    col,
                    pos.visual_line,
                    &rest,
                    remaining_width,
                    |_| self.style,
                    true,
                    false,
                );
                col_offset += rest.width();
            }
        }

        // Render mid-line overflow at EOL (preview chars beyond original line length)
        if let Some(overflow) = self.overflow_text {
            // Render at end of line (virt_off.col - 1 for newline cell)
            let col_pos = virt_off.col.saturating_sub(1);
            let col = renderer.viewport.x + col_pos as u16;
            let max_width = renderer.viewport.width.saturating_sub(col_pos as u16) as usize;

            renderer.set_string_truncated(
                col,
                pos.visual_line,
                overflow,
                max_width,
                |_| self.style,
                true,
                false,
            );
            col_offset += overflow.width();
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
