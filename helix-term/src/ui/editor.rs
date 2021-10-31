use crate::{
    commands,
    compositor::{Component, Context, EventResult},
    key,
    keymap::{KeymapResult, KeymapResultKind, Keymaps},
    ui::{Completion, ProgressSpinners},
};

use helix_core::{
    coords_at_pos,
    graphemes::{ensure_grapheme_boundary_next, next_grapheme_boundary, prev_grapheme_boundary},
    movement::Direction,
    syntax::{self, HighlightEvent},
    unicode::segmentation::UnicodeSegmentation,
    unicode::width::UnicodeWidthStr,
    LineEnding, Position, Range, Selection,
};
use helix_view::{
    document::Mode,
    editor::LineNumber,
    graphics::{CursorKind, Modifier, Rect, Style},
    info::Info,
    input::KeyEvent,
    keyboard::{KeyCode, KeyModifiers},
    Document, Editor, Theme, View,
};
use std::borrow::Cow;

use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use tui::buffer::Buffer as Surface;

pub struct EditorView {
    keymaps: Keymaps,
    on_next_key: Option<Box<dyn FnOnce(&mut commands::Context, KeyEvent)>>,
    last_insert: (commands::Command, Vec<KeyEvent>),
    pub(crate) completion: Option<Completion>,
    spinners: ProgressSpinners,
    autoinfo: Option<Info>,
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
            last_insert: (commands::Command::normal_mode, Vec::new()),
            completion: None,
            spinners: ProgressSpinners::default(),
            autoinfo: None,
        }
    }

    pub fn spinners_mut(&mut self) -> &mut ProgressSpinners {
        &mut self.spinners
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_view(
        &self,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
        loader: &syntax::Loader,
        config: &helix_view::editor::Config,
    ) {
        let inner = view.inner_area();
        let area = view.area;

        let highlights = Self::doc_syntax_highlights(doc, view.offset, inner.height, theme, loader);
        let highlights = syntax::merge(highlights, Self::doc_diagnostics_highlights(doc, theme));
        let highlights: Box<dyn Iterator<Item = HighlightEvent>> = if is_focused {
            Box::new(syntax::merge(
                highlights,
                Self::doc_selection_highlights(doc, view, theme),
            ))
        } else {
            Box::new(highlights)
        };

        Self::render_text_highlights(doc, view.offset, inner, surface, theme, highlights);
        Self::render_gutter(doc, view, view.area, surface, theme, is_focused, config);

        if is_focused {
            Self::render_focused_view_elements(view, doc, inner, theme, surface);
        }

        // if we're not at the edge of the screen, draw a right border
        if viewport.right() != view.area.right() {
            let x = area.right();
            let border_style = theme.get("ui.window");
            for y in area.top()..area.bottom() {
                surface
                    .get_mut(x, y)
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

    /// Get syntax highlights for a document in a view represented by the first line
    /// and column (`offset`) and the last line. This is done instead of using a view
    /// directly to enable rendering syntax highlighted docs anywhere (eg. picker preview)
    #[allow(clippy::too_many_arguments)]
    pub fn doc_syntax_highlights<'doc>(
        doc: &'doc Document,
        offset: Position,
        height: u16,
        theme: &Theme,
        loader: &syntax::Loader,
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

        // TODO: range doesn't actually restrict source, just highlight range
        let highlights = match doc.syntax() {
            Some(syntax) => {
                let scopes = theme.scopes();
                syntax
                    .highlight_iter(text.slice(..), Some(range), None, |language| {
                        loader.language_configuration_for_injection_string(language)
                            .and_then(|language_config| {
                                let config = language_config.highlight_config(scopes)?;
                                let config_ref = config.as_ref();
                                // SAFETY: the referenced `HighlightConfiguration` behind
                                // the `Arc` is guaranteed to remain valid throughout the
                                // duration of the highlight.
                                let config_ref = unsafe {
                                    std::mem::transmute::<
                                        _,
                                        &'static syntax::HighlightConfiguration,
                                    >(config_ref)
                                };
                                Some(config_ref)
                            })
                    })
                    .map(|event| event.unwrap())
                    .collect() // TODO: we collect here to avoid holding the lock, fix later
            }
            None => vec![HighlightEvent::Source {
                start: range.start,
                end: range.end,
            }],
        }
        .into_iter()
        .map(move |event| match event {
            // convert byte offsets to char offset
            HighlightEvent::Source { start, end } => {
                let start = ensure_grapheme_boundary_next(text, text.byte_to_char(start));
                let end = ensure_grapheme_boundary_next(text, text.byte_to_char(end));
                HighlightEvent::Source { start, end }
            }
            event => event,
        });

        Box::new(highlights)
    }

    /// Get highlight spans for document diagnostics
    pub fn doc_diagnostics_highlights(
        doc: &Document,
        theme: &Theme,
    ) -> Vec<(usize, std::ops::Range<usize>)> {
        let diagnostic_scope = theme
            .find_scope_index("diagnostic")
            .or_else(|| theme.find_scope_index("ui.cursor"))
            .or_else(|| theme.find_scope_index("ui.selection"))
            .expect(
                "at least one of the following scopes must be defined in the theme: `diagnostic`, `ui.cursor`, or `ui.selection`",
            );

        doc.diagnostics()
            .iter()
            .map(|diagnostic| {
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
    ) -> Vec<(usize, std::ops::Range<usize>)> {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);
        let primary_idx = selection.primary_index();

        let selection_scope = theme
            .find_scope_index("ui.selection")
            .expect("could not find `ui.selection` scope in the theme!");
        let base_cursor_scope = theme
            .find_scope_index("ui.cursor")
            .unwrap_or(selection_scope);

        let cursor_scope = match doc.mode() {
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
            let (cursor_scope, selection_scope) = if i == primary_idx {
                (primary_cursor_scope, primary_selection_scope)
            } else {
                (cursor_scope, selection_scope)
            };

            // Special-case: cursor at end of the rope.
            if range.head == range.anchor && range.head == text.len_chars() {
                spans.push((cursor_scope, range.head..range.head + 1));
                continue;
            }

            let range = range.min_width_1(text);
            if range.head > range.anchor {
                // Standard case.
                let cursor_start = prev_grapheme_boundary(text, range.head);
                spans.push((selection_scope, range.anchor..cursor_start));
                spans.push((cursor_scope, cursor_start..range.head));
            } else {
                // Reverse case.
                let cursor_end = next_grapheme_boundary(text, range.head);
                spans.push((cursor_scope, range.head..cursor_end));
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
    ) {
        let text = doc.text().slice(..);

        let mut spans = Vec::new();
        let mut visual_x = 0u16;
        let mut line = 0u16;
        let tab_width = doc.tab_width();
        let tab = " ".repeat(tab_width);

        let text_style = theme.get("ui.text");

        'outer: for event in highlights {
            match event {
                HighlightEvent::HighlightStart(span) => {
                    spans.push(span);
                }
                HighlightEvent::HighlightEnd => {
                    spans.pop();
                }
                HighlightEvent::Source { start, end } => {
                    // `unwrap_or_else` part is for off-the-end indices of
                    // the rope, to allow cursor highlighting at the end
                    // of the rope.
                    let text = text.get_slice(start..end).unwrap_or_else(|| " ".into());

                    use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                    let style = spans.iter().fold(text_style, |acc, span| {
                        let style = theme.get(theme.scopes()[span.0].as_str());
                        acc.patch(style)
                    });

                    for grapheme in RopeGraphemes::new(text) {
                        let out_of_bounds = visual_x < offset.col as u16
                            || visual_x >= viewport.width + offset.col as u16;

                        if LineEnding::from_rope_slice(&grapheme).is_some() {
                            if !out_of_bounds {
                                // we still want to render an empty cell with the style
                                surface.set_string(
                                    viewport.x + visual_x - offset.col as u16,
                                    viewport.y + line,
                                    " ",
                                    style,
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

                            let (grapheme, width) = if grapheme == "\t" {
                                // make sure we display tab as appropriate amount of spaces
                                (tab.as_str(), tab_width)
                            } else {
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
                                    style,
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

            let pos = match_brackets::find(syntax, doc.text(), pos)
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

                    surface
                        .get_mut(viewport.x + pos.col as u16, viewport.y + pos.row as u16)
                        .set_style(style);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_gutter(
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
        config: &helix_view::editor::Config,
    ) {
        let text = doc.text().slice(..);
        let last_line = view.last_line(doc);

        let linenr = theme.get("ui.linenr");
        let linenr_select: Style = theme.try_get("ui.linenr.selected").unwrap_or(linenr);

        let warning = theme.get("warning");
        let error = theme.get("error");
        let info = theme.get("info");
        let hint = theme.get("hint");

        // Whether to draw the line number for the last line of the
        // document or not.  We only draw it if it's not an empty line.
        let draw_last = text.line_to_byte(last_line) < text.len_bytes();

        let current_line = doc
            .text()
            .char_to_line(doc.selection(view.id).primary().cursor(text));

        // it's used inside an iterator so the collect isn't needless:
        // https://github.com/rust-lang/rust-clippy/issues/6164
        #[allow(clippy::needless_collect)]
        let cursors: Vec<_> = doc
            .selection(view.id)
            .iter()
            .map(|range| range.cursor_line(text))
            .collect();

        for (i, line) in (view.offset.row..(last_line + 1)).enumerate() {
            use helix_core::diagnostic::Severity;
            if let Some(diagnostic) = doc.diagnostics().iter().find(|d| d.line == line) {
                surface.set_stringn(
                    viewport.x,
                    viewport.y + i as u16,
                    "●",
                    1,
                    match diagnostic.severity {
                        Some(Severity::Error) => error,
                        Some(Severity::Warning) | None => warning,
                        Some(Severity::Info) => info,
                        Some(Severity::Hint) => hint,
                    },
                );
            }

            let selected = cursors.contains(&line);

            let text = if line == last_line && !draw_last {
                "    ~".into()
            } else {
                let line = match config.line_number {
                    LineNumber::Absolute => line + 1,
                    LineNumber::Relative => {
                        if current_line == line {
                            line + 1
                        } else {
                            abs_diff(current_line, line)
                        }
                    }
                };
                format!("{:>5}", line)
            };
            surface.set_stringn(
                viewport.x + 1,
                viewport.y + i as u16,
                text,
                5,
                if selected && is_focused {
                    linenr_select
                } else {
                    linenr
                },
            );
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
            widgets::{Paragraph, Widget},
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

        // Vec::with_capacity(diagnostics.len()); // rough estimate
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

        let paragraph = Paragraph::new(lines).alignment(Alignment::Right);
        let width = 80.min(viewport.width);
        let height = 15.min(viewport.height);
        paragraph.render(
            Rect::new(viewport.right() - width, viewport.y + 1, width, height),
            surface,
        );
    }

    #[allow(clippy::too_many_arguments)]
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

        if let Some(path) = doc.relative_path() {
            let path = path.to_string_lossy();

            let title = format!("{}{}", path, if doc.is_modified() { "[+]" } else { "" });
            surface.set_stringn(
                viewport.x + 8,
                viewport.y,
                title,
                viewport.width.saturating_sub(6) as usize,
                base_style,
            );
        }

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

        // let indent_info = match doc.indent_style {
        //     IndentStyle::Tabs => "tabs",
        //     IndentStyle::Spaces(1) => "spaces:1",
        //     IndentStyle::Spaces(2) => "spaces:2",
        //     IndentStyle::Spaces(3) => "spaces:3",
        //     IndentStyle::Spaces(4) => "spaces:4",
        //     IndentStyle::Spaces(5) => "spaces:5",
        //     IndentStyle::Spaces(6) => "spaces:6",
        //     IndentStyle::Spaces(7) => "spaces:7",
        //     IndentStyle::Spaces(8) => "spaces:8",
        //     _ => "indent:ERROR",
        // };

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
    }

    /// Handle events by looking them up in `self.keymaps`. Returns None
    /// if event was handled (a command was executed or a subkeymap was
    /// activated). Only KeymapResultKind::{NotFound, Cancelled} is returned
    /// otherwise.
    fn handle_keymap_event(
        &mut self,
        mode: Mode,
        cxt: &mut commands::Context,
        event: KeyEvent,
    ) -> Option<KeymapResult> {
        self.autoinfo = None;
        let key_result = self.keymaps.get_mut(&mode).unwrap().get(event);
        self.autoinfo = key_result.sticky.map(|node| node.infobox());

        match &key_result.kind {
            KeymapResultKind::Matched(command) => command.execute(cxt),
            KeymapResultKind::Pending(node) => self.autoinfo = Some(node.infobox()),
            KeymapResultKind::NotFound | KeymapResultKind::Cancelled(_) => return Some(key_result),
        }
        None
    }

    fn insert_mode(&mut self, cx: &mut commands::Context, event: KeyEvent) {
        if let Some(keyresult) = self.handle_keymap_event(Mode::Insert, cx, event) {
            match keyresult.kind {
                KeymapResultKind::NotFound => {
                    if let Some(ch) = event.char() {
                        commands::insert::insert_char(cx, ch)
                    }
                }
                KeymapResultKind::Cancelled(pending) => {
                    for ev in pending {
                        match ev.char() {
                            Some(ch) => commands::insert::insert_char(cx, ch),
                            None => {
                                if let KeymapResultKind::Matched(command) =
                                    self.keymaps.get_mut(&Mode::Insert).unwrap().get(ev).kind
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
        match event {
            // count handling
            key!(i @ '0'..='9') => {
                let i = i.to_digit(10).unwrap() as usize;
                cxt.editor.count =
                    std::num::NonZeroUsize::new(cxt.editor.count.map_or(i, |c| c.get() * 10 + i));
            }
            // special handling for repeat operator
            key!('.') => {
                // first execute whatever put us into insert mode
                self.last_insert.0.execute(cxt);
                // then replay the inputs
                for &key in &self.last_insert.1.clone() {
                    self.insert_mode(cxt, key)
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

        // TODO : propagate required size on resize to completion too
        completion.required_size((size.width, size.height));
        self.completion = Some(completion);
    }

    pub fn clear_completion(&mut self, editor: &mut Editor) {
        self.completion = None;
        // Clear any savepoints
        let (_, doc) = current!(editor);
        doc.savepoint = None;
        editor.clear_idle_timer(); // don't retrigger
    }
}

impl EditorView {
    fn handle_mouse_event(
        &mut self,
        event: MouseEvent,
        cxt: &mut commands::Context,
    ) -> EventResult {
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
                    view.pos_at_screen_coords(&editor.documents[view.doc], row, column)
                        .map(|pos| (pos, view.id))
                });

                if let Some((pos, view_id)) = result {
                    let doc = &mut editor.documents[editor.tree.get(view_id).doc];

                    if modifiers == crossterm::event::KeyModifiers::ALT {
                        let selection = doc.selection(view_id).clone();
                        doc.set_selection(view_id, selection.push(Range::point(pos)));
                    } else {
                        doc.set_selection(view_id, Selection::point(pos));
                    }

                    editor.tree.focus = view_id;

                    return EventResult::Consumed(None);
                }

                EventResult::Ignored
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
                    None => return EventResult::Ignored,
                };

                let mut selection = doc.selection(view.id).clone();
                let primary = selection.primary_mut();
                *primary = Range::new(primary.anchor, pos);
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
                    view.pos_at_screen_coords(&cxt.editor.documents[view.doc], row, column)
                        .map(|_| view.id)
                });

                match result {
                    Some(view_id) => cxt.editor.tree.focus = view_id,
                    None => return EventResult::Ignored,
                }

                let offset = cxt.editor.config.scroll_lines.abs() as usize;
                commands::scroll(cxt, offset, direction);

                cxt.editor.tree.focus = current_view;

                EventResult::Consumed(None)
            }

            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                ..
            } => {
                if !cxt.editor.config.middle_click_paste {
                    return EventResult::Ignored;
                }

                let (view, doc) = current!(cxt.editor);
                let range = doc.selection(view.id).primary();

                if range.to() - range.from() <= 1 {
                    return EventResult::Ignored;
                }

                commands::Command::yank_main_selection_to_primary_clipboard.execute(cxt);

                EventResult::Consumed(None)
            }

            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Middle),
                row,
                column,
                modifiers,
                ..
            } => {
                let editor = &mut cxt.editor;
                if !editor.config.middle_click_paste {
                    return EventResult::Ignored;
                }

                if modifiers == crossterm::event::KeyModifiers::ALT {
                    commands::Command::replace_selections_with_primary_clipboard.execute(cxt);

                    return EventResult::Consumed(None);
                }

                let result = editor.tree.views().find_map(|(view, _focus)| {
                    view.pos_at_screen_coords(&editor.documents[view.doc], row, column)
                        .map(|pos| (pos, view.id))
                });

                if let Some((pos, view_id)) = result {
                    let doc = &mut editor.documents[editor.tree.get(view_id).doc];
                    doc.set_selection(view_id, Selection::point(pos));
                    editor.tree.focus = view_id;
                    commands::Command::paste_primary_clipboard_before.execute(cxt);
                    return EventResult::Consumed(None);
                }

                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }
}

impl Component for EditorView {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let mut cxt = commands::Context {
            editor: &mut cx.editor,
            count: None,
            register: None,
            callback: None,
            on_next_key_callback: None,
            jobs: cx.jobs,
        };

        match event {
            Event::Resize(_width, _height) => {
                // Ignore this event, we handle resizing just before rendering to screen.
                // Handling it here but not re-rendering will cause flashing
                EventResult::Consumed(None)
            }
            Event::Key(key) => {
                cxt.editor.reset_idle_timer();
                let mut key = KeyEvent::from(key);
                canonicalize_key(&mut key);
                // clear status
                cxt.editor.status_msg = None;

                let (_, doc) = current!(cxt.editor);
                let mode = doc.mode();

                if let Some(on_next_key) = self.on_next_key.take() {
                    // if there's a command waiting input, do that first
                    on_next_key(&mut cxt, key);
                } else {
                    match mode {
                        Mode::Insert => {
                            // record last_insert key
                            self.last_insert.1.push(key);

                            // let completion swallow the event if necessary
                            let mut consumed = false;
                            if let Some(completion) = &mut self.completion {
                                // use a fake context here
                                let mut cx = Context {
                                    editor: cxt.editor,
                                    jobs: cxt.jobs,
                                    scroll: None,
                                };
                                let res = completion.handle_event(event, &mut cx);

                                if let EventResult::Consumed(callback) = res {
                                    consumed = true;

                                    if callback.is_some() {
                                        // assume close_fn
                                        self.clear_completion(cxt.editor);
                                    }
                                }
                            }

                            // if completion didn't take the event, we pass it onto commands
                            if !consumed {
                                self.insert_mode(&mut cxt, key);

                                // lastly we recalculate completion
                                if let Some(completion) = &mut self.completion {
                                    completion.update(&mut cxt);
                                    if completion.is_empty() {
                                        self.clear_completion(cxt.editor);
                                    }
                                }
                            }
                        }
                        mode => self.command_mode(mode, &mut cxt, key),
                    }
                }

                self.on_next_key = cxt.on_next_key_callback.take();
                // appease borrowck
                let callback = cxt.callback.take();

                // if the command consumed the last view, skip the render.
                // on the next loop cycle the Application will then terminate.
                if cxt.editor.should_close() {
                    return EventResult::Ignored;
                }

                let (view, doc) = current!(cxt.editor);
                view.ensure_cursor_in_view(doc, cxt.editor.config.scrolloff);

                // mode transitions
                match (mode, doc.mode()) {
                    (Mode::Normal, Mode::Insert) => {
                        // HAXX: if we just entered insert mode from normal, clear key buf
                        // and record the command that got us into this mode.

                        // how we entered insert mode is important, and we should track that so
                        // we can repeat the side effect.

                        self.last_insert.0 =
                            match self.keymaps.get_mut(&mode).unwrap().get(key).kind {
                                KeymapResultKind::Matched(command) => command,
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

            Event::Mouse(event) => self.handle_mouse_event(event, &mut cxt),
        }
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // clear with background color
        surface.set_style(area, cx.editor.theme.get("ui.background"));

        // if the terminal size suddenly changed, we need to trigger a resize
        cx.editor.resize(area.clip_bottom(1)); // -1 from bottom for commandline

        for (view, is_focused) in cx.editor.tree.views() {
            let doc = cx.editor.document(view.doc).unwrap();
            let loader = &cx.editor.syn_loader;
            self.render_view(
                doc,
                view,
                area,
                surface,
                &cx.editor.theme,
                is_focused,
                loader,
                &cx.editor.config,
            );
        }

        if let Some(ref mut info) = self.autoinfo {
            info.render(area, surface, cx);
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
            surface.set_string(
                area.x + area.width.saturating_sub(key_width),
                area.y + area.height.saturating_sub(1),
                disp.get(disp.len().saturating_sub(key_width as usize)..)
                    .unwrap_or(&disp),
                cx.editor.theme.get("ui.text"),
            );
        }

        if let Some(completion) = self.completion.as_mut() {
            completion.render(area, surface, cx);
        }
    }

    fn cursor(&self, _area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        // match view.doc.mode() {
        //     Mode::Insert => write!(stdout, "\x1B[6 q"),
        //     mode => write!(stdout, "\x1B[2 q"),
        // };
        editor.cursor()
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

#[inline]
const fn abs_diff(a: usize, b: usize) -> usize {
    if a > b {
        a - b
    } else {
        b - a
    }
}
