use crate::commands;
use crate::compositor::{Component, Compositor, Context, EventResult};
use crate::keymap::{self, Keymaps};
use crate::ui::text_color;

use helix_core::{indent::TAB_WIDTH, syntax::HighlightEvent, Position, Range, State};
use helix_view::{document::Mode, Document, Editor, Theme, View};
use std::borrow::Cow;

use crossterm::{
    cursor,
    event::{read, Event, EventStream, KeyCode, KeyEvent},
};
use tui::{
    backend::CrosstermBackend,
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Modifier, Style},
};

pub struct EditorView {
    keymap: Keymaps,
}

const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter

impl EditorView {
    pub fn new() -> Self {
        Self {
            keymap: keymap::default(),
        }
    }
    pub fn render_view(
        &self,
        view: &mut View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
    ) {
        let area = Rect::new(
            viewport.x + OFFSET,
            viewport.y,
            viewport.width - OFFSET,
            viewport.height - 1,
        ); // - 1 for statusline
        self.render_buffer(view, area, surface, theme, is_focused);

        // clear with background color
        // TODO: this seems to prevent setting style later
        // surface.set_style(viewport, theme.get("ui.background"));

        let area = Rect::new(
            viewport.x,
            viewport.y + viewport.height - 1,
            viewport.width,
            1,
        );
        self.render_statusline(&view.doc, area, surface, theme, is_focused);
    }

    // TODO: ideally not &mut View but highlights require it because of cursor cache
    pub fn render_buffer(
        &self,
        view: &mut View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
    ) {
        // TODO: inefficient, should feed chunks.iter() to tree_sitter.parse_with(|offset, pos|)
        let text = view.doc.text();
        let source_code = text.to_string();

        let last_line = view.last_line();

        let range = {
            // calculate viewport byte ranges
            let start = text.line_to_byte(view.first_line);
            let end = text.line_to_byte(last_line) + text.line(last_line).len_bytes();

            start..end
        };

        // TODO: range doesn't actually restrict source, just highlight range
        // TODO: cache highlight results
        // TODO: only recalculate when state.doc is actually modified
        let highlights: Vec<_> = match view.doc.syntax.as_mut() {
            Some(syntax) => {
                syntax
                    .highlight_iter(source_code.as_bytes(), Some(range), None, |_| None)
                    .unwrap()
                    .collect() // TODO: we collect here to avoid double borrow, fix later
            }
            None => vec![Ok(HighlightEvent::Source {
                start: range.start,
                end: range.end,
            })],
        };
        let mut spans = Vec::new();
        let mut visual_x = 0;
        let mut line = 0u16;
        let visible_selections: Vec<Range> = if is_focused {
            view.doc
                .state
                .selection()
                .ranges()
                .iter()
                // TODO: limit selection to one in viewport
                // .filter(|range| !range.is_empty()) // && range.overlaps(&Range::new(start, end + 1))
                .copied()
                .collect()
        } else {
            // don't render selections on unfocused windows
            Vec::new()
        };

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
                    let text = view.doc.text();
                    let start = text.byte_to_char(start);
                    let end = text.byte_to_char(end);

                    let text = text.slice(start..end);

                    use helix_core::graphemes::{grapheme_width, RopeGraphemes};

                    let style = match spans.first() {
                        Some(span) => theme.get(theme.scopes()[span.0].as_str()),
                        None => Style::default().fg(Color::Rgb(164, 160, 232)), // lavender
                    };

                    // TODO: we could render the text to a surface, then cache that, that
                    // way if only the selection/cursor changes we can copy from cache
                    // and paint the new cursor.
                    // We could keep a single resizable surface on the View for that.

                    let mut char_index = start;

                    // iterate over range char by char
                    for grapheme in RopeGraphemes::new(&text) {
                        // TODO: track current char_index

                        if grapheme == "\n" {
                            visual_x = 0;
                            line += 1;

                            // TODO: with proper iter this shouldn't be necessary
                            if line >= viewport.height {
                                break 'outer;
                            }
                        } else if grapheme == "\t" {
                            visual_x += (TAB_WIDTH as u16);
                        } else {
                            if visual_x >= viewport.width {
                                // if we're offscreen just keep going until we hit a new line
                                // TODO: will need tweaking when we also take into account
                                // horizontal scrolling
                                continue;
                            }

                            // Cow will prevent allocations if span contained in a single slice
                            // which should really be the majority case
                            let grapheme = Cow::from(grapheme);
                            let width = grapheme_width(&grapheme) as u16;

                            // TODO: this should really happen as an after pass
                            let style = if visible_selections
                                .iter()
                                .any(|range| range.contains(char_index))
                            {
                                // cedar
                                style.clone().bg(Color::Rgb(128, 47, 0))
                            } else {
                                style
                            };

                            let style = if visible_selections
                                .iter()
                                .any(|range| range.head == char_index)
                            {
                                style.clone().bg(Color::Rgb(255, 255, 255))
                            } else {
                                style
                            };

                            // ugh, improve with a traverse method
                            // or interleave highlight spans with selection and diagnostic spans
                            let is_diagnostic = view.doc.diagnostics.iter().any(|diagnostic| {
                                diagnostic.range.0 <= char_index && diagnostic.range.1 > char_index
                            });

                            let style = if is_diagnostic {
                                style.clone().add_modifier(Modifier::UNDERLINED)
                            } else {
                                style
                            };

                            // TODO: paint cursor heads except primary

                            surface.set_string(
                                viewport.x + visual_x,
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

        let style: Style = theme.get("ui.linenr");
        let warning: Style = theme.get("warning");
        let last_line = view.last_line();
        for (i, line) in (view.first_line..last_line).enumerate() {
            if view.doc.diagnostics.iter().any(|d| d.line == line) {
                surface.set_stringn(
                    viewport.x + 0 - OFFSET,
                    viewport.y + i as u16,
                    "●",
                    1,
                    warning,
                );
            }

            surface.set_stringn(
                viewport.x + 1 - OFFSET,
                viewport.y + i as u16,
                format!("{:>5}", line + 1),
                5,
                style,
            );
        }
    }

    pub fn render_statusline(
        &self,
        doc: &Document,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
        is_focused: bool,
    ) {
        let mode = match doc.mode() {
            Mode::Insert => "INS",
            Mode::Normal => "NOR",
            Mode::Goto => "GOTO",
        };
        // TODO: share text_color styles inside theme
        let text_color = if is_focused {
            Style::default().fg(Color::Rgb(219, 191, 239)) // lilac
        } else {
            Style::default().fg(Color::Rgb(164, 160, 232)) // lavender
        };
        // statusline
        surface.set_style(
            Rect::new(viewport.x, viewport.y, viewport.width, 1),
            theme.get("ui.statusline"),
        );
        if is_focused {
            surface.set_string(viewport.x + 1, viewport.y, mode, text_color);
        }

        if let Some(path) = doc.relative_path() {
            let path = path.to_string_lossy();
            surface.set_stringn(
                viewport.x + 6,
                viewport.y,
                path,
                viewport.width.saturating_sub(6) as usize,
                text_color,
            );
            // TODO: append [+] if modified
        }

        surface.set_string(
            viewport.x + viewport.width.saturating_sub(10),
            viewport.y,
            format!("{}", doc.diagnostics.len()),
            text_color,
        );
    }
}

impl Component for EditorView {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        match event {
            Event::Resize(width, height) => {
                // HAXX: offset the render area height by 1 to account for prompt/commandline
                cx.editor.tree.resize(Rect::new(0, 0, width, height - 1));
                // TODO: restore view.ensure_cursor_in_view();
                EventResult::Consumed(None)
            }
            Event::Key(event) => {
                let view = cx.editor.view_mut();

                let keys = vec![event];
                // TODO: sequences (`gg`)
                let mode = view.doc.mode();
                // TODO: handle count other than 1
                let mut cxt = commands::Context {
                    executor: cx.executor,
                    editor: &mut cx.editor,
                    count: 1,
                    callback: None,
                };

                match mode {
                    Mode::Insert => {
                        if let Some(command) = self.keymap[&Mode::Insert].get(&keys) {
                            command(&mut cxt);
                        } else if let KeyEvent {
                            code: KeyCode::Char(c),
                            ..
                        } = event
                        {
                            commands::insert::insert_char(&mut cxt, c);
                        }
                    }
                    mode => {
                        if let Some(command) = self.keymap[&mode].get(&keys) {
                            command(&mut cxt);

                            // TODO: simplistic ensure cursor in view for now
                        }
                    }
                }

                // appease borrowck
                let callback = cxt.callback.take();
                drop(cxt);
                cx.editor.view_mut().ensure_cursor_in_view();

                EventResult::Consumed(callback)
            }
            Event::Mouse(_) => EventResult::Ignored,
        }
    }

    fn render(&self, mut area: Rect, surface: &mut Surface, cx: &mut Context) {
        // SAFETY: we cheat around the view_mut() borrow because it doesn't allow us to also borrow
        // theme. Theme is immutable mutating view won't disrupt theme_ref.
        let theme_ref = unsafe { &*(&cx.editor.theme as *const Theme) };

        for (view, is_focused) in cx.editor.tree.views() {
            // TODO: use parent area
            self.render_view(view, view.area, surface, theme_ref, is_focused);
        }

        // TODO: drop unwrap
    }

    fn cursor_position(&self, area: Rect, editor: &Editor) -> Option<Position> {
        // match view.doc.mode() {
        //     Mode::Insert => write!(stdout, "\x1B[6 q"),
        //     mode => write!(stdout, "\x1B[2 q"),
        // };
        let view = editor.view();
        let cursor = view.doc.state.selection().cursor();

        let mut pos = view
            .screen_coords_at_pos(&view.doc.text().slice(..), cursor)
            .expect("Cursor is out of bounds.");
        pos.col += view.area.x as usize + area.x as usize + OFFSET as usize;
        pos.row += view.area.y as usize + area.y as usize;
        Some(pos)
    }
}
