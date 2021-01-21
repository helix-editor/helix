use crate::theme::Theme;
use crate::{Document, View};

use std::path::PathBuf;

use anyhow::Error;

pub struct Editor {
    pub views: Vec<View>,
    pub focus: usize,
    pub should_close: bool,
    pub theme: Theme, // TODO: share one instance
    pub language_servers: helix_lsp::Registry,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        let theme = Theme::default();
        let language_servers = helix_lsp::Registry::new();

        Self {
            views: Vec::new(),
            focus: 0,
            should_close: false,
            theme,
            language_servers,
        }
    }

    pub fn open(
        &mut self,
        path: PathBuf,
        size: (u16, u16),
        executor: &smol::Executor,
    ) -> Result<(), Error> {
        let pos = self.views.len();
        let mut doc = Document::load(path, self.theme.scopes())?;

        // try to find a language server based on the language name
        let language_server = doc
            .language
            .as_ref()
            .and_then(|language| self.language_servers.get(&language, &executor));

        if let Some(language_server) = language_server {
            // TODO: do this everywhere
            doc.set_language_server(Some(language_server.clone()));

            smol::block_on(language_server.text_document_did_open(
                doc.url().unwrap(),
                doc.version,
                doc.text(),
            ))
            .unwrap();
        }

        self.views.push(View::new(doc, size)?);
        self.focus = pos;
        Ok(())
    }

    pub fn view(&self) -> &View {
        self.views.get(self.focus).unwrap()
    }

    pub fn view_mut(&mut self) -> &mut View {
        self.views.get_mut(self.focus).unwrap()
    }
}
