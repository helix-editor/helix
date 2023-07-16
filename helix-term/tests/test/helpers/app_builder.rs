use crossterm::event::Event;
use helix_term::{application::Application, args::Args, config::Config, keymap::merge_keys};
use helix_view::editor::LspConfig;
use std::{mem::replace, path::PathBuf};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::test::backend::TestBackend;

use super::TestApplication;

pub struct AppBuilder {
    args: Args,
    config: Config,
    raw_lang_overrides: Option<String>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            args: Args::default(),
            config: Config {
                editor: helix_view::editor::Config {
                    lsp: LspConfig {
                        enable: false,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                keys: helix_term::keymap::default(),
                ..Default::default()
            },
            raw_lang_overrides: None,
        }
    }
}

impl AppBuilder {
    pub fn with_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.args.files.push((path.into(), Default::default()));
        self
    }

    pub fn with_config(mut self, mut config: Config) -> Self {
        let keys = replace(&mut config.keys, helix_term::keymap::default());
        merge_keys(&mut config.keys, keys);
        self.config = config;
        self
    }

    /// Raw TOML string.
    pub fn lang_config_overrides(mut self, raw: String) -> Self {
        self.raw_lang_overrides = Some(raw);
        self
    }

    pub fn build(
        self,
    ) -> anyhow::Result<(
        TestApplication,
        UnboundedSender<Result<Event, std::io::Error>>,
    )> {
        // Unwrap will be error error if logging system has been
        // initialized by another test.
        let _ = helix_term::log::setup_logging(std::io::stdout(), None);

        let mut language_config = helix_loader::config::default_lang_config();
        if let Some(overrides) = self.raw_lang_overrides {
            language_config =
                helix_loader::merge_toml_values(language_config, toml::from_str(&overrides)?, 3);
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let rx_stream = UnboundedReceiverStream::new(rx);

        Ok((
            TestApplication(Application::<TestBackend>::new(
                TestBackend::new(120, 150, rx_stream),
                self.args,
                self.config,
                language_config.try_into()?,
            )?),
            tx,
        ))
    }
}
