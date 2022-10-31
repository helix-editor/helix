use std::cell::Cell;

use helix_core::Position;
use helix_view::{
    graphics::{CursorKind, Rect},
    input,
    terminal::ChordState,
    theme::Modifier,
    Editor,
};
use tui::buffer::Buffer;

use crate::compositor::{Context, EventResult};

pub struct Terminal {
    size: Cell<(u16, u16)>,
    prev_pressed: Cell<bool>,
}

impl Terminal {
    pub fn new() -> Self {
        Self {
            size: Cell::new((24, 80)),
            prev_pressed: Cell::new(false),
        }
    }

    pub fn render(&self, area: Rect, surface: &mut Buffer, cx: &mut Context) {
        let tab_height = 1;

        if self.size.get() != (area.height - tab_height, area.width)
            && area.height > 0
            && area.width > 0
        {
            self.size.set((area.height - tab_height, area.width));

            cx.editor
                .terminals
                .handle_input_event(&input::Event::Resize(self.size.get().1, self.size.get().0));
        }

        let theme = &cx.editor.theme;
        let chord_state = cx.editor.terminals.chord_state();
        let active_tab_style = theme.get("ui.selection.active");
        let inactive_tab_style = theme.get("ui.text");

        let terminals = cx.editor.terminals.terminals();

        if let Some((active_terminal_id, term)) = cx.editor.terminals.get_active() {
            let content = term.renderable_content();

            surface.clear(area);

            for cell in content.display_iter {
                if let Some(c) = surface.get_mut(
                    area.left() + cell.point.column.0 as u16,
                    area.top()
                        + (cell
                            .point
                            .line
                            .0
                            .saturating_add(content.display_offset as i32))
                            as u16,
                ) {
                    let style = helix_view::theme::Style::reset()
                        .bg(helix_view::terminal::color_from_ansi(cell.bg))
                        .fg(helix_view::terminal::color_from_ansi(cell.fg))
                        .add_modifier(Modifier::from_bits(cell.flags.bits()).unwrap());

                    let style = if let Some(col) = cell.underline_color() {
                        style.underline_color(helix_view::terminal::color_from_ansi(col))
                    } else {
                        style
                    };

                    c.reset();
                    c.set_char(cell.c);
                    c.set_style(style);
                }
            }

            let tab_area = Rect::new(
                area.x,
                area.y + area.height - tab_height,
                area.width,
                tab_height,
            );

            let mut x_offset = tab_area.x;

            let mode_text = format!(" {} ", chord_state.to_string());
            let tab_width = mode_text.len() as u16;
            if x_offset + tab_width <= tab_area.right() {
                surface.set_string(x_offset, tab_area.y, &mode_text, active_tab_style);
                x_offset += tab_width;
            }

            for id in terminals {
                let is_active = *id == active_terminal_id;
                let style = if is_active {
                    active_tab_style
                } else {
                    inactive_tab_style
                };
                let tab_text = format!(" {} ", id);

                let tab_width = tab_text.len() as u16;
                if x_offset + tab_width > tab_area.right() {
                    break; // Avoid overflowing the tab area
                }

                surface.set_string(x_offset, tab_area.y, &tab_text, style);
                x_offset += tab_width;
            }
        }
    }

    pub(crate) fn get_cursor(
        &self,
        mut area: Rect,
        editor: &Editor,
    ) -> Option<(Option<Position>, CursorKind)> {
        let terminals = editor.terminals.terminals();
        if !terminals.is_empty() {
            let tab_height = 1;
            area.height -= tab_height;
        }

        editor.terminals.get_active().map(|(_, term)| {
            if term.grid().display_offset() != 0 {
                (None, CursorKind::Hidden)
            } else {
                let pt = term.grid().cursor.point;
                (
                    Some(Position {
                        row: area.y as usize + pt.line.0 as usize,
                        col: area.x as usize + pt.column.0 as usize,
                    }),
                    helix_view::terminal::cursor_kind_from_ansi(term.cursor_style().shape),
                )
            }
        })
    }

    pub(crate) fn handle_event(
        &self,
        event: &input::Event,
        context: &mut Context,
    ) -> Option<EventResult> {
        if context.editor.terminals.visible {
            match event {
                input::Event::Key(input::KeyEvent {
                    code: input::KeyCode::Char('\\'),
                    ..
                }) => {
                    if self.prev_pressed.get() {
                        context.editor.terminals.visible = false;
                        Some(EventResult::Consumed(None))
                    } else {
                        self.prev_pressed.set(true);

                        if context.editor.terminals.handle_input_event(event) {
                            Some(EventResult::Consumed(None))
                        } else {
                            Some(EventResult::Ignored(None))
                        }
                    }
                }

                input::Event::Resize(_, _) => Some(EventResult::Ignored(None)),

                event => {
                    if let input::Event::Key(..) = &event {
                        self.prev_pressed.set(false);
                    }

                    if context.editor.terminals.handle_input_event(event) {
                        Some(EventResult::Consumed(None))
                    } else {
                        Some(EventResult::Ignored(None))
                    }
                }
            }
        } else {
            None
        }
    }
}
