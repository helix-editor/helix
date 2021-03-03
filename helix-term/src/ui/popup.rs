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

// TODO: share logic with Menu, it's essentially Popup(render_fn), but render fn needs to return
// a width/height hint. maybe Popup(Box<Component>)

pub struct Popup {
    contents: Box<dyn Component>,
    position: Option<Position>,
}

impl Popup {
    // TODO: it's like a slimmed down picker, share code? (picker = menu + prompt with different
    // rendering)
    pub fn new(contents: Box<dyn Component>) -> Self {
        Self {
            contents,
            position: None,
        }
    }

    pub fn set_position(&mut self, pos: Option<Position>) {
        self.position = pos;
    }
}

impl Component for Popup {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        let key = match event {
            Event::Key(event) => event,
            Event::Resize(width, height) => {
                // TODO: calculate inner area, call component's handle_event with that area
                return EventResult::Ignored;
            }
            _ => return EventResult::Ignored,
        };

        let close_fn = EventResult::Consumed(Some(Box::new(
            |compositor: &mut Compositor, editor: &mut Editor| {
                // remove the layer
                compositor.pop();
            },
        )));

        match key {
            // esc or ctrl-c aborts the completion and closes the menu
            KeyEvent {
                code: KeyCode::Esc, ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            } => close_fn,
            _ => self.contents.handle_event(event, cx),
        }
        // for some events, we want to process them but send ignore, specifically all input except
        // tab/enter/ctrl-k or whatever will confirm the selection/ ctrl-n/ctrl-p for scroll.
    }

    fn render(&self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        use tui::text::Text;
        use tui::widgets::{Paragraph, Widget, Wrap};

        let position = self
            .position
            .or_else(|| cx.editor.cursor_position())
            .unwrap_or_default();

        let (width, height) = self
            .contents
            .size_hint(viewport)
            .expect("Component needs size_hint implemented in order to be embedded in a popup");

        let width = width.min(150) as u16;
        let height = height.min(13) as u16;

        // -- make sure frame doesn't stick out of bounds
        let mut rel_x = position.col as u16;
        let mut rel_y = position.row as u16;
        if viewport.width <= rel_x + width {
            rel_x -= ((rel_x + width) - viewport.width)
        };

        // TODO: be able to specify orientation preference. We want above for most popups, below
        // for menus/autocomplete.
        if height <= rel_y {
            rel_y -= height // position above point
        } else {
            rel_y += 1 // position below point
        }

        let area = Rect::new(rel_x, rel_y, width, height);
        // clip to viewport
        let area = viewport.intersection(Rect::new(rel_x, rel_y, width, height));

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

        self.contents.render(area, surface, cx);
    }
}
