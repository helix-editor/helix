use crate::{
    commands,
    compositor::{Component, Context, EventResult},
    key,
    keymap::{KeymapResult, Keymaps},
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
    completion: Option<Completion>,
    spinners: ProgressSpinners,
    pub autoinfo: Option<Info>,
}

pub const GUTTER_OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter

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
    ) {
        let area = Rect::new(
            view.area.x + GUTTER_OFFSET,
            view.area.y,
            view.area.width - GUTTER_OFFSET,
            view.area.height.saturating_sub(1),
        ); // - 1 for statusline

        self.render_buffer(doc, view, area, surface, theme, is_focused, loader);

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

        self.render_diagnostics(doc, view, area, surface, theme, is_focused);

        let area = Rect::new(
            view.area.x,
            view.area.y + view.area.height.saturating_sub(1),
            view.area.width,
            1,
        );
        self.render_statusline(doc, view, area, surface, theme, is_focused);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_buffer(
        &self,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
        loader: &syntax::Loader,
    ) {
        let text = doc.text().slice(..);

        let last_line = view.last_line(doc);

        let range = {
            // calculate viewport byte ranges
            let start = text.line_to_byte(view.first_line);
            let end = text.line_to_byte(last_line + 1);

            start..end
        };

        // TODO: range doesn't actually restrict source, just highlight range
        let highlights: Vec<_> = match doc.syntax() {
            Some(syntax) => {
                let scopes = theme.scopes();
                syntax
                    .highlight_iter(text.slice(..), Some(range), None, |language| {
                        loader
                                .language_config_for_scope(&format!("source.{}", language))
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
                    .collect() // TODO: we collect here to avoid holding the lock, fix later
            }
            None => vec![Ok(HighlightEvent::Source {
                start: range.start,
                end: range.end,
            })],
        };
        let mut spans = Vec::new();
        let mut visual_x = 0u16;
        let mut line = 0u16;
        let tab_width = doc.tab_width();
        let tab = " ".repeat(tab_width);

        let highlights = highlights.into_iter().map(|event| match event.unwrap() {
            // convert byte offsets to char offset
            HighlightEvent::Source { start, end } => {
                let start = ensure_grapheme_boundary_next(text, text.byte_to_char(start));
                let end = ensure_grapheme_boundary_next(text, text.byte_to_char(end));
                HighlightEvent::Source { start, end }
            }
            event => event,
        });

        let selections = doc.selection(view.id);
        let primary_idx = selections.primary_index();

        let selection_scope = theme
            .find_scope_index("ui.selection")
            .expect("no selection scope found!");

        let base_cursor_scope = theme
            .find_scope_index("ui.cursor")
            .unwrap_or(selection_scope);

        let cursor_scope = match doc.mode() {
            Mode::Insert => theme.find_scope_index("ui.cursor.insert"),
            Mode::Select => theme.find_scope_index("ui.cursor.select"),
            Mode::Normal => Some(base_cursor_scope),
        }
        .unwrap_or(base_cursor_scope);

        let highlights: Box<dyn Iterator<Item = HighlightEvent>> = if is_focused {
            // TODO: primary + insert mode patching:
            // (ui.cursor.primary).patch(mode).unwrap_or(cursor)
            let primary_cursor_scope = theme
                .find_scope_index("ui.cursor.primary")
                .unwrap_or(cursor_scope);
            let primary_selection_scope = theme
                .find_scope_index("ui.selection.primary")
                .unwrap_or(selection_scope);

            // inject selections as highlight scopes
            let mut spans: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
            for (i, range) in selections.iter().enumerate() {
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

            Box::new(syntax::merge(highlights, spans))
        } else {
            Box::new(highlights)
        };

        // diagnostic injection
        let diagnostic_scope = theme.find_scope_index("diagnostic").unwrap_or(cursor_scope);
        let highlights = Box::new(syntax::merge(
            highlights,
            doc.diagnostics()
                .iter()
                .map(|diagnostic| {
                    (
                        diagnostic_scope,
                        diagnostic.range.start..diagnostic.range.end,
                    )
                })
                .collect(),
        ));

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

                    let style = spans.iter().fold(theme.get("ui.text"), |acc, span| {
                        let style = theme.get(theme.scopes()[span.0].as_str());
                        acc.patch(style)
                    });

                    for grapheme in RopeGraphemes::new(text) {
                        let out_of_bounds = visual_x < view.first_col as u16
                            || visual_x >= viewport.width + view.first_col as u16;

                        if LineEnding::from_rope_slice(&grapheme).is_some() {
                            if !out_of_bounds {
                                // we still want to render an empty cell with the style
                                surface.set_string(
                                    viewport.x + visual_x - view.first_col as u16,
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
                                    viewport.x + visual_x - view.first_col as u16,
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

        // render gutters

        let linenr: Style = theme.get("ui.linenr");
        let warning: Style = theme.get("warning");
        let error: Style = theme.get("error");
        let info: Style = theme.get("info");
        let hint: Style = theme.get("hint");

        // Whether to draw the line number for the last line of the
        // document or not.  We only draw it if it's not an empty line.
        let draw_last = text.line_to_byte(last_line) < text.len_bytes();

        for (i, line) in (view.first_line..(last_line + 1)).enumerate() {
            use helix_core::diagnostic::Severity;
            if let Some(diagnostic) = doc.diagnostics().iter().find(|d| d.line == line) {
                surface.set_stringn(
                    viewport.x - GUTTER_OFFSET,
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

            // Line numbers having selections are rendered
            // differently, further below.
            let line_number_text = if line == last_line && !draw_last {
                "    ~".into()
            } else {
                format!("{:>5}", line + 1)
            };
            surface.set_stringn(
                viewport.x + 1 - GUTTER_OFFSET,
                viewport.y + i as u16,
                line_number_text,
                5,
                linenr,
            );
        }

        // render selections and selected linenr(s)
        let linenr_select: Style = theme
            .try_get("ui.linenr.selected")
            .unwrap_or_else(|| theme.get("ui.linenr"));

        if is_focused {
            let screen = {
                let start = text.line_to_char(view.first_line);
                let end = text.line_to_char(last_line + 1) + 1; // +1 for cursor at end of text.
                Range::new(start, end)
            };

            let selection = doc.selection(view.id);

            for selection in selection.iter().filter(|range| range.overlaps(&screen)) {
                let head = view.screen_coords_at_pos(
                    doc,
                    text,
                    if selection.head > selection.anchor {
                        selection.head - 1
                    } else {
                        selection.head
                    },
                );
                if let Some(head) = head {
                    // Draw line number for selected lines.
                    let line_number = view.first_line + head.row;
                    let line_number_text = if line_number == last_line && !draw_last {
                        "    ~".into()
                    } else {
                        format!("{:>5}", line_number + 1)
                    };
                    surface.set_stringn(
                        viewport.x + 1 - GUTTER_OFFSET,
                        viewport.y + head.row as u16,
                        line_number_text,
                        5,
                        linenr_select,
                    );

                    // TODO: set cursor position for IME
                    if let Some(syntax) = doc.syntax() {
                        use helix_core::match_brackets;
                        let pos = doc
                            .selection(view.id)
                            .primary()
                            .cursor(doc.text().slice(..));
                        let pos = match_brackets::find(syntax, doc.text(), pos)
                            .and_then(|pos| view.screen_coords_at_pos(doc, text, pos));

                        if let Some(pos) = pos {
                            // ensure col is on screen
                            if (pos.col as u16) < viewport.width + view.first_col as u16
                                && pos.col >= view.first_col
                            {
                                let style = theme.try_get("ui.cursor.match").unwrap_or_else(|| {
                                    Style::default()
                                        .add_modifier(Modifier::REVERSED)
                                        .add_modifier(Modifier::DIM)
                                });

                                surface
                                    .get_mut(
                                        viewport.x + pos.col as u16,
                                        viewport.y + pos.row as u16,
                                    )
                                    .set_style(style);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn render_diagnostics(
        &self,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        _is_focused: bool,
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

        let warning: Style = theme.get("warning");
        let error: Style = theme.get("error");
        let info: Style = theme.get("info");
        let hint: Style = theme.get("hint");

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
            Rect::new(
                viewport.right() - width,
                viewport.y as u16 + 1,
                width,
                height,
            ),
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

        let style = if is_focused {
            theme.get("ui.statusline")
        } else {
            theme.get("ui.statusline.inactive")
        };
        // statusline
        surface.set_style(Rect::new(viewport.x, viewport.y, viewport.width, 1), style);
        if is_focused {
            surface.set_string(viewport.x + 1, viewport.y, mode, style);
        }
        surface.set_string(viewport.x + 5, viewport.y, progress, style);

        if let Some(path) = doc.relative_path() {
            let path = path.to_string_lossy();

            let title = format!("{}{}", path, if doc.is_modified() { "[+]" } else { "" });
            surface.set_stringn(
                viewport.x + 8,
                viewport.y,
                title,
                viewport.width.saturating_sub(6) as usize,
                style,
            );
        }

        //-------------------------------
        // Right side of the status line.
        //-------------------------------

        // Compute the individual info strings.
        let diag_count = format!("{}", doc.diagnostics().len());
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
        let position_info = {
            let pos = coords_at_pos(
                doc.text().slice(..),
                doc.selection(view.id)
                    .primary()
                    .cursor(doc.text().slice(..)),
            );
            format!("{}:{}", pos.row + 1, pos.col + 1) // convert to 1-indexing
        };

        // Render them to the status line together.
        let right_side_text = format!(
            "{}    {} ",
            &diag_count[..diag_count.len().min(4)],
            // indent_info,
            position_info
        );
        let text_len = right_side_text.len() as u16;
        surface.set_string(
            viewport.x + viewport.width.saturating_sub(text_len),
            viewport.y,
            right_side_text,
            style,
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
        self.autoinfo = None;
        match self.keymaps.get_mut(&mode).unwrap().get(event) {
            KeymapResult::Matched(command) => command.execute(cxt),
            KeymapResult::Pending(node) => self.autoinfo = Some(node.into()),
            k @ KeymapResult::NotFound | k @ KeymapResult::Cancelled(_) => return Some(k),
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
                                    self.keymaps.get_mut(&Mode::Insert).unwrap().get(ev)
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
                cxt.selected_register = cxt.editor.selected_register.take();

                self.handle_keymap_event(mode, cxt, event);
                if self.keymaps.pending().is_empty() {
                    cxt.editor.count = None
                }
            }
        }
    }

    pub fn set_completion(
        &mut self,
        items: Vec<helix_lsp::lsp::CompletionItem>,
        offset_encoding: helix_lsp::OffsetEncoding,
        trigger_offset: usize,
        size: Rect,
    ) {
        let mut completion = Completion::new(items, offset_encoding, trigger_offset);
        // TODO : propagate required size on resize to completion too
        completion.required_size((size.width, size.height));
        self.completion = Some(completion);
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
            selected_register: helix_view::RegisterSelection::default(),
            editor: &mut cx.editor,
            count: None,
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
                                        self.completion = None;
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
                                        self.completion = None;
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

                        self.last_insert.0 = match self.keymaps.get_mut(&mode).unwrap().get(key) {
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

            Event::Mouse(event) => self.handle_mouse_event(event, &mut cxt),
        }
    }

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // clear with background color
        surface.set_style(area, cx.editor.theme.get("ui.background"));

        // if the terminal size suddenly changed, we need to trigger a resize
        cx.editor
            .resize(Rect::new(area.x, area.y, area.width, area.height - 1)); // - 1 to account for commandline

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
            );
        }

        if let Some(ref info) = self.autoinfo {
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

        if let Some(completion) = &self.completion {
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
