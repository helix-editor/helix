use helix_core::{coords_at_pos, encoding, Position};
use helix_lsp::lsp::DiagnosticSeverity;
use helix_view::document::DEFAULT_LANGUAGE_NAME;
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::Rect,
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
        }
    }
}

pub fn render(context: &mut RenderContext, viewport: Rect, surface: &mut Surface) {
    let base_style = if context.focused {
        context.editor.theme.get("ui.statusline")
    } else {
        context.editor.theme.get("ui.statusline.inactive")
    };

    surface.set_style(viewport.with_height(1), base_style);

    let statusline = render_statusline(context, viewport.width as usize);

    surface.set_spans(
        viewport.x,
        viewport.y,
        &statusline,
        statusline.width() as u16,
    );
}

pub fn render_statusline<'a>(context: &mut RenderContext, width: usize) -> Spans<'a> {
    let config = context.editor.config();

    let element_ids = &config.statusline.left;
    let mut left = element_ids
        .iter()
        .map(|element_id| get_render_function(*element_id))
        .flat_map(|render| render(context).0)
        .collect::<Vec<Span>>();

    let element_ids = &config.statusline.center;
    let mut center = element_ids
        .iter()
        .map(|element_id| get_render_function(*element_id))
        .flat_map(|render| render(context).0)
        .collect::<Vec<Span>>();

    let element_ids = &config.statusline.right;
    let mut right = element_ids
        .iter()
        .map(|element_id| get_render_function(*element_id))
        .flat_map(|render| render(context).0)
        .collect::<Vec<Span>>();

    let left_area_width: usize = left.iter().map(|s| s.width()).sum();
    let center_area_width: usize = center.iter().map(|s| s.width()).sum();
    let right_area_width: usize = right.iter().map(|s| s.width()).sum();

    let min_spacing_between_areas = 1usize;
    let sides_space_required = left_area_width + right_area_width + min_spacing_between_areas;
    let total_space_required = sides_space_required + center_area_width + min_spacing_between_areas;

    let mut statusline: Vec<Span> = vec![];

    if center_area_width > 0 && total_space_required <= width {
        // SAFETY: this subtraction cannot underflow because `left_area_width + center_area_width + right_area_width`
        // is smaller than `total_space_required`, which is smaller than `width` in this branch.
        let total_spacers = width - (left_area_width + center_area_width + right_area_width);
        // This is how much padding space it would take on either side to align the center area to the middle.
        let center_margin = (width - center_area_width) / 2;
        let left_spacers = if left_area_width < center_margin && right_area_width < center_margin {
            // Align the center area to the middle if there is enough space on both sides.
            center_margin - left_area_width
        } else {
            // Otherwise split the available space evenly and use it as margin.
            // The center element won't be aligned to the middle but it will be evenly
            // spaced between the left and right areas.
            total_spacers / 2
        };
        let right_spacers = total_spacers - left_spacers;

        statusline.append(&mut left);
        statusline.push(" ".repeat(left_spacers).into());
        statusline.append(&mut center);
        statusline.push(" ".repeat(right_spacers).into());
        statusline.append(&mut right);
    } else if right_area_width > 0 && sides_space_required <= width {
        let side_areas_width = left_area_width + right_area_width;
        statusline.append(&mut left);
        statusline.push(" ".repeat(width - side_areas_width).into());
        statusline.append(&mut right);
    } else if left_area_width <= width {
        statusline.append(&mut left);
    }

    statusline.into()
}

fn get_render_function<'a>(
    element_id: StatusLineElementID,
) -> impl Fn(&RenderContext) -> Spans<'a> {
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

fn render_mode<'a>(context: &RenderContext) -> Spans<'a> {
    let visible = context.focused;
    let config = context.editor.config();
    let modenames = &config.statusline.mode;
    let modename = if visible {
        match context.editor.mode() {
            Mode::Insert => modenames.insert.clone(),
            Mode::Select => modenames.select.clone(),
            Mode::Normal => modenames.normal.clone(),
        }
    } else {
        // If not focused, explicitly leave an empty space.
        " ".into()
    };
    let modename = format!(" {} ", modename);
    if visible && config.color_modes {
        Span::styled(
            modename,
            match context.editor.mode() {
                Mode::Insert => context.editor.theme.get("ui.statusline.insert"),
                Mode::Select => context.editor.theme.get("ui.statusline.select"),
                Mode::Normal => context.editor.theme.get("ui.statusline.normal"),
            },
        )
        .into()
    } else {
        Span::raw(modename).into()
    }
}

// TODO think about handling multiple language servers
fn render_lsp_spinner<'a>(context: &RenderContext) -> Spans<'a> {
    let language_server = context.doc.language_servers().next();
    Span::raw(
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
    )
    .into()
}

fn render_diagnostics<'a>(context: &RenderContext) -> Spans<'a> {
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

    let mut output = Spans::default();

    if warnings > 0 {
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("warning"),
        ));
        output.0.push(Span::raw(format!(" {} ", warnings)));
    }

    if errors > 0 {
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("error"),
        ));
        output.0.push(Span::raw(format!(" {} ", errors)));
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
        output.0.push(Span::raw(" W "));
    }

    if warnings > 0 {
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("warning"),
        ));
        output.0.push(Span::raw(format!(" {} ", warnings)));
    }

    if errors > 0 {
        output.0.push(Span::styled(
            "●".to_string(),
            context.editor.theme.get("error"),
        ));
        output.0.push(Span::raw(format!(" {} ", errors)));
    }

    output
}

fn render_selections<'a>(context: &RenderContext) -> Spans<'a> {
    let count = context.doc.selection(context.view.id).len();
    Span::raw(format!(
        " {} sel{} ",
        count,
        if count == 1 { "" } else { "s" }
    ))
    .into()
}

fn render_primary_selection_length<'a>(context: &RenderContext) -> Spans<'a> {
    let tot_sel = context.doc.selection(context.view.id).primary().len();
    Span::raw(format!(
        " {} char{} ",
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
    Span::raw(format!(" {}:{} ", position.row + 1, position.col + 1)).into()
}

fn render_total_line_numbers<'a>(context: &RenderContext) -> Spans<'a> {
    let total_line_numbers = context.doc.text().len_lines();
    Span::raw(format!(" {} ", total_line_numbers)).into()
}

fn render_position_percentage<'a>(context: &RenderContext) -> Spans<'a> {
    let position = get_position(context);
    let maxrows = context.doc.text().len_lines();
    Span::raw(format!("{}%", (position.row + 1) * 100 / maxrows)).into()
}

fn render_file_encoding<'a>(context: &RenderContext) -> Spans<'a> {
    let enc = context.doc.encoding();

    if enc != encoding::UTF_8 {
        Span::raw(format!(" {} ", enc.name())).into()
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

    Span::raw(format!(" {} ", line_ending)).into()
}

fn render_file_type<'a>(context: &RenderContext) -> Spans<'a> {
    let file_type = context.doc.language_name().unwrap_or(DEFAULT_LANGUAGE_NAME);

    Span::raw(format!(" {} ", file_type)).into()
}

fn render_file_name<'a>(context: &RenderContext) -> Spans<'a> {
    let title = {
        let rel_path = context.doc.relative_path();
        let path = rel_path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
        format!(" {} ", path)
    };

    Span::raw(title).into()
}

fn render_file_absolute_path<'a>(context: &RenderContext) -> Spans<'a> {
    let title = {
        let path = context.doc.path();
        let path = path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
        format!(" {} ", path)
    };

    Span::raw(title).into()
}

fn render_file_modification_indicator<'a>(context: &RenderContext) -> Spans<'a> {
    let title = (if context.doc.is_modified() {
        "[+]"
    } else {
        "   "
    })
    .to_string();

    Span::raw(title).into()
}

fn render_read_only_indicator<'a>(context: &RenderContext) -> Spans<'a> {
    let title = if context.doc.readonly {
        " [readonly] "
    } else {
        ""
    }
    .to_string();
    Span::raw(title).into()
}

fn render_file_base_name<'a>(context: &RenderContext) -> Spans<'a> {
    let title = {
        let rel_path = context.doc.relative_path();
        let path = rel_path
            .as_ref()
            .and_then(|p| p.file_name().map(|s| s.to_string_lossy()))
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into());
        format!(" {} ", path)
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
    Span::raw(" ").into()
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
        Span::raw(format!(" reg={} ", reg)).into()
    } else {
        Spans::default()
    }
}
