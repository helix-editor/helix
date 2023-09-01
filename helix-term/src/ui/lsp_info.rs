use crate::compositor::{Component, Compositor, Context, EventResult};
use crate::key;
use helix_core::syntax::LanguageServerConfiguration;
use helix_view::graphics::Rect;
use helix_view::info::{Info, Location};
use helix_view::input::Event;
use helix_view::Editor;
use tui::buffer::Buffer as Surface;

pub struct LspInfo(Info);

struct ServerOk;
struct ServerError(String);

struct ServerState<'a> {
    name: &'a str,
    status: Result<ServerOk, ServerError>,
}

impl LspInfo {
    pub fn new(editor: &Editor) -> Self {
        LspInfo(Info::new(
            "Lsp Info",
            LspInfo::get_info(editor),
            Location::Center,
        ))
    }

    fn get_server_states(editor: &Editor) -> Vec<ServerState> {
        let doc = doc!(editor);

        let connected_names = doc
            .language_servers()
            .map(|ls| ls.name())
            .collect::<Vec<_>>();

        let server_configs = editor.syn_loader.language_server_configs();

        doc.language_config()
            .map(|config| {
                config
                    .language_servers
                    .iter()
                    .map(|ls| match connected_names.contains(&ls.name.as_str()) {
                        true => ServerState {
                            name: &ls.name,
                            status: Ok(ServerOk),
                        },
                        false => ServerState {
                            name: &ls.name,
                            status: Err(ServerError(LspInfo::get_error_message(
                                &ls.name,
                                server_configs.get(&ls.name),
                            ))),
                        },
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn get_info(editor: &Editor) -> String {
        let server_states = LspInfo::get_server_states(editor);

        format!(
            "Server status:\n\
                    {}\n\
                    \n\
                    Press q or Esc to close",
            server_states
                .iter()
                .map(|state| {
                    let prefix = match state.status {
                        Ok(_) => "✓",
                        Err(_) => "✘",
                    };

                    let error = if let Err(e) = &state.status {
                        "\n      ".to_string() + e.0.as_str()
                    } else {
                        "".to_string()
                    };

                    format!("  {} {}{}", prefix, state.name, error)
                })
                .reduce(|acc, i| acc + "\n" + i.as_str())
                .unwrap_or_default()
        )
    }

    fn get_error_message(_name: &str, config: Option<&LanguageServerConfiguration>) -> String {
        match config {
            Some(config) => {
                if let Err(e) = which::which(&config.command) {
                    return e.to_string();
                }

                "Unknown error".to_string()
            }
            None => "No configuration supplied".to_string(),
        }
    }
}

impl Component for LspInfo {
    fn render(&mut self, viewport: Rect, surface: &mut Surface, cx: &mut Context) {
        self.0.render(viewport, surface, cx)
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
