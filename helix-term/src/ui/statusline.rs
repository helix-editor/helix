use helix_core::{coords_at_pos, encoding, Position};
use helix_lsp::lsp::DiagnosticSeverity;
use helix_view::document::DEFAULT_LANGUAGE_NAME;
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::Rect,
    theme::Style,
    Document, Editor, View,
};

use crate::ui::ProgressSpinners;

use helix_view::editor::StatusLineElement as StatusLineElementID;
use tui::buffer::Buffer as Surface;
use tui::text::{Span, Spans};

pub struct RenderContext<'a> {
    pub editor: &'a Editor,
    pub doc: &'a Document,
    pub view: &'a View,
    pub focused: bool,
    pub spinners: &'a ProgressSpinners,
    pub parts: RenderBuffer<'a>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        editor: &'a Editor,
        doc: &'a Document,
        view: &'a View,
        focused: bool,
        spinners: &'a ProgressSpinners,
    ) -> Self {
        RenderContext {
            editor,
            doc,
            view,
            focused,
            spinners,
            parts: RenderBuffer::default(),
        }
    }
}

#[derive(Default)]
pub struct RenderBuffer<'a> {
    pub left: Spans<'a>,
    pub center: Spans<'a>,
    pub right: Spans<'a>,
}

pub fn render(context: &mut RenderContext, viewport: Rect, surface: &mut Surface) {
    let base_style = if context.focused {
        context.editor.theme.get("ui.statusline")
    } else {
        context.editor.theme.get("ui.statusline.inactive")
    };

    surface.set_style(viewport.with_height(1), base_style);

    let write_left = |context: &mut RenderContext, text, style| {
        append(&mut context.parts.left, text, &base_style, style)
    };
    let write_center = |context: &mut RenderContext, text, style| {
        append(&mut context.parts.center, text, &base_style, style)
    };
    let write_right = |context: &mut RenderContext, text, style| {
        append(&mut context.parts.right, text, &base_style, style)
    };

    // Left side of the status line.

    let config = context.editor.config();

    let element_ids = &config.statusline.left;
    element_ids
        .iter()
        .map(|element_id| get_render_function(*element_id))
        .for_each(|render| render(context, write_left));

    surface.set_spans(
        viewport.x,
        viewport.y,
        &context.parts.left,
        context.parts.left.width() as u16,
    );

    // Right side of the status line.

    let element_ids = &config.statusline.right;
    element_ids
        .iter()
        .map(|element_id| get_render_function(*element_id))
        .for_each(|render| render(context, write_right));

    surface.set_spans(
        viewport.x
            + viewport
                .width
                .saturating_sub(context.parts.right.width() as u16),
        viewport.y,
        &context.parts.right,
        context.parts.right.width() as u16,
    );

    // Center of the status line.

    let element_ids = &config.statusline.center;
    element_ids
        .iter()
        .map(|element_id| get_render_function(*element_id))
        .for_each(|render| render(context, write_center));

    // Width of the empty space between the left and center area and between the center and right area.
    let spacing = 1u16;

    let edge_width = context.parts.left.width().max(context.parts.right.width()) as u16;
    let center_max_width = viewport.width.saturating_sub(2 * edge_width + 2 * spacing);
    let center_width = center_max_width.min(context.parts.center.width() as u16);

    surface.set_spans(
        viewport.x + viewport.width / 2 - center_width / 2,
        viewport.y,
        &context.parts.center,
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
        helix_view::editor::StatusLineElement::Mode => render_mode,
        helix_view::editor::StatusLineElement::Spinner => render_lsp_spinner,
        helix_view::editor::StatusLineElement::FileBaseName => render_file_base_name,
        helix_view::editor::StatusLineElement::FileName => render_file_name,
        helix_view::editor::StatusLineElement::FileAbsolutePath => render_file_absolute_path,
        helix_view::editor::StatusLineElement::FileModificationIndicator => {
            render_file_modification_indicator
        }
        helix_view::editor::StatusLineElement::ReadOnlyIndicator => render_read_only_indicator,
        helix_view::editor::StatusLineElement::FileEncoding => render_file_encoding,
        helix_view::editor::StatusLineElement::FileLineEnding => render_file_line_ending,
        helix_view::editor::StatusLineElement::FileType => render_file_type,
        helix_view::editor::StatusLineElement::Diagnostics => render_diagnostics,
        helix_view::editor::StatusLineElement::WorkspaceDiagnostics => render_workspace_diagnostics,
        helix_view::editor::StatusLineElement::Selections => render_selections,
        helix_view::editor::StatusLineElement::PrimarySelectionLength => {
            render_primary_selection_length
        }
        helix_view::editor::StatusLineElement::Position => render_position,
        helix_view::editor::StatusLineElement::PositionPercentage => render_position_percentage,
        helix_view::editor::StatusLineElement::TotalLineNumbers => render_total_line_numbers,
        helix_view::editor::StatusLineElement::Separator => render_separator,
        helix_view::editor::StatusLineElement::Spacer => render_spacer,
        helix_view::editor::StatusLineElement::VersionControl => render_version_control,
        helix_view::editor::StatusLineElement::Register => render_register,
    }
}

fn render_mode<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let visible = context.focused;
    let config = context.editor.config();
    let modenames = &config.statusline.mode;
    write(
        context,
        format!(
            " {} ",
            if visible {
                match context.editor.mode() {
                    Mode::Insert => &modenames.insert,
                    Mode::Select => &modenames.select,
                    Mode::Normal => &modenames.normal,
                }
            } else {
                // If not focused, explicitly leave an empty space instead of returning None.
                "   "
            }
        ),
        if visible && config.color_modes {
            match context.editor.mode() {
                Mode::Insert => Some(context.editor.theme.get("ui.statusline.insert")),
                Mode::Select => Some(context.editor.theme.get("ui.statusline.select")),
                Mode::Normal => Some(context.editor.theme.get("ui.statusline.normal")),
            }
        } else {
            None
        },
    );
}

// TODO think about handling multiple language servers
fn render_lsp_spinner<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let language_server = context.doc.language_servers().next();
    write(
        context,
        language_server
            .and_then(|srv| {
                context
                    .spinners
                    .get(srv.id())
                    .and_then(|spinner| spinner.frame())
            })
            // Even if there's no spinner; reserve its space to avoid elements frequently shifting.
            .unwrap_or(" ")
            .to_string(),
        None,
    );
}

fn render_diagnostics<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let (warnings, errors) = context
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
        write(
            context,
            context
                .editor
                .config()
                .icons
                .diagnostic
                .warning()
                .to_string(),
            Some(context.editor.theme.get("warning")),
        );
        write(context, format!(" {} ", warnings), None);
    }

    if errors > 0 {
        write(
            context,
            context.editor.config().icons.diagnostic.error().to_string(),
            Some(context.editor.theme.get("error")),
        );
        write(context, format!(" {} ", errors), None);
    }
}

fn render_workspace_diagnostics<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let (warnings, errors) =
        context
            .editor
            .diagnostics
            .values()
            .flatten()
            .fold((0, 0), |mut counts, (diag, _)| {
                match diag.severity {
                    Some(DiagnosticSeverity::WARNING) => counts.0 += 1,
                    Some(DiagnosticSeverity::ERROR) | None => counts.1 += 1,
                    _ => {}
                }
                counts
            });

    if warnings > 0 || errors > 0 {
        write(context, " W ".into(), None);
    }

    if warnings > 0 {
        write(
            context,
            context
                .editor
                .config()
                .icons
                .diagnostic
                .warning()
                .to_string(),
            Some(context.editor.theme.get("warning")),
        );
        write(context, format!(" {} ", warnings), None);
    }

    if errors > 0 {
        write(
            context,
            context.editor.config().icons.diagnostic.error().to_string(),
            Some(context.editor.theme.get("error")),
        );
        write(context, format!(" {} ", errors), None);
    }
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

fn render_primary_selection_length<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let tot_sel = context.doc.selection(context.view.id).primary().len();
    write(
        context,
        format!(" {} char{} ", tot_sel, if tot_sel == 1 { "" } else { "s" }),
        None,
    );
}

fn get_position(context: &RenderContext) -> Position {
    coords_at_pos(
        context.doc.text().slice(..),
        context
            .doc
            .selection(context.view.id)
            .primary()
            .cursor(context.doc.text().slice(..)),
    )
}

fn render_position<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let position = get_position(context);
    write(
        context,
        format!(" {}:{} ", position.row + 1, position.col + 1),
        None,
    );
}

fn render_total_line_numbers<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let total_line_numbers = context.doc.text().len_lines();

    write(context, format!(" {} ", total_line_numbers), None);
}

fn render_position_percentage<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let position = get_position(context);
    let maxrows = context.doc.text().len_lines();
    write(
        context,
        format!("{}%", (position.row + 1) * 100 / maxrows),
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

fn render_file_line_ending<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    use helix_core::LineEnding::*;
    let line_ending = match context.doc.line_ending {
        Crlf => "CRLF",
        LF => "LF",
        #[cfg(feature = "unicode-lines")]
        VT => "VT", // U+000B -- VerticalTab
        #[cfg(feature = "unicode-lines")]
        FF => "FF", // U+000C -- FormFeed
        #[cfg(feature = "unicode-lines")]
        CR => "CR", // U+000D -- CarriageReturn
        #[cfg(feature = "unicode-lines")]
        Nel => "NEL", // U+0085 -- NextLine
        #[cfg(feature = "unicode-lines")]
        LS => "LS", // U+2028 -- Line Separator
        #[cfg(feature = "unicode-lines")]
        PS => "PS", // U+2029 -- ParagraphSeparator
    };

    write(context, format!(" {} ", line_ending), None);
}

fn render_file_type<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let icons = &context.editor.config().icons;

    let icon = icons
        .mime
        .get(context.doc.language_name().unwrap_or(DEFAULT_LANGUAGE_NAME));

    write(context, format!(" {} ", icon), None);
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
        format!(" {} ", path)
    };

    write(context, title, None);
}

fn render_file_absolute_path<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let title = {
        let path = context.doc.path();
        let path = path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
        format!(" {} ", path)
    };

    write(context, title, None);
}

fn render_file_modification_indicator<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let title = (if context.doc.is_modified() {
        "[+]"
    } else {
        "   "
    })
    .to_string();

    write(context, title, None);
}

fn render_read_only_indicator<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let title = if context.doc.readonly {
        " [readonly] "
    } else {
        ""
    }
    .to_string();
    write(context, title, None);
}

fn render_file_base_name<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let title = {
        let rel_path = context.doc.relative_path();
        let path = rel_path
            .as_ref()
            .and_then(|p| p.file_name().map(|s| s.to_string_lossy()))
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
        format!(" {} ", path)
    };

    write(context, title, None);
}

fn render_separator<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let sep = &context.editor.config().statusline.separator;

    write(
        context,
        sep.to_string(),
        Some(context.editor.theme.get("ui.statusline.separator")),
    );
}

fn render_spacer<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    write(context, String::from(" "), None);
}

fn render_version_control<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    let head = context.doc.version_control_head().unwrap_or_default();

    let icons = &context.editor.config().icons;
    let icon = icons.vcs.icon();

    let vcs = if head.is_empty() {
        format!("{head}")
    } else {
        format!("{icon} {head}")
    };

    write(context, vcs, None);
}

fn render_register<F>(context: &mut RenderContext, write: F)
where
    F: Fn(&mut RenderContext, String, Option<Style>) + Copy,
{
    if let Some(reg) = context.editor.selected_register {
        write(context, format!(" reg={} ", reg), None)
    }
}
