use crate::theme::Theme;
use crate::tree::Tree;
use crate::{Document, View};

use std::path::PathBuf;

use slotmap::DefaultKey as Key;

use anyhow::Error;

pub struct Editor {
    pub tree: Tree,
    // pub documents: Vec<Document>,
    pub count: Option<usize>,
    pub theme: Theme,
    pub language_servers: helix_lsp::Registry,
}

impl Editor {
    pub fn new(mut area: tui::layout::Rect) -> Self {
        let theme = Theme::default();
        let language_servers = helix_lsp::Registry::new();

        // HAXX: offset the render area height by 1 to account for prompt/commandline
        area.height -= 1;

        Self {
            tree: Tree::new(area),
            count: None,
            theme,
            language_servers,
        }
    }

    pub fn open(&mut self, path: PathBuf, executor: &smol::Executor) -> Result<(), Error> {
        // TODO: try to find an open view/buffer first

        let mut doc = Document::load(path, self.theme.scopes())?;

        // try to find a language server based on the language name
        let language_server = doc
            .language
            .as_ref()
            .and_then(|language| self.language_servers.get(language, &executor));

        if let Some(language_server) = language_server {
            doc.set_language_server(Some(language_server.clone()));

            let language_id = doc
                .language()
                .and_then(|s| s.split(".").last()) // source.rust
                .map(ToOwned::to_owned)
                .unwrap_or_default();

            smol::block_on(language_server.text_document_did_open(
                doc.url().unwrap(),
                doc.version(),
                doc.text(),
                language_id,
            ))
            .unwrap();
        }

        let view = View::new(doc)?;
        self.tree.insert(view);
        Ok(())
    }

    pub fn close(&mut self, id: Key) {
        self.tree.remove(id)
    }

    pub fn should_close(&mut self) -> bool {
        self.tree.is_empty()
    }

    pub fn view(&self) -> &View {
        self.tree.get(self.tree.focus)
    }

    pub fn view_mut(&mut self) -> &mut View {
        self.tree.get_mut(self.tree.focus)
    }

    pub fn cursor_position(&self) -> Option<helix_core::Position> {
        const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter
        let view = self.view();
        let cursor = view.doc.selection().cursor();
        if let Some(mut pos) = view.screen_coords_at_pos(view.doc.text().slice(..), cursor) {
            pos.col += view.area.x as usize + OFFSET as usize;
            pos.row += view.area.y as usize;
            return Some(pos);
        }
        None
    }
}
