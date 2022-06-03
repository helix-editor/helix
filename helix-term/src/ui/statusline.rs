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

pub struct RenderContext<'a> {
    pub doc: &'a Document,
    pub view: &'a View,
    pub theme: &'a Theme,
    pub focused: bool,
    pub spinners: &'a ProgressSpinners,
    pub buffer: RenderBuffer<'a>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        doc: &'a Document,
        view: &'a View,
        theme: &'a Theme,
        focused: bool,
        spinners: &'a ProgressSpinners,
    ) -> Self {
        RenderContext {
            doc,
            view,
            theme,
            focused,
            spinners,
            buffer: RenderBuffer::default(),
        }
    }
}

#[derive(Default)]
pub struct RenderBuffer<'a> {
    pub left: Spans<'a>,
    pub center: Spans<'a>,
    pub right: Spans<'a>,
}

pub struct StatusLine;

impl StatusLine {
    pub fn render(
        editor: &Editor,
        context: &mut RenderContext,
        viewport: Rect,
        surface: &mut Surface,
    ) {
        let base_style = if context.focused {
            context.theme.get("ui.statusline")
        } else {
            context.theme.get("ui.statusline.inactive")
        };

        surface.set_style(viewport.with_height(1), base_style);

        let write_left = |context: &mut RenderContext, text, style| {
            Self::append(&mut context.buffer.left, text, &base_style, style)
        };
        let write_center = |context: &mut RenderContext, text, style| {
            Self::append(&mut context.buffer.center, text, &base_style, style)
        };
        let write_right = |context: &mut RenderContext, text, style| {
            Self::append(&mut context.buffer.right, text, &base_style, style)
        };

        // Left side of the status line.

        let element_ids = &editor.config().status_line.left;
        element_ids
            .iter()
            .map(|element_id| Self::get_render_function(*element_id))
            .for_each(|render| render(context, write_left));

        surface.set_spans(
            viewport.x,
            viewport.y,
            &context.buffer.left,
            context.buffer.left.width() as u16,
        );

        // Right side of the status line.

        let element_ids = &editor.config().status_line.right;
        element_ids
            .iter()
            .map(|element_id| Self::get_render_function(*element_id))
            .for_each(|render| render(context, write_right));

        surface.set_spans(
            viewport.x
                + viewport
                    .width
                    .saturating_sub(context.buffer.right.width() as u16),
            viewport.y,
            &context.buffer.right,
            context.buffer.right.width() as u16,
        );

        // Center of the status line.

        let element_ids = &editor.config().status_line.center;
        element_ids
            .iter()
            .map(|element_id| Self::get_render_function(*element_id))
            .for_each(|render| render(context, write_center));

        // Width of the empty space between the left and center area and between the center and right area.
        let spacing = 1u16;

        let edge_width = context
            .buffer
            .left
            .width()
            .max(context.buffer.right.width()) as u16;
        let center_max_width = viewport.width - (2 * edge_width + 2 * spacing);
        let center_width = center_max_width.min(context.buffer.center.width() as u16);

        surface.set_spans(
            viewport.x + viewport.width / 2 - center_width / 2,
            viewport.y,
            &context.buffer.center,
            center_width,
        );
    }

    fn append(buffer: &mut Spans, text: String, base_style: &Style, style: Option<Style>) {
        buffer.0.push(Span::styled(
            text,
            style.map_or(*base_style, |s| (*base_style).patch(s)),
        ));
    }

    fn get_render_function<F>(element_id: StatusLineElementID) -> impl Fn(&mut RenderContext, F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        match element_id {
            helix_view::editor::StatusLineElement::Mode => Self::render_mode,
            helix_view::editor::StatusLineElement::Spinner => Self::render_lsp_spinner,
            helix_view::editor::StatusLineElement::FileName => Self::render_file_name,
            helix_view::editor::StatusLineElement::FileEncoding => Self::render_file_encoding,
            helix_view::editor::StatusLineElement::FileType => Self::render_file_type,
            helix_view::editor::StatusLineElement::Diagnostics => Self::render_diagnostics,
            helix_view::editor::StatusLineElement::Selections => Self::render_selections,
            helix_view::editor::StatusLineElement::Position => Self::render_position,
        }
    }

    fn render_mode<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        let visible = context.focused;

        write(
            context,
            format!(
                " {} ",
                match context.doc.mode() {
                    Mode::Insert if visible => "INS",
                    Mode::Select if visible => "SEL",
                    Mode::Normal if visible => "NOR",
                    // If not focused, explicitly leave an empty space instead of returning None.
                    _ => "   ",
                }
            ),
            None,
        );
    }

    fn render_lsp_spinner<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        write(
            context,
            format!(
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
            None,
        );
    }

    fn render_diagnostics<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        let (warnings, errors) =
            context
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

        if warnings > 0 {
            Self::render_diagnostics_warning_state(context, write);
            Self::render_diagnostics_warning_count(context, warnings, write);
        }

        if errors > 0 {
            Self::render_diagnostics_error_state(context, write);
            Self::render_diagnostics_error_count(context, errors, write);
        }
    }

    fn render_diagnostics_warning_state<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        write(context, "●".to_string(), Some(context.theme.get("warning")));
    }

    fn render_diagnostics_warning_count<F>(
        context: &mut RenderContext,
        warning_count: usize,
        write: F,
    ) where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        write(context, format!(" {} ", warning_count), None);
    }

    fn render_diagnostics_error_state<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        write(context, "●".to_string(), Some(context.theme.get("error")));
    }

    fn render_diagnostics_error_count<F>(context: &mut RenderContext, error_count: usize, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        write(context, format!(" {} ", error_count), None);
    }

    fn render_selections<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        let count = context.doc.selection(context.view.id).len();
        write(
            context,
            format!(" {} sel{} ", count, if count == 1 { "" } else { "s" }),
            None,
        );
    }

    fn render_position<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        let position = coords_at_pos(
            context.doc.text().slice(..),
            context
                .doc
                .selection(context.view.id)
                .primary()
                .cursor(context.doc.text().slice(..)),
        );

        write(
            context,
            format!(" {}:{} ", position.row + 1, position.col + 1),
            None,
        );
    }

    fn render_file_encoding<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        let enc = context.doc.encoding();

        if enc != encoding::UTF_8 {
            write(context, format!(" {} ", enc.name()), None);
        }
    }

    fn render_file_type<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
        let file_type = context.doc.language_id().unwrap_or("text");

        write(context, format!(" {} ", file_type), None);
    }

    fn render_file_name<F>(context: &mut RenderContext, write: F)
    where
        F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
    {
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

        write(context, title, None);
    }
}
