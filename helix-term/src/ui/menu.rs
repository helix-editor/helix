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

    cursor: Option<usize>,

    format_fn: Box<dyn Fn(&T) -> Cow<str>>,
    callback_fn: Box<dyn Fn(&mut Editor, Option<&T>, MenuEvent)>,

    scroll: usize,
    size: (u16, u16),
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
            cursor: None,
            format_fn: Box::new(format_fn),
            callback_fn: Box::new(callback_fn),
            scroll: 0,
            size: (0, 0),
        }
    }

    pub fn move_up(&mut self) {
        // TODO: wrap around to end
        let pos = self.cursor.map(|i| i.saturating_sub(1)).unwrap_or(0) % self.options.len();
        self.cursor = Some(pos);
        self.adjust_scroll();
    }

    pub fn move_down(&mut self) {
        let pos = self.cursor.map(|i| i + 1).unwrap_or(0) % self.options.len();
        self.cursor = Some(pos);
        self.adjust_scroll();
    }

    fn adjust_scroll(&mut self) {
        let win_height = self.size.1 as usize;
        if let Some(cursor) = self.cursor {
            let mut scroll = self.scroll;
            if cursor > (win_height + scroll).saturating_sub(1) {
                // scroll down
                scroll += cursor - (win_height + scroll).saturating_sub(1)
            } else if cursor < scroll {
                // scroll up
                scroll = cursor
            }
            self.scroll = scroll;
        }
    }

    pub fn selection(&self) -> Option<&T> {
        self.cursor.and_then(|cursor| self.options.get(cursor))
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

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let width = std::cmp::min(30, viewport.0);

        const MAX: usize = 5;
        let height = std::cmp::min(self.options.len(), MAX);
        let height = std::cmp::min(height, viewport.1 as usize);

        self.size = (width as u16, height as u16);

        // adjust scroll offsets if size changed
        self.adjust_scroll();

        Some(self.size)
    }

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let style = Style::default().fg(Color::Rgb(164, 160, 232)); // lavender
        let selected = Style::default().fg(Color::Rgb(255, 255, 255));

        let scroll = self.scroll;
        let len = self.options.len();

        let win_height = area.height as usize;

        fn div_ceil(a: usize, b: usize) -> usize {
            (a + b - 1) / a
        }

        let scroll_height = std::cmp::min(div_ceil(win_height.pow(2), len), win_height as usize);

        let scroll_line = (win_height - scroll_height) * scroll
            / std::cmp::max(1, len.saturating_sub(win_height));

        for (i, option) in self.options[scroll..(scroll + win_height).min(len)]
            .iter()
            .enumerate()
        {
            let line = Some(i + scroll);
            // TODO: set bg for the whole row if selected
            surface.set_stringn(
                area.x,
                area.y + i as u16,
                (self.format_fn)(option),
                area.width as usize - 1,
                if line == self.cursor { selected } else { style },
            );

            let is_marked = i >= scroll_line && i < scroll_line + scroll_height;

            if is_marked {
                let cell = surface.get_mut(area.x + area.width - 2, area.y + i as u16);
                cell.set_symbol("▐ ");
                cell.set_style(selected);
                // cell.set_style(if is_marked { selected } else { style });
            }
        }
    }
}
