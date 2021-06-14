use crate::{theme::Theme, tree::Tree, Document, DocumentId, RegisterSelection, View, ViewId};
use tui::layout::Rect;

use std::path::PathBuf;

use slotmap::SlotMap;

use anyhow::Error;

pub use helix_core::diagnostic::Severity;

#[derive(Debug)]
pub struct Editor {
    pub tree: Tree,
    pub documents: SlotMap<DocumentId, Document>,
    pub count: Option<std::num::NonZeroUsize>,
    pub register: RegisterSelection,
    pub theme: Theme,
    pub language_servers: helix_lsp::Registry,

    pub status_msg: Option<(String, Severity)>,
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Replace,
    HorizontalSplit,
    VerticalSplit,
}

impl Editor {
    pub fn new(mut area: tui::layout::Rect) -> Self {
        use helix_core::config_dir;
        let config = std::fs::read(config_dir().join("theme.toml"));
        // load $HOME/.config/helix/theme.toml, fallback to default config
        let toml = config
            .as_deref()
            .unwrap_or(include_bytes!("../../theme.toml"));
        let theme: Theme = toml::from_slice(toml).expect("failed to parse theme.toml");

        // initialize language registry
        use helix_core::syntax::{Loader, LOADER};

        // load $HOME/.config/helix/languages.toml, fallback to default config
        let config = std::fs::read(helix_core::config_dir().join("languages.toml"));
        let toml = config
            .as_deref()
            .unwrap_or(include_bytes!("../../languages.toml"));

        let config = toml::from_slice(toml).expect("Could not parse languages.toml");
        LOADER.get_or_init(|| Loader::new(config, theme.scopes().to_vec()));

        let language_servers = helix_lsp::Registry::new();

        // HAXX: offset the render area height by 1 to account for prompt/commandline
        area.height -= 1;

        Self {
            tree: Tree::new(area),
            documents: SlotMap::with_key(),
            count: None,
            register: RegisterSelection::default(),
            theme,
            language_servers,
            status_msg: None,
        }
    }

    pub fn clear_status(&mut self) {
        self.status_msg = None;
    }

    pub fn set_status(&mut self, status: String) {
        self.status_msg = Some((status, Severity::Info));
    }

    pub fn set_error(&mut self, error: String) {
        self.status_msg = Some((error, Severity::Error));
    }

    fn _refresh(&mut self) {
        for (view, _) in self.tree.views_mut() {
            let doc = &self.documents[view.doc];
            view.ensure_cursor_in_view(doc)
        }
    }

    pub fn switch(&mut self, id: DocumentId, action: Action) {
        use crate::tree::Layout;
        use helix_core::Selection;

        if !self.documents.contains_key(id) {
            log::error!("cannot switch to document that does not exist (anymore)");
            return;
        }

        match action {
            Action::Replace => {
                let view = self.view();
                let jump = (
                    view.doc,
                    self.documents[view.doc].selection(view.id).clone(),
                );

                let view = self.view_mut();
                view.jumps.push(jump);
                view.last_accessed_doc = Some(view.doc);
                view.doc = id;
                view.first_line = 0;

                let (view, doc) = self.current();

                // initialize selection for view
                let selection = doc
                    .selections
                    .entry(view.id)
                    .or_insert_with(|| Selection::point(0));
                // TODO: reuse align_view
                let pos = selection.cursor();
                let line = doc.text().char_to_line(pos);
                view.first_line = line.saturating_sub(view.area.height as usize / 2);

                return;
            }
            Action::HorizontalSplit => {
                let view = View::new(id);
                let view_id = self.tree.split(view, Layout::Horizontal);
                // initialize selection for view
                let doc = &mut self.documents[id];
                doc.selections.insert(view_id, Selection::point(0));
            }
            Action::VerticalSplit => {
                let view = View::new(id);
                let view_id = self.tree.split(view, Layout::Vertical);
                // initialize selection for view
                let doc = &mut self.documents[id];
                doc.selections.insert(view_id, Selection::point(0));
            }
        }

        self._refresh();
    }

    pub fn new_file(&mut self, action: Action) -> DocumentId {
        use helix_core::Rope;
        let doc = Document::new(Rope::from("\n"));
        let id = self.documents.insert(doc);
        self.documents[id].id = id;
        self.switch(id, action);
        id
    }

    pub fn open(&mut self, path: PathBuf, action: Action) -> Result<DocumentId, Error> {
        let path = crate::document::canonicalize_path(&path)?;

        let id = self
            .documents()
            .find(|doc| doc.path() == Some(&path))
            .map(|doc| doc.id);

        let id = if let Some(id) = id {
            id
        } else {
            let mut doc = Document::load(path)?;

            // try to find a language server based on the language name
            let language_server = doc
                .language
                .as_ref()
                .and_then(|language| self.language_servers.get(language).ok());

            if let Some(language_server) = language_server {
                doc.set_language_server(Some(language_server.clone()));

                let language_id = doc
                    .language()
                    .and_then(|s| s.split('.').last()) // source.rust
                    .map(ToOwned::to_owned)
                    .unwrap_or_default();

                tokio::spawn(language_server.text_document_did_open(
                    doc.url().unwrap(),
                    doc.version(),
                    doc.text(),
                    language_id,
                ));
            }

            let id = self.documents.insert(doc);
            self.documents[id].id = id;
            id
        };

        self.switch(id, action);
        Ok(id)
    }

    pub fn close(&mut self, id: ViewId, close_buffer: bool) {
        let view = self.tree.get(self.tree.focus);
        // remove selection
        self.documents[view.doc].selections.remove(&id);

        if close_buffer {
            // get around borrowck issues
            let language_servers = &mut self.language_servers;
            let doc = &self.documents[view.doc];

            let language_server = doc
                .language
                .as_ref()
                .and_then(|language| language_servers.get(language).ok());
            if let Some(language_server) = language_server {
                tokio::spawn(language_server.text_document_did_close(doc.identifier()));
            }
            self.documents.remove(view.doc);
        }

        self.tree.remove(id);
        self._refresh();
    }

    pub fn resize(&mut self, area: Rect) {
        if self.tree.resize(area) {
            self._refresh();
        };
    }

    pub fn focus_next(&mut self) {
        self.tree.focus_next();
    }

    pub fn should_close(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn current(&mut self) -> (&mut View, &mut Document) {
        let view = self.tree.get_mut(self.tree.focus);
        let doc = &mut self.documents[view.doc];
        (view, doc)
    }

    pub fn view(&self) -> &View {
        self.tree.get(self.tree.focus)
    }

    pub fn view_mut(&mut self) -> &mut View {
        self.tree.get_mut(self.tree.focus)
    }

    pub fn ensure_cursor_in_view(&mut self, id: ViewId) {
        let view = self.tree.get_mut(id);
        let doc = &self.documents[view.doc];
        view.ensure_cursor_in_view(doc)
    }

    pub fn document(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(id)
    }

    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.iter().map(|(_id, doc)| doc)
    }

    // pub fn current_document(&self) -> Document {
    //     let id = self.view().doc;
    //     let doc = &mut editor.documents[id];
    // }

    pub fn cursor_position(&self) -> Option<helix_core::Position> {
        const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter
        let view = self.view();
        let doc = &self.documents[view.doc];
        let cursor = doc.selection(view.id).cursor();
        if let Some(mut pos) = view.screen_coords_at_pos(doc, doc.text().slice(..), cursor) {
            pos.col += view.area.x as usize + OFFSET as usize;
            pos.row += view.area.y as usize;
            return Some(pos);
        }
        None
    }
}
