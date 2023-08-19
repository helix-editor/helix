use crate::compositor::{Component, Compositor, Context, EventResult};
use crate::key;
use helix_view::graphics::Rect;
use helix_view::info::{Info, Location};
use helix_view::input::Event;
use helix_view::Editor;
use tui::buffer::Buffer as Surface;

pub struct LspInfo {
    info: Info,
}

impl LspInfo {
    pub fn new(editor: &Editor) -> Self {
        let doc = doc!(editor);

        let connected_names = doc
            .language_servers()
            .map(|ls| ls.name())
            .collect::<Vec<_>>();

        let configured_names = doc
            .language_config()
            .map(|config| {
                config
                    .language_servers
                    .iter()
                    .map(|ls| {
                        let prefix = match connected_names.contains(&ls.name.as_str()) {
                            true => "✓",
                            false => "✘",
                        };

                        format!("  {} {}", prefix, ls.name)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        LspInfo {
            info: Info::new(
                "Lsp Info",
                format!(
                    "Server status:\n\
                    {}\n\
                    \n\
                    Press q or Esc to close",
                    configured_names.join("\n")
                ),
                Location::Center,
            ),
        }
    }
}

impl Component for LspInfo {
    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        self.info.render(viewport, surface, cx)
    }

    fn handle_event(&mut self, event: &Event, _ctx: &mut Context) -> EventResult {
        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor, _cx| {
            // remove the layer
            compositor.pop();
        })));

        match event {
            Event::Key(key_event) => match key_event {
                key!('q') | key!(Esc) => close_fn,
                _ => EventResult::Consumed(None),
            },
            _ => EventResult::Ignored(None),
        }
    }
}
