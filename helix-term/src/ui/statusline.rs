use helix_core::{coords_at_pos, encoding};
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::Rect,
    Document, Editor, View,
};

use crate::ui::ProgressSpinners;

use tui::buffer::Buffer as Surface;

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
        use tui::text::{Span, Spans};

        //-------------------------------
        // Left side of the status line.
        //-------------------------------

        let mode = match doc.mode() {
            Mode::Insert => "INS",
            Mode::Select => "SEL",
            Mode::Normal => "NOR",
        };
        let progress = doc
            .language_server()
            .and_then(|srv| spinners.get(srv.id()).and_then(|spinner| spinner.frame()))
            .unwrap_or("");

        let base_style = if is_focused {
            editor.theme.get("ui.statusline")
        } else {
            editor.theme.get("ui.statusline.inactive")
        };
        // statusline
        surface.set_style(viewport.with_height(1), base_style);
        if is_focused {
            surface.set_string(viewport.x + 1, viewport.y, mode, base_style);
        }
        surface.set_string(viewport.x + 5, viewport.y, progress, base_style);

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
        let warning_style = editor.theme.get("warning");
        let error_style = editor.theme.get("error");
        for i in 0..2 {
            let (count, style) = match i {
                0 => (warnings, warning_style),
                1 => (errors, error_style),
                _ => unreachable!(),
            };
            if count == 0 {
                continue;
            }
            let style = base_style.patch(style);
            right_side_text.0.push(Span::styled("●", style));
            right_side_text
                .0
                .push(Span::styled(format!(" {} ", count), base_style));
        }

        // Selections
        let sels_count = doc.selection(view.id).len();
        right_side_text.0.push(Span::styled(
            format!(
                " {} sel{} ",
                sels_count,
                if sels_count == 1 { "" } else { "s" }
            ),
            base_style,
        ));

        // Position
        let pos = coords_at_pos(
            doc.text().slice(..),
            doc.selection(view.id)
                .primary()
                .cursor(doc.text().slice(..)),
        );
        right_side_text.0.push(Span::styled(
            format!(" {}:{} ", pos.row + 1, pos.col + 1), // Convert to 1-indexing.
            base_style,
        ));

        // Encoding
        let enc = doc.encoding();
        if enc != encoding::UTF_8 {
            right_side_text
                .0
                .push(Span::styled(format!(" {} ", enc.name()), base_style));
        }

        // File type
        let file_type = doc.language_id().unwrap_or("text");
        right_side_text
            .0
            .push(Span::styled(format!(" {} ", file_type), base_style));

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
        let title = {
            let rel_path = doc.relative_path();
            let path = rel_path
                .as_ref()
                .map(|p| p.to_string_lossy())
                .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
            format!("{}{}", path, if doc.is_modified() { "[+]" } else { "" })
        };

        surface.set_string_truncated(
            viewport.x + 8, // 8: 1 space + 3 char mode string + 1 space + 1 spinner + 1 space
            viewport.y,
            &title,
            viewport
                .width
                .saturating_sub(6)
                .saturating_sub(right_side_text.width() as u16 + 1) as usize, // "+ 1": a space between the title and the selection info
            |_| base_style,
            true,
            true,
        );
    }
}
