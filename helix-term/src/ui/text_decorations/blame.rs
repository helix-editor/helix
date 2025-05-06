use helix_core::Position;

use helix_view::theme::Style;

use crate::ui::document::{LinePos, TextRenderer};
use crate::ui::text_decorations::Decoration;

pub enum LineBlame {
    OneLine((usize, String)),
    // Optimization: Use `Vec<T>` insted of `HashMap<usize, T>`
    // because we know that the amount of lines visible in the viewport X3 cannot be a very large number,
    // most likely up to a few hundred. In the absolute extreme case, maybe 5,000.
    ManyLines(Vec<Option<String>>),
}

pub struct InlineBlame {
    lines: LineBlame,
    style: Style,
}

impl InlineBlame {
    pub fn new(style: Style, lines: LineBlame) -> Self {
        InlineBlame { style, lines }
    }
}

impl Decoration for InlineBlame {
    fn render_virt_lines(
        &mut self,
        renderer: &mut TextRenderer,
        pos: LinePos,
        virt_off: Position,
    ) -> Position {
        // Only render inline blame on the last visual line
        // This prevents it being rendered multiple times per line, if the
        // line is wrapping.
        //
        // We want to render it on the last visual line because unless the user
        // has `soft-wrap.wrap-at-text-width` enabled, for a document line that wraps
        // there will only be space to render inline blame at the last doc line
        if !pos.is_last_visual_line {
            return Position::new(0, 0);
        }

        let blame = match &self.lines {
            LineBlame::OneLine((line, blame)) => {
                if line == &pos.doc_line {
                    // do not draw inline blame for lines that have no content in them
                    blame
                } else {
                    return Position::new(0, 0);
                }
            }
            LineBlame::ManyLines(lines) => {
                if let Some(Some(blame)) = lines.get(pos.doc_line) {
                    blame
                } else {
                    // do not draw inline blame for lines that have no content in them
                    return Position::new(0, 0);
                }
            }
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
                        blame,
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
