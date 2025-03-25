use std::collections::HashMap;

use helix_core::Position;

use helix_view::theme::Style;
use helix_view::Theme;

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

pub struct InlineBlame {
    lines: HashMap<usize, String>,
    style: Style,
}

impl InlineBlame {
    pub fn new(theme: &Theme, lines: HashMap<usize, String>) -> Self {
        InlineBlame {
            style: theme.get("ui.virtual.inline-blame"),
            lines,
        }
    }
}

impl Decoration for InlineBlame {
    fn render_virt_lines(
        &mut self,
        renderer: &mut TextRenderer,
        pos: LinePos,
        virt_off: Position,
    ) -> Position {
        let Some(blame) = self.lines.get(&pos.doc_line) else {
            // do not draw inline blame for lines that have no content in them
            return Position::new(0, 0);
        };

        // where the line in the document ends
        let end_of_line = virt_off.col as u16;
        // length of line in the document
        // draw the git blame 6 spaces after the end of the line
        let start_drawing_at = end_of_line + 6;

        let amount_of_characters_drawn = renderer
            .column_in_bounds(start_drawing_at as usize, 1)
            .then(|| {
                // the column where we stop drawing the blame
                let stopped_drawing_at = renderer
                    .set_string_truncated(
                        renderer.viewport.x + start_drawing_at,
                        pos.visual_line,
                        &blame,
                        renderer.viewport.width.saturating_sub(start_drawing_at) as usize,
                        |_| self.style,
                        true,
                        false,
                    )
                    .0;

                let line_length = end_of_line - renderer.offset.col as u16;

                stopped_drawing_at - line_length
            })
            .unwrap_or_default();

        Position::new(0, amount_of_characters_drawn as usize)
    }
}
