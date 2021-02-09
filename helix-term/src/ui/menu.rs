use crate::compositor::{Component, Compositor, Context, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::buffer::Buffer as Surface;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

use std::borrow::Cow;

use helix_core::Position;
use helix_view::Editor;

// TODO: factor out a popup component that we can reuse for displaying docs on autocomplete,
// diagnostics popups, etc.

pub struct Menu<T> {
    options: Vec<T>,

    cursor: usize,

    position: Position,

    format_fn: Box<dyn Fn(&T) -> Cow<str>>,
    callback_fn: Box<dyn Fn(&mut Editor, Option<&T>, MenuEvent)>,
}

impl<T> Menu<T> {
    // TODO: it's like a slimmed down picker, share code? (picker = menu + prompt with different
    // rendering)
    pub fn new(
        options: Vec<T>,
        format_fn: impl Fn(&T) -> Cow<str> + 'static,
        callback_fn: impl Fn(&mut Editor, Option<&T>, MenuEvent) + 'static,
    ) -> Self {
        Self {
            options,
            cursor: 0,
            position: Position::default(),
            format_fn: Box::new(format_fn),
            callback_fn: Box::new(callback_fn),
        }
    }

    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        // TODO: len - 1
        if self.cursor < self.options.len() {
            self.cursor += 1;
        }
    }

    pub fn selection(&self) -> Option<&T> {
        self.options.get(self.cursor)
    }
}

use super::PromptEvent as MenuEvent;

impl<T> Component for Menu<T> {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let event = match event {
            Event::Key(event) => event,
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(
            |compositor: &mut Compositor, editor: &mut Editor| {
                // remove the layer
                compositor.pop();
            },
        )));

        match event {
            // esc or ctrl-c aborts the completion and closes the menu
            KeyEvent {
                code: KeyCode::Esc, ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                (self.callback_fn)(cx.editor, self.selection(), MenuEvent::Abort);
                return close_fn;
            }
            // arrow up/ctrl-p/shift-tab prev completion choice (including updating the doc)
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::SHIFT,
            }
            | KeyEvent {
                code: KeyCode::Up, ..
            }
            | KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.move_up();
                (self.callback_fn)(cx.editor, self.selection(), MenuEvent::Update);
                return EventResult::Consumed(None);
            }
            // arrow down/ctrl-n/tab advances completion choice (including updating the doc)
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
            }
            | KeyEvent {
                code: KeyCode::Down,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.move_down();
                (self.callback_fn)(cx.editor, self.selection(), MenuEvent::Update);
                return EventResult::Consumed(None);
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                (self.callback_fn)(cx.editor, self.selection(), MenuEvent::Validate);
                return close_fn;
            }
            // KeyEvent {
            //     code: KeyCode::Char(c),
            //     modifiers: KeyModifiers::NONE,
            // } => {
            //     self.insert_char(c);
            //     (self.callback_fn)(cx.editor, &self.line, MenuEvent::Update);
            // }

            // / -> edit_filter?
            //
            // enter confirms the match and closes the menu
            // typing filters the menu
            // if we run out of options the menu closes itself
            _ => (),
        }
        // for some events, we want to process them but send ignore, specifically all input except
        // tab/enter/ctrl-k or whatever will confirm the selection/ ctrl-n/ctrl-p for scroll.
        // EventResult::Consumed(None)
        EventResult::Ignored
    }
    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // render a box at x, y. Width equal to max width of item.
        // initially limit to n items, add support for scrolling
        //
        const MAX: usize = 5;
        let rows = std::cmp::min(self.options.len(), MAX) as u16;
        let area = Rect::new(self.position.col as u16, self.position.row as u16, 30, rows);

        // clear area
        let background = cx.editor.theme.get("ui.popup");
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = surface.get_mut(x, y);
                cell.reset();
                // cell.symbol.clear();
                cell.set_style(background);
            }
        }

        // -- Render the contents:

        let style = Style::default().fg(Color::Rgb(164, 160, 232)); // lavender
        let selected = Style::default().fg(Color::Rgb(255, 255, 255));

        for (i, option) in self.options.iter().take(rows as usize).enumerate() {
            // TODO: set bg for the whole row if selected
            surface.set_stringn(
                area.x,
                area.y + i as u16,
                (self.format_fn)(option),
                area.width as usize - 1,
                if i == self.cursor { selected } else { style },
            );
        }
    }
}
