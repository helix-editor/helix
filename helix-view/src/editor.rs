use crate::{
    clipboard::{get_clipboard_provider, ClipboardProvider},
    graphics::{CursorKind, Rect},
    theme::{self, Theme},
    tree::Tree,
    Document, DocumentId, View, ViewId,
};

use futures_util::future;
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use slotmap::SlotMap;

use anyhow::Error;

pub use helix_core::diagnostic::Severity;
pub use helix_core::register::Registers;
use helix_core::syntax;
use helix_core::Position;

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Complete;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Incomplete;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Config<State> {
    /// Padding to keep between the edge of the screen and the cursor when scrolling. Defaults to 5.
    scrolloff: Option<usize>,
    /// Number of lines to scroll at once. Defaults to 3
    scroll_lines: Option<isize>,
    /// Mouse support. Defaults to true.
    mouse: Option<bool>,
    /// Shell to use for shell commands. Defaults to ["cmd", "/C"] on Windows and ["sh", "-c"] otherwise.
    shell: Option<Vec<String>>,
    /// Line number mode.
    line_number: Option<LineNumber>,
    /// Middle click paste support. Defaults to true
    middle_click_paste: Option<bool>,
    /// Smart case: Option<Case insensitive searching unless pattern contains upper case characters. Defaults to true.
    smart_case: Option<bool>,
    /// Automatic insertion of pairs to parentheses, brackets, etc. Defaults to true.
    auto_pairs: Option<bool>,

    #[serde(skip)]
    state: PhantomData<State>,
}

impl<T> std::fmt::Debug for Config<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("scrolloff", &self.scrolloff)
            .field("scroll_lines", &self.scroll_lines)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineNumber {
    /// Show absolute line number
    Absolute,

    /// Show relative line number to the primary cursor
    Relative,
}

impl<T> Default for Config<T> {
    fn default() -> Self {
        Self {
            scrolloff: Some(5),
            scroll_lines: Some(3),
            mouse: Some(true),
            shell: if cfg!(windows) {
                Some(vec!["cmd".to_owned(), "/C".to_owned()])
            } else {
                Some(vec!["sh".to_owned(), "-c".to_owned()])
            },
            line_number: Some(LineNumber::Absolute),
            middle_click_paste: Some(true),
            smart_case: Some(true),
            auto_pairs: Some(true),

            state: Default::default(),
        }
    }
}

/// A complete config means all of it's fields must contain something.
/// This allow us to unwrap all of its fields safely.
impl Config<Complete> {
    pub fn scrolloff(&self) -> &usize {
        self.scrolloff.as_ref().unwrap()
    }
    pub fn scrolloff_mut(&mut self) -> &mut usize {
        self.scrolloff.as_mut().unwrap()
    }

    pub fn scroll_lines(&self) -> &isize {
        self.scroll_lines.as_ref().unwrap()
    }
    pub fn scroll_lines_mut(&mut self) -> &mut isize {
        self.scroll_lines.as_mut().unwrap()
    }

    pub fn mouse(&self) -> &bool {
        self.mouse.as_ref().unwrap()
    }
    pub fn mouse_mut(&mut self) -> &mut bool {
        self.mouse.as_mut().unwrap()
    }

    pub fn shell(&self) -> &Vec<String> {
        self.shell.as_ref().unwrap()
    }
    pub fn shell_mut(&mut self) -> &mut Vec<String> {
        self.shell.as_mut().unwrap()
    }

    pub fn line_number(&self) -> &LineNumber {
        self.line_number.as_ref().unwrap()
    }
    pub fn line_number_mut(&mut self) -> &mut LineNumber {
        self.line_number.as_mut().unwrap()
    }

    pub fn middle_click_paste(&self) -> &bool {
        self.middle_click_paste.as_ref().unwrap()
    }
    pub fn middle_click_paste_mut(&mut self) -> &mut bool {
        self.middle_click_paste.as_mut().unwrap()
    }

    pub fn smart_case(&self) -> &bool {
        self.smart_case.as_ref().unwrap()
    }
    pub fn smart_case_mut(&mut self) -> &mut bool {
        self.smart_case.as_mut().unwrap()
    }

    pub fn auto_pairs(&self) -> bool {
        self.auto_pairs.unwrap()
    }
    pub fn auto_pairs_mut(&mut self) -> &mut bool {
        self.auto_pairs.as_mut().unwrap()
    }

    pub fn apply_incomplete_config(self, config: Config<Incomplete>) -> Config<Complete> {
        Self {
            scrolloff: Some(*config.scrolloff().unwrap_or(self.scrolloff())),
            scroll_lines: Some(*config.scroll_lines().unwrap_or(self.scroll_lines())),
            mouse: Some(*config.mouse().unwrap_or(self.mouse())),
            shell: Some(config.shell().unwrap_or(self.shell()).clone()),
            line_number: Some(config.line_number().unwrap_or(self.line_number()).clone()),
            middle_click_paste: Some(
                *config
                    .middle_click_paste()
                    .unwrap_or(self.middle_click_paste()),
            ),
            smart_case: Some(*config.smart_case().unwrap_or(self.smart_case())),
            auto_pairs: Some(*config.auto_pairs().unwrap_or(&self.auto_pairs())),
            state: Default::default(),
        }
    }
}

/// No assumption are made on a `Incomplete` config, we may be missing some fields so all getter returns `Option`.
impl Config<Incomplete> {
    pub fn scrolloff(&self) -> Option<&usize> {
        self.scrolloff.as_ref()
    }
    pub fn scrolloff_mut(&mut self) -> Option<&mut usize> {
        self.scrolloff.as_mut()
    }

    pub fn scroll_lines(&self) -> Option<&isize> {
        self.scroll_lines.as_ref()
    }
    pub fn scroll_lines_mut(&mut self) -> Option<&mut isize> {
        self.scroll_lines.as_mut()
    }

    pub fn mouse(&self) -> Option<&bool> {
        self.mouse.as_ref()
    }
    pub fn mouse_mut(&mut self) -> Option<&mut bool> {
        self.mouse.as_mut()
    }

    pub fn shell(&self) -> Option<&Vec<String>> {
        self.shell.as_ref()
    }
    pub fn shell_mut(&mut self) -> Option<&mut Vec<String>> {
        self.shell.as_mut()
    }

    pub fn line_number(&self) -> Option<&LineNumber> {
        self.line_number.as_ref()
    }
    pub fn line_number_mut(&mut self) -> Option<&mut LineNumber> {
        self.line_number.as_mut()
    }

    pub fn middle_click_paste(&self) -> Option<&bool> {
        self.middle_click_paste.as_ref()
    }
    pub fn middle_click_paste_mut(&mut self) -> Option<&mut bool> {
        self.middle_click_paste.as_mut()
    }

    pub fn smart_case(&self) -> Option<&bool> {
        self.smart_case.as_ref()
    }
    pub fn smart_case_mut(&mut self) -> Option<&mut bool> {
        self.smart_case.as_mut()
    }

    pub fn auto_pairs(&self) -> Option<&bool> {
        self.auto_pairs.as_ref()
    }
    pub fn auto_pairs_mut(&mut self) -> Option<&mut bool> {
        self.auto_pairs.as_mut()
    }
}

#[derive(Debug)]
pub struct Editor {
    pub tree: Tree,
    pub documents: SlotMap<DocumentId, Document>,
    pub count: Option<std::num::NonZeroUsize>,
    pub selected_register: Option<char>,
    pub registers: Registers,
    pub theme: Theme,
    pub language_servers: helix_lsp::Registry,
    pub clipboard_provider: Box<dyn ClipboardProvider>,

    pub syn_loader: Arc<syntax::Loader>,
    pub theme_loader: Arc<theme::Loader>,

    pub status_msg: Option<(String, Severity)>,

    pub config: Config<Complete>,
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Load,
    Replace,
    HorizontalSplit,
    VerticalSplit,
}

impl Editor {
    pub fn new(
        mut area: Rect,
        themes: Arc<theme::Loader>,
        config_loader: Arc<syntax::Loader>,
        config: Config<Complete>,
    ) -> Self {
        let language_servers = helix_lsp::Registry::new();

        // HAXX: offset the render area height by 1 to account for prompt/commandline
        area.height -= 1;

        Self {
            tree: Tree::new(area),
            documents: SlotMap::with_key(),
            count: None,
            selected_register: None,
            theme: themes.default(),
            language_servers,
            syn_loader: config_loader,
            theme_loader: themes,
            registers: Registers::default(),
            clipboard_provider: get_clipboard_provider(),
            status_msg: None,
            config,
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
            config.reconfigure(scopes);
        }

        self.theme = theme;
        self._refresh();
    }

    pub fn set_theme_from_name(&mut self, theme: &str) -> anyhow::Result<()> {
        use anyhow::Context;
        let theme = self
            .theme_loader
            .load(theme.as_ref())
            .with_context(|| format!("failed setting theme `{}`", theme))?;
        self.set_theme(theme);
        Ok(())
    }

    fn _refresh(&mut self) {
        for (view, _) in self.tree.views_mut() {
            let doc = &self.documents[view.doc];
            view.ensure_cursor_in_view(doc, *self.config.scrolloff())
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
                view.offset = Position::default();

                let (view, doc) = current!(self);

                // initialize selection for view
                doc.selections
                    .entry(view.id)
                    .or_insert_with(|| Selection::point(0));
                // TODO: reuse align_view
                let pos = doc
                    .selection(view.id)
                    .primary()
                    .cursor(doc.text().slice(..));
                let line = doc.text().char_to_line(pos);
                view.offset.row = line.saturating_sub(view.inner_area().height as usize / 2);

                return;
            }
            Action::Load => {
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
        let doc = Document::default();
        let id = self.documents.insert(doc);
        self.documents[id].id = id;
        self.switch(id, action);
        id
    }

    pub fn open(&mut self, path: PathBuf, action: Action) -> Result<DocumentId, Error> {
        let path = helix_core::path::get_canonicalized_path(&path)?;

        let id = self
            .documents()
            .find(|doc| doc.path() == Some(&path))
            .map(|doc| doc.id);

        let id = if let Some(id) = id {
            id
        } else {
            let mut doc = Document::open(&path, None, Some(&self.theme), Some(&self.syn_loader))?;

            // try to find a language server based on the language name
            let language_server = doc.language.as_ref().and_then(|language| {
                self.language_servers
                    .get(language)
                    .map_err(|e| {
                        log::error!("Failed to get LSP, {}, for `{}`", e, language.scope())
                    })
                    .ok()
            });

            if let Some(language_server) = language_server {
                let language_id = doc
                    .language()
                    .and_then(|s| s.split('.').last()) // source.rust
                    .map(ToOwned::to_owned)
                    .unwrap_or_default();

                // TODO: this now races with on_init code if the init happens too quickly
                tokio::spawn(language_server.text_document_did_open(
                    doc.url().unwrap(),
                    doc.version(),
                    doc.text(),
                    language_id,
                ));

                doc.set_language_server(Some(language_server));
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
            let doc = &self.documents[view.doc];

            if let Some(language_server) = doc.language_server() {
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
        view.ensure_cursor_in_view(doc, *self.config.scrolloff())
    }

    #[inline]
    pub fn document(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(id)
    }

    #[inline]
    pub fn document_mut(&mut self, id: DocumentId) -> Option<&mut Document> {
        self.documents.get_mut(id)
    }

    #[inline]
    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }

    #[inline]
    pub fn documents_mut(&mut self) -> impl Iterator<Item = &mut Document> {
        self.documents.values_mut()
    }

    pub fn document_by_path<P: AsRef<Path>>(&self, path: P) -> Option<&Document> {
        self.documents()
            .find(|doc| doc.path().map(|p| p == path.as_ref()).unwrap_or(false))
    }

    pub fn document_by_path_mut<P: AsRef<Path>>(&mut self, path: P) -> Option<&mut Document> {
        self.documents_mut()
            .find(|doc| doc.path().map(|p| p == path.as_ref()).unwrap_or(false))
    }

    pub fn cursor(&self) -> (Option<Position>, CursorKind) {
        let view = view!(self);
        let doc = &self.documents[view.doc];
        let cursor = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));
        if let Some(mut pos) = view.screen_coords_at_pos(doc, doc.text().slice(..), cursor) {
            let inner = view.inner_area();
            pos.col += inner.x as usize;
            pos.row += inner.y as usize;
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
