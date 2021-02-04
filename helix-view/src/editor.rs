use crate::theme::Theme;
use crate::tree::Tree;
use crate::{Document, View};
use slotmap::DefaultKey as Key;

use std::path::PathBuf;

use anyhow::Error;

pub struct Editor {
    pub tree: Tree,
    // pub documents: Vec<Document>,
    pub should_close: bool,
    pub theme: Theme, // TODO: share one instance
    pub language_servers: helix_lsp::Registry,
}

impl Editor {
    pub fn new(area: tui::layout::Rect) -> Self {
        let theme = Theme::default();
        let language_servers = helix_lsp::Registry::new();

        Self {
            tree: Tree::new(area),
            should_close: false,
            theme,
            language_servers,
        }
    }

    pub fn open(&mut self, path: PathBuf, executor: &smol::Executor) -> Result<(), Error> {
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

        let view = View::new(doc)?;
        self.tree.insert(view);
        Ok(())
    }

    pub fn view(&self) -> &View {
        self.tree.get(self.tree.focus)
    }

    pub fn view_mut(&mut self) -> &mut View {
        self.tree.get_mut(self.tree.focus)
    }
}
