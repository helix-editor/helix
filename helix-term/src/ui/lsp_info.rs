use std::fmt::Display;

use crate::compositor::{Component, Compositor, Context, EventResult};
use crate::key;
use helix_core::syntax::{FormatterConfiguration, LanguageServerConfiguration};
use helix_view::graphics::Rect;
use helix_view::info::{Info, Location};
use helix_view::input::Event;
use helix_view::Editor;
use tui::buffer::Buffer as Surface;

pub struct LspInfo(Info);

struct ServerInfo<'a> {
    name: &'a str,
    config: &'a LanguageServerConfiguration,
    client: Option<&'a helix_lsp::Client>,
}

impl LspInfo {
    pub fn new(editor: &Editor) -> Self {
        Self(Info::new(
            "Lsp Info",
            LspInfo::get_info(editor),
            Location::Center,
        ))
    }

    fn get_server_info(editor: &Editor) -> Vec<ServerInfo> {
        let doc = doc!(editor);

        let configured_names = doc
            .language_config()
            .map(|config| {
                config
                    .language_servers
                    .iter()
                    .map(|ls| ls.name.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        editor
            .syn_loader
            .language_server_configs()
            .iter()
            .filter(|(name, _)| configured_names.contains(&name.as_str()))
            .map(|(name, config)| {
                let client = doc.language_servers().find(|client| client.name() == name);

                ServerInfo {
                    name,
                    config,
                    client,
                }
            })
            .collect::<Vec<_>>()
    }

    fn format_info(info: ServerInfo) -> String {
        let (icon, id, message) = match info.client {
            Some(client) => ("✓", Some(client.id()), None),
            None => ("✘", None, Some(Self::get_error_message(&info))),
        };
        let command = info.config.command.clone()
            + info
                .config
                .args
                .iter()
                .map(|s| String::from(" ") + s.as_str())
                .collect::<String>()
                .as_str();

        format!(
            r#"  {icon} {name} (id: {id})
    command: `{command}`
    {message}
"#,
            name = info.name,
            id = id.map(|id| id.to_string()).unwrap_or("none".into()),
            message = message.unwrap_or_default(),
        )
    }

    fn pretty_print_array<T: Display>(array: &[T]) -> String {
        match array.len() {
            0 => "[]".into(),
            1 => array[0].to_string(),
            _ => format!(
                "[{}]",
                array
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    fn pretty_print_formatter(formatter: Option<&FormatterConfiguration>) -> String {
        match formatter {
            Some(config) => {
                let command = config.command.clone()
                    + config
                        .args
                        .iter()
                        .map(|s| String::from(" ") + s.as_str())
                        .collect::<String>()
                        .as_str();

                format!("`{command}`")
            }
            None => "No formatter configured".into(),
        }
    }

    fn get_info(editor: &Editor) -> String {
        let doc = doc!(editor);
        if doc.language_config().is_none() {
            return r#"Press q or Esc to close this window

Document has no associated lsp configuration"#
                .to_string();
        }

        let config = doc.language_config().unwrap();
        let server_info = Self::get_server_info(editor)
            .into_iter()
            .map(Self::format_info)
            .collect::<String>();

        format!(
            r#"Press q or Esc to close this window

Detected language: {lang}
File-type(s): {ft}
Root(s): {roots}
Formatter: {formatter}

Detected clients:
{server_info}
"#,
            lang = config.language_id,
            ft = Self::pretty_print_array(&config.file_types),
            roots = Self::pretty_print_array(&config.roots),
            formatter = Self::pretty_print_formatter(config.formatter.as_ref()),
        )
    }

    fn get_error_message(info: &ServerInfo) -> String {
        if let Err(e) = which::which(&info.config.command) {
            return e.to_string();
        }

        "Unknown error, check the logs".to_string()
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
