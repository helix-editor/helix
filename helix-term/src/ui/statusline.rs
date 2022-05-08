use helix_core::{coords_at_pos, encoding};
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::Rect,
    theme::Style,
    Document, Editor, Theme, View,
};

use crate::ui::ProgressSpinners;

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

struct RenderBuffer<'a> {
    pub left: Spans<'a>,
    pub center: Spans<'a>,
    pub right: Spans<'a>,
}

impl<'a> RenderBuffer<'a> {
    pub fn new() -> Self {
        return Self {
            left: Spans::default(),
            center: Spans::default(),
            right: Spans::default(),
        };
    }
}

pub struct StatusLine;

impl StatusLine {
    pub fn render(
        editor: &Editor,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        is_focused: bool,
        spinners: &ProgressSpinners,
    ) {
        let mut buffer = RenderBuffer::new();

        // Left side of the status line.

        let base_style = if is_focused {
            editor.theme.get("ui.statusline")
        } else {
            editor.theme.get("ui.statusline.inactive")
        };

        surface.set_style(viewport.with_height(1), base_style);

        let mode_element = Self::render_mode(doc, is_focused);
        Self::append_left(&mut buffer, &base_style, mode_element);

        let spinner_element = Self::render_lsp_spinner(doc, spinners);
        Self::append_left(&mut buffer, &base_style, spinner_element);

        // TODO: why x+1?
        surface.set_spans(
            viewport.x + 1,
            viewport.y,
            &buffer.left,
            buffer.left.width() as u16,
        );

        // Right side of the status line.

        // Diagnostics
        let diags = doc.diagnostics().iter().fold((0, 0), |mut counts, diag| {
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
                    Self::render_diagnostics_warning_state(&editor.theme),
                    Self::render_diagnostics_warning_count(warnings),
                ),
                1 => (
                    errors,
                    Self::render_diagnostics_error_state(&editor.theme),
                    Self::render_diagnostics_error_count(errors),
                ),
                _ => unreachable!(),
            };

            if count > 0 {
                Self::append_right(&mut buffer, &base_style, state_element);
                Self::append_right(&mut buffer, &base_style, count_element);
            }
        }

        // Selections
        let sels_count = doc.selection(view.id).len();
        let selections_element = Self::render_selections(sels_count);
        Self::append_right(&mut buffer, &base_style, selections_element);

        // Position
        let position_element = Self::render_position(doc, view);
        Self::append_right(&mut buffer, &base_style, position_element);

        // Encoding
        if let Some(encoding_element) = Self::render_encoding(doc) {
            Self::append_right(&mut buffer, &base_style, encoding_element);
        }

        // File type
        let file_type_element = Self::render_file_type(doc);
        Self::append_right(&mut buffer, &base_style, file_type_element);

        // Render to the statusline.
        surface.set_spans(
            viewport.x + viewport.width.saturating_sub(buffer.right.width() as u16),
            viewport.y,
            &buffer.right,
            buffer.right.width() as u16,
        );

        // Center of the status line.

        let title_element = Self::render_file_name(doc);
        Self::append_center(&mut buffer, &base_style, title_element);

        // Width of the empty space between the left and center area and between the center and right area.
        let spacing = 1u16;

        let edge_width = buffer.left.width().max(buffer.right.width()) as u16;
        //let center_width = viewport.width
        //    - (buffer.left.width() as u16 + buffer.right.width() as u16 + 2 * spacing);
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
                .map_or(*base_style, |s| base_style.clone().patch(s)),
        ));
    }

    fn append_center(buffer: &mut RenderBuffer, base_style: &Style, element: StatusLineElement) {
        buffer.center.0.push(Span::styled(
            element.text,
            element
                .style
                .map_or(*base_style, |s| base_style.clone().patch(s)),
        ));
    }

    fn append_right(buffer: &mut RenderBuffer, base_style: &Style, element: StatusLineElement) {
        buffer.right.0.push(Span::styled(
            element.text,
            element
                .style
                .map_or(*base_style, |s| base_style.clone().patch(s)),
        ));
    }

    fn render_mode(doc: &Document, is_focused: bool) -> StatusLineElement {
        return StatusLineElement {
            text: format!(
                "{}",
                match doc.mode() {
                    Mode::Insert if is_focused => "INS",
                    Mode::Select if is_focused => "SEL",
                    Mode::Normal if is_focused => "NOR",
                    // If not focused, explicitly leave an empty space instead of returning None.
                    _ => "   ",
                }
            ),
            style: None,
        };
    }

    fn render_lsp_spinner(doc: &Document, spinners: &ProgressSpinners) -> StatusLineElement {
        return StatusLineElement {
            text: format!(
                " {} ",
                doc.language_server()
                    .and_then(|srv| spinners.get(srv.id()).and_then(|spinner| spinner.frame()))
                    // Even if there's no spinner; reserve its space to avoid elements frequently shifting.
                    .unwrap_or(" ")
            ),
            style: None,
        };
    }

    fn render_diagnostics_warning_state(theme: &Theme) -> StatusLineElement {
        return StatusLineElement {
            text: format!("●"),
            style: Some(theme.get("warning")),
        };
    }

    fn render_diagnostics_warning_count(warnings: usize) -> StatusLineElement {
        return StatusLineElement {
            text: format!(" {} ", warnings),
            style: None,
        };
    }

    fn render_diagnostics_error_state(theme: &Theme) -> StatusLineElement {
        return StatusLineElement {
            text: format!("●"),
            style: Some(theme.get("error")),
        };
    }

    fn render_diagnostics_error_count(errors: usize) -> StatusLineElement {
        return StatusLineElement {
            text: format!(" {} ", errors),
            style: None,
        };
    }

    fn render_selections(selections: usize) -> StatusLineElement {
        return StatusLineElement {
            text: format!(
                " {} sel{} ",
                &selections,
                if selections == 1 { "" } else { "s" }
            ),
            style: None,
        };
    }

    fn render_position(doc: &Document, view: &View) -> StatusLineElement {
        let position = coords_at_pos(
            doc.text().slice(..),
            doc.selection(view.id)
                .primary()
                .cursor(doc.text().slice(..)),
        );

        return StatusLineElement {
            text: format!(" {}:{} ", position.row + 1, position.col + 1),
            style: None,
        };
    }

    fn render_encoding(doc: &Document) -> Option<StatusLineElement> {
        let enc = doc.encoding();

        if enc != encoding::UTF_8 {
            return Some(StatusLineElement {
                text: format!(" {} ", enc.name()),
                style: None,
            });
        } else {
            return None;
        }
    }

    fn render_file_type(doc: &Document) -> StatusLineElement {
        let file_type = doc.language_id().unwrap_or("text");

        return StatusLineElement {
            text: format!(" {} ", file_type),
            style: None,
        };
    }

    fn render_file_name(doc: &Document) -> StatusLineElement {
        let title = {
            let rel_path = doc.relative_path();
            let path = rel_path
                .as_ref()
                .map(|p| p.to_string_lossy())
                .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
            format!("{}{}", path, if doc.is_modified() { "[+]" } else { "" })
        };

        return StatusLineElement {
            text: title,
            style: None,
        };
    }
}
