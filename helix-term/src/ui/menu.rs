use crate::compositor::{Component, Compositor, Context, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::{
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Style},
    widgets::Table,
};

pub use tui::widgets::{Cell, Row};

use std::borrow::Cow;

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use fuzzy_matcher::FuzzyMatcher;

use helix_core::Position;
use helix_view::Editor;

pub trait Item {
    // TODO: sort_text
    fn filter_text(&self) -> &str;

    fn label(&self) -> &str;
    fn row(&self) -> Row;
}

pub struct Menu<T: Item> {
    options: Vec<T>,

    cursor: Option<usize>,

    matcher: Box<Matcher>,
    /// (index, score)
    matches: Vec<(usize, i64)>,

    callback_fn: Box<dyn Fn(&mut Editor, Option<&T>, MenuEvent)>,

    scroll: usize,
    size: (u16, u16),
}

impl<T: Item> Menu<T> {
    // TODO: it's like a slimmed down picker, share code? (picker = menu + prompt with different
    // rendering)
    pub fn new(
        options: Vec<T>,
        callback_fn: impl Fn(&mut Editor, Option<&T>, MenuEvent) + 'static,
    ) -> Self {
        let mut menu = Self {
            options,
            matcher: Box::new(Matcher::default()),
            matches: Vec::new(),
            cursor: None,
            callback_fn: Box::new(callback_fn),
            scroll: 0,
            size: (0, 0),
        };

        // TODO: scoring on empty input should just use a fastpath
        menu.score("");

        menu
    }

    pub fn score(&mut self, pattern: &str) {
        // need to borrow via pattern match otherwise it complains about simultaneous borrow
        let Self {
            ref mut options,
            ref mut matcher,
            ref mut matches,
            ..
        } = *self;

        // reuse the matches allocation
        matches.clear();
        matches.extend(
            self.options
                .iter()
                .enumerate()
                .filter_map(|(index, option)| {
                    let text = option.filter_text();
                    // TODO: using fuzzy_indices could give us the char idx for match highlighting
                    matcher
                        .fuzzy_match(&text, pattern)
                        .map(|score| (index, score))
                }),
        );
        matches.sort_unstable_by_key(|(_, score)| -score);

        // reset cursor position
        self.cursor = None;
        self.scroll = 0;
    }

    pub fn move_up(&mut self) {
        // TODO: wrap around to end
        let pos = self.cursor.map_or(0, |i| i.saturating_sub(1)) % self.options.len();
        self.cursor = Some(pos);
        self.adjust_scroll();
    }

    pub fn move_down(&mut self) {
        let pos = self.cursor.map_or(0, |i| i + 1) % self.options.len();
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
        self.cursor.and_then(|cursor| {
            self.matches
                .get(cursor)
                .map(|(index, _score)| &self.options[*index])
        })
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }
}

use super::PromptEvent as MenuEvent;

impl<T: Item + 'static> Component for Menu<T> {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let event = match event {
            Event::Key(event) => event,
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor| {
            // remove the layer
            compositor.pop();
        })));

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
                if let Some(selection) = self.selection() {
                    (self.callback_fn)(cx.editor, Some(selection), MenuEvent::Validate);
                }
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

        const MAX: usize = 10;
        let height = std::cmp::min(self.options.len(), MAX);
        let height = std::cmp::min(height, viewport.1 as usize);

        self.size = (width as u16, height as u16);

        // adjust scroll offsets if size changed
        self.adjust_scroll();

        Some(self.size)
    }

    // TODO: required size should re-trigger when we filter items so we can draw a smaller menu

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let style = cx.editor.theme.get("ui.text");
        let selected = cx.editor.theme.get("ui.menu.selected");

        let scroll = self.scroll;

        let options: Vec<_> = self
            .matches
            .iter()
            .map(|(index, _score)| {
                // (index, self.options.get(*index).unwrap()) // get_unchecked
                &self.options[*index] // get_unchecked
            })
            .collect();

        let len = options.len();

        let win_height = area.height as usize;

        const fn div_ceil(a: usize, b: usize) -> usize {
            (a + b - 1) / a
        }

        let scroll_height = std::cmp::min(div_ceil(win_height.pow(2), len), win_height as usize);

        let scroll_line = (win_height - scroll_height) * scroll
            / std::cmp::max(1, len.saturating_sub(win_height));

        use tui::layout::Constraint;
        let rows = options.iter().map(|option| option.row());
        let table = Table::new(rows)
            .style(style)
            .highlight_style(selected)
            .column_spacing(1)
            .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)]);

        use tui::widgets::TableState;

        table.render_table(
            area,
            surface,
            &mut TableState {
                offset: scroll,
                selected: self.cursor,
            },
        );

        // // TODO: set bg for the whole row if selected
        // if line == self.cursor {
        //     surface.set_style(
        //         Rect::new(area.x, area.y + i as u16, area.width - 1, 1),
        //         selected,
        //     )
        // }

        for (i, option) in (scroll..(scroll + win_height).min(len)).enumerate() {
            let is_marked = i >= scroll_line && i < scroll_line + scroll_height;

            if is_marked {
                let cell = surface.get_mut(area.x + area.width - 2, area.y + i as u16);
                cell.set_symbol("▐ ");
                // cell.set_style(selected);
                // cell.set_style(if is_marked { selected } else { style });
            }
        }
    }
}
