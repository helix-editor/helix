use crate::compositor::{Component, Compositor, Context, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::buffer::Buffer as Surface;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;
use ignore::Walk;

use std::path::PathBuf;

use crate::ui::{Prompt, PromptEvent};
use helix_core::Position;
use helix_view::Editor;

pub struct Picker {
    files: Vec<PathBuf>,
    // filter: String,
    matcher: Box<Matcher>,
    /// (index, score)
    matches: Vec<(usize, i64)>,

    cursor: usize,
    // pattern: String,
    prompt: Prompt,
}

impl Picker {
    pub fn new() -> Self {
        let files = Walk::new("./").filter_map(|entry| match entry {
            Ok(entry) => {
                // filter dirs, but we might need special handling for symlinks!
                if !entry.file_type().unwrap().is_dir() {
                    Some(entry.into_path())
                } else {
                    None
                }
            }
            Err(_err) => None,
        });

        let prompt = Prompt::new(
            "".to_string(),
            |pattern: &str| Vec::new(),
            |editor: &mut Editor, pattern: &str, event: PromptEvent| {
                //
            },
        );

        const MAX: usize = 1024;

        let mut picker = Self {
            files: files.take(MAX).collect(),
            matcher: Box::new(Matcher::default()),
            matches: Vec::new(),
            cursor: 0,
            prompt,
        };

        // TODO: scoring on empty input should just use a fastpath
        picker.score();

        picker
    }

    pub fn score(&mut self) {
        // need to borrow via pattern match otherwise it complains about simultaneous borrow
        let Self {
            ref mut files,
            ref mut matcher,
            ref mut matches,
            ..
        } = *self;

        let pattern = &self.prompt.line;

        // reuse the matches allocation
        matches.clear();
        matches.extend(self.files.iter().enumerate().filter_map(|(index, path)| {
            match path.to_str() {
                // TODO: using fuzzy_indices could give us the char idx for match highlighting
                Some(path) => matcher
                    .fuzzy_match(path, pattern)
                    .map(|score| (index, score)),
                None => None,
            }
        }));
        matches.sort_unstable_by_key(|(_, score)| -score);

        // reset cursor position
        self.cursor = 0;
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        // TODO: len - 1
        if self.cursor < self.files.len() {
            self.cursor += 1;
        }
    }

    pub fn selection(&self) -> Option<&PathBuf> {
        self.matches
            .get(self.cursor)
            .map(|(index, _score)| &self.files[*index])
    }
}

// process:
// - read all the files into a list, maxed out at a large value
// - on input change:
//  - score all the names in relation to input

impl Component for Picker {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let key_event = match event {
            Event::Key(event) => event,
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor| {
            // remove the layer
            compositor.pop();
        })));

        match key_event {
            // KeyEvent {
            //     code: KeyCode::Char(c),
            //     modifiers: KeyModifiers::NONE,
            // } => {
            //     self.insert_char(c);
            //     (self.callback_fn)(cx.editor, &self.line, PromptEvent::Update);
            // }
            KeyEvent {
                code: KeyCode::Up, ..
            }
            | KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::CONTROL,
            } => self.move_up(),
            KeyEvent {
                code: KeyCode::Down,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::CONTROL,
            } => self.move_down(),
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                return close_fn;
            }
            _ => {
                match self.prompt.handle_event(event, cx) {
                    EventResult::Consumed(_) => {
                        // TODO: recalculate only if pattern changed
                        self.score();
                    }
                    _ => (),
                }
            }
        }

        EventResult::Consumed(None)
    }

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let padding_vertical = area.height * 20 / 100;
        let padding_horizontal = area.width * 20 / 100;

        let area = Rect::new(
            area.x + padding_horizontal,
            area.y + padding_vertical,
            area.width - padding_horizontal * 2,
            area.height - padding_vertical * 2,
        );

        // -- Render the frame:

        // clear  area
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                surface.get_mut(x, y).reset()
            }
        }

        use tui::widgets::Widget;
        // don't like this but the lifetime sucks
        let block = Block::default().borders(Borders::ALL);

        // calculate the inner area inside the box
        let inner = block.inner(area);

        block.render(area, surface);
        // TODO: abstract into a clear(area) fn
        // surface.set_style(inner, Style::default().bg(Color::Rgb(150, 50, 0)));

        // -- Render the input bar:

        let area = Rect::new(inner.x + 1, inner.y, inner.width - 1, 1);
        self.prompt.render(area, surface, cx);

        // -- Separator
        use tui::widgets::BorderType;
        let style = Style::default().fg(Color::Rgb(90, 89, 119));
        let symbols = BorderType::line_symbols(BorderType::Plain);
        for x in inner.left()..inner.right() {
            surface
                .get_mut(x, inner.y + 1)
                .set_symbol(symbols.horizontal)
                .set_style(style);
        }

        // -- Render the contents:

        let style = Style::default().fg(Color::Rgb(164, 160, 232)); // lavender
        let selected = Style::default().fg(Color::Rgb(255, 255, 255));

        let rows = inner.height - 2; // -1 for search bar

        let files = self.matches.iter().map(|(index, _score)| {
            (index, self.files.get(*index).unwrap()) // get_unchecked
        });

        for (i, (_index, file)) in files.take(rows as usize).enumerate() {
            if i == self.cursor {
                surface.set_string(inner.x + 1, inner.y + 2 + i as u16, ">", selected);
            }

            surface.set_stringn(
                inner.x + 3,
                inner.y + 2 + i as u16,
                file.strip_prefix("./").unwrap().to_str().unwrap(), // TODO: render paths without ./
                inner.width as usize - 1,
                if i == self.cursor { selected } else { style },
            );
        }
    }

    fn cursor_position(&self, area: Rect, ctx: &mut Context) -> Option<Position> {
        self.prompt.cursor_position(area, ctx)
    }
}
