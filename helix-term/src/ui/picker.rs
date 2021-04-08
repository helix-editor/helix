use crate::compositor::{Component, Compositor, Context, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::{
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;

use std::borrow::Cow;

use crate::ui::{Prompt, PromptEvent};
use helix_core::Position;
use helix_view::editor::Action;
use helix_view::Editor;

pub struct Picker<T> {
    options: Vec<T>,
    // filter: String,
    matcher: Box<Matcher>,
    /// (index, score)
    matches: Vec<(usize, i64)>,

    cursor: usize,
    // pattern: String,
    prompt: Prompt,

    format_fn: Box<dyn Fn(&T) -> Cow<str>>,
    callback_fn: Box<dyn Fn(&mut Editor, &T, Action)>,
}

impl<T> Picker<T> {
    pub fn new(
        options: Vec<T>,
        format_fn: impl Fn(&T) -> Cow<str> + 'static,
        callback_fn: impl Fn(&mut Editor, &T, Action) + 'static,
    ) -> Self {
        let prompt = Prompt::new(
            "".to_string(),
            |pattern: &str| Vec::new(),
            |editor: &mut Editor, pattern: &str, event: PromptEvent| {
                //
            },
        );

        let mut picker = Self {
            options,
            matcher: Box::new(Matcher::default()),
            matches: Vec::new(),
            cursor: 0,
            prompt,
            format_fn: Box::new(format_fn),
            callback_fn: Box::new(callback_fn),
        };

        // TODO: scoring on empty input should just use a fastpath
        picker.score();

        picker
    }

    pub fn score(&mut self) {
        // need to borrow via pattern match otherwise it complains about simultaneous borrow
        let Self {
            ref mut options,
            ref mut matcher,
            ref mut matches,
            ref format_fn,
            ..
        } = *self;

        let pattern = &self.prompt.line;

        // reuse the matches allocation
        matches.clear();
        matches.extend(
            self.options
                .iter()
                .enumerate()
                .filter_map(|(index, option)| {
                    // TODO: maybe using format_fn isn't the best idea here
                    let text = (format_fn)(option);
                    // TODO: using fuzzy_indices could give us the char idx for match highlighting
                    matcher
                        .fuzzy_match(&text, pattern)
                        .map(|score| (index, score))
                }),
        );
        matches.sort_unstable_by_key(|(_, score)| -score);

        // reset cursor position
        self.cursor = 0;
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
        self.matches
            .get(self.cursor)
            .map(|(index, _score)| &self.options[*index])
    }
}

// process:
// - read all the files into a list, maxed out at a large value
// - on input change:
//  - score all the names in relation to input

impl<T: 'static> Component for Picker<T> {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let key_event = match event {
            Event::Key(event) => event,
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(
            |compositor: &mut Compositor, editor: &mut Editor| {
                // remove the layer
                compositor.pop();
            },
        )));

        match key_event {
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
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(&mut cx.editor, option, Action::Replace);
                }
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(&mut cx.editor, option, Action::HorizontalSplit);
                }
                return close_fn;
            }
            KeyEvent {
                code: KeyCode::Char('v'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if let Some(option) = self.selection() {
                    (self.callback_fn)(&mut cx.editor, option, Action::VerticalSplit);
                }
                return close_fn;
            }
            _ => {
                if let EventResult::Consumed(_) = self.prompt.handle_event(event, cx) {
                    // TODO: recalculate only if pattern changed
                    self.score();
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

        // clear area
        let background = cx.editor.theme.get("ui.background");
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = surface.get_mut(x, y);
                cell.reset();
                // cell.symbol.clear();
                cell.set_style(background);
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
            (index, self.options.get(*index).unwrap()) // get_unchecked
        });

        for (i, (_index, option)) in files.take(rows as usize).enumerate() {
            if i == self.cursor {
                surface.set_string(inner.x + 1, inner.y + 2 + i as u16, ">", selected);
            }

            surface.set_stringn(
                inner.x + 3,
                inner.y + 2 + i as u16,
                (self.format_fn)(option),
                inner.width as usize - 1,
                if i == self.cursor { selected } else { style },
            );
        }
    }

    fn cursor_position(&self, area: Rect, editor: &Editor) -> Option<Position> {
        self.prompt.cursor_position(area, editor)
    }
}
