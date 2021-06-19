use crate::{
    commands,
    compositor::{Component, Compositor, Context, EventResult},
    key,
    keymap::{self, Keymaps},
    ui::Completion,
};

use helix_core::{
    coords_at_pos,
    syntax::{self, HighlightEvent},
    Position, Range,
};
use helix_view::{document::Mode, Document, Editor, Theme, View};
use std::borrow::Cow;

use crossterm::{
    cursor,
    event::{read, Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
};
use tui::{
    backend::CrosstermBackend,
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Modifier, Style},
    terminal::CursorKind,
};

pub struct EditorView {
    keymaps: Keymaps,
    on_next_key: Option<Box<dyn FnOnce(&mut commands::Context, KeyEvent)>>,
    last_insert: (commands::Command, Vec<KeyEvent>),
    completion: Option<Completion>,
}

const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter

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
        }
    }

    pub fn render_view(
        &self,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
    ) {
        let area = Rect::new(
            view.area.x + OFFSET,
            view.area.y,
            view.area.width - OFFSET,
            view.area.height.saturating_sub(1),
        ); // - 1 for statusline

        self.render_buffer(doc, view, area, surface, theme, is_focused);

        // if we're not at the edge of the screen, draw a right border
        if viewport.right() != view.area.right() {
            let x = area.right();
            let border_style = theme.get("ui.window");
            for y in area.top()..area.bottom() {
                surface
                    .get_mut(x, y)
                    // .set_symbol(tui::symbols::line::VERTICAL)
                    .set_symbol(" ")
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

    pub fn render_buffer(
        &self,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
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
        // TODO: cache highlight results
        // TODO: only recalculate when state.doc is actually modified
        let highlights: Vec<_> = match doc.syntax() {
            Some(syntax) => {
                syntax
                    .highlight_iter(text.slice(..), Some(range), None, |_| None)
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

        'outer: for event in highlights {
            match event.unwrap() {
                HighlightEvent::HighlightStart(span) => {
                    spans.push(span);
                }
                HighlightEvent::HighlightEnd => {
                    spans.pop();
                }
                HighlightEvent::Source { start, end } => {
                    // TODO: filter out spans out of viewport for now..

                    // TODO: do these before iterating
                    let start = text.byte_to_char(start);
                    let end = text.byte_to_char(end);

                    let text = text.slice(start..end);

                    use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                    // TODO: scope matching: biggest union match? [string] & [html, string], [string, html] & [ string, html]
                    // can do this by sorting our theme matches based on array len (longest first) then stopping at the
                    // first rule that matches (rule.all(|scope| scopes.contains(scope)))
                    // log::info!(
                    //     "scopes: {:?}",
                    //     spans
                    //         .iter()
                    //         .map(|span| theme.scopes()[span.0].as_str())
                    //         .collect::<Vec<_>>()
                    // );
                    let style = match spans.first() {
                        Some(span) => theme.get(theme.scopes()[span.0].as_str()),
                        None => theme.get("ui.text"),
                    };

                    // TODO: we could render the text to a surface, then cache that, that
                    // way if only the selection/cursor changes we can copy from cache
                    // and paint the new cursor.
                    // We could keep a single resizable surface on the View for that.

                    let mut char_index = start;

                    // iterate over range char by char
                    for grapheme in RopeGraphemes::new(text) {
                        if grapheme == "\n" {
                            visual_x = 0;
                            line += 1;

                            // TODO: with proper iter this shouldn't be necessary
                            if line >= viewport.height {
                                break 'outer;
                            }
                        } else if grapheme == "\t" {
                            visual_x = visual_x.saturating_add(tab_width as u16);
                        } else {
                            let out_of_bounds = visual_x < view.first_col as u16
                                || visual_x >= viewport.width + view.first_col as u16;

                            // Cow will prevent allocations if span contained in a single slice
                            // which should really be the majority case
                            let grapheme = Cow::from(grapheme);
                            let width = grapheme_width(&grapheme) as u16;

                            if out_of_bounds {
                                // if we're offscreen just keep going until we hit a new line
                                visual_x = visual_x.saturating_add(width);
                                continue;
                            }

                            // ugh,interleave highlight spans with diagnostic spans
                            let is_diagnostic = doc.diagnostics().iter().any(|diagnostic| {
                                diagnostic.range.start <= char_index
                                    && diagnostic.range.end > char_index
                            });

                            let style = if is_diagnostic {
                                style.add_modifier(Modifier::UNDERLINED)
                            } else {
                                style
                            };

                            surface.set_string(
                                viewport.x + visual_x - view.first_col as u16,
                                viewport.y + line,
                                grapheme,
                                style,
                            );

                            visual_x += width;
                        }

                        char_index += 1;
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

        for (i, line) in (view.first_line..last_line).enumerate() {
            use helix_core::diagnostic::Severity;
            if let Some(diagnostic) = doc.diagnostics().iter().find(|d| d.line == line) {
                surface.set_stringn(
                    viewport.x - OFFSET,
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

            // line numbers having selections are rendered differently
            surface.set_stringn(
                viewport.x + 1 - OFFSET,
                viewport.y + i as u16,
                format!("{:>5}", line + 1),
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
                let end = text.line_to_char(last_line + 1);
                Range::new(start, end)
            };
            let cursor_style = Style::default()
                // .bg(Color::Rgb(255, 255, 255))
                .add_modifier(Modifier::REVERSED);

            let selection_style = theme.get("ui.selection");

            for selection in doc
                .selection(view.id)
                .iter()
                .filter(|range| range.overlaps(&screen))
            {
                // TODO: render also if only one of the ranges is in viewport
                let mut start = view.screen_coords_at_pos(doc, text, selection.anchor);
                let mut end = view.screen_coords_at_pos(doc, text, selection.head);

                let head = end;

                if selection.head < selection.anchor {
                    std::mem::swap(&mut start, &mut end);
                }
                let start = start.unwrap_or_else(|| Position::new(0, 0));
                let end = end.unwrap_or_else(|| {
                    Position::new(viewport.height as usize, viewport.width as usize)
                });

                if start.row == end.row {
                    surface.set_style(
                        Rect::new(
                            viewport.x + start.col as u16,
                            viewport.y + start.row as u16,
                            // .min is important, because set_style does a
                            // for i in area.left()..area.right() and
                            // area.right = x + width !!! which shouldn't be > then surface.area.right()
                            // This is checked by a debug_assert! in Buffer::index_of
                            ((end.col - start.col) as u16 + 1).min(
                                surface
                                    .area
                                    .width
                                    .saturating_sub(viewport.x + start.col as u16),
                            ),
                            1,
                        ),
                        selection_style,
                    );
                } else {
                    surface.set_style(
                        Rect::new(
                            viewport.x + start.col as u16,
                            viewport.y + start.row as u16,
                            // text.line(view.first_line).len_chars() as u16 - start.col as u16,
                            viewport.width.saturating_sub(start.col as u16),
                            1,
                        ),
                        selection_style,
                    );
                    for i in start.row + 1..end.row {
                        surface.set_style(
                            Rect::new(
                                viewport.x,
                                viewport.y + i as u16,
                                // text.line(view.first_line + i).len_chars() as u16,
                                viewport.width,
                                1,
                            ),
                            selection_style,
                        );
                    }
                    surface.set_style(
                        Rect::new(
                            viewport.x,
                            viewport.y + end.row as u16,
                            (end.col as u16).min(viewport.width),
                            1,
                        ),
                        selection_style,
                    );
                }

                // cursor
                if let Some(head) = head {
                    surface.set_style(
                        Rect::new(
                            viewport.x + head.col as u16,
                            viewport.y + head.row as u16,
                            1,
                            1,
                        ),
                        cursor_style,
                    );
                    surface.set_stringn(
                        viewport.x + 1 - OFFSET,
                        viewport.y + head.row as u16,
                        format!("{:>5}", view.first_line + head.row + 1),
                        5,
                        linenr_select,
                    );
                    // TODO: set cursor position for IME
                    if let Some(syntax) = doc.syntax() {
                        use helix_core::match_brackets;
                        let pos = doc.selection(view.id).cursor();
                        let pos = match_brackets::find(syntax, doc.text(), pos);
                        if let Some(pos) = pos {
                            let pos = view.screen_coords_at_pos(doc, text, pos);
                            if let Some(pos) = pos {
                                if (pos.col as u16) < viewport.width + view.first_col as u16
                                    && pos.col >= view.first_col
                                {
                                    let style = Style::default()
                                        .add_modifier(Modifier::REVERSED)
                                        .add_modifier(Modifier::DIM);

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
    }

    pub fn render_diagnostics(
        &self,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
    ) {
        use helix_core::diagnostic::Severity;
        use tui::{
            layout::Alignment,
            text::Text,
            widgets::{Paragraph, Widget},
        };

        let cursor = doc.selection(view.id).cursor();
        let line = doc.text().char_to_line(cursor);

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

        if let Some(path) = doc.relative_path() {
            let path = path.to_string_lossy();

            let title = format!("{}{}", path, if doc.is_modified() { "[+]" } else { "" });
            surface.set_stringn(
                viewport.x + 6,
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
            let pos = coords_at_pos(doc.text().slice(..), doc.selection(view.id).cursor());
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

    fn insert_mode(&self, cx: &mut commands::Context, event: KeyEvent) {
        if let Some(command) = self.keymaps[&Mode::Insert].get(&event) {
            command.execute(cx);
        } else if let KeyEvent {
            code: KeyCode::Char(ch),
            ..
        } = event
        {
            commands::insert::insert_char(cx, ch);
        }
    }

    fn command_mode(&self, mode: Mode, cxt: &mut commands::Context, event: KeyEvent) {
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
                for key in &self.last_insert.1 {
                    self.insert_mode(cxt, *key)
                }
            }
            _ => {
                // set the count
                cxt.count = cxt.editor.count.take();
                // TODO: edge case: 0j -> reset to 1
                // if this fails, count was Some(0)
                // debug_assert!(cxt.count != 0);

                // set the register
                cxt.selected_register = cxt.editor.selected_register.take();

                if let Some(command) = self.keymaps[&mode].get(&event) {
                    command.execute(cxt);
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

impl Component for EditorView {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        match event {
            Event::Resize(width, height) => {
                // HAXX: offset the render area height by 1 to account for prompt/commandline
                cx.editor.resize(Rect::new(0, 0, width, height - 1));
                EventResult::Consumed(None)
            }
            Event::Key(mut key) => {
                canonicalize_key(&mut key);
                // clear status
                cx.editor.status_msg = None;

                let (view, doc) = current!(cx.editor);
                let mode = doc.mode();

                let mut cxt = commands::Context {
                    selected_register: helix_view::RegisterSelection::default(),
                    editor: &mut cx.editor,
                    count: None,
                    callback: None,
                    on_next_key_callback: None,
                    callbacks: cx.callbacks,
                };

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
                                    callbacks: cxt.callbacks,
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
                if cx.editor.should_close() {
                    return EventResult::Ignored;
                }

                let (view, doc) = current!(cx.editor);
                view.ensure_cursor_in_view(doc);

                // mode transitions
                match (mode, doc.mode()) {
                    (Mode::Normal, Mode::Insert) => {
                        // HAXX: if we just entered insert mode from normal, clear key buf
                        // and record the command that got us into this mode.

                        // how we entered insert mode is important, and we should track that so
                        // we can repeat the side effect.

                        self.last_insert.0 = self.keymaps[&mode][&key];
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
            Event::Mouse(_) => EventResult::Ignored,
        }
    }

    fn render(&self, mut area: Rect, surface: &mut Surface, cx: &mut Context) {
        // clear with background color
        surface.set_style(area, cx.editor.theme.get("ui.background"));

        // if the terminal size suddenly changed, we need to trigger a resize
        cx.editor
            .resize(Rect::new(area.x, area.y, area.width, area.height - 1)); // - 1 to account for commandline

        for (view, is_focused) in cx.editor.tree.views() {
            let doc = cx.editor.document(view.doc).unwrap();
            self.render_view(doc, view, area, surface, &cx.editor.theme, is_focused);
        }

        // render status msg
        if let Some((status_msg, severity)) = &cx.editor.status_msg {
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

        if let Some(completion) = &self.completion {
            completion.render(area, surface, cx)
            // render completion here
        }
    }

    fn cursor(&self, area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
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
