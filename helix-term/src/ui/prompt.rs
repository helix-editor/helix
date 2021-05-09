use crate::compositor::{Component, Compositor, Context, EventResult};
use crate::ui;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use helix_core::Position;
use helix_view::{Editor, Theme};
use std::{borrow::Cow, ops::RangeFrom};

pub type Completion = (RangeFrom<usize>, Cow<'static, str>);

pub struct Prompt {
    prompt: String,
    pub line: String,
    cursor: usize,
    completion: Vec<Completion>,
    selection: Option<usize>,
    completion_fn: Box<dyn FnMut(&str) -> Vec<Completion>>,
    callback_fn: Box<dyn FnMut(&mut Editor, &str, PromptEvent)>,
    pub doc_fn: Box<dyn Fn(&str) -> Option<&'static str>>,
}

#[derive(PartialEq)]
pub enum PromptEvent {
    /// The prompt input has been updated.
    Update,
    /// Validate and finalize the change.
    Validate,
    /// Abort the change, reverting to the initial state.
    Abort,
}

impl Prompt {
    pub fn new(
        prompt: String,
        mut completion_fn: impl FnMut(&str) -> Vec<Completion> + 'static,
        callback_fn: impl FnMut(&mut Editor, &str, PromptEvent) + 'static,
    ) -> Self {
        Self {
            prompt,
            line: String::new(),
            cursor: 0,
            completion: completion_fn(""),
            selection: None,
            completion_fn: Box::new(completion_fn),
            callback_fn: Box::new(callback_fn),
            doc_fn: Box::new(|_| None),
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.line.insert(self.cursor, c);
        self.cursor += 1;
        self.completion = (self.completion_fn)(&self.line);
        self.exit_selection();
    }

    pub fn move_char_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1)
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
        let index = self.selection.map_or(0, |i| i + 1) % self.completion.len();
        self.selection = Some(index);

        let (range, item) = &self.completion[index];

        self.line.replace_range(range.clone(), item);

        self.move_end();
        // TODO: recalculate completion when completion item is accepted, (Enter)
    }
    pub fn exit_selection(&mut self) {
        self.selection = None;
    }
}

use tui::{
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Modifier, Style},
};

const BASE_WIDTH: u16 = 30;

impl Prompt {
    pub fn render_prompt(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let theme = &cx.editor.theme;
        let text_color = theme.get("ui.text.focus");
        // completion

        let max_col = area.width / BASE_WIDTH;
        let height = ((self.completion.len() as u16 + max_col - 1) / max_col);
        let completion_area = Rect::new(
            area.x,
            (area.height - height).saturating_sub(1),
            area.width,
            height,
        );

        if !self.completion.is_empty() {
            let area = completion_area;
            let background = theme.get("ui.statusline");

            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    let cell = surface.get_mut(x, y);
                    cell.reset();
                    cell.set_style(background);
                }
            }

            let mut row = 0;
            let mut col = 0;

            for (i, (_range, completion)) in self.completion.iter().enumerate() {
                let color = if Some(i) == self.selection {
                    Style::default().bg(Color::Rgb(104, 60, 232))
                } else {
                    text_color
                };
                surface.set_stringn(
                    area.x + 1 + col * BASE_WIDTH,
                    area.y + row,
                    &completion,
                    BASE_WIDTH as usize - 1,
                    color,
                );
                row += 1;
                if row > area.height - 1 {
                    row = 0;
                    col += 1;
                }
                if col > max_col {
                    break;
                }
            }
        }

        if let Some(doc) = (self.doc_fn)(&self.line) {
            let text = ui::Text::new(doc.to_string());

            let area = Rect::new(
                completion_area.x,
                completion_area.y - 3,
                completion_area.width,
                3,
            );

            let background = theme.get("ui.window");
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    let cell = surface.get_mut(x, y);
                    cell.reset();
                    cell.set_style(background);
                }
            }

            use tui::layout::Margin;
            text.render(
                area.inner(&Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
                surface,
                cx,
            );
        }

        let line = area.height - 1;
        // render buffer text
        surface.set_string(area.x, area.y + line, &self.prompt, text_color);
        surface.set_string(
            area.x + self.prompt.len() as u16,
            area.y + line,
            &self.line,
            text_color,
        );
    }
}

impl Component for Prompt {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let event = match event {
            Event::Key(event) => event,
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor| {
            // remove the layer
            compositor.pop();
        })));

        match event {
            // char or shift char
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
            }
            | KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::SHIFT,
            } => {
                self.insert_char(c);
                (self.callback_fn)(cx.editor, &self.line, PromptEvent::Update);
            }
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                (self.callback_fn)(cx.editor, &self.line, PromptEvent::Abort);
                return close_fn;
            }
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
            } => {
                self.delete_char_backwards();
                (self.callback_fn)(cx.editor, &self.line, PromptEvent::Update);
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                (self.callback_fn)(cx.editor, &self.line, PromptEvent::Validate);
                return close_fn;
            }
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
        self.render_prompt(area, surface, cx)
    }

    fn cursor_position(&self, area: Rect, editor: &Editor) -> Option<Position> {
        Some(Position::new(
            area.height as usize,
            area.x as usize + self.prompt.len() + self.cursor,
        ))
    }
}
