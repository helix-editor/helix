use crate::commands;
use crate::View;
use crossterm::event::{KeyCode, KeyEvent};
use std::string::String;

pub struct Prompt {
    pub buffer: String,
    pub cursor_loc: usize,
}

impl Prompt {
    pub fn new() -> Prompt {
        let prompt = Prompt {
            buffer: String::from(":"), // starting prompt symbol
            cursor_loc: 0,
        };
        prompt
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.push(c);
    }

    pub fn handle_keyevent(&mut self, key_event: KeyEvent, view: &mut View) {
        match key_event {
            KeyEvent {
                code: KeyCode::Char(c),
                ..
            } => self.insert_char(c),
            KeyEvent {
                code: KeyCode::Esc, ..
            } => commands::normal_mode(view, 1),
            _ => (),
        }
    }
}
