use crate::Editor;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
        if !self.completion.is_empty() {
            self.completion_selection_index = self
                .completion_selection_index
                .map(|i| {
                    if i == self.completion.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                })
                .or(Some(0));
            self.line = String::from(
                self.completion
                    .get(self.completion_selection_index.unwrap())
                    .unwrap(),
            );
        }
    }

    pub fn exit_selection(&mut self) {
        self.completion_selection_index = None;
    }

    pub fn handle_input(&mut self, key_event: KeyEvent, editor: &mut Editor) {
        match key_event {
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
            } => (self.callback_fn)(editor, &self.line),
            KeyEvent {
                code: KeyCode::Tab, ..
            } => self.change_completion_selection(),
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            } => self.exit_selection(),
            _ => (),
        }
    }
}
