use crate::{
    commands::{self, OnKeyCallback},
    compositor::{Component, Context, Event, EventResult},
    job::{self, Callback},
    key,
    keymap::{KeymapResult, Keymaps},
    ui::{
        document::{render_document, LinePos, TextRenderer, TranslatedPosition},
        Completion, Explorer, ProgressSpinners,
    },
};

use helix_core::{
    diagnostic::NumberOrString,
    graphemes::{
        ensure_grapheme_boundary_next_byte, next_grapheme_boundary, prev_grapheme_boundary,
    },
    movement::Direction,
    syntax::{self, HighlightEvent},
    text_annotations::TextAnnotations,
    unicode::width::UnicodeWidthStr,
    visual_offset_from_block, Position, Range, Selection, Transaction,
};
use helix_view::{
    document::{Mode, SavePoint, SCRATCH_BUFFER_NAME},
    editor::{CompleteAction, CursorShapeConfig, ExplorerPosition},
    graphics::{Color, CursorKind, Modifier, Rect, Style},
    input::{KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    keyboard::{KeyCode, KeyModifiers},
    Document, Editor, Theme, View,
};
use std::{mem::take, num::NonZeroUsize, path::PathBuf, rc::Rc, sync::Arc};

use tui::{buffer::Buffer as Surface, text::Span};

use super::statusline;
use super::{document::LineDecoration, lsp::SignatureHelp};

pub struct EditorView {
    pub keymaps: Keymaps,
    on_next_key: Option<OnKeyCallback>,
    pseudo_pending: Vec<KeyEvent>,
    pub(crate) last_insert: (commands::MappableCommand, Vec<InsertEvent>),
    pub(crate) completion: Option<Completion>,
    spinners: ProgressSpinners,
    pub(crate) explorer: Option<Explorer>,
}

#[derive(Debug, Clone)]
pub enum InsertEvent {
    Key(KeyEvent),
    CompletionApply(CompleteAction),
    TriggerCompletion,
    RequestCompletion,
}

impl Default for EditorView {
    fn default() -> Self {
        Self::new(Keymaps::default())
    }
}

impl EditorView {
    pub fn new(keymaps: Keymaps) -> Self {
        Self {
            keymaps,
            on_next_key: None,
            pseudo_pending: Vec::new(),
            last_insert: (commands::MappableCommand::normal_mode, Vec::new()),
            completion: None,
            spinners: ProgressSpinners::default(),
            explorer: None,
        }
    }

    pub fn spinners_mut(&mut self) -> &mut ProgressSpinners {
        &mut self.spinners
    }

    pub fn render_view(
        &self,
        editor: &Editor,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        is_focused: bool,
    ) {
        let inner = view.inner_area(doc);
        let area = view.area;
        let theme = &editor.theme;
        let config = editor.config();

        let text_annotations = view.text_annotations(doc, Some(theme));
        let mut line_decorations: Vec<Box<dyn LineDecoration>> = Vec::new();
        let mut translated_positions: Vec<TranslatedPosition> = Vec::new();

        if is_focused && config.cursorline {
            line_decorations.push(Self::cursorline_decorator(doc, view, theme))
        }

        if is_focused && config.cursorcolumn {
            Self::highlight_cursorcolumn(doc, view, surface, theme, inner, &text_annotations);
        }

        // Set DAP highlights, if needed.
        if let Some(frame) = editor.current_stack_frame() {
            let dap_line = frame.line.saturating_sub(1);
            let style = theme.get("ui.highlight.frameline");
            let line_decoration = move |renderer: &mut TextRenderer, pos: LinePos| {
                if pos.doc_line != dap_line {
                    return;
                }
                renderer.surface.set_style(
                    Rect::new(inner.x, inner.y + pos.visual_line, inner.width, 1),
                    style,
                );
            };

            line_decorations.push(Box::new(line_decoration));
        }

        let mut highlights =
            Self::doc_syntax_highlights(doc, view.offset.anchor, inner.height, theme);
        let overlay_highlights = Self::overlay_syntax_highlights(
            doc,
            view.offset.anchor,
            inner.height,
            &text_annotations,
        );
        if !overlay_highlights.is_empty() {
            highlights = Box::new(syntax::merge(highlights, overlay_highlights));
        }

        for diagnostic in Self::doc_diagnostics_highlights(doc, theme) {
            // Most of the `diagnostic` Vecs are empty most of the time. Skipping
            // a merge for any empty Vec saves a significant amount of work.
            if diagnostic.is_empty() {
                continue;
            }
            highlights = Box::new(syntax::merge(highlights, diagnostic));
        }

        let highlights: Box<dyn Iterator<Item = HighlightEvent>> = if is_focused {
            let highlights = syntax::merge(
                highlights,
                Self::doc_selection_highlights(
                    editor.mode(),
                    doc,
                    view,
                    theme,
                    &config.cursor_shape,
                ),
            );
            let focused_view_elements = Self::highlight_focused_view_elements(view, doc, theme);
            if focused_view_elements.is_empty() {
                Box::new(highlights)
            } else {
                Box::new(syntax::merge(highlights, focused_view_elements))
            }
        } else {
            Box::new(highlights)
        };

        Self::render_gutter(
            editor,
            doc,
            view,
            view.area,
            theme,
            is_focused,
            &mut line_decorations,
        );

        if is_focused {
            let cursor = doc
                .selection(view.id)
                .primary()
                .cursor(doc.text().slice(..));
            // set the cursor_cache to out of view in case the position is not found
            editor.cursor_cache.set(Some(None));
            let update_cursor_cache =
                |_: &mut TextRenderer, pos| editor.cursor_cache.set(Some(Some(pos)));
            translated_positions.push((cursor, Box::new(update_cursor_cache)));
        }

        render_document(
            surface,
            inner,
            doc,
            view.offset,
            &text_annotations,
            highlights,
            theme,
            &mut line_decorations,
            &mut translated_positions,
        );
        Self::render_rulers(editor, doc, view, inner, surface, theme);

        // if we're not at the edge of the screen, draw a right border
        if viewport.right() != view.area.right() {
            let x = area.right();
            let border_style = theme.get("ui.window");
            for y in area.top()..area.bottom() {
                surface[(x, y)]
                    .set_symbol(tui::symbols::line::VERTICAL)
                    //.set_symbol(" ")
                    .set_style(border_style);
            }
        }

        Self::render_diagnostics(doc, view, inner, surface, theme);

        let statusline_area = view
            .area
            .clip_top(view.area.height.saturating_sub(1))
            .clip_bottom(1); // -1 from bottom to remove commandline

        let mut context =
            statusline::RenderContext::new(editor, doc, view, is_focused, &self.spinners);

        statusline::render(&mut context, statusline_area, surface);
    }

    pub fn render_rulers(
        editor: &Editor,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
    ) {
        let editor_rulers = &editor.config().rulers;
        let ruler_theme = theme
            .try_get("ui.virtual.ruler")
            .unwrap_or_else(|| Style::default().bg(Color::Red));

        let rulers = doc
            .language_config()
            .and_then(|config| config.rulers.as_ref())
            .unwrap_or(editor_rulers);

        rulers
            .iter()
            // View might be horizontally scrolled, convert from absolute distance
            // from the 1st column to relative distance from left of viewport
            .filter_map(|ruler| ruler.checked_sub(1 + view.offset.horizontal_offset as u16))
            .filter(|ruler| ruler < &viewport.width)
            .map(|ruler| viewport.clip_left(ruler).with_width(1))
            .for_each(|area| surface.set_style(area, ruler_theme))
    }

    pub fn overlay_syntax_highlights(
        doc: &Document,
        anchor: usize,
        height: u16,
        text_annotations: &TextAnnotations,
    ) -> Vec<(usize, std::ops::Range<usize>)> {
        let text = doc.text().slice(..);
        let row = text.char_to_line(anchor.min(text.len_chars()));

        let range = {
            // Calculate viewport byte ranges:
            // Saturating subs to make it inclusive zero indexing.
            let last_line = text.len_lines().saturating_sub(1);
            let last_visible_line = (row + height as usize).saturating_sub(1).min(last_line);
            let start = text.line_to_byte(row.min(last_line));
            let end = text.line_to_byte(last_visible_line + 1);

            start..end
        };

        text_annotations.collect_overlay_highlights(range)
    }

    /// Get syntax highlights for a document in a view represented by the first line
    /// and column (`offset`) and the last line. This is done instead of using a view
    /// directly to enable rendering syntax highlighted docs anywhere (eg. picker preview)
    pub fn doc_syntax_highlights<'doc>(
        doc: &'doc Document,
        anchor: usize,
        height: u16,
        _theme: &Theme,
    ) -> Box<dyn Iterator<Item = HighlightEvent> + 'doc> {
        let text = doc.text().slice(..);
        let row = text.char_to_line(anchor.min(text.len_chars()));

        let range = {
            // Calculate viewport byte ranges:
            // Saturating subs to make it inclusive zero indexing.
            let last_line = text.len_lines().saturating_sub(1);
            let last_visible_line = (row + height as usize).saturating_sub(1).min(last_line);
            let start = text.line_to_byte(row.min(last_line));
            let end = text.line_to_byte(last_visible_line + 1);

            start..end
        };

        match doc.syntax() {
            Some(syntax) => {
                let iter = syntax
                    // TODO: range doesn't actually restrict source, just highlight range
                    .highlight_iter(text.slice(..), Some(range), None)
                    .map(|event| event.unwrap())
                    .map(move |event| match event {
                        // TODO: use byte slices directly
                        // convert byte offsets to char offset
                        HighlightEvent::Source { start, end } => {
                            let start =
                                text.byte_to_char(ensure_grapheme_boundary_next_byte(text, start));
                            let end =
                                text.byte_to_char(ensure_grapheme_boundary_next_byte(text, end));
                            HighlightEvent::Source { start, end }
                        }
                        event => event,
                    });

                Box::new(iter)
            }
            None => Box::new(
                [HighlightEvent::Source {
                    start: text.byte_to_char(range.start),
                    end: text.byte_to_char(range.end),
                }]
                .into_iter(),
            ),
        }
    }

    /// Get highlight spans for document diagnostics
    pub fn doc_diagnostics_highlights(
        doc: &Document,
        theme: &Theme,
    ) -> [Vec<(usize, std::ops::Range<usize>)>; 5] {
        use helix_core::diagnostic::Severity;
        let get_scope_of = |scope| {
            theme
            .find_scope_index_exact(scope)
            // get one of the themes below as fallback values
            .or_else(|| theme.find_scope_index_exact("diagnostic"))
            .or_else(|| theme.find_scope_index_exact("ui.cursor"))
            .or_else(|| theme.find_scope_index_exact("ui.selection"))
            .expect(
                "at least one of the following scopes must be defined in the theme: `diagnostic`, `ui.cursor`, or `ui.selection`",
            )
        };

        // basically just queries the theme color defined in the config
        let hint = get_scope_of("diagnostic.hint");
        let info = get_scope_of("diagnostic.info");
        let warning = get_scope_of("diagnostic.warning");
        let error = get_scope_of("diagnostic.error");
        let r#default = get_scope_of("diagnostic"); // this is a bit redundant but should be fine

        let mut default_vec: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
        let mut info_vec = Vec::new();
        let mut hint_vec = Vec::new();
        let mut warning_vec = Vec::new();
        let mut error_vec = Vec::new();

        for diagnostic in doc.diagnostics() {
            // Separate diagnostics into different Vecs by severity.
            let (vec, scope) = match diagnostic.severity {
                Some(Severity::Info) => (&mut info_vec, info),
                Some(Severity::Hint) => (&mut hint_vec, hint),
                Some(Severity::Warning) => (&mut warning_vec, warning),
                Some(Severity::Error) => (&mut error_vec, error),
                _ => (&mut default_vec, r#default),
            };

            // If any diagnostic overlaps ranges with the prior diagnostic,
            // merge the two together. Otherwise push a new span.
            match vec.last_mut() {
                Some((_, range)) if diagnostic.range.start <= range.end => {
                    // This branch merges overlapping diagnostics, assuming that the current
                    // diagnostic starts on range.start or later. If this assertion fails,
                    // we will discard some part of `diagnostic`. This implies that
                    // `doc.diagnostics()` is not sorted by `diagnostic.range`.
                    debug_assert!(range.start <= diagnostic.range.start);
                    range.end = diagnostic.range.end.max(range.end)
                }
                _ => vec.push((scope, diagnostic.range.start..diagnostic.range.end)),
            }
        }

        [default_vec, info_vec, hint_vec, warning_vec, error_vec]
    }

    /// Get highlight spans for selections in a document view.
    pub fn doc_selection_highlights(
        mode: Mode,
        doc: &Document,
        view: &View,
        theme: &Theme,
        cursor_shape_config: &CursorShapeConfig,
    ) -> Vec<(usize, std::ops::Range<usize>)> {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);
        let primary_idx = selection.primary_index();

        let cursorkind = cursor_shape_config.from_mode(mode);
        let cursor_is_block = cursorkind == CursorKind::Block;

        let selection_scope = theme
            .find_scope_index_exact("ui.selection")
            .expect("could not find `ui.selection` scope in the theme!");
        let primary_selection_scope = theme
            .find_scope_index_exact("ui.selection.primary")
            .unwrap_or(selection_scope);

        let base_cursor_scope = theme
            .find_scope_index_exact("ui.cursor")
            .unwrap_or(selection_scope);
        let base_primary_cursor_scope = theme
            .find_scope_index("ui.cursor.primary")
            .unwrap_or(base_cursor_scope);

        let cursor_scope = match mode {
            Mode::Insert => theme.find_scope_index_exact("ui.cursor.insert"),
            Mode::Select => theme.find_scope_index_exact("ui.cursor.select"),
            Mode::Normal => theme.find_scope_index_exact("ui.cursor.normal"),
        }
        .unwrap_or(base_cursor_scope);

        let primary_cursor_scope = match mode {
            Mode::Insert => theme.find_scope_index_exact("ui.cursor.primary.insert"),
            Mode::Select => theme.find_scope_index_exact("ui.cursor.primary.select"),
            Mode::Normal => theme.find_scope_index_exact("ui.cursor.primary.normal"),
        }
        .unwrap_or(base_primary_cursor_scope);

        let mut spans: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
        for (i, range) in selection.iter().enumerate() {
            let selection_is_primary = i == primary_idx;
            let (cursor_scope, selection_scope) = if selection_is_primary {
                (primary_cursor_scope, primary_selection_scope)
            } else {
                (cursor_scope, selection_scope)
            };

            // Special-case: cursor at end of the rope.
            if range.head == range.anchor && range.head == text.len_chars() {
                if !selection_is_primary || cursor_is_block {
                    // Bar and underline cursors are drawn by the terminal
                    // BUG: If the editor area loses focus while having a bar or
                    // underline cursor (eg. when a regex prompt has focus) then
                    // the primary cursor will be invisible. This doesn't happen
                    // with block cursors since we manually draw *all* cursors.
                    spans.push((cursor_scope, range.head..range.head + 1));
                }
                continue;
            }

            let range = range.min_width_1(text);
            if range.head > range.anchor {
                // Standard case.
                let cursor_start = prev_grapheme_boundary(text, range.head);
                // non block cursors look like they exclude the cursor
                let selection_end =
                    if selection_is_primary && !cursor_is_block && mode != Mode::Insert {
                        range.head
                    } else {
                        cursor_start
                    };
                spans.push((selection_scope, range.anchor..selection_end));
                if !selection_is_primary || cursor_is_block {
                    spans.push((cursor_scope, cursor_start..range.head));
                }
            } else {
                // Reverse case.
                let cursor_end = next_grapheme_boundary(text, range.head);
                if !selection_is_primary || cursor_is_block {
                    spans.push((cursor_scope, range.head..cursor_end));
                }
                // non block cursors look like they exclude the cursor
                let selection_start = if selection_is_primary
                    && !cursor_is_block
                    && !(mode == Mode::Insert && cursor_end == range.anchor)
                {
                    range.head
                } else {
                    cursor_end
                };
                spans.push((selection_scope, selection_start..range.anchor));
            }
        }

        spans
    }

    /// Render brace match, etc (meant for the focused view only)
    pub fn highlight_focused_view_elements(
        view: &View,
        doc: &Document,
        theme: &Theme,
    ) -> Vec<(usize, std::ops::Range<usize>)> {
        // Highlight matching braces
        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            use helix_core::match_brackets;
            let pos = doc.selection(view.id).primary().cursor(text);

            if let Some(pos) = match_brackets::find_matching_bracket(syntax, doc.text(), pos) {
                // ensure col is on screen
                if let Some(highlight) = theme.find_scope_index_exact("ui.cursor.match") {
                    return vec![(highlight, pos..pos + 1)];
                }
            }
        }
        Vec::new()
    }

    /// Render bufferline at the top
    pub fn render_bufferline(editor: &Editor, viewport: Rect, surface: &mut Surface) {
        let scratch = PathBuf::from(SCRATCH_BUFFER_NAME); // default filename to use for scratch buffer
        surface.clear_with(
            viewport,
            editor
                .theme
                .try_get("ui.bufferline.background")
                .unwrap_or_else(|| editor.theme.get("ui.statusline")),
        );

        let bufferline_active = editor
            .theme
            .try_get("ui.bufferline.active")
            .unwrap_or_else(|| editor.theme.get("ui.statusline.active"));

        let bufferline_inactive = editor
            .theme
            .try_get("ui.bufferline")
            .unwrap_or_else(|| editor.theme.get("ui.statusline.inactive"));

        let mut x = viewport.x;
        let current_doc = view!(editor).doc;

        for doc in editor.documents() {
            let fname = doc
                .path()
                .unwrap_or(&scratch)
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            let style = if current_doc == doc.id() {
                bufferline_active
            } else {
                bufferline_inactive
            };

            let text = format!(" {}{} ", fname, if doc.is_modified() { "[+]" } else { "" });
            let used_width = viewport.x.saturating_sub(x);
            let rem_width = surface.area.width.saturating_sub(used_width);

            x = surface
                .set_stringn(x, viewport.y, text, rem_width as usize, style)
                .0;

            if x >= surface.area.right() {
                break;
            }
        }
    }

    pub fn render_gutter<'d>(
        editor: &'d Editor,
        doc: &'d Document,
        view: &View,
        viewport: Rect,
        theme: &Theme,
        is_focused: bool,
        line_decorations: &mut Vec<Box<(dyn LineDecoration + 'd)>>,
    ) {
        let text = doc.text().slice(..);
        let cursors: Rc<[_]> = doc
            .selection(view.id)
            .iter()
            .map(|range| range.cursor_line(text))
            .collect();

        let mut offset = 0;

        let gutter_style = theme.get("ui.gutter");
        let gutter_selected_style = theme.get("ui.gutter.selected");
        let gutter_style_virtual = theme.get("ui.gutter.virtual");
        let gutter_selected_style_virtual = theme.get("ui.gutter.selected.virtual");

        for gutter_type in view.gutters() {
            let mut gutter = gutter_type.style(editor, doc, view, theme, is_focused);
            let width = gutter_type.width(view, doc);
            // avoid lots of small allocations by reusing a text buffer for each line
            let mut text = String::with_capacity(width);
            let cursors = cursors.clone();
            let gutter_decoration = move |renderer: &mut TextRenderer, pos: LinePos| {
                // TODO handle softwrap in gutters
                let selected = cursors.contains(&pos.doc_line);
                let x = viewport.x + offset;
                let y = viewport.y + pos.visual_line;

                let gutter_style = match (selected, pos.first_visual_line) {
                    (false, true) => gutter_style,
                    (true, true) => gutter_selected_style,
                    (false, false) => gutter_style_virtual,
                    (true, false) => gutter_selected_style_virtual,
                };

                if let Some(style) =
                    gutter(pos.doc_line, selected, pos.first_visual_line, &mut text)
                {
                    renderer
                        .surface
                        .set_stringn(x, y, &text, width, gutter_style.patch(style));
                } else {
                    renderer.surface.set_style(
                        Rect {
                            x,
                            y,
                            width: width as u16,
                            height: 1,
                        },
                        gutter_style,
                    );
                }
                text.clear();
            };
            line_decorations.push(Box::new(gutter_decoration));

            offset += width as u16;
        }
    }

    pub fn render_diagnostics(
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
    ) {
        use helix_core::diagnostic::Severity;
        use tui::{
            layout::Alignment,
            text::Text,
            widgets::{Paragraph, Widget, Wrap},
        };

        let cursor = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));

        let diagnostics = doc.diagnostics().iter().filter(|diagnostic| {
            diagnostic.range.start <= cursor && diagnostic.range.end >= cursor
        });

        let warning = theme.get("warning");
        let error = theme.get("error");
        let info = theme.get("info");
        let hint = theme.get("hint");

        let mut lines = Vec::new();
        let background_style = theme.get("ui.background");
        for diagnostic in diagnostics {
            let style = Style::reset()
                .patch(background_style)
                .patch(match diagnostic.severity {
                    Some(Severity::Error) => error,
                    Some(Severity::Warning) | None => warning,
                    Some(Severity::Info) => info,
                    Some(Severity::Hint) => hint,
                });
            let text = Text::styled(&diagnostic.message, style);
            lines.extend(text.lines);
            let code = diagnostic.code.as_ref().map(|x| match x {
                NumberOrString::Number(n) => format!("({n})"),
                NumberOrString::String(s) => format!("({s})"),
            });
            if let Some(code) = code {
                let span = Span::styled(code, style);
                lines.push(span.into());
            }
        }

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: true });
        let width = 100.min(viewport.width);
        let height = 15.min(viewport.height);
        paragraph.render(
            Rect::new(viewport.right() - width, viewport.y + 1, width, height),
            surface,
        );
    }

    /// Apply the highlighting on the lines where a cursor is active
    pub fn cursorline_decorator(
        doc: &Document,
        view: &View,
        theme: &Theme,
    ) -> Box<dyn LineDecoration> {
        let text = doc.text().slice(..);
        // TODO only highlight the visual line that contains the cursor instead of the full visual line
        let primary_line = doc.selection(view.id).primary().cursor_line(text);

        // The secondary_lines do contain the primary_line, it doesn't matter
        // as the else-if clause in the loop later won't test for the
        // secondary_lines if primary_line == line.
        // It's used inside a loop so the collect isn't needless:
        // https://github.com/rust-lang/rust-clippy/issues/6164
        #[allow(clippy::needless_collect)]
        let secondary_lines: Vec<_> = doc
            .selection(view.id)
            .iter()
            .map(|range| range.cursor_line(text))
            .collect();

        let primary_style = theme.get("ui.cursorline.primary");
        let secondary_style = theme.get("ui.cursorline.secondary");
        let viewport = view.area;

        let line_decoration = move |renderer: &mut TextRenderer, pos: LinePos| {
            let area = Rect::new(viewport.x, viewport.y + pos.visual_line, viewport.width, 1);
            if primary_line == pos.doc_line {
                renderer.surface.set_style(area, primary_style);
            } else if secondary_lines.binary_search(&pos.doc_line).is_ok() {
                renderer.surface.set_style(area, secondary_style);
            }
        };

        Box::new(line_decoration)
    }

    /// Apply the highlighting on the columns where a cursor is active
    pub fn highlight_cursorcolumn(
        doc: &Document,
        view: &View,
        surface: &mut Surface,
        theme: &Theme,
        viewport: Rect,
        text_annotations: &TextAnnotations,
    ) {
        let text = doc.text().slice(..);

        // Manual fallback behaviour:
        // ui.cursorcolumn.{p/s} -> ui.cursorcolumn -> ui.cursorline.{p/s}
        let primary_style = theme
            .try_get_exact("ui.cursorcolumn.primary")
            .or_else(|| theme.try_get_exact("ui.cursorcolumn"))
            .unwrap_or_else(|| theme.get("ui.cursorline.primary"));
        let secondary_style = theme
            .try_get_exact("ui.cursorcolumn.secondary")
            .or_else(|| theme.try_get_exact("ui.cursorcolumn"))
            .unwrap_or_else(|| theme.get("ui.cursorline.secondary"));

        let inner_area = view.inner_area(doc);

        let selection = doc.selection(view.id);
        let primary = selection.primary();
        let text_format = doc.text_format(viewport.width, None);
        for range in selection.iter() {
            let is_primary = primary == *range;
            let cursor = range.cursor(text);

            let Position { col, .. } =
                visual_offset_from_block(text, cursor, cursor, &text_format, text_annotations).0;

            // if the cursor is horizontally in the view
            if col >= view.offset.horizontal_offset
                && inner_area.width > (col - view.offset.horizontal_offset) as u16
            {
                let area = Rect::new(
                    inner_area.x + (col - view.offset.horizontal_offset) as u16,
                    view.area.y,
                    1,
                    view.area.height,
                );
                if is_primary {
                    surface.set_style(area, primary_style)
                } else {
                    surface.set_style(area, secondary_style)
                }
            }
        }
    }

    /// Handle events by looking them up in `self.keymaps`. Returns None
    /// if event was handled (a command was executed or a subkeymap was
    /// activated). Only KeymapResult::{NotFound, Cancelled} is returned
    /// otherwise.
    fn handle_keymap_event(
        &mut self,
        mode: Mode,
        cxt: &mut commands::Context,
        event: KeyEvent,
    ) -> Option<KeymapResult> {
        let mut last_mode = mode;
        self.pseudo_pending.extend(self.keymaps.pending());
        let key_result = self.keymaps.get(mode, event);
        cxt.editor.autoinfo = self.keymaps.sticky().map(|node| node.infobox());

        let mut execute_command = |command: &commands::MappableCommand| {
            command.execute(cxt);
            let current_mode = cxt.editor.mode();
            match (last_mode, current_mode) {
                (Mode::Normal, Mode::Insert) => {
                    // HAXX: if we just entered insert mode from normal, clear key buf
                    // and record the command that got us into this mode.

                    // how we entered insert mode is important, and we should track that so
                    // we can repeat the side effect.
                    self.last_insert.0 = command.clone();
                    self.last_insert.1.clear();

                    commands::signature_help_impl(cxt, commands::SignatureHelpInvoked::Automatic);
                }
                (Mode::Insert, Mode::Normal) => {
                    // if exiting insert mode, remove completion
                    self.completion = None;
                    cxt.editor.completion_request_handle = None;

                    // TODO: Use an on_mode_change hook to remove signature help
                    cxt.jobs.callback(async {
                        let call: job::Callback =
                            Callback::EditorCompositor(Box::new(|_editor, compositor| {
                                compositor.remove(SignatureHelp::ID);
                            }));
                        Ok(call)
                    });
                }
                _ => (),
            }
            last_mode = current_mode;
        };

        match &key_result {
            KeymapResult::Matched(command) => {
                execute_command(command);
            }
            KeymapResult::Pending(node) => cxt.editor.autoinfo = Some(node.infobox()),
            KeymapResult::MatchedSequence(commands) => {
                for command in commands {
                    execute_command(command);
                }
            }
            KeymapResult::NotFound | KeymapResult::Cancelled(_) => return Some(key_result),
        }
        None
    }

    fn insert_mode(&mut self, cx: &mut commands::Context, event: KeyEvent) {
        if let Some(keyresult) = self.handle_keymap_event(Mode::Insert, cx, event) {
            match keyresult {
                KeymapResult::NotFound => {
                    if let Some(ch) = event.char() {
                        commands::insert::insert_char(cx, ch)
                    }
                }
                KeymapResult::Cancelled(pending) => {
                    for ev in pending {
                        match ev.char() {
                            Some(ch) => commands::insert::insert_char(cx, ch),
                            None => {
                                if let KeymapResult::Matched(command) =
                                    self.keymaps.get(Mode::Insert, ev)
                                {
                                    command.execute(cx);
                                }
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn command_mode(&mut self, mode: Mode, cxt: &mut commands::Context, event: KeyEvent) {
        match (event, cxt.editor.count) {
            // count handling
            (key!(i @ '0'), Some(_)) | (key!(i @ '1'..='9'), _) => {
                let i = i.to_digit(10).unwrap() as usize;
                cxt.editor.count =
                    std::num::NonZeroUsize::new(cxt.editor.count.map_or(i, |c| c.get() * 10 + i));
            }
            // special handling for repeat operator
            (key!('.'), _) if self.keymaps.pending().is_empty() => {
                for _ in 0..cxt.editor.count.map_or(1, NonZeroUsize::into) {
                    // first execute whatever put us into insert mode
                    self.last_insert.0.execute(cxt);
                    let mut last_savepoint = None;
                    let mut last_request_savepoint = None;
                    // then replay the inputs
                    for key in self.last_insert.1.clone() {
                        match key {
                            InsertEvent::Key(key) => self.insert_mode(cxt, key),
                            InsertEvent::CompletionApply(compl) => {
                                let (view, doc) = current!(cxt.editor);

                                if let Some(last_savepoint) = last_savepoint.as_deref() {
                                    doc.restore(view, last_savepoint);
                                }

                                let text = doc.text().slice(..);
                                let cursor = doc.selection(view.id).primary().cursor(text);

                                let shift_position =
                                    |pos: usize| -> usize { pos + cursor - compl.trigger_offset };

                                let tx = Transaction::change(
                                    doc.text(),
                                    compl.changes.iter().cloned().map(|(start, end, t)| {
                                        (shift_position(start), shift_position(end), t)
                                    }),
                                );
                                doc.apply(&tx, view.id);
                            }
                            InsertEvent::TriggerCompletion => {
                                last_savepoint = take(&mut last_request_savepoint);
                            }
                            InsertEvent::RequestCompletion => {
                                let (view, doc) = current!(cxt.editor);
                                last_request_savepoint = Some(doc.savepoint(view));
                            }
                        }
                    }
                }
                cxt.editor.count = None;
            }
            _ => {
                // set the count
                cxt.count = cxt.editor.count;
                // TODO: edge case: 0j -> reset to 1
                // if this fails, count was Some(0)
                // debug_assert!(cxt.count != 0);

                // set the register
                cxt.register = cxt.editor.selected_register.take();

                self.handle_keymap_event(mode, cxt, event);
                if self.keymaps.pending().is_empty() {
                    cxt.editor.count = None
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_completion(
        &mut self,
        editor: &mut Editor,
        savepoint: Arc<SavePoint>,
        items: Vec<helix_lsp::lsp::CompletionItem>,
        offset_encoding: helix_lsp::OffsetEncoding,
        start_offset: usize,
        trigger_offset: usize,
        size: Rect,
    ) -> Option<Rect> {
        let mut completion = Completion::new(
            editor,
            savepoint,
            items,
            offset_encoding,
            start_offset,
            trigger_offset,
        );

        if completion.is_empty() {
            // skip if we got no completion results
            return None;
        }

        let area = completion.area(size, editor);
        editor.last_completion = None;
        self.last_insert.1.push(InsertEvent::TriggerCompletion);

        // TODO : propagate required size on resize to completion too
        completion.required_size((size.width, size.height));
        self.completion = Some(completion);
        Some(area)
    }

    pub fn clear_completion(&mut self, editor: &mut Editor) {
        self.completion = None;

        // Clear any savepoints
        editor.clear_idle_timer(); // don't retrigger
    }

    pub fn handle_idle_timeout(&mut self, cx: &mut commands::Context) -> EventResult {
        commands::compute_inlay_hints_for_all_views(cx.editor, cx.jobs);

        if let Some(completion) = &mut self.completion {
            return if completion.ensure_item_resolved(cx) {
                EventResult::Consumed(None)
            } else {
                EventResult::Ignored(None)
            };
        }

        if cx.editor.mode != Mode::Insert || !cx.editor.config().auto_completion {
            return EventResult::Ignored(None);
        }

        crate::commands::insert::idle_completion(cx);

        EventResult::Consumed(None)
    }
}

impl EditorView {
    fn handle_mouse_event(
        &mut self,
        event: &MouseEvent,
        cxt: &mut commands::Context,
    ) -> EventResult {
        if event.kind != MouseEventKind::Moved {
            cxt.editor.reset_idle_timer();
        }

        let config = cxt.editor.config();
        let MouseEvent {
            kind,
            row,
            column,
            modifiers,
            ..
        } = *event;

        let pos_and_view = |editor: &Editor, row, column, ignore_virtual_text| {
            editor.tree.views().find_map(|(view, _focus)| {
                view.pos_at_screen_coords(
                    &editor.documents[&view.doc],
                    row,
                    column,
                    ignore_virtual_text,
                )
                .map(|pos| (pos, view.id))
            })
        };

        let gutter_coords_and_view = |editor: &Editor, row, column| {
            editor.tree.views().find_map(|(view, _focus)| {
                view.gutter_coords_at_screen_coords(row, column)
                    .map(|coords| (coords, view.id))
            })
        };

        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let editor = &mut cxt.editor;

                if let Some((pos, view_id)) = pos_and_view(editor, row, column, true) {
                    let doc = doc_mut!(editor, &view!(editor, view_id).doc);

                    if modifiers == KeyModifiers::ALT {
                        let selection = doc.selection(view_id).clone();
                        doc.set_selection(view_id, selection.push(Range::point(pos)));
                    } else {
                        doc.set_selection(view_id, Selection::point(pos));
                    }

                    editor.focus(view_id);
                    editor.ensure_cursor_in_view(view_id);

                    return EventResult::Consumed(None);
                }

                if let Some((coords, view_id)) = gutter_coords_and_view(editor, row, column) {
                    editor.focus(view_id);

                    let (view, doc) = current!(cxt.editor);

                    let path = match doc.path() {
                        Some(path) => path.clone(),
                        None => return EventResult::Ignored(None),
                    };

                    if let Some(char_idx) =
                        view.pos_at_visual_coords(doc, coords.row as u16, coords.col as u16, true)
                    {
                        let line = doc.text().char_to_line(char_idx);
                        commands::dap_toggle_breakpoint_impl(cxt, path, line);
                        return EventResult::Consumed(None);
                    }
                }

                EventResult::Ignored(None)
            }

            MouseEventKind::Drag(MouseButton::Left) => {
                let (view, doc) = current!(cxt.editor);

                let pos = match view.pos_at_screen_coords(doc, row, column, true) {
                    Some(pos) => pos,
                    None => return EventResult::Ignored(None),
                };

                let mut selection = doc.selection(view.id).clone();
                let primary = selection.primary_mut();
                *primary = primary.put_cursor(doc.text().slice(..), pos, true);
                doc.set_selection(view.id, selection);
                let view_id = view.id;
                cxt.editor.ensure_cursor_in_view(view_id);
                EventResult::Consumed(None)
            }

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let current_view = cxt.editor.tree.focus;

                let direction = match event.kind {
                    MouseEventKind::ScrollUp => Direction::Backward,
                    MouseEventKind::ScrollDown => Direction::Forward,
                    _ => unreachable!(),
                };

                match pos_and_view(cxt.editor, row, column, false) {
                    Some((_, view_id)) => cxt.editor.tree.focus = view_id,
                    None => return EventResult::Ignored(None),
                }

                let offset = config.scroll_lines.unsigned_abs();
                commands::scroll(cxt, offset, direction);

                cxt.editor.tree.focus = current_view;
                cxt.editor.ensure_cursor_in_view(current_view);

                EventResult::Consumed(None)
            }

            MouseEventKind::Up(MouseButton::Left) => {
                if !config.middle_click_paste {
                    return EventResult::Ignored(None);
                }

                let (view, doc) = current!(cxt.editor);

                if doc
                    .selection(view.id)
                    .primary()
                    .slice(doc.text().slice(..))
                    .len_chars()
                    <= 1
                {
                    return EventResult::Ignored(None);
                }

                commands::MappableCommand::yank_main_selection_to_primary_clipboard.execute(cxt);

                EventResult::Consumed(None)
            }

            MouseEventKind::Up(MouseButton::Right) => {
                if let Some((coords, view_id)) = gutter_coords_and_view(cxt.editor, row, column) {
                    cxt.editor.focus(view_id);

                    let (view, doc) = current!(cxt.editor);
                    if let Some(pos) =
                        view.pos_at_visual_coords(doc, coords.row as u16, coords.col as u16, true)
                    {
                        doc.set_selection(view_id, Selection::point(pos));
                        if modifiers == KeyModifiers::ALT {
                            commands::MappableCommand::dap_edit_log.execute(cxt);
                        } else {
                            commands::MappableCommand::dap_edit_condition.execute(cxt);
                        }

                        return EventResult::Consumed(None);
                    }
                }

                EventResult::Ignored(None)
            }

            MouseEventKind::Up(MouseButton::Middle) => {
                let editor = &mut cxt.editor;
                if !config.middle_click_paste {
                    return EventResult::Ignored(None);
                }

                if modifiers == KeyModifiers::ALT {
                    commands::MappableCommand::replace_selections_with_primary_clipboard
                        .execute(cxt);

                    return EventResult::Consumed(None);
                }

                if let Some((pos, view_id)) = pos_and_view(editor, row, column, true) {
                    let doc = doc_mut!(editor, &view!(editor, view_id).doc);
                    doc.set_selection(view_id, Selection::point(pos));
                    cxt.editor.focus(view_id);
                    commands::MappableCommand::paste_primary_clipboard_before.execute(cxt);

                    return EventResult::Consumed(None);
                }

                EventResult::Ignored(None)
            }

            _ => EventResult::Ignored(None),
        }
    }
}

impl Component for EditorView {
    fn handle_event(
        &mut self,
        event: &Event,
        context: &mut crate::compositor::Context,
    ) -> EventResult {
        if let Some(explore) = self.explorer.as_mut() {
            if let EventResult::Consumed(callback) = explore.handle_event(event, context) {
                return EventResult::Consumed(callback);
            }
        }
        let mut cx = commands::Context {
            editor: context.editor,
            count: None,
            register: None,
            callback: None,
            on_next_key_callback: None,
            jobs: context.jobs,
        };

        match event {
            Event::Paste(contents) => {
                cx.count = cx.editor.count;
                commands::paste_bracketed_value(&mut cx, contents.clone());
                cx.editor.count = None;

                let config = cx.editor.config();
                let mode = cx.editor.mode();
                let (view, doc) = current!(cx.editor);
                view.ensure_cursor_in_view(doc, config.scrolloff);

                // Store a history state if not in insert mode. Otherwise wait till we exit insert
                // to include any edits to the paste in the history state.
                if mode != Mode::Insert {
                    doc.append_changes_to_history(view);
                }

                EventResult::Consumed(None)
            }
            Event::Resize(_width, _height) => {
                // Ignore this event, we handle resizing just before rendering to screen.
                // Handling it here but not re-rendering will cause flashing
                EventResult::Consumed(None)
            }
            Event::Key(mut key) => {
                cx.editor.reset_idle_timer();
                canonicalize_key(&mut key);

                // clear status
                cx.editor.status_msg = None;

                let mode = cx.editor.mode();
                let (view, _) = current!(cx.editor);
                let focus = view.id;

                if let Some(on_next_key) = self.on_next_key.take() {
                    // if there's a command waiting input, do that first
                    on_next_key(&mut cx, key);
                } else {
                    match mode {
                        Mode::Insert => {
                            // let completion swallow the event if necessary
                            let mut consumed = false;
                            if let Some(completion) = &mut self.completion {
                                let res = {
                                    // use a fake context here
                                    let mut cx = Context {
                                        editor: cx.editor,
                                        jobs: cx.jobs,
                                        scroll: None,
                                    };
                                    completion.handle_event(event, &mut cx)
                                };

                                if let EventResult::Consumed(callback) = res {
                                    consumed = true;

                                    if callback.is_some() {
                                        // assume close_fn
                                        self.clear_completion(cx.editor);

                                        // In case the popup was deleted because of an intersection w/ the auto-complete menu.
                                        commands::signature_help_impl(
                                            &mut cx,
                                            commands::SignatureHelpInvoked::Automatic,
                                        );
                                    }
                                }
                            }

                            // if completion didn't take the event, we pass it onto commands
                            if !consumed {
                                if let Some(compl) = cx.editor.last_completion.take() {
                                    self.last_insert.1.push(InsertEvent::CompletionApply(compl));
                                }

                                self.insert_mode(&mut cx, key);

                                // record last_insert key
                                self.last_insert.1.push(InsertEvent::Key(key));

                                // lastly we recalculate completion
                                if let Some(completion) = &mut self.completion {
                                    completion.update(&mut cx);
                                    if completion.is_empty() {
                                        self.clear_completion(cx.editor);
                                    }
                                }
                            }
                        }
                        mode => self.command_mode(mode, &mut cx, key),
                    }
                }

                self.on_next_key = cx.on_next_key_callback.take();
                match self.on_next_key {
                    Some(_) => self.pseudo_pending.push(key),
                    None => self.pseudo_pending.clear(),
                }

                // appease borrowck
                let callback = cx.callback.take();

                // if the command consumed the last view, skip the render.
                // on the next loop cycle the Application will then terminate.
                if cx.editor.should_close() {
                    return EventResult::Ignored(None);
                }

                // if the focused view still exists and wasn't closed
                if cx.editor.tree.contains(focus) {
                    let config = cx.editor.config();
                    let mode = cx.editor.mode();
                    let view = view_mut!(cx.editor, focus);
                    let doc = doc_mut!(cx.editor, &view.doc);

                    view.ensure_cursor_in_view(doc, config.scrolloff);

                    // Store a history state if not in insert mode. This also takes care of
                    // committing changes when leaving insert mode.
                    if mode != Mode::Insert {
                        doc.append_changes_to_history(view);
                    }
                }

                EventResult::Consumed(callback)
            }

            Event::Mouse(event) => self.handle_mouse_event(event, &mut cx),
            Event::IdleTimeout => self.handle_idle_timeout(&mut cx),
            Event::FocusGained => EventResult::Ignored(None),
            Event::FocusLost => {
                if context.editor.config().auto_save {
                    if let Err(e) = commands::typed::write_all_impl(context, false, false) {
                        context.editor.set_error(format!("{}", e));
                    }
                }
                EventResult::Consumed(None)
            }
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // clear with background color
        surface.set_style(area, cx.editor.theme.get("ui.background"));
        let config = cx.editor.config();

        let editor_area = area.clip_bottom(1);

        // check if bufferline should be rendered
        use helix_view::editor::BufferLine;
        let use_bufferline = match config.bufferline {
            BufferLine::Always => true,
            BufferLine::Multiple if cx.editor.documents.len() > 1 => true,
            _ => false,
        };

        let editor_area = if use_bufferline {
            editor_area.clip_top(1)
        } else {
            editor_area
        };

        let editor_area = if let Some(explorer) = &self.explorer {
            let explorer_column_width = if explorer.is_opened() {
                explorer.column_width().saturating_add(2)
            } else {
                0
            };
            // For future developer:
            // We should have a Dock trait that allows a component to dock to the top/left/bottom/right
            // of another component.
            match config.explorer.position {
                ExplorerPosition::Left => editor_area.clip_left(explorer_column_width),
                ExplorerPosition::Right => editor_area.clip_right(explorer_column_width),
            }
        } else {
            editor_area
        };

        // if the terminal size suddenly changed, we need to trigger a resize
        cx.editor.resize(editor_area);

        if let Some(explorer) = self.explorer.as_mut() {
            if !explorer.is_focus() {
                let area = if use_bufferline {
                    area.clip_top(1)
                } else {
                    area
                };
                explorer.render(area, surface, cx);
            }
        }

        if use_bufferline {
            Self::render_bufferline(cx.editor, area.with_height(1), surface);
        }

        for (view, is_focused) in cx.editor.tree.views() {
            let doc = cx.editor.document(view.doc).unwrap();
            self.render_view(cx.editor, doc, view, area, surface, is_focused);
        }

        if config.auto_info {
            if let Some(mut info) = cx.editor.autoinfo.take() {
                info.render(area, surface, cx);
                cx.editor.autoinfo = Some(info)
            }
        }

        let key_width = 15u16; // for showing pending keys
        let mut status_msg_width = 0;

        // render status msg
        if let Some((status_msg, severity)) = &cx.editor.status_msg {
            status_msg_width = status_msg.width();
            use helix_view::editor::Severity;
            let style = if *severity == Severity::Error {
                cx.editor.theme.get("error")
            } else {
                cx.editor.theme.get("ui.text")
            };

            surface.set_string(
                area.x,
                area.y + area.height.saturating_sub(1),
                status_msg,
                style,
            );
        }

        if area.width.saturating_sub(status_msg_width as u16) > key_width {
            let mut disp = String::new();
            if let Some(count) = cx.editor.count {
                disp.push_str(&count.to_string())
            }
            for key in self.keymaps.pending() {
                disp.push_str(&key.key_sequence_format());
            }
            for key in &self.pseudo_pending {
                disp.push_str(&key.key_sequence_format());
            }
            let style = cx.editor.theme.get("ui.text");
            let macro_width = if cx.editor.macro_recording.is_some() {
                3
            } else {
                0
            };
            surface.set_string(
                area.x + area.width.saturating_sub(key_width + macro_width),
                area.y + area.height.saturating_sub(1),
                disp.get(disp.len().saturating_sub(key_width as usize)..)
                    .unwrap_or(&disp),
                style,
            );
            if let Some((reg, _)) = cx.editor.macro_recording {
                let disp = format!("[{}]", reg);
                let style = style
                    .fg(helix_view::graphics::Color::Yellow)
                    .add_modifier(Modifier::BOLD);
                surface.set_string(
                    area.x + area.width.saturating_sub(3),
                    area.y + area.height.saturating_sub(1),
                    &disp,
                    style,
                );
            }
        }

        if let Some(completion) = self.completion.as_mut() {
            completion.render(area, surface, cx);
        }

        if let Some(explore) = self.explorer.as_mut() {
            if explore.is_focus() {
                let area = if use_bufferline {
                    area.clip_top(1)
                } else {
                    area
                };
                explore.render(area, surface, cx);
            }
        }
    }

    fn cursor(&self, _area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        if let Some(explore) = &self.explorer {
            if explore.is_focus() {
                let cursor = explore.cursor(_area, editor);
                if cursor.0.is_some() {
                    return cursor;
                }
            }
        }
        match editor.cursor() {
            // All block cursors are drawn manually
            (pos, CursorKind::Block) => (pos, CursorKind::Hidden),
            cursor => cursor,
        }
    }
}

fn canonicalize_key(key: &mut KeyEvent) {
    if let KeyEvent {
        code: KeyCode::Char(_),
        modifiers: _,
    } = key
    {
        key.modifiers.remove(KeyModifiers::SHIFT)
    }
}
