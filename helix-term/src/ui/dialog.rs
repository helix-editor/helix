use crate::compositor::{Callback, Component, Context, Event, EventResult};
use crate::key;
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, Borders, Widget},
};

use helix_view::graphics::{Margin, Modifier, Rect, Style, UnderlineStyle};
use helix_view::input::{KeyCode, KeyEvent};

pub struct Choice {
    pub name: String,
    pub keys: Vec<KeyEvent>,
}

impl Choice {
    pub fn new(name: String, keys: Vec<KeyEvent>) -> Self {
        Self { name, keys }
    }
}

impl From<String> for Choice {
    fn from(name: String) -> Self {
        let c = name.chars().next();
        Self::new(name, vec![key!(c.unwrap().to_ascii_lowercase())])
    }
}

impl From<&str> for Choice {
    fn from(name: &str) -> Self {
        Self::from(name.to_string())
    }
}

pub struct Dialog<T: Component> {
    contents: T,
    choices: Vec<Choice>,
    callback_fn: Option<Box<dyn FnOnce(&mut Context, &Choice)>>,
    selected: usize,
    child_size: (u16, u16),
}

impl<T: Component> Dialog<T> {
    pub fn new(
        contents: T,
        choices: Vec<Choice>,
        callback_fn: impl FnOnce(&mut Context, &Choice) + 'static,
    ) -> anyhow::Result<Self> {
        if choices.is_empty() {
            anyhow::bail!("dialog box had no choices");
        }
        Ok(Self {
            contents,
            choices,
            callback_fn: Some(Box::new(callback_fn)),
            selected: 0,
            child_size: (0, 0),
        })
    }
}

impl<T: Component> Component for Dialog<T> {
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        let key = match event {
            Event::Key(event) => *event,
            // TODO: mouse events
            _ => return EventResult::Ignored(None),
        };

        let close_fn: Callback = Box::new(|compositor, _| {
            compositor.pop();
        });

        let close = match key {
            key!(Enter) => {
                (self.callback_fn.take().unwrap())(cx, &self.choices[self.selected]);
                true
            }
            key!(Left) => {
                self.selected =
                    self.selected.wrapping_add(self.choices.len() - 1) % self.choices.len();
                false
            }
            key!(Right) => {
                self.selected = self.selected.wrapping_add(1) % self.choices.len();
                false
            }
            _ => {
                if let Some(choice) = self
                    .choices
                    .iter()
                    .find(|choice| choice.keys.contains(&key))
                {
                    (self.callback_fn.take().unwrap())(cx, choice);
                    true
                } else {
                    false
                }
            }
        };
        EventResult::Consumed(if close { Some(close_fn) } else { None })
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let child_size = self.contents.required_size(viewport).unwrap_or((0, 0));
        self.child_size = child_size;
        let choices_width = 3
            + 5 * self.choices.len()
            + self
                .choices
                .iter()
                .map(|choice| choice.name.len())
                .sum::<usize>();
        Some((
            (child_size.0 + 4).max(choices_width.try_into().unwrap()),
            (child_size.1 + 6).max(7),
        ))
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let size = self
            .required_size((area.width, area.height))
            .unwrap_or((area.width, area.height));
        let area = area
            .clip_left((area.width - size.0) / 2)
            .clip_top((area.height - size.1) / 2)
            .with_width(size.0)
            .with_height(size.1);

        let background = cx.editor.theme.get("ui.popup");
        surface.clear_with(area, background);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area).inner(&Margin::horizontal(1));
        block.render(area, surface);

        self.contents.render(inner, surface, cx);

        let mut button_x = inner.x;
        for (idx, choice) in self.choices.iter().enumerate() {
            let button = Rect::new(
                button_x,
                inner.y + self.child_size.1 + 1,
                (choice.name.len() + 4).try_into().unwrap(),
                3,
            );
            let block = Block::default().borders(Borders::ALL);
            let block = if self.selected == idx {
                let reverse = Style::default().add_modifier(Modifier::REVERSED);
                block.border_style(reverse).style(reverse)
            } else {
                block
            };
            let inner = block.inner(button).inner(&Margin::horizontal(1));
            block.render(button, surface);
            let mut spans = vec![];
            if let Some(c) = choice.keys.iter().find_map(|key| {
                if let KeyCode::Char(c) = key.code {
                    Some(c)
                } else {
                    None
                }
            }) {
                let pat = &[c.to_ascii_lowercase(), c.to_ascii_uppercase()];
                if let Some(idx) = choice.name.find(pat) {
                    let (pre, rest) = choice.name.split_at(idx);
                    if !pre.is_empty() {
                        spans.push(tui::text::Span::raw(pre.to_string()));
                    }
                    spans.push(tui::text::Span::styled(
                        rest.chars().next().unwrap().to_string(),
                        Style::default().underline_style(UnderlineStyle::Line),
                    ));
                    spans.push(tui::text::Span::raw(
                        rest.strip_prefix(pat).unwrap().to_string(),
                    ));
                } else {
                    spans.push(tui::text::Span::raw(choice.name.clone()));
                }
            } else {
                spans.push(tui::text::Span::raw(choice.name.clone()));
            }
            let text = tui::text::Text::from(tui::text::Spans::from(spans));
            crate::ui::Text::from(text).render(inner, surface, cx);
            button_x += 5 + u16::try_from(choice.name.len()).unwrap();
        }
    }
}
