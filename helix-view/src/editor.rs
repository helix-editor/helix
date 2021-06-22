use crate::clipboard::{get_clipboard_provider, ClipboardProvider};
use crate::{
    theme::{self, Theme},
    tree::Tree,
    Document, DocumentId, RegisterSelection, View, ViewId,
};
use helix_core::syntax;
use tui::layout::Rect;
use tui::terminal::CursorKind;

use futures_util::future;
use std::{path::PathBuf, sync::Arc, time::Duration};

use slotmap::SlotMap;

use anyhow::Error;

pub use helix_core::diagnostic::Severity;
pub use helix_core::register::Registers;
use helix_core::{Position, DEFAULT_LINE_ENDING};

#[derive(Debug)]
pub struct Editor {
    pub tree: Tree,
    pub documents: SlotMap<DocumentId, Document>,
    pub count: Option<std::num::NonZeroUsize>,
    pub selected_register: RegisterSelection,
    pub registers: Registers,
    pub theme: Theme,
    pub language_servers: helix_lsp::Registry,
    pub clipboard_provider: Box<dyn ClipboardProvider>,

    pub syn_loader: Arc<syntax::Loader>,
    pub theme_loader: Arc<theme::Loader>,

    pub status_msg: Option<(String, Severity)>,
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Replace,
    HorizontalSplit,
    VerticalSplit,
}

impl Editor {
    pub fn new(
        mut area: tui::layout::Rect,
        themes: Arc<theme::Loader>,
        config_loader: Arc<syntax::Loader>,
    ) -> Self {
        let language_servers = helix_lsp::Registry::new();

        // HAXX: offset the render area height by 1 to account for prompt/commandline
        area.height -= 1;

        Self {
            tree: Tree::new(area),
            documents: SlotMap::with_key(),
            count: None,
            selected_register: RegisterSelection::default(),
            theme: themes.default(),
            language_servers,
            syn_loader: config_loader,
            theme_loader: themes,
            registers: Registers::default(),
            clipboard_provider: get_clipboard_provider(),
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

    pub fn set_theme(&mut self, theme: Theme) {
        let scopes = theme.scopes();
        for config in self
            .syn_loader
            .language_configs_iter()
            .filter(|cfg| cfg.is_highlight_initialized())
        {
            config.reconfigure_highlight(scopes);
        }

        self.theme = theme;
        self._refresh();
    }

    pub fn set_theme_from_name(&mut self, theme: &str) {
        let theme = match self.theme_loader.load(theme.as_ref()) {
            Ok(theme) => theme,
            Err(e) => {
                log::warn!("failed setting theme `{}` - {}", theme, e);
                return;
            }
        };

        self.set_theme(theme);
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
                let view = view!(self);
                let jump = (
                    view.doc,
                    self.documents[view.doc].selection(view.id).clone(),
                );

                let view = view_mut!(self);
                view.jumps.push(jump);
                view.last_accessed_doc = Some(view.doc);
                view.doc = id;
                view.first_line = 0;

                let (view, doc) = current!(self);

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
        let doc = Document::new(Rope::from(DEFAULT_LINE_ENDING.as_str()));
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
            let mut doc = Document::load(path, Some(&self.theme), Some(&self.syn_loader))?;

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

    pub fn documents_mut(&mut self) -> impl Iterator<Item = &mut Document> {
        self.documents.iter_mut().map(|(_id, doc)| doc)
    }

    // pub fn current_document(&self) -> Document {
    //     let id = self.view().doc;
    //     let doc = &mut editor.documents[id];
    // }

    pub fn cursor(&self) -> (Option<Position>, CursorKind) {
        const OFFSET: u16 = 7; // 1 diagnostic + 5 linenr + 1 gutter
        let view = view!(self);
        let doc = &self.documents[view.doc];
        let cursor = doc.selection(view.id).cursor();
        if let Some(mut pos) = view.screen_coords_at_pos(doc, doc.text().slice(..), cursor) {
            pos.col += view.area.x as usize + OFFSET as usize;
            pos.row += view.area.y as usize;
            (Some(pos), CursorKind::Hidden)
        } else {
            (None, CursorKind::Hidden)
        }
    }

    /// Closes language servers with timeout. The default timeout is 500 ms, use
    /// `timeout` parameter to override this.
    pub async fn close_language_servers(
        &self,
        timeout: Option<u64>,
    ) -> Result<(), tokio::time::error::Elapsed> {
        tokio::time::timeout(
            Duration::from_millis(timeout.unwrap_or(500)),
            future::join_all(
                self.language_servers
                    .iter_clients()
                    .map(|client| client.force_shutdown()),
            ),
        )
        .await
        .map(|_| ())
    }
}
