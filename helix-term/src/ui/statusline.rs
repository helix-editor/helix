use helix_core::{
    coords_at_pos,
    encoding::{self, Encoding},
    Position,
};
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::Rect,
    theme::Style,
    Document, Editor, Theme, View,
};

use crate::ui::ProgressSpinners;

use tui::buffer::Buffer as Surface;
use tui::text::{Span, Spans};

struct StatusLineElement {
    /// The element
    pub text: String,

    /// The style to be used to render the element (this style will be merged with the base style).
    /// If not set, a default base style will be used.
    pub style: Option<Style>,
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
        //-------------------------------
        // Left side of the status line.
        //-------------------------------

        let base_style = if is_focused {
            editor.theme.get("ui.statusline")
        } else {
            editor.theme.get("ui.statusline.inactive")
        };

        surface.set_style(viewport.with_height(1), base_style);

        if is_focused {
            let mode = Self::render_mode(doc);
            surface.set_string(
                viewport.x + 1,
                viewport.y,
                mode.text,
                mode.style
                    .map_or(base_style, |s| base_style.clone().patch(s)),
            );
        }

        let spinner = Self::render_lsp_spinner(doc, spinners);
        surface.set_string(
            viewport.x + 5,
            viewport.y,
            spinner.text,
            spinner
                .style
                .map_or(base_style, |s| base_style.clone().patch(s)),
        );

        //-------------------------------
        // Right side of the status line.
        //-------------------------------

        let mut right_side_text = Spans::default();

        // Compute the individual info strings and add them to `right_side_text`.

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
                right_side_text.0.push(Span::styled(
                    state_element.text,
                    state_element
                        .style
                        .map_or(base_style, |s| base_style.clone().patch(s)),
                ));

                right_side_text.0.push(Span::styled(
                    count_element.text,
                    count_element
                        .style
                        .map_or(base_style, |s| base_style.clone().patch(s)),
                ));
            }
        }

        // Selections
        let sels_count = doc.selection(view.id).len();
        let selections_element = Self::render_selections(sels_count);
        right_side_text.0.push(Span::styled(
            selections_element.text,
            selections_element
                .style
                .map_or(base_style, |s| base_style.clone().patch(s)),
        ));

        // Position
        let pos = coords_at_pos(
            doc.text().slice(..),
            doc.selection(view.id)
                .primary()
                .cursor(doc.text().slice(..)),
        );
        let position_element = Self::render_position(&pos);
        right_side_text.0.push(Span::styled(
            position_element.text,
            position_element
                .style
                .map_or(base_style, |s| base_style.clone().patch(s)),
        ));

        // Encoding
        let enc = doc.encoding();
        if enc != encoding::UTF_8 {
            let encoding_element = Self::render_encoding(enc);
            right_side_text.0.push(Span::styled(
                encoding_element.text,
                encoding_element
                    .style
                    .map_or(base_style, |s| base_style.clone().patch(s)),
            ));
        }

        // File type
        let file_type = doc.language_id().unwrap_or("text");
        let file_type_element = Self::render_file_type(file_type);
        right_side_text.0.push(Span::styled(
            file_type_element.text,
            file_type_element
                .style
                .map_or(base_style, |s| base_style.clone().patch(s)),
        ));

        // Render to the statusline.
        surface.set_spans(
            viewport.x
                + viewport
                    .width
                    .saturating_sub(right_side_text.width() as u16),
            viewport.y,
            &right_side_text,
            right_side_text.width() as u16,
        );

        //-------------------------------
        // Middle / File path / Title
        //-------------------------------
        let title_element = Self::render_file_name(doc);

        surface.set_string_truncated(
            viewport.x + 8, // 8: 1 space + 3 char mode string + 1 space + 1 spinner + 1 space
            viewport.y,
            title_element.text.as_str(),
            viewport
                .width
                .saturating_sub(6)
                .saturating_sub(right_side_text.width() as u16 + 1) as usize, // "+ 1": a space between the title and the selection info
            |_| {
                title_element
                    .style
                    .map_or(base_style, |s| base_style.clone().patch(s))
            },
            true,
            true,
        );
    }

    fn render_mode(doc: &Document) -> StatusLineElement {
        return StatusLineElement {
            text: format!(
                "{}",
                match doc.mode() {
                    Mode::Insert => "INS",
                    Mode::Select => "SEL",
                    Mode::Normal => "NOR",
                }
            ),
            style: None,
        };
    }

    fn render_lsp_spinner(doc: &Document, spinners: &ProgressSpinners) -> StatusLineElement {
        return StatusLineElement {
            text: format!(
                "{}",
                doc.language_server()
                    .and_then(|srv| spinners.get(srv.id()).and_then(|spinner| spinner.frame()))
                    .unwrap_or("")
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

    fn render_position(position: &Position) -> StatusLineElement {
        return StatusLineElement {
            text: format!(" {}:{} ", position.row + 1, position.col + 1),
            style: None,
        };
    }

    fn render_encoding(encoding: &'static Encoding) -> StatusLineElement {
        return StatusLineElement {
            text: format!(" {} ", encoding.name()),
            style: None,
        };
    }

    fn render_file_type(file_type: &str) -> StatusLineElement {
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
