use std::borrow::Cow;

use helix_view::{graphics::Rect, Editor};

use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, Widget as _},
};

use crate::compositor::{Component, Context, Event, EventResult};

use super::{Menu, PromptEvent, Text};

pub struct Select<T: AsRef<str> + Sync + Send + 'static> {
    message: Text,

    options: Menu<SelectItem<T>>,
}

struct SelectItem<T: AsRef<str>>(T);

impl<T: AsRef<str> + Sync + Send + 'static> super::menu::Item for SelectItem<T> {
    type Data = ();

    fn format(&self, _data: &Self::Data) -> tui::widgets::Row {
        self.0.as_ref().into()
    }
}

impl<T: AsRef<str> + Sync + Send + 'static> Select<T> {
    pub fn new<M, I, F>(message: M, options: I, callback: F) -> Self
    where
        M: Into<Cow<'static, str>>,

        I: IntoIterator<Item = T>,

        F: Fn(&mut Editor, &T, PromptEvent) + 'static,
    {
        let message = tui::text::Text::from(message.into()).into();

        let options: Vec<_> = options.into_iter().map(SelectItem).collect();

        assert!(!options.is_empty());

        let mut menu = Menu::new(options, (), move |editor, option, event| {
            // Options are non-empty (asserted above) and an option is selected by default,

            // so `option` must be Some here.

            let option = &option.unwrap().0;

            callback(editor, option, event)
        })
        .auto_close(true);

        // Select the first option by default.

        menu.move_down();

        Self {
            message,

            options: menu,
        }
    }
}

impl<T: AsRef<str> + Sync + Send + 'static> Component for Select<T> {
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        self.options.handle_event(event, cx)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let (message_width, message_height) = self.message.required_size(viewport).unwrap();

        let (menu_width, menu_height) = self.options.required_size(viewport).unwrap();

        Some((
            menu_width.max(message_width + 2),
            message_height + menu_height + 2,
        ))
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        const BLOCK: Block<'_> = Block::bordered();

        // +---------------------+

        // | message             |

        // +---------------------+

        //   options menu

        //

        //

        // Limit the text width to 80% of the screen or 80 columns, whichever is

        // smaller.

        let max_width = 80.min(((area.width as u32) * 80u32 / 100) as u16);

        let (message_width, message_height) =
            super::text::required_size(&self.message.contents, max_width);

        let (_, menu_height) = self
            .options
            .required_size((max_width, area.height))
            .unwrap();

        // + 2 for borders

        let width = message_width + 2;

        let height = message_height + 2 + menu_height;

        let area = Rect {
            x: (area.width / 2) - width / 2,

            y: (area.height / 2) - height / 2,

            width,

            height,
        };

        // Message

        let background = cx.editor.theme.get("ui.background");

        let message_box = area.with_height(message_height + 2);

        surface.clear_with(message_box, background);

        BLOCK.render(message_box, surface);

        let message_area = BLOCK.inner(message_box);

        self.message.render(message_area, surface, cx);

        // Options menu

        let menu_area = area.clip_top(message_height + 2);

        self.options.render(menu_area, surface, cx);
    }
}
