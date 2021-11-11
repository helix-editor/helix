use crate::compositor::{Component, Compositor, Context, EventResult};
use crate::{alt, ctrl, key, ui};
use crossterm::event::Event;
use helix_view::input::KeyEvent;
use helix_view::keyboard::{KeyCode, KeyModifiers};
use std::{borrow::Cow, ops::RangeFrom};
use tui::buffer::Buffer as Surface;

use helix_core::{
    unicode::segmentation::GraphemeCursor, unicode::width::UnicodeWidthStr, Position,
};
use helix_view::{
    graphics::{CursorKind, Margin, Rect},
    Editor,
};

pub type Completion = (RangeFrom<usize>, Cow<'static, str>);

pub struct Prompt {
    prompt: Cow<'static, str>,
    pub line: String,
    cursor: usize,
    completion: Vec<Completion>,
    selection: Option<usize>,
    history_register: Option<char>,
    history_pos: Option<usize>,
    completion_fn: Box<dyn FnMut(&str) -> Vec<Completion>>,
    callback_fn: Box<dyn FnMut(&mut Context, &str, PromptEvent)>,
    pub doc_fn: Box<dyn Fn(&str) -> Option<&'static str>>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PromptEvent {
    /// The prompt input has been updated.
    Update,
    /// Validate and finalize the change.
    Validate,
    /// Abort the change, reverting to the initial state.
    Abort,
}

pub enum CompletionDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy)]
pub enum Movement {
    BackwardChar(usize),
    BackwardWord(usize),
    ForwardChar(usize),
    ForwardWord(usize),
    StartOfLine,
    EndOfLine,
    None,
}

impl Prompt {
    pub fn new(
        prompt: Cow<'static, str>,
        history_register: Option<char>,
        mut completion_fn: impl FnMut(&str) -> Vec<Completion> + 'static,
        callback_fn: impl FnMut(&mut Context, &str, PromptEvent) + 'static,
    ) -> Self {
        Self {
            prompt,
            line: String::new(),
            cursor: 0,
            completion: completion_fn(""),
            selection: None,
            history_register,
            history_pos: None,
            completion_fn: Box::new(completion_fn),
            callback_fn: Box::new(callback_fn),
            doc_fn: Box::new(|_| None),
        }
    }

    /// Compute the cursor position after applying movement
    /// Taken from: https://github.com/wez/wezterm/blob/e0b62d07ca9bf8ce69a61e30a3c20e7abc48ce7e/termwiz/src/lineedit/mod.rs#L516-L611
    fn eval_movement(&self, movement: Movement) -> usize {
        match movement {
            Movement::BackwardChar(rep) => {
                let mut position = self.cursor;
                for _ in 0..rep {
                    let mut cursor = GraphemeCursor::new(position, self.line.len(), false);
                    if let Ok(Some(pos)) = cursor.prev_boundary(&self.line, 0) {
                        position = pos;
                    } else {
                        break;
                    }
                }
                position
            }
            Movement::BackwardWord(rep) => {
                let char_indices: Vec<(usize, char)> = self.line.char_indices().collect();
                if char_indices.is_empty() {
                    return self.cursor;
                }
                let mut char_position = char_indices
                    .iter()
                    .position(|(idx, _)| *idx == self.cursor)
                    .unwrap_or(char_indices.len() - 1);

                for _ in 0..rep {
                    if char_position == 0 {
                        break;
                    }

                    let mut found = None;
                    for prev in (0..char_position - 1).rev() {
                        if char_indices[prev].1.is_whitespace() {
                            found = Some(prev + 1);
                            break;
                        }
                    }

                    char_position = found.unwrap_or(0);
                }
                char_indices[char_position].0
            }
            Movement::ForwardWord(rep) => {
                let char_indices: Vec<(usize, char)> = self.line.char_indices().collect();
                if char_indices.is_empty() {
                    return self.cursor;
                }
                let mut char_position = char_indices
                    .iter()
                    .position(|(idx, _)| *idx == self.cursor)
                    .unwrap_or_else(|| char_indices.len());

                for _ in 0..rep {
                    // Skip any non-whitespace characters
                    while char_position < char_indices.len()
                        && !char_indices[char_position].1.is_whitespace()
                    {
                        char_position += 1;
                    }

                    // Skip any whitespace characters
                    while char_position < char_indices.len()
                        && char_indices[char_position].1.is_whitespace()
                    {
                        char_position += 1;
                    }

                    // We are now on the start of the next word
                }
                char_indices
                    .get(char_position)
                    .map(|(i, _)| *i)
                    .unwrap_or_else(|| self.line.len())
            }
            Movement::ForwardChar(rep) => {
                let mut position = self.cursor;
                for _ in 0..rep {
                    let mut cursor = GraphemeCursor::new(position, self.line.len(), false);
                    if let Ok(Some(pos)) = cursor.next_boundary(&self.line, 0) {
                        position = pos;
                    } else {
                        break;
                    }
                }
                position
            }
            Movement::StartOfLine => 0,
            Movement::EndOfLine => {
                let mut cursor =
                    GraphemeCursor::new(self.line.len().saturating_sub(1), self.line.len(), false);
                if let Ok(Some(pos)) = cursor.next_boundary(&self.line, 0) {
                    pos
                } else {
                    self.cursor
                }
            }
            Movement::None => self.cursor,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.line.insert(self.cursor, c);
        let mut cursor = GraphemeCursor::new(self.cursor, self.line.len(), false);
        if let Ok(Some(pos)) = cursor.next_boundary(&self.line, 0) {
            self.cursor = pos;
        }
        self.completion = (self.completion_fn)(&self.line);
        self.exit_selection();
    }

    pub fn insert_str(&mut self, s: &str) {
        self.line.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    pub fn move_cursor(&mut self, movement: Movement) {
        let pos = self.eval_movement(movement);
        self.cursor = pos
    }

    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.line.len();
    }

    pub fn delete_char_backwards(&mut self) {
        let pos = self.eval_movement(Movement::BackwardChar(1));
        self.line.replace_range(pos..self.cursor, "");
        self.cursor = pos;

        self.exit_selection();
        self.completion = (self.completion_fn)(&self.line);
    }

    pub fn delete_char_forwards(&mut self) {
        let pos = self.eval_movement(Movement::ForwardChar(1));
        self.line.replace_range(self.cursor..pos, "");

        self.exit_selection();
        self.completion = (self.completion_fn)(&self.line);
    }

    pub fn delete_word_backwards(&mut self) {
        let pos = self.eval_movement(Movement::BackwardWord(1));
        self.line.replace_range(pos..self.cursor, "");
        self.cursor = pos;

        self.exit_selection();
        self.completion = (self.completion_fn)(&self.line);
    }

    pub fn kill_to_start_of_line(&mut self) {
        let pos = self.eval_movement(Movement::StartOfLine);
        self.line.replace_range(pos..self.cursor, "");
        self.cursor = pos;

        self.exit_selection();
        self.completion = (self.completion_fn)(&self.line);
    }

    pub fn kill_to_end_of_line(&mut self) {
        let pos = self.eval_movement(Movement::EndOfLine);
        self.line.replace_range(self.cursor..pos, "");

        self.exit_selection();
        self.completion = (self.completion_fn)(&self.line);
    }

    pub fn clear(&mut self) {
        self.line.clear();
        self.cursor = 0;
        self.completion = (self.completion_fn)(&self.line);
        self.exit_selection();
    }

    pub fn change_history(&mut self, register: &[String], direction: CompletionDirection) {
        if register.is_empty() {
            return;
        }

        let end = register.len().saturating_sub(1);

        let index = match direction {
            CompletionDirection::Forward => self.history_pos.map_or(0, |i| i + 1),
            CompletionDirection::Backward => {
                self.history_pos.unwrap_or(register.len()).saturating_sub(1)
            }
        }
        .min(end);

        self.line = register[index].clone();

        self.history_pos = Some(index);

        self.move_end();
    }

    pub fn change_completion_selection(&mut self, direction: CompletionDirection) {
        if self.completion.is_empty() {
            return;
        }

        let index = match direction {
            CompletionDirection::Forward => self.selection.map_or(0, |i| i + 1),
            CompletionDirection::Backward => {
                self.selection.unwrap_or(0) + self.completion.len() - 1
            }
        } % self.completion.len();

        self.selection = Some(index);

        let (range, item) = &self.completion[index];

        self.line.replace_range(range.clone(), item);

        self.move_end();
    }

    pub fn exit_selection(&mut self) {
        self.selection = None;
    }
}

const BASE_WIDTH: u16 = 30;

impl Prompt {
    pub fn render_prompt(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let theme = &cx.editor.theme;
        let prompt_color = theme.get("ui.text");
        let completion_color = theme.get("ui.statusline");
        let selected_color = theme.get("ui.menu.selected");
        // completion

        let max_len = self
            .completion
            .iter()
            .map(|(_, completion)| completion.len() as u16)
            .max()
            .unwrap_or(BASE_WIDTH)
            .max(BASE_WIDTH);

        let cols = std::cmp::max(1, area.width / max_len);
        let col_width = (area.width - (cols)) / cols;

        let height = ((self.completion.len() as u16 + cols - 1) / cols)
            .min(10) // at most 10 rows (or less)
            .min(area.height.saturating_sub(1));

        let completion_area = Rect::new(
            area.x,
            (area.height - height).saturating_sub(1),
            area.width,
            height,
        );

        if !self.completion.is_empty() {
            let area = completion_area;
            let background = theme.get("ui.statusline");

            let items = height as usize * cols as usize;

            let offset = self
                .selection
                .map(|selection| selection / items * items)
                .unwrap_or_default();

            surface.clear_with(area, background);

            let mut row = 0;
            let mut col = 0;

            for (i, (_range, completion)) in
                self.completion.iter().enumerate().skip(offset).take(items)
            {
                let color = if Some(i) == self.selection {
                    selected_color // TODO: just invert bg
                } else {
                    completion_color
                };
                surface.set_stringn(
                    area.x + col * (1 + col_width),
                    area.y + row,
                    &completion,
                    col_width.saturating_sub(1) as usize,
                    color,
                );
                row += 1;
                if row > area.height - 1 {
                    row = 0;
                    col += 1;
                }
            }
        }

        if let Some(doc) = (self.doc_fn)(&self.line) {
            let mut text = ui::Text::new(doc.to_string());

            let viewport = area;
            let area = viewport.intersection(Rect::new(
                completion_area.x,
                completion_area.y.saturating_sub(3),
                BASE_WIDTH * 3,
                3,
            ));

            let background = theme.get("ui.help");
            surface.clear_with(area, background);

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
        surface.set_string(area.x, area.y + line, &self.prompt, prompt_color);
        surface.set_string(
            area.x + self.prompt.len() as u16,
            area.y + line,
            &self.line,
            prompt_color,
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

        match event.into() {
            ctrl!('c') | key!(Esc) => {
                (self.callback_fn)(cx, &self.line, PromptEvent::Abort);
                return close_fn;
            }
            alt!('b') | alt!(Left) => self.move_cursor(Movement::BackwardWord(1)),
            alt!('f') | alt!(Right) => self.move_cursor(Movement::ForwardWord(1)),
            ctrl!('b') | key!(Left) => self.move_cursor(Movement::BackwardChar(1)),
            ctrl!('f') | key!(Right) => self.move_cursor(Movement::ForwardChar(1)),
            ctrl!('e') | key!(End) => self.move_end(),
            ctrl!('a') | key!(Home) => self.move_start(),
            ctrl!('w') => self.delete_word_backwards(),
            ctrl!('k') => self.kill_to_end_of_line(),
            ctrl!('u') => self.kill_to_start_of_line(),
            ctrl!('h') | key!(Backspace) => {
                self.delete_char_backwards();
                (self.callback_fn)(cx, &self.line, PromptEvent::Update);
            }
            ctrl!('d') | key!(Delete) => {
                self.delete_char_forwards();
                (self.callback_fn)(cx, &self.line, PromptEvent::Update);
            }
            ctrl!('s') => {
                let (view, doc) = current!(cx.editor);
                let text = doc.text().slice(..);

                use helix_core::textobject;
                let range = textobject::textobject_word(
                    text,
                    doc.selection(view.id).primary(),
                    textobject::TextObject::Inside,
                    1,
                    false,
                );
                let line = text.slice(range.from()..range.to()).to_string();
                if !line.is_empty() {
                    self.insert_str(line.as_str());
                    (self.callback_fn)(cx, &self.line, PromptEvent::Update);
                }
            }
            key!(Enter) => {
                if self.selection.is_some() && self.line.ends_with('/') {
                    self.completion = (self.completion_fn)(&self.line);
                    self.exit_selection();
                } else {
                    (self.callback_fn)(cx, &self.line, PromptEvent::Validate);

                    if let Some(register) = self.history_register {
                        // store in history
                        let register = cx.editor.registers.get_mut(register);
                        register.push(self.line.clone());
                    }
                    return close_fn;
                }
            }
            ctrl!('p') | key!(Up) => {
                if let Some(register) = self.history_register {
                    let register = cx.editor.registers.get_mut(register);
                    self.change_history(register.read(), CompletionDirection::Backward);
                    (self.callback_fn)(cx, &self.line, PromptEvent::Update);
                }
            }
            ctrl!('n') | key!(Down) => {
                if let Some(register) = self.history_register {
                    let register = cx.editor.registers.get_mut(register);
                    self.change_history(register.read(), CompletionDirection::Forward);
                    (self.callback_fn)(cx, &self.line, PromptEvent::Update);
                }
            }
            key!(Tab) => {
                self.change_completion_selection(CompletionDirection::Forward);
                (self.callback_fn)(cx, &self.line, PromptEvent::Update)
            }
            key!(BackTab) => {
                self.change_completion_selection(CompletionDirection::Backward);
                (self.callback_fn)(cx, &self.line, PromptEvent::Update)
            }
            ctrl!('q') => self.exit_selection(),
            // any char event that's not combined with control or mapped to any other combo
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
            } if !modifiers.contains(KeyModifiers::CONTROL) => {
                self.insert_char(c);
                (self.callback_fn)(cx, &self.line, PromptEvent::Update);
            }
            _ => (),
        };

        EventResult::Consumed(None)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        self.render_prompt(area, surface, cx)
    }

    fn cursor(&self, area: Rect, _editor: &Editor) -> (Option<Position>, CursorKind) {
        let line = area.height as usize - 1;
        (
            Some(Position::new(
                area.y as usize + line,
                area.x as usize
                    + self.prompt.len()
                    + UnicodeWidthStr::width(&self.line[..self.cursor]),
            )),
            CursorKind::Block,
        )
    }
}
