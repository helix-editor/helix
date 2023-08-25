use helix_core::{coords_at_pos, encoding, Position};
use helix_lsp::lsp::DiagnosticSeverity;
use helix_view::document::DEFAULT_LANGUAGE_NAME;
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::Rect,
    Document, Editor, View,
};

use crate::ui::ProgressSpinners;
use crate::ui::Spinner;

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

fn join_with_spaces<'a, I: Iterator<Item = Span<'a>>>(iter: I) -> Spans<'a> {
    let mut spans = Vec::new();
    for elem in iter {
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }
        spans.push(elem);
    }
    spans.into()
}

pub fn render(context: &mut RenderContext, viewport: Rect, surface: &mut Surface) {
    let base_style = if context.focused {
        context.editor.theme.get("ui.statusline")
    } else {
        context.editor.theme.get("ui.statusline.inactive")
    };

    surface.set_style(viewport.with_height(1), base_style);

    // Left side of the status line.

    let config = context.editor.config();

    let element_ids = &config.statusline.left;
    context.parts.left = join_with_spaces(
        element_ids
            .iter()
            .map(|element_id| get_render_function(*element_id))
            .flat_map(|render| render(context).0),
    );

    surface.set_spans(
        viewport.x,
        viewport.y,
        &context.parts.left,
        context.parts.left.width() as u16,
    );

    // Right side of the status line.

    let element_ids = &config.statusline.right;
    context.parts.right = join_with_spaces(
        element_ids
            .iter()
            .map(|element_id| get_render_function(*element_id))
            .flat_map(|render| render(context).0),
    );

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
    context.parts.center = join_with_spaces(
        element_ids
            .iter()
            .map(|element_id| get_render_function(*element_id))
            .flat_map(|render| render(context).0),
    );

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

fn get_render_function<'a>(
    element_id: StatusLineElementID,
) -> impl Fn(&RenderContext) -> Spans<'a> {
    match element_id {
        helix_view::editor::StatusLineElement::Mode => render_mode,
        helix_view::editor::StatusLineElement::Spinner => render_lsp_spinner,
        helix_view::editor::StatusLineElement::FileBaseName => render_file_base_name,
        helix_view::editor::StatusLineElement::FileName => render_file_name,
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

fn render_mode<'a>(context: &RenderContext) -> Spans<'a> {
    let visible = context.focused;
    let config = context.editor.config();
    let modenames = &config.statusline.mode;
    if visible {
        let modename = match context.editor.mode() {
            Mode::Insert => modenames.insert.clone(),
            Mode::Select => modenames.select.clone(),
            Mode::Normal => modenames.normal.clone(),
        };
        let style = match context.editor.mode() {
            Mode::Insert => context.editor.theme.get("ui.statusline.insert"),
            Mode::Select => context.editor.theme.get("ui.statusline.select"),
            Mode::Normal => context.editor.theme.get("ui.statusline.normal"),
        };
        if config.color_modes {
            Span::styled(modename, style).into()
        } else {
            Span::raw(modename).into()
        }
    } else {
        Spans::default()
    }
}

// TODO think about handling multiple language servers
fn render_lsp_spinner<'a>(context: &RenderContext) -> Spans<'a> {
    context
        .doc
        .language_servers()
        .next()
        .and_then(|srv| context.spinners.get(srv.id()).and_then(Spinner::frame))
        .map(|frame| Span::raw(frame.to_string()).into())
        .unwrap_or_else(Spans::default)
}

fn render_diagnostics<'a>(context: &RenderContext) -> Spans<'a> {
    let (warnings, errors) = context
        .doc
        .shown_diagnostics()
        .fold((0, 0), |mut counts, diag| {
            use helix_core::diagnostic::Severity;
            match diag.severity {
                Some(Severity::Warning) => counts.0 += 1,
                Some(Severity::Error) | None => counts.1 += 1,
                _ => {}
            }
            counts
        });

    let mut output = Spans::default();

    if warnings > 0 {
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("warning"),
        ));
        output.0.push(Span::raw(format!(" {}", warnings)));
    }

    if errors > 0 {
        if warnings > 0 {
            output.0.push(Span::raw(" "));
        }
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("error"),
        ));
        output.0.push(Span::raw(format!(" {}", errors)));
    }

    output
}

fn render_workspace_diagnostics<'a>(context: &RenderContext) -> Spans<'a> {
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

    let mut output = Spans::default();

    if warnings > 0 || errors > 0 {
        output.0.push(Span::raw("W"));
    }

    if warnings > 0 {
        output.0.push(Span::raw(" "));
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("warning"),
        ));
        output.0.push(Span::raw(format!(" {}", warnings)));
    }

    if errors > 0 {
        output.0.push(Span::raw(" "));
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("error"),
        ));
        output.0.push(Span::raw(format!(" {}", errors)));
    }

    output
}

fn render_selections<'a>(context: &RenderContext) -> Spans<'a> {
    let count = context.doc.selection(context.view.id).len();
    Span::raw(format!(
        "{} sel{}",
        count,
        if count == 1 { "" } else { "s" }
    ))
    .into()
}

fn render_primary_selection_length<'a>(context: &RenderContext) -> Spans<'a> {
    let tot_sel = context.doc.selection(context.view.id).primary().len();
    Span::raw(format!(
        "{} char{}",
        tot_sel,
        if tot_sel == 1 { "" } else { "s" }
    ))
    .into()
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

fn render_position<'a>(context: &RenderContext) -> Spans<'a> {
    let position = get_position(context);
    Span::raw(format!("{}:{}", position.row + 1, position.col + 1)).into()
}

fn render_total_line_numbers<'a>(context: &RenderContext) -> Spans<'a> {
    let total_line_numbers = context.doc.text().len_lines();
    Span::raw(format!("{}", total_line_numbers)).into()
}

fn render_position_percentage<'a>(context: &RenderContext) -> Spans<'a> {
    let position = get_position(context);
    let maxrows = context.doc.text().len_lines();
    Span::raw(format!("{}%", (position.row + 1) * 100 / maxrows)).into()
}

fn render_file_encoding<'a>(context: &RenderContext) -> Spans<'a> {
    let enc = context.doc.encoding();

    if enc != encoding::UTF_8 {
        Span::raw(enc.name()).into()
    } else {
        Spans::default()
    }
}

fn render_file_line_ending<'a>(context: &RenderContext) -> Spans<'a> {
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

    Span::raw(line_ending).into()
}

fn render_file_type<'a>(context: &RenderContext) -> Spans<'a> {
    let file_type = context.doc.language_name().unwrap_or(DEFAULT_LANGUAGE_NAME);

    Span::raw(file_type.to_string()).into()
}

fn render_file_name<'a>(context: &RenderContext) -> Spans<'a> {
    let title = {
        let rel_path = context.doc.relative_path();
        let path = rel_path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
        format!("{}", path)
    };

    Span::raw(title).into()
}

fn render_file_modification_indicator<'a>(context: &RenderContext) -> Spans<'a> {
    if context.doc.is_modified() {
        Span::raw("[+]").into()
    } else {
        Spans::default()
    }
}

fn render_read_only_indicator<'a>(context: &RenderContext) -> Spans<'a> {
    if context.doc.readonly {
        Span::raw("[readonly]").into()
    } else {
        Spans::default()
    }
}

fn render_file_base_name<'a>(context: &RenderContext) -> Spans<'a> {
    let title = {
        let rel_path = context.doc.relative_path();
        let path = rel_path
            .as_ref()
            .and_then(|p| p.as_path().file_name().map(|s| s.to_string_lossy()))
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
        format!("{}", path)
    };

    Span::raw(title).into()
}

fn render_separator<'a>(context: &RenderContext) -> Spans<'a> {
    let sep = &context.editor.config().statusline.separator;

    Span::styled(
        sep.to_string(),
        context.editor.theme.get("ui.statusline.separator"),
    )
    .into()
}

fn render_spacer<'a>(_context: &RenderContext) -> Spans<'a> {
    Span::raw("").into()
}

fn render_version_control<'a>(context: &RenderContext) -> Spans<'a> {
    let head = context
        .doc
        .version_control_head()
        .unwrap_or_default()
        .to_string();

    Span::raw(head).into()
}

fn render_register<'a>(context: &RenderContext) -> Spans<'a> {
    if let Some(reg) = context.editor.selected_register {
        Span::raw(format!("reg={}", reg)).into()
    } else {
        Spans::default()
    }
}
