use crate::{
    commands,
    compositor::{Component, Context, EventResult},
    key,
    keymap::{KeymapResult, Keymaps},
    ui::{Completion, ProgressSpinners},
};

use helix_core::{
    coords_at_pos, encoding,
    graphemes::{
        ensure_grapheme_boundary_next_byte, next_grapheme_boundary, prev_grapheme_boundary,
    },
    movement::Direction,
    syntax::{self, HighlightEvent},
    unicode::segmentation::UnicodeSegmentation,
    unicode::width::UnicodeWidthStr,
    LineEnding, Position, Range, Selection, Transaction,
};
use helix_view::{
    document::{Mode, SCRATCH_BUFFER_NAME},
    editor::{CompleteAction, CursorShapeConfig},
    graphics::{CursorKind, Modifier, Rect, Style},
    input::KeyEvent,
    keyboard::{KeyCode, KeyModifiers},
    Document, Editor, Theme, View,
};
use std::borrow::Cow;

use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use tui::buffer::Buffer as Surface;

pub struct EditorView {
    pub keymaps: Keymaps,
    on_next_key: Option<Box<dyn FnOnce(&mut commands::Context, KeyEvent)>>,
    last_insert: (commands::MappableCommand, Vec<InsertEvent>),
    pub(crate) completion: Option<Completion>,
    spinners: ProgressSpinners,
}

#[derive(Debug, Clone)]
pub enum InsertEvent {
    Key(KeyEvent),
    CompletionApply(CompleteAction),
    TriggerCompletion,
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
            last_insert: (commands::MappableCommand::normal_mode, Vec::new()),
            completion: None,
            spinners: ProgressSpinners::default(),
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
        let inner = view.inner_area();
        let area = view.area;
        let theme = &editor.theme;

        // DAP: Highlight current stack frame position
        let stack_frame = editor.debugger.as_ref().and_then(|debugger| {
            if let (Some(frame), Some(thread_id)) = (debugger.active_frame, debugger.thread_id) {
                debugger
                    .stack_frames
                    .get(&thread_id)
                    .and_then(|bt| bt.get(frame))
            } else {
                None
            }
        });
        if let Some(frame) = stack_frame {
            if doc.path().is_some()
                && frame
                    .source
                    .as_ref()
                    .and_then(|source| source.path.as_ref())
                    == doc.path()
            {
                let line = frame.line - 1; // convert to 0-indexing
                if line >= view.offset.row && line < view.offset.row + area.height as usize {
                    surface.set_style(
                        Rect::new(
                            area.x,
                            area.y + (line - view.offset.row) as u16,
                            area.width,
                            1,
                        ),
                        theme.get("ui.highlight"),
                    );
                }
            }
        }

        let highlights = Self::doc_syntax_highlights(doc, view.offset, inner.height, theme);
        let highlights = syntax::merge(highlights, Self::doc_diagnostics_highlights(doc, theme));
        let highlights: Box<dyn Iterator<Item = HighlightEvent>> = if is_focused {
            Box::new(syntax::merge(
                highlights,
                Self::doc_selection_highlights(doc, view, theme, &editor.config().cursor_shape),
            ))
        } else {
            Box::new(highlights)
        };

        Self::render_text_highlights(
            doc,
            view.offset,
            inner,
            surface,
            theme,
            highlights,
            &editor.config().whitespace,
        );
        Self::render_gutter(editor, doc, view, view.area, surface, theme, is_focused);
        Self::render_rulers(editor, doc, view, inner, surface, theme);

        if is_focused {
            Self::render_focused_view_elements(view, doc, inner, theme, surface);
        }

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

        self.render_diagnostics(doc, view, inner, surface, theme);

        let statusline_area = view
            .area
            .clip_top(view.area.height.saturating_sub(1))
            .clip_bottom(1); // -1 from bottom to remove commandline
        self.render_statusline(doc, view, statusline_area, surface, theme, is_focused);
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
        let ruler_theme = theme.get("ui.virtual.ruler");

        let rulers = doc
            .language_config()
            .and_then(|config| config.rulers.as_ref())
            .unwrap_or(editor_rulers);

        rulers
            .iter()
            // View might be horizontally scrolled, convert from absolute distance
            // from the 1st column to relative distance from left of viewport
            .filter_map(|ruler| ruler.checked_sub(1 + view.offset.col as u16))
            .filter(|ruler| ruler < &viewport.width)
            .map(|ruler| viewport.clip_left(ruler).with_width(1))
            .for_each(|area| surface.set_style(area, ruler_theme))
    }

    /// Get syntax highlights for a document in a view represented by the first line
    /// and column (`offset`) and the last line. This is done instead of using a view
    /// directly to enable rendering syntax highlighted docs anywhere (eg. picker preview)
    pub fn doc_syntax_highlights<'doc>(
        doc: &'doc Document,
        offset: Position,
        height: u16,
        _theme: &Theme,
    ) -> Box<dyn Iterator<Item = HighlightEvent> + 'doc> {
        let text = doc.text().slice(..);
        let last_line = std::cmp::min(
            // Saturating subs to make it inclusive zero indexing.
            (offset.row + height as usize).saturating_sub(1),
            doc.text().len_lines().saturating_sub(1),
        );

        let range = {
            // calculate viewport byte ranges
            let start = text.line_to_byte(offset.row);
            let end = text.line_to_byte(last_line + 1);

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
    ) -> Vec<(usize, std::ops::Range<usize>)> {
        use helix_core::diagnostic::Severity;
        let get_scope_of = |scope| {
            theme
            .find_scope_index(scope)
            // get one of the themes below as fallback values
            .or_else(|| theme.find_scope_index("diagnostic"))
            .or_else(|| theme.find_scope_index("ui.cursor"))
            .or_else(|| theme.find_scope_index("ui.selection"))
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

        doc.diagnostics()
            .iter()
            .map(|diagnostic| {
                let diagnostic_scope = match diagnostic.severity {
                    Some(Severity::Info) => info,
                    Some(Severity::Hint) => hint,
                    Some(Severity::Warning) => warning,
                    Some(Severity::Error) => error,
                    _ => r#default,
                };
                (
                    diagnostic_scope,
                    diagnostic.range.start..diagnostic.range.end,
                )
            })
            .collect()
    }

    /// Get highlight spans for selections in a document view.
    pub fn doc_selection_highlights(
        doc: &Document,
        view: &View,
        theme: &Theme,
        cursor_shape_config: &CursorShapeConfig,
    ) -> Vec<(usize, std::ops::Range<usize>)> {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);
        let primary_idx = selection.primary_index();

        let mode = doc.mode();
        let cursorkind = cursor_shape_config.from_mode(mode);
        let cursor_is_block = cursorkind == CursorKind::Block;

        let selection_scope = theme
            .find_scope_index("ui.selection")
            .expect("could not find `ui.selection` scope in the theme!");
        let base_cursor_scope = theme
            .find_scope_index("ui.cursor")
            .unwrap_or(selection_scope);

        let cursor_scope = match mode {
            Mode::Insert => theme.find_scope_index("ui.cursor.insert"),
            Mode::Select => theme.find_scope_index("ui.cursor.select"),
            Mode::Normal => Some(base_cursor_scope),
        }
        .unwrap_or(base_cursor_scope);

        let primary_cursor_scope = theme
            .find_scope_index("ui.cursor.primary")
            .unwrap_or(cursor_scope);
        let primary_selection_scope = theme
            .find_scope_index("ui.selection.primary")
            .unwrap_or(selection_scope);

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
                spans.push((selection_scope, range.anchor..cursor_start));
                if !selection_is_primary || cursor_is_block {
                    spans.push((cursor_scope, cursor_start..range.head));
                }
            } else {
                // Reverse case.
                let cursor_end = next_grapheme_boundary(text, range.head);
                if !selection_is_primary || cursor_is_block {
                    spans.push((cursor_scope, range.head..cursor_end));
                }
                spans.push((selection_scope, cursor_end..range.anchor));
            }
        }

        spans
    }

    pub fn render_text_highlights<H: Iterator<Item = HighlightEvent>>(
        doc: &Document,
        offset: Position,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        highlights: H,
        whitespace: &helix_view::editor::WhitespaceConfig,
    ) {
        use helix_view::editor::WhitespaceRenderValue;

        // It's slightly more efficient to produce a full RopeSlice from the Rope, then slice that a bunch
        // of times than it is to always call Rope::slice/get_slice (it will internally always hit RSEnum::Light).
        let text = doc.text().slice(..);

        let mut spans = Vec::new();
        let mut visual_x = 0u16;
        let mut line = 0u16;
        let tab_width = doc.tab_width();
        let tab = if whitespace.render.tab() == WhitespaceRenderValue::All {
            (1..tab_width).fold(whitespace.characters.tab.to_string(), |s, _| s + " ")
        } else {
            " ".repeat(tab_width)
        };
        let space = whitespace.characters.space.to_string();
        let nbsp = whitespace.characters.nbsp.to_string();
        let newline = if whitespace.render.newline() == WhitespaceRenderValue::All {
            whitespace.characters.newline.to_string()
        } else {
            " ".to_string()
        };

        let text_style = theme.get("ui.text");
        let whitespace_style = theme.get("ui.virtual.whitespace");

        'outer: for event in highlights {
            match event {
                HighlightEvent::HighlightStart(span) => {
                    spans.push(span);
                }
                HighlightEvent::HighlightEnd => {
                    spans.pop();
                }
                HighlightEvent::Source { start, end } => {
                    let is_trailing_cursor = text.len_chars() < end;

                    // `unwrap_or_else` part is for off-the-end indices of
                    // the rope, to allow cursor highlighting at the end
                    // of the rope.
                    let text = text.get_slice(start..end).unwrap_or_else(|| " ".into());
                    let style = spans
                        .iter()
                        .fold(text_style, |acc, span| acc.patch(theme.highlight(span.0)));

                    let space = if whitespace.render.space() == WhitespaceRenderValue::All
                        && !is_trailing_cursor
                    {
                        &space
                    } else {
                        " "
                    };

                    let nbsp = if whitespace.render.nbsp() == WhitespaceRenderValue::All
                        && text.len_chars() < end
                    {
                        &nbsp
                    } else {
                        " "
                    };

                    use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                    for grapheme in RopeGraphemes::new(text) {
                        let out_of_bounds = visual_x < offset.col as u16
                            || visual_x >= viewport.width + offset.col as u16;

                        if LineEnding::from_rope_slice(&grapheme).is_some() {
                            if !out_of_bounds {
                                // we still want to render an empty cell with the style
                                surface.set_string(
                                    viewport.x + visual_x - offset.col as u16,
                                    viewport.y + line,
                                    &newline,
                                    style.patch(whitespace_style),
                                );
                            }

                            visual_x = 0;
                            line += 1;

                            // TODO: with proper iter this shouldn't be necessary
                            if line >= viewport.height {
                                break 'outer;
                            }
                        } else {
                            let grapheme = Cow::from(grapheme);
                            let is_whitespace;

                            let (grapheme, width) = if grapheme == "\t" {
                                is_whitespace = true;
                                // make sure we display tab as appropriate amount of spaces
                                let visual_tab_width = tab_width - (visual_x as usize % tab_width);
                                let grapheme_tab_width =
                                    helix_core::str_utils::char_to_byte_idx(&tab, visual_tab_width);

                                (&tab[..grapheme_tab_width], visual_tab_width)
                            } else if grapheme == " " {
                                is_whitespace = true;
                                (space, 1)
                            } else if grapheme == "\u{00A0}" {
                                is_whitespace = true;
                                (nbsp, 1)
                            } else {
                                is_whitespace = false;
                                // Cow will prevent allocations if span contained in a single slice
                                // which should really be the majority case
                                let width = grapheme_width(&grapheme);
                                (grapheme.as_ref(), width)
                            };

                            if !out_of_bounds {
                                // if we're offscreen just keep going until we hit a new line
                                surface.set_string(
                                    viewport.x + visual_x - offset.col as u16,
                                    viewport.y + line,
                                    grapheme,
                                    if is_whitespace {
                                        style.patch(whitespace_style)
                                    } else {
                                        style
                                    },
                                );
                            }

                            visual_x = visual_x.saturating_add(width as u16);
                        }
                    }
                }
            }
        }
    }

    /// Render brace match, etc (meant for the focused view only)
    pub fn render_focused_view_elements(
        view: &View,
        doc: &Document,
        viewport: Rect,
        theme: &Theme,
        surface: &mut Surface,
    ) {
        // Highlight matching braces
        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            use helix_core::match_brackets;
            let pos = doc.selection(view.id).primary().cursor(text);

            let pos = match_brackets::find_matching_bracket(syntax, doc.text(), pos)
                .and_then(|pos| view.screen_coords_at_pos(doc, text, pos));

            if let Some(pos) = pos {
                // ensure col is on screen
                if (pos.col as u16) < viewport.width + view.offset.col as u16
                    && pos.col >= view.offset.col
                {
                    let style = theme.try_get("ui.cursor.match").unwrap_or_else(|| {
                        Style::default()
                            .add_modifier(Modifier::REVERSED)
                            .add_modifier(Modifier::DIM)
                    });

                    surface[(viewport.x + pos.col as u16, viewport.y + pos.row as u16)]
                        .set_style(style);
                }
            }
        }
    }

    pub fn render_gutter(
        editor: &Editor,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
    ) {
        let text = doc.text().slice(..);
        let last_line = view.last_line(doc);

        // it's used inside an iterator so the collect isn't needless:
        // https://github.com/rust-lang/rust-clippy/issues/6164
        #[allow(clippy::needless_collect)]
        let cursors: Vec<_> = doc
            .selection(view.id)
            .iter()
            .map(|range| range.cursor_line(text))
            .collect();

        let mut offset = 0;

        let gutter_style = theme.get("ui.gutter");

        // avoid lots of small allocations by reusing a text buffer for each line
        let mut text = String::with_capacity(8);

        for (constructor, width) in &view.gutters {
            let gutter = constructor(editor, doc, view, theme, is_focused, *width);
            text.reserve(*width); // ensure there's enough space for the gutter
            for (i, line) in (view.offset.row..(last_line + 1)).enumerate() {
                let selected = cursors.contains(&line);
                let x = viewport.x + offset;
                let y = viewport.y + i as u16;

                if let Some(style) = gutter(line, selected, &mut text) {
                    surface.set_stringn(x, y, &text, *width, gutter_style.patch(style));
                } else {
                    surface.set_style(
                        Rect {
                            x,
                            y,
                            width: *width as u16,
                            height: 1,
                        },
                        gutter_style,
                    );
                }
                text.clear();
            }

            offset += *width as u16;
        }
    }

    pub fn render_diagnostics(
        &self,
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
        for diagnostic in diagnostics {
            let text = Text::styled(
                &diagnostic.message,
                match diagnostic.severity {
                    Some(Severity::Error) => error,
                    Some(Severity::Warning) | None => warning,
                    Some(Severity::Info) => info,
                    Some(Severity::Hint) => hint,
                },
            );
            lines.extend(text.lines);
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

    pub fn render_statusline(
        &self,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
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
            .and_then(|srv| {
                self.spinners
                    .get(srv.id())
                    .and_then(|spinner| spinner.frame())
            })
            .unwrap_or("");

        let base_style = if is_focused {
            theme.get("ui.statusline")
        } else {
            theme.get("ui.statusline.inactive")
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
        let warning_style = theme.get("warning");
        let error_style = theme.get("error");
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

        let enc = doc.encoding();
        if enc != encoding::UTF_8 {
            right_side_text
                .0
                .push(Span::styled(format!(" {} ", enc.name()), base_style));
        }

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
        let key_result = self.keymaps.get(mode, event);
        cxt.editor.autoinfo = self.keymaps.sticky().map(|node| node.infobox());

        match &key_result {
            KeymapResult::Matched(command) => command.execute(cxt),
            KeymapResult::Pending(node) => cxt.editor.autoinfo = Some(node.infobox()),
            KeymapResult::MatchedSequence(commands) => {
                for command in commands {
                    command.execute(cxt);
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
                // first execute whatever put us into insert mode
                self.last_insert.0.execute(cxt);
                // then replay the inputs
                for key in self.last_insert.1.clone() {
                    match key {
                        InsertEvent::Key(key) => self.insert_mode(cxt, key),
                        InsertEvent::CompletionApply(compl) => {
                            let (view, doc) = current!(cxt.editor);

                            doc.restore(view.id);

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
                            let (_, doc) = current!(cxt.editor);
                            doc.savepoint();
                        }
                    }
                }
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

    pub fn set_completion(
        &mut self,
        editor: &mut Editor,
        items: Vec<helix_lsp::lsp::CompletionItem>,
        offset_encoding: helix_lsp::OffsetEncoding,
        start_offset: usize,
        trigger_offset: usize,
        size: Rect,
    ) {
        let mut completion =
            Completion::new(editor, items, offset_encoding, start_offset, trigger_offset);

        if completion.is_empty() {
            // skip if we got no completion results
            return;
        }

        // Immediately initialize a savepoint
        doc_mut!(editor).savepoint();

        editor.last_completion = None;
        self.last_insert.1.push(InsertEvent::TriggerCompletion);

        // TODO : propagate required size on resize to completion too
        completion.required_size((size.width, size.height));
        self.completion = Some(completion);
    }

    pub fn clear_completion(&mut self, editor: &mut Editor) {
        self.completion = None;

        // Clear any savepoints
        let doc = doc_mut!(editor);
        doc.savepoint = None;
        editor.clear_idle_timer(); // don't retrigger
    }

    pub fn handle_idle_timeout(&mut self, cx: &mut crate::compositor::Context) -> EventResult {
        if self.completion.is_some()
            || !cx.editor.config().auto_completion
            || doc!(cx.editor).mode != Mode::Insert
        {
            return EventResult::Ignored(None);
        }

        let mut cx = commands::Context {
            register: None,
            editor: cx.editor,
            jobs: cx.jobs,
            count: None,
            callback: None,
            on_next_key_callback: None,
        };
        crate::commands::insert::idle_completion(&mut cx);

        EventResult::Consumed(None)
    }
}

impl EditorView {
    fn handle_mouse_event(
        &mut self,
        event: MouseEvent,
        cxt: &mut commands::Context,
    ) -> EventResult {
        let config = cxt.editor.config();
        match event {
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                row,
                column,
                modifiers,
                ..
            } => {
                let editor = &mut cxt.editor;

                let result = editor.tree.views().find_map(|(view, _focus)| {
                    view.pos_at_screen_coords(&editor.documents[&view.doc], row, column)
                        .map(|pos| (pos, view.id))
                });

                if let Some((pos, view_id)) = result {
                    let doc = editor.document_mut(editor.tree.get(view_id).doc).unwrap();

                    if modifiers == crossterm::event::KeyModifiers::ALT {
                        let selection = doc.selection(view_id).clone();
                        doc.set_selection(view_id, selection.push(Range::point(pos)));
                    } else {
                        doc.set_selection(view_id, Selection::point(pos));
                    }

                    editor.tree.focus = view_id;

                    return EventResult::Consumed(None);
                }

                let result = editor.tree.views().find_map(|(view, _focus)| {
                    view.gutter_coords_at_screen_coords(row, column)
                        .map(|coords| (coords, view.id))
                });

                if let Some((coords, view_id)) = result {
                    editor.tree.focus = view_id;

                    let view = editor.tree.get(view_id);
                    let doc = editor.documents.get_mut(&view.doc).unwrap();

                    let path = match doc.path() {
                        Some(path) => path.clone(),
                        None => return EventResult::Ignored(None),
                    };

                    let line = coords.row + view.offset.row;
                    if line < doc.text().len_lines() {
                        commands::dap_toggle_breakpoint_impl(cxt, path, line);
                        return EventResult::Consumed(None);
                    }
                }

                EventResult::Ignored(None)
            }

            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                row,
                column,
                ..
            } => {
                let (view, doc) = current!(cxt.editor);

                let pos = match view.pos_at_screen_coords(doc, row, column) {
                    Some(pos) => pos,
                    None => return EventResult::Ignored(None),
                };

                let mut selection = doc.selection(view.id).clone();
                let primary = selection.primary_mut();
                *primary = primary.put_cursor(doc.text().slice(..), pos, true);
                doc.set_selection(view.id, selection);
                EventResult::Consumed(None)
            }

            MouseEvent {
                kind: MouseEventKind::ScrollUp | MouseEventKind::ScrollDown,
                row,
                column,
                ..
            } => {
                let current_view = cxt.editor.tree.focus;

                let direction = match event.kind {
                    MouseEventKind::ScrollUp => Direction::Backward,
                    MouseEventKind::ScrollDown => Direction::Forward,
                    _ => unreachable!(),
                };

                let result = cxt.editor.tree.views().find_map(|(view, _focus)| {
                    view.pos_at_screen_coords(&cxt.editor.documents[&view.doc], row, column)
                        .map(|_| view.id)
                });

                match result {
                    Some(view_id) => cxt.editor.tree.focus = view_id,
                    None => return EventResult::Ignored(None),
                }

                let offset = config.scroll_lines.abs() as usize;
                commands::scroll(cxt, offset, direction);

                cxt.editor.tree.focus = current_view;

                EventResult::Consumed(None)
            }

            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                ..
            } => {
                if !config.middle_click_paste {
                    return EventResult::Ignored(None);
                }

                let (view, doc) = current!(cxt.editor);
                let range = doc.selection(view.id).primary();

                if range.to() - range.from() <= 1 {
                    return EventResult::Ignored(None);
                }

                commands::MappableCommand::yank_main_selection_to_primary_clipboard.execute(cxt);

                EventResult::Consumed(None)
            }

            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Right),
                row,
                column,
                modifiers,
                ..
            } => {
                let result = cxt.editor.tree.views().find_map(|(view, _focus)| {
                    view.gutter_coords_at_screen_coords(row, column)
                        .map(|coords| (coords, view.id))
                });

                if let Some((coords, view_id)) = result {
                    cxt.editor.tree.focus = view_id;

                    let view = cxt.editor.tree.get(view_id);
                    let doc = cxt.editor.documents.get_mut(&view.doc).unwrap();
                    let line = coords.row + view.offset.row;
                    if let Ok(pos) = doc.text().try_line_to_char(line) {
                        doc.set_selection(view_id, Selection::point(pos));
                        if modifiers == crossterm::event::KeyModifiers::ALT {
                            commands::MappableCommand::dap_edit_log.execute(cxt);
                        } else {
                            commands::MappableCommand::dap_edit_condition.execute(cxt);
                        }

                        return EventResult::Consumed(None);
                    }
                }
                EventResult::Ignored(None)
            }

            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Middle),
                row,
                column,
                modifiers,
                ..
            } => {
                let editor = &mut cxt.editor;
                if !config.middle_click_paste {
                    return EventResult::Ignored(None);
                }

                if modifiers == crossterm::event::KeyModifiers::ALT {
                    commands::MappableCommand::replace_selections_with_primary_clipboard
                        .execute(cxt);

                    return EventResult::Consumed(None);
                }

                let result = editor.tree.views().find_map(|(view, _focus)| {
                    view.pos_at_screen_coords(&editor.documents[&view.doc], row, column)
                        .map(|pos| (pos, view.id))
                });

                if let Some((pos, view_id)) = result {
                    let doc = editor.document_mut(editor.tree.get(view_id).doc).unwrap();
                    doc.set_selection(view_id, Selection::point(pos));
                    editor.tree.focus = view_id;
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
        event: Event,
        context: &mut crate::compositor::Context,
    ) -> EventResult {
        let mut cx = commands::Context {
            editor: context.editor,
            count: None,
            register: None,
            callback: None,
            on_next_key_callback: None,
            jobs: context.jobs,
        };

        match event {
            Event::Resize(_width, _height) => {
                // Ignore this event, we handle resizing just before rendering to screen.
                // Handling it here but not re-rendering will cause flashing
                EventResult::Consumed(None)
            }
            Event::Key(key) => {
                cx.editor.reset_idle_timer();
                let mut key = KeyEvent::from(key);
                canonicalize_key(&mut key);

                // clear status
                cx.editor.status_msg = None;

                let doc = doc!(cx.editor);
                let mode = doc.mode();

                if let Some(on_next_key) = self.on_next_key.take() {
                    // if there's a command waiting input, do that first
                    on_next_key(&mut cx, key);
                } else {
                    match mode {
                        Mode::Insert => {
                            // let completion swallow the event if necessary
                            let mut consumed = false;
                            if let Some(completion) = &mut self.completion {
                                // use a fake context here
                                let mut cx = Context {
                                    editor: cx.editor,
                                    jobs: cx.jobs,
                                    scroll: None,
                                };
                                let res = completion.handle_event(event, &mut cx);

                                if let EventResult::Consumed(callback) = res {
                                    consumed = true;

                                    if callback.is_some() {
                                        // assume close_fn
                                        self.clear_completion(cx.editor);
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
                // appease borrowck
                let callback = cx.callback.take();

                // if the command consumed the last view, skip the render.
                // on the next loop cycle the Application will then terminate.
                if cx.editor.should_close() {
                    return EventResult::Ignored(None);
                }
                let config = cx.editor.config();
                let (view, doc) = current!(cx.editor);
                view.ensure_cursor_in_view(doc, config.scrolloff);

                // Store a history state if not in insert mode. This also takes care of
                // committing changes when leaving insert mode.
                if doc.mode() != Mode::Insert {
                    doc.append_changes_to_history(view.id);
                }

                // mode transitions
                match (mode, doc.mode()) {
                    (Mode::Normal, Mode::Insert) => {
                        // HAXX: if we just entered insert mode from normal, clear key buf
                        // and record the command that got us into this mode.

                        // how we entered insert mode is important, and we should track that so
                        // we can repeat the side effect.

                        self.last_insert.0 = match self.keymaps.get(mode, key) {
                            KeymapResult::Matched(command) => command,
                            // FIXME: insert mode can only be entered through single KeyCodes
                            _ => unimplemented!(),
                        };
                        self.last_insert.1.clear();
                    }
                    (Mode::Insert, Mode::Normal) => {
                        // if exiting insert mode, remove completion
                        self.completion = None;
                    }
                    _ => (),
                }

                EventResult::Consumed(callback)
            }

            Event::Mouse(event) => self.handle_mouse_event(event, &mut cx),
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // clear with background color
        surface.set_style(area, cx.editor.theme.get("ui.background"));
        let config = cx.editor.config();
        // if the terminal size suddenly changed, we need to trigger a resize
        cx.editor.resize(area.clip_bottom(1)); // -1 from bottom for commandline

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
                let s = key.to_string();
                if s.graphemes(true).count() > 1 {
                    disp.push_str(&format!("<{}>", s));
                } else {
                    disp.push_str(&s);
                }
            }
            if let Some(pseudo_pending) = &cx.editor.pseudo_pending {
                disp.push_str(pseudo_pending.as_str())
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
    }

    fn cursor(&self, _area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
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
