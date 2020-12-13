use crate::compositor::{Component, Context, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use helix_core::Position;
use helix_view::Editor;
use helix_view::Theme;
use std::string::String;

pub struct Prompt {
    pub prompt: String,
    pub line: String,
    pub cursor: usize,
    pub completion: Vec<String>,
    pub should_close: bool,
    pub completion_selection_index: Option<usize>,
    completion_fn: Box<dyn FnMut(&str) -> Vec<String>>,
    callback_fn: Box<dyn FnMut(&mut Editor, &str)>,
}

impl Prompt {
    pub fn new(
        prompt: String,
        mut completion_fn: impl FnMut(&str) -> Vec<String> + 'static,
        callback_fn: impl FnMut(&mut Editor, &str) + 'static,
    ) -> Prompt {
        Prompt {
            prompt,
            line: String::new(),
            cursor: 0,
            completion: completion_fn(""),
            should_close: false,
            completion_selection_index: None,
            completion_fn: Box::new(completion_fn),
            callback_fn: Box::new(callback_fn),
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.line.insert(self.cursor, c);
        self.cursor += 1;
        self.completion = (self.completion_fn)(&self.line);
        self.exit_selection();
    }

    pub fn move_char_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_char_right(&mut self) {
        if self.cursor < self.line.len() {
            self.cursor += 1;
        }
    }

    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.line.len();
    }

    pub fn delete_char_backwards(&mut self) {
        if self.cursor > 0 {
            self.line.remove(self.cursor - 1);
            self.cursor -= 1;
            self.completion = (self.completion_fn)(&self.line);
        }
        self.exit_selection();
    }

    pub fn change_completion_selection(&mut self) {
        if self.completion.is_empty() {
            return;
        }
        let index =
            self.completion_selection_index.map(|i| i + 1).unwrap_or(0) % self.completion.len();
        self.completion_selection_index = Some(index);
        self.line = self.completion[index].clone();
    }
    pub fn exit_selection(&mut self) {
        self.completion_selection_index = None;
    }
}

use tui::{
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Modifier, Style},
};

const BASE_WIDTH: u16 = 30;
use crate::ui::text_color;

impl Prompt {
    pub fn render_prompt(&self, area: Rect, surface: &mut Surface, theme: &Theme) {
        let text_color = text_color();
        // completion
        if !self.completion.is_empty() {
            // TODO: find out better way of clearing individual lines of the screen
            let mut row = 0;
            let mut col = 0;
            let max_col = area.width / BASE_WIDTH;
            let col_height = ((self.completion.len() as u16 + max_col - 1) / max_col);

            for i in (3..col_height + 3) {
                surface.set_string(
                    0,
                    area.height - i as u16,
                    " ".repeat(area.width as usize),
                    text_color,
                );
            }
            surface.set_style(
                Rect::new(0, area.height - col_height - 2, area.width, col_height),
                theme.get("ui.statusline"),
            );
            for (i, command) in self.completion.iter().enumerate() {
                let color = if self.completion_selection_index.is_some()
                    && i == self.completion_selection_index.unwrap()
                {
                    Style::default().bg(Color::Rgb(104, 060, 232))
                } else {
                    text_color
                };
                surface.set_stringn(
                    1 + col * BASE_WIDTH,
                    area.height - col_height - 2 + row,
                    &command,
                    BASE_WIDTH as usize - 1,
                    color,
                );
                row += 1;
                if row > col_height - 1 {
                    row = 0;
                    col += 1;
                }
                if col > max_col {
                    break;
                }
            }
        }
        // render buffer text
        surface.set_string(1, area.height - 1, &self.prompt, text_color);
        surface.set_string(2, area.height - 1, &self.line, text_color);
    }
}

impl Component for Prompt {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let event = match event {
            Event::Key(event) => event,
            _ => return EventResult::Ignored,
        };

        match event {
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
            } => self.insert_char(c),
            KeyEvent {
                code: KeyCode::Esc, ..
            } => self.should_close = true,
            KeyEvent {
                code: KeyCode::Right,
                ..
            } => self.move_char_right(),
            KeyEvent {
                code: KeyCode::Left,
                ..
            } => self.move_char_left(),
            KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::CONTROL,
            } => self.move_end(),
            KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            } => self.move_start(),
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
            } => self.delete_char_backwards(),
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => (self.callback_fn)(cx.editor, &self.line),
            KeyEvent {
                code: KeyCode::Tab, ..
            } => self.change_completion_selection(),
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            } => self.exit_selection(),
            _ => (),
        };

        EventResult::Consumed(None)
    }

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        self.render_prompt(area, surface, &cx.editor.theme)
    }

    fn cursor_position(&self, area: Rect, ctx: &mut Context) -> Option<Position> {
        Some(Position::new(
            area.height as usize - 1,
            area.x as usize + 2 + self.cursor,
        ))
    }
}
