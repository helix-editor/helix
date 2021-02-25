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

pub struct Popup {
    contents: String,
    position: Position,
}

impl Popup {
    // TODO: it's like a slimmed down picker, share code? (picker = menu + prompt with different
    // rendering)
    pub fn new(contents: String) -> Self {
        Self {
            contents,
            position: Position::default(),
        }
    }

    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }
}

impl Component for Popup {
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
                return close_fn;
            }
            _ => (),
        }
        // for some events, we want to process them but send ignore, specifically all input except
        // tab/enter/ctrl-k or whatever will confirm the selection/ ctrl-n/ctrl-p for scroll.
        // EventResult::Consumed(None)
        EventResult::Consumed(None)
    }
    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // render a box at x, y. Width equal to max width of item.
        const MAX: usize = 15;
        let rows = std::cmp::min(self.contents.lines().count(), MAX) as u16; // inefficient
        let area = Rect::new(self.position.col as u16, self.position.row as u16, 80, rows);

        // clear area
        let background = cx.editor.theme.get("ui.popup");
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = surface.get_mut(x, y);
                cell.reset();
                // cell.symbol.clear();
                cell.set_style(background);
            }
        }

        // -- Render the contents:

        let style = Style::default().fg(Color::Rgb(164, 160, 232)); // lavender

        use tui::text::Text;
        use tui::widgets::{Paragraph, Widget, Wrap};
        let contents = Text::from(self.contents.clone());
        let par = Paragraph::new(contents).wrap(Wrap { trim: false });

        par.render(area, surface);
    }
}
