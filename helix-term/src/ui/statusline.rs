use helix_core::{coords_at_pos, encoding};
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::Rect,
    theme::Style,
    Document, Editor, Theme, View,
};

use crate::ui::ProgressSpinners;

use helix_view::editor::StatusLineElement as StatusLineElementID;
use tui::buffer::Buffer as Surface;
use tui::text::{Span, Spans};

/// A status line element contains the information about a component which can be displayed in the status line.
struct StatusLineElement {
    /// The element
    pub text: String,

    /// The style to be used to render the element (this style will be merged with the base style).
    /// If not set, a default base style will be used.
    pub style: Option<Style>,
}

pub struct RenderContext<'a> {
    pub doc: &'a Document,
    pub view: &'a View,
    pub theme: &'a Theme,
    pub focused: bool,
    pub spinners: &'a ProgressSpinners,
}

struct RenderBuffer<'a> {
    pub left: Spans<'a>,
    pub center: Spans<'a>,
    pub right: Spans<'a>,
}

impl<'a> RenderBuffer<'a> {
    pub fn new() -> Self {
        Self {
            left: Spans::default(),
            center: Spans::default(),
            right: Spans::default(),
        }
    }
}

pub struct StatusLine;

impl StatusLine {
    pub fn render(editor: &Editor, context: &RenderContext, viewport: Rect, surface: &mut Surface) {
        let mut buffer = RenderBuffer::new();

        let base_style = if context.focused {
            context.theme.get("ui.statusline")
        } else {
            context.theme.get("ui.statusline.inactive")
        };

        surface.set_style(viewport.with_height(1), base_style);

        // Left side of the status line.

        for element_id in &editor.config().status_line.left {
            let elements = Self::render_element(context, *element_id);
            for element in elements {
                Self::append_left(&mut buffer, &base_style, element);
            }
        }

        surface.set_spans(
            viewport.x,
            viewport.y,
            &buffer.left,
            buffer.left.width() as u16,
        );

        // Right side of the status line.

        for element_id in &editor.config().status_line.right {
            let elements = Self::render_element(context, *element_id);
            for element in elements {
                Self::append_right(&mut buffer, &base_style, element);
            }
        }

        surface.set_spans(
            viewport.x + viewport.width.saturating_sub(buffer.right.width() as u16),
            viewport.y,
            &buffer.right,
            buffer.right.width() as u16,
        );

        // Center of the status line.

        for element_id in &editor.config().status_line.center {
            let elements = Self::render_element(context, *element_id);
            for element in elements {
                Self::append_center(&mut buffer, &base_style, element);
            }
        }

        // Width of the empty space between the left and center area and between the center and right area.
        let spacing = 1u16;

        let edge_width = buffer.left.width().max(buffer.right.width()) as u16;
        let center_max_width = viewport.width - (2 * edge_width + 2 * spacing);
        let center_width = center_max_width.min(buffer.center.width() as u16);

        surface.set_spans(
            viewport.x + viewport.width / 2 - center_width / 2,
            viewport.y,
            &buffer.center,
            center_width,
        );
    }

    fn append_left(buffer: &mut RenderBuffer, base_style: &Style, element: StatusLineElement) {
        buffer.left.0.push(Span::styled(
            element.text,
            element
                .style
                .map_or(*base_style, |s| (*base_style).patch(s)),
        ));
    }

    fn append_center(buffer: &mut RenderBuffer, base_style: &Style, element: StatusLineElement) {
        buffer.center.0.push(Span::styled(
            element.text,
            element
                .style
                .map_or(*base_style, |s| (*base_style).patch(s)),
        ));
    }

    fn append_right(buffer: &mut RenderBuffer, base_style: &Style, element: StatusLineElement) {
        buffer.right.0.push(Span::styled(
            element.text,
            element
                .style
                .map_or(*base_style, |s| (*base_style).patch(s)),
        ));
    }

    fn render_element(
        context: &RenderContext,
        element_id: StatusLineElementID,
    ) -> Vec<StatusLineElement> {
        match element_id {
            helix_view::editor::StatusLineElement::Mode => vec![Self::render_mode(context)],
            helix_view::editor::StatusLineElement::Spinner => {
                vec![Self::render_lsp_spinner(context)]
            }
            helix_view::editor::StatusLineElement::FileName => {
                vec![Self::render_file_name(context)]
            }
            helix_view::editor::StatusLineElement::FileEncoding => {
                Self::render_file_encoding(context).map_or(Vec::with_capacity(0), |e| vec![e])
            }
            helix_view::editor::StatusLineElement::FileType => {
                vec![Self::render_file_type(context)]
            }
            helix_view::editor::StatusLineElement::Diagnostics => Self::render_diagnostics(context),
            helix_view::editor::StatusLineElement::Selections => {
                vec![Self::render_selections(context)]
            }
            helix_view::editor::StatusLineElement::Position => vec![Self::render_position(context)],
        }
    }

    fn render_mode(context: &RenderContext) -> StatusLineElement {
        let visible = context.focused;
        return StatusLineElement {
            text: format!(
                " {} ",
                match context.doc.mode() {
                    Mode::Insert if visible => "INS",
                    Mode::Select if visible => "SEL",
                    Mode::Normal if visible => "NOR",
                    // If not focused, explicitly leave an empty space instead of returning None.
                    _ => "   ",
                }
            ),
            style: None,
        };
    }

    fn render_lsp_spinner(context: &RenderContext) -> StatusLineElement {
        return StatusLineElement {
            text: format!(
                " {} ",
                context
                    .doc
                    .language_server()
                    .and_then(|srv| context
                        .spinners
                        .get(srv.id())
                        .and_then(|spinner| spinner.frame()))
                    // Even if there's no spinner; reserve its space to avoid elements frequently shifting.
                    .unwrap_or(" ")
            ),
            style: None,
        };
    }

    fn render_diagnostics(context: &RenderContext) -> Vec<StatusLineElement> {
        // 2 diagnostics types, each consisting of 2 elements (state + count)
        let mut elements: Vec<StatusLineElement> = Vec::with_capacity(4);

        let diags = context
            .doc
            .diagnostics()
            .iter()
            .fold((0, 0), |mut counts, diag| {
                use helix_core::diagnostic::Severity;
                match diag.severity {
                    Some(Severity::Warning) => counts.0 += 1,
                    Some(Severity::Error) | None => counts.1 += 1,
                    _ => {}
                }
                counts
            });
        let (warnings, errors) = diags;

        for i in 0..2 {
            let (count, state_element, count_element) = match i {
                0 => (
                    warnings,
                    Self::render_diagnostics_warning_state(context),
                    Self::render_diagnostics_warning_count(context, warnings),
                ),
                1 => (
                    errors,
                    Self::render_diagnostics_error_state(context),
                    Self::render_diagnostics_error_count(context, errors),
                ),
                _ => unreachable!(),
            };

            if count > 0 {
                elements.push(state_element);
                elements.push(count_element);
            }
        }

        elements
    }

    fn render_diagnostics_warning_state(context: &RenderContext) -> StatusLineElement {
        StatusLineElement {
            text: "●".to_string(),
            style: Some(context.theme.get("warning")),
        }
    }

    fn render_diagnostics_warning_count(
        _context: &RenderContext,
        warning_count: usize,
    ) -> StatusLineElement {
        StatusLineElement {
            text: format!(" {} ", warning_count),
            style: None,
        }
    }

    fn render_diagnostics_error_state(context: &RenderContext) -> StatusLineElement {
        StatusLineElement {
            text: "●".to_string(),
            style: Some(context.theme.get("error")),
        }
    }

    fn render_diagnostics_error_count(
        _context: &RenderContext,
        error_count: usize,
    ) -> StatusLineElement {
        StatusLineElement {
            text: format!(" {} ", error_count),
            style: None,
        }
    }

    fn render_selections(context: &RenderContext) -> StatusLineElement {
        let count = context.doc.selection(context.view.id).len();
        StatusLineElement {
            text: format!(" {} sel{} ", count, if count == 1 { "" } else { "s" }),
            style: None,
        }
    }

    fn render_position(context: &RenderContext) -> StatusLineElement {
        let position = coords_at_pos(
            context.doc.text().slice(..),
            context
                .doc
                .selection(context.view.id)
                .primary()
                .cursor(context.doc.text().slice(..)),
        );

        return StatusLineElement {
            text: format!(" {}:{} ", position.row + 1, position.col + 1),
            style: None,
        };
    }

    fn render_file_encoding(context: &RenderContext) -> Option<StatusLineElement> {
        let enc = context.doc.encoding();

        if enc != encoding::UTF_8 {
            Some(StatusLineElement {
                text: format!(" {} ", enc.name()),
                style: None,
            })
        } else {
            None
        }
    }

    fn render_file_type(context: &RenderContext) -> StatusLineElement {
        let file_type = context.doc.language_id().unwrap_or("text");

        StatusLineElement {
            text: format!(" {} ", file_type),
            style: None,
        }
    }

    fn render_file_name(context: &RenderContext) -> StatusLineElement {
        let title = {
            let rel_path = context.doc.relative_path();
            let path = rel_path
                .as_ref()
                .map(|p| p.to_string_lossy())
                .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
            format!(
                " {}{} ",
                path,
                if context.doc.is_modified() { "[+]" } else { "" }
            )
        };

        StatusLineElement {
            text: title,
            style: None,
        }
    }
}
