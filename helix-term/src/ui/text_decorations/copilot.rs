use helix_core::doc_formatter::FormattedGrapheme;
use helix_core::Position;

use helix_view::theme::Style;
use helix_view::{Document, Theme, ViewId};

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

/// Draws the multi-line portion of a Copilot ghost text suggestion.
///
/// The first line of the suggestion is rendered inline after the cursor via an
/// `InlineAnnotation`; this decoration paints every subsequent line onto the
/// virtual lines reserved by
/// [`CopilotGhostText`](helix_view::annotations::copilot::CopilotGhostText)
/// directly below the cursor's document line. Each line is drawn verbatim so its
/// own leading whitespace/indentation is preserved.
pub struct InlineCopilot<'a> {
    cursor: usize,
    lines: &'a [String],
    style: Style,
    anchored: bool,
}

impl<'a> InlineCopilot<'a> {
    pub fn new(doc: &'a Document, theme: &Theme, view_id: ViewId) -> Option<Self> {
        let completion = doc
            .copilot_completion()
            .filter(|completion| completion.view_id == view_id)?;
        if completion.ghost_lines.is_empty() {
            return None;
        }
        let highlight = theme
            .find_highlight("ui.virtual.copilot")
            .or_else(|| theme.find_highlight("ui.virtual.inlay-hint"));
        let style = match highlight {
            Some(highlight) => theme.get("ui.text").patch(theme.highlight(highlight)),
            None => theme.get("ui.text"),
        };
        Some(InlineCopilot {
            cursor: completion.anchor,
            lines: &completion.ghost_lines,
            style,
            anchored: false,
        })
    }
}

impl Decoration for InlineCopilot<'_> {
    fn reset_pos(&mut self, pos: usize) -> usize {
        self.anchored = false;
        if self.cursor >= pos {
            self.cursor
        } else {
            usize::MAX
        }
    }

    fn skip_concealed_anchor(&mut self, conceal_end_char_idx: usize) -> usize {
        if self.cursor >= conceal_end_char_idx {
            self.cursor
        } else {
            usize::MAX
        }
    }

    fn decorate_grapheme(
        &mut self,
        _renderer: &mut TextRenderer,
        grapheme: &FormattedGrapheme,
    ) -> usize {
        if grapheme.char_idx == self.cursor {
            self.anchored = true;
        }
        usize::MAX
    }

    fn render_virt_lines(
        &mut self,
        renderer: &mut TextRenderer,
        pos: LinePos,
        virt_off: Position,
    ) -> Position {
        if !self.anchored {
            return Position::new(0, 0);
        }
        self.anchored = false;

        let first_row = pos.visual_line as usize + virt_off.row;
        let width = renderer.viewport.width as usize;
        let max_row = renderer.offset.row + renderer.viewport.height as usize;
        for (i, line) in self.lines.iter().enumerate() {
            let row = first_row + i;
            // Skip lines that fall outside the visible viewport so very tall
            // suggestions never draw past the text area (and never overflow).
            if row < renderer.offset.row || row >= max_row {
                continue;
            }
            renderer.set_string_truncated(
                renderer.viewport.x,
                row as u16,
                line,
                width,
                |_| self.style,
                false,
                false,
            );
        }
        Position::new(self.lines.len(), 0)
    }
}
