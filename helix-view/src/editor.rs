use crate::{
    clipboard::{get_clipboard_provider, ClipboardProvider},
    document::{Mode, SCRATCH_BUFFER_NAME},
    graphics::{CursorKind, Rect},
    info::Info,
    input::KeyEvent,
    theme::{self, Theme},
    tree::{self, Tree},
    Document, DocumentId, View, ViewId,
};

use futures_util::future;
use futures_util::stream::select_all::SelectAll;
use tokio_stream::wrappers::UnboundedReceiverStream;

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    io::stdin,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::{sleep, Duration, Instant, Sleep},
};

use anyhow::{bail, Error};

pub use helix_core::diagnostic::Severity;
pub use helix_core::register::Registers;
use helix_core::{
    auto_pairs::AutoPairs,
    syntax::{self, AutoPairConfig},
    Change,
};
use helix_core::{Position, Selection};
use helix_dap as dap;

use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};

use arc_swap::access::{DynAccess, DynGuard};

fn deserialize_duration_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}

fn serialize_duration_millis<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(
        duration
            .as_millis()
            .try_into()
            .map_err(|_| serde::ser::Error::custom("duration value overflowed u64"))?,
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct FilePickerConfig {
    /// IgnoreOptions
    /// Enables ignoring hidden files.
    /// Whether to hide hidden files in file picker and global search results. Defaults to true.
    pub hidden: bool,
    /// Enables reading ignore files from parent directories. Defaults to true.
    pub parents: bool,
    /// Enables reading `.ignore` files.
    /// Whether to hide files listed in .ignore in file picker and global search results. Defaults to true.
    pub ignore: bool,
    /// Enables reading `.gitignore` files.
    /// Whether to hide files listed in .gitignore in file picker and global search results. Defaults to true.
    pub git_ignore: bool,
    /// Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option.
    /// Whether to hide files listed in global .gitignore in file picker and global search results. Defaults to true.
    pub git_global: bool,
    /// Enables reading `.git/info/exclude` files.
    /// Whether to hide files listed in .git/info/exclude in file picker and global search results. Defaults to true.
    pub git_exclude: bool,
    /// WalkBuilder options
    /// Maximum Depth to recurse directories in file picker and global search. Defaults to `None`.
    pub max_depth: Option<usize>,
}

impl Default for FilePickerConfig {
    fn default() -> Self {
        Self {
            hidden: true,
            parents: true,
            ignore: true,
            git_ignore: true,
            git_global: true,
            git_exclude: true,
            max_depth: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    /// Padding to keep between the edge of the screen and the cursor when scrolling. Defaults to 5.
    pub scrolloff: usize,
    /// Number of lines to scroll at once. Defaults to 3
    pub scroll_lines: isize,
    /// Mouse support. Defaults to true.
    pub mouse: bool,
    /// Shell to use for shell commands. Defaults to ["cmd", "/C"] on Windows and ["sh", "-c"] otherwise.
    pub shell: Vec<String>,
    /// Line number mode.
    pub line_number: LineNumber,
    /// Middle click paste support. Defaults to true.
    pub middle_click_paste: bool,
    /// Automatic insertion of pairs to parentheses, brackets,
    /// etc. Optionally, this can be a list of 2-tuples to specify a
    /// global list of characters to pair. Defaults to true.
    pub auto_pairs: AutoPairConfig,
    /// Automatic auto-completion, automatically pop up without user trigger. Defaults to true.
    pub auto_completion: bool,
    /// Time in milliseconds since last keypress before idle timers trigger.
    /// Used for autocompletion, set to 0 for instant. Defaults to 400ms.
    #[serde(
        serialize_with = "serialize_duration_millis",
        deserialize_with = "deserialize_duration_millis"
    )]
    pub idle_timeout: Duration,
    pub completion_trigger_len: u8,
    /// Whether to display infoboxes. Defaults to true.
    pub auto_info: bool,
    pub file_picker: FilePickerConfig,
    /// Shape for cursor in each mode
    pub cursor_shape: CursorShapeConfig,
    /// Set to `true` to override automatic detection of terminal truecolor support in the event of a false negative. Defaults to `false`.
    pub true_color: bool,
    /// Search configuration.
    #[serde(default)]
    pub search: SearchConfig,
    pub lsp: LspConfig,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LspConfig {
    pub display_messages: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct SearchConfig {
    /// Smart case: Case insensitive searching unless pattern contains upper case characters. Defaults to true.
    pub smart_case: bool,
    /// Whether the search should wrap after depleting the matches. Default to true.
    pub wrap_around: bool,
}

// Cursor shape is read and used on every rendered frame and so needs
// to be fast. Therefore we avoid a hashmap and use an enum indexed array.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorShapeConfig([CursorKind; 3]);

impl CursorShapeConfig {
    pub fn from_mode(&self, mode: Mode) -> CursorKind {
        self.get(mode as usize).copied().unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for CursorShapeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let m = HashMap::<Mode, CursorKind>::deserialize(deserializer)?;
        let into_cursor = |mode: Mode| m.get(&mode).copied().unwrap_or_default();
        Ok(CursorShapeConfig([
            into_cursor(Mode::Normal),
            into_cursor(Mode::Select),
            into_cursor(Mode::Insert),
        ]))
    }
}

impl Serialize for CursorShapeConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        let modes = [Mode::Normal, Mode::Select, Mode::Insert];
        for mode in modes {
            map.serialize_entry(&mode, &self.from_mode(mode))?;
        }
        map.end()
    }
}

impl std::ops::Deref for CursorShapeConfig {
    type Target = [CursorKind; 3];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for CursorShapeConfig {
    fn default() -> Self {
        Self([CursorKind::Block; 3])
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineNumber {
    /// Show absolute line number
    Absolute,

    /// If focused and in normal/select mode, show relative line number to the primary cursor.
    /// If unfocused or in insert mode, show absolute line number.
    Relative,
}

impl std::str::FromStr for LineNumber {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "absolute" | "abs" => Ok(Self::Absolute),
            "relative" | "rel" => Ok(Self::Relative),
            _ => anyhow::bail!("Line number can only be `absolute` or `relative`."),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scrolloff: 5,
            scroll_lines: 3,
            mouse: true,
            shell: if cfg!(windows) {
                vec!["cmd".to_owned(), "/C".to_owned()]
            } else {
                vec!["sh".to_owned(), "-c".to_owned()]
            },
            line_number: LineNumber::Absolute,
            middle_click_paste: true,
            auto_pairs: AutoPairConfig::default(),
            auto_completion: true,
            idle_timeout: Duration::from_millis(400),
            completion_trigger_len: 2,
            auto_info: true,
            file_picker: FilePickerConfig::default(),
            cursor_shape: CursorShapeConfig::default(),
            true_color: false,
            search: SearchConfig::default(),
            lsp: LspConfig::default(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            wrap_around: true,
            smart_case: true,
        }
    }
}

pub struct Motion(pub Box<dyn Fn(&mut Editor)>);
impl Motion {
    pub fn run(&self, e: &mut Editor) {
        (self.0)(e)
    }
}
impl std::fmt::Debug for Motion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("motion")
    }
}

#[derive(Debug, Clone, Default)]
pub struct Breakpoint {
    pub id: Option<usize>,
    pub verified: bool,
    pub message: Option<String>,

    pub line: usize,
    pub column: Option<usize>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

pub struct Editor {
    pub tree: Tree,
    pub next_document_id: DocumentId,
    pub documents: BTreeMap<DocumentId, Document>,
    pub count: Option<std::num::NonZeroUsize>,
    pub selected_register: Option<char>,
    pub registers: Registers,
    pub macro_recording: Option<(char, Vec<KeyEvent>)>,
    pub theme: Theme,
    pub language_servers: helix_lsp::Registry,

    pub debugger: Option<dap::Client>,
    pub debugger_events: SelectAll<UnboundedReceiverStream<dap::Payload>>,
    pub breakpoints: HashMap<PathBuf, Vec<Breakpoint>>,

    pub clipboard_provider: Box<dyn ClipboardProvider>,

    pub syn_loader: Arc<syntax::Loader>,
    pub theme_loader: Arc<theme::Loader>,

    pub status_msg: Option<(Cow<'static, str>, Severity)>,
    pub autoinfo: Option<Info>,

    pub config: Box<dyn DynAccess<Config>>,
    pub auto_pairs: Option<AutoPairs>,

    pub idle_timer: Pin<Box<Sleep>>,
    pub last_motion: Option<Motion>,
    pub pseudo_pending: Option<String>,

    pub last_completion: Option<CompleteAction>,

    pub exit_code: i32,

    pub config_events: (UnboundedSender<ConfigEvent>, UnboundedReceiver<ConfigEvent>),
}

#[derive(Debug, Clone)]
pub enum ConfigEvent {
    Refresh,
    Update(Config),
}

#[derive(Debug, Clone)]
pub struct CompleteAction {
    pub trigger_offset: usize,
    pub changes: Vec<Change>,
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
        theme_loader: Arc<theme::Loader>,
        syn_loader: Arc<syntax::Loader>,
        config: Box<dyn DynAccess<Config>>,
    ) -> Self {
        let language_servers = helix_lsp::Registry::new();
        let conf = config.load();
        let auto_pairs = (&conf.auto_pairs).into();

        // HAXX: offset the render area height by 1 to account for prompt/commandline
        area.height -= 1;

        Self {
            tree: Tree::new(area),
            next_document_id: DocumentId::default(),
            documents: BTreeMap::new(),
            count: None,
            selected_register: None,
            macro_recording: None,
            theme: theme_loader.default(),
            language_servers,
            debugger: None,
            debugger_events: SelectAll::new(),
            breakpoints: HashMap::new(),
            syn_loader,
            theme_loader,
            registers: Registers::default(),
            clipboard_provider: get_clipboard_provider(),
            status_msg: None,
            autoinfo: None,
            idle_timer: Box::pin(sleep(conf.idle_timeout)),
            last_motion: None,
            last_completion: None,
            pseudo_pending: None,
            config,
            auto_pairs,
            exit_code: 0,
            config_events: unbounded_channel(),
        }
    }

    pub fn config(&self) -> DynGuard<Config> {
        self.config.load()
    }

    pub fn clear_idle_timer(&mut self) {
        // equivalent to internal Instant::far_future() (30 years)
        self.idle_timer
            .as_mut()
            .reset(Instant::now() + Duration::from_secs(86400 * 365 * 30));
    }

    pub fn reset_idle_timer(&mut self) {
        let config = self.config();
        self.idle_timer
            .as_mut()
            .reset(Instant::now() + config.idle_timeout);
    }

    pub fn clear_status(&mut self) {
        self.status_msg = None;
    }

    #[inline]
    pub fn set_status<T: Into<Cow<'static, str>>>(&mut self, status: T) {
        self.status_msg = Some((status.into(), Severity::Info));
    }

    #[inline]
    pub fn set_error<T: Into<Cow<'static, str>>>(&mut self, error: T) {
        self.status_msg = Some((error.into(), Severity::Error));
    }

    pub fn set_theme(&mut self, theme: Theme) {
        // `ui.selection` is the only scope required to be able to render a theme.
        if theme.find_scope_index("ui.selection").is_none() {
            self.set_error("Invalid theme: `ui.selection` required");
            return;
        }

        let scopes = theme.scopes();
        self.syn_loader.set_scopes(scopes.to_vec());

        self.theme = theme;
        self._refresh();
    }

    /// Refreshes the language server for a given document
    pub fn refresh_language_server(&mut self, doc_id: DocumentId) -> Option<()> {
        let doc = self.documents.get_mut(&doc_id)?;
        Self::launch_language_server(&mut self.language_servers, doc)
    }

    /// Launch a language server for a given document
    fn launch_language_server(ls: &mut helix_lsp::Registry, doc: &mut Document) -> Option<()> {
        // if doc doesn't have a URL it's a scratch buffer, ignore it
        let doc_url = doc.url()?;

        // try to find a language server based on the language name
        let language_server = doc.language.as_ref().and_then(|language| {
            ls.get(language)
                .map_err(|e| {
                    log::error!(
                        "Failed to initialize the LSP for `{}` {{ {} }}",
                        language.scope(),
                        e
                    )
                })
                .ok()
        });
        if let Some(language_server) = language_server {
            // only spawn a new lang server if the servers aren't the same
            if Some(language_server.id()) != doc.language_server().map(|server| server.id()) {
                if let Some(language_server) = doc.language_server() {
                    tokio::spawn(language_server.text_document_did_close(doc.identifier()));
                }

                let language_id = doc.language_id().map(ToOwned::to_owned).unwrap_or_default();

                // TODO: this now races with on_init code if the init happens too quickly
                tokio::spawn(language_server.text_document_did_open(
                    doc_url,
                    doc.version(),
                    doc.text(),
                    language_id,
                ));

                doc.set_language_server(Some(language_server));
            }
        }
        Some(())
    }

    fn _refresh(&mut self) {
        let config = self.config();
        for (view, _) in self.tree.views_mut() {
            let doc = &self.documents[&view.doc];
            view.ensure_cursor_in_view(doc, config.scrolloff)
        }
    }

    fn replace_document_in_view(&mut self, current_view: ViewId, doc_id: DocumentId) {
        let view = self.tree.get_mut(current_view);
        view.doc = doc_id;
        view.offset = Position::default();

        let doc = self.documents.get_mut(&doc_id).unwrap();

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
    }

    pub fn switch(&mut self, id: DocumentId, action: Action) {
        use crate::tree::Layout;

        if !self.documents.contains_key(&id) {
            log::error!("cannot switch to document that does not exist (anymore)");
            return;
        }

        match action {
            Action::Replace => {
                let (view, doc) = current_ref!(self);
                // If the current view is an empty scratch buffer and is not displayed in any other views, delete it.
                // Boolean value is determined before the call to `view_mut` because the operation requires a borrow
                // of `self.tree`, which is mutably borrowed when `view_mut` is called.
                let remove_empty_scratch = !doc.is_modified()
                    // If the buffer has no path and is not modified, it is an empty scratch buffer.
                    && doc.path().is_none()
                    // If the buffer we are changing to is not this buffer
                    && id != doc.id
                    // Ensure the buffer is not displayed in any other splits.
                    && !self
                        .tree
                        .traverse()
                        .any(|(_, v)| v.doc == doc.id && v.id != view.id);

                let (view, doc) = current!(self);
                if remove_empty_scratch {
                    // Copy `doc.id` into a variable before calling `self.documents.remove`, which requires a mutable
                    // borrow, invalidating direct access to `doc.id`.
                    let id = doc.id;
                    self.documents.remove(&id);
                } else {
                    let jump = (view.doc, doc.selection(view.id).clone());
                    view.jumps.push(jump);
                    // Set last accessed doc if it is a different document
                    if doc.id != id {
                        view.last_accessed_doc = Some(view.doc);
                        // Set last modified doc if modified and last modified doc is different
                        if std::mem::take(&mut doc.modified_since_accessed)
                            && view.last_modified_docs[0] != Some(view.doc)
                        {
                            view.last_modified_docs = [Some(view.doc), view.last_modified_docs[0]];
                        }
                    }
                }

                let view_id = view.id;
                self.replace_document_in_view(view_id, id);

                return;
            }
            Action::Load => {
                let view_id = view!(self).id;
                let doc = self.documents.get_mut(&id).unwrap();
                if doc.selections().is_empty() {
                    doc.selections.insert(view_id, Selection::point(0));
                }
                return;
            }
            Action::HorizontalSplit | Action::VerticalSplit => {
                let view = View::new(id);
                let view_id = self.tree.split(
                    view,
                    match action {
                        Action::HorizontalSplit => Layout::Horizontal,
                        Action::VerticalSplit => Layout::Vertical,
                        _ => unreachable!(),
                    },
                );
                // initialize selection for view
                let doc = self.documents.get_mut(&id).unwrap();
                doc.selections.insert(view_id, Selection::point(0));
            }
        }

        self._refresh();
    }

    /// Generate an id for a new document and register it.
    fn new_document(&mut self, mut doc: Document) -> DocumentId {
        let id = self.next_document_id;
        // Safety: adding 1 from 1 is fine, probably impossible to reach usize max
        self.next_document_id =
            DocumentId(unsafe { NonZeroUsize::new_unchecked(self.next_document_id.0.get() + 1) });
        doc.id = id;
        self.documents.insert(id, doc);
        id
    }

    fn new_file_from_document(&mut self, action: Action, doc: Document) -> DocumentId {
        let id = self.new_document(doc);
        self.switch(id, action);
        id
    }

    pub fn new_file(&mut self, action: Action) -> DocumentId {
        self.new_file_from_document(action, Document::default())
    }

    pub fn new_file_from_stdin(&mut self, action: Action) -> Result<DocumentId, Error> {
        let (rope, encoding) = crate::document::from_reader(&mut stdin(), None)?;
        Ok(self.new_file_from_document(action, Document::from(rope, Some(encoding))))
    }

    pub fn open(&mut self, path: PathBuf, action: Action) -> Result<DocumentId, Error> {
        let path = helix_core::path::get_canonicalized_path(&path)?;
        let id = self.document_by_path(&path).map(|doc| doc.id);

        let id = if let Some(id) = id {
            id
        } else {
            let mut doc = Document::open(&path, None, Some(self.syn_loader.clone()))?;

            let _ = Self::launch_language_server(&mut self.language_servers, &mut doc);

            self.new_document(doc)
        };

        self.switch(id, action);
        Ok(id)
    }

    pub fn close(&mut self, id: ViewId) {
        let view = self.tree.get(self.tree.focus);
        // remove selection
        self.documents
            .get_mut(&view.doc)
            .unwrap()
            .selections
            .remove(&id);

        self.tree.remove(id);
        self._refresh();
    }

    pub fn close_document(&mut self, doc_id: DocumentId, force: bool) -> anyhow::Result<()> {
        let doc = match self.documents.get(&doc_id) {
            Some(doc) => doc,
            None => bail!("document does not exist"),
        };

        if !force && doc.is_modified() {
            bail!(
                "buffer {:?} is modified",
                doc.relative_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into())
            );
        }

        if let Some(language_server) = doc.language_server() {
            tokio::spawn(language_server.text_document_did_close(doc.identifier()));
        }

        let views_to_close = self
            .tree
            .views()
            .filter_map(|(view, _focus)| {
                if view.doc == doc_id {
                    Some(view.id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for view_id in views_to_close {
            self.close(view_id);
        }

        self.documents.remove(&doc_id);

        // If the document we removed was visible in all views, we will have no more views. We don't
        // want to close the editor just for a simple buffer close, so we need to create a new view
        // containing either an existing document, or a brand new document.
        if self.tree.views().next().is_none() {
            let doc_id = self
                .documents
                .iter()
                .map(|(&doc_id, _)| doc_id)
                .next()
                .unwrap_or_else(|| self.new_document(Document::default()));
            let view = View::new(doc_id);
            let view_id = self.tree.insert(view);
            let doc = self.documents.get_mut(&doc_id).unwrap();
            doc.selections.insert(view_id, Selection::point(0));
        }

        self._refresh();

        Ok(())
    }

    pub fn resize(&mut self, area: Rect) {
        if self.tree.resize(area) {
            self._refresh();
        };
    }

    pub fn focus_next(&mut self) {
        self.tree.focus_next();
    }

    pub fn focus_right(&mut self) {
        self.tree.focus_direction(tree::Direction::Right);
    }

    pub fn focus_left(&mut self) {
        self.tree.focus_direction(tree::Direction::Left);
    }

    pub fn focus_up(&mut self) {
        self.tree.focus_direction(tree::Direction::Up);
    }

    pub fn focus_down(&mut self) {
        self.tree.focus_direction(tree::Direction::Down);
    }

    pub fn should_close(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn ensure_cursor_in_view(&mut self, id: ViewId) {
        let config = self.config();
        let view = self.tree.get_mut(id);
        let doc = &self.documents[&view.doc];
        view.ensure_cursor_in_view(doc, config.scrolloff)
    }

    #[inline]
    pub fn document(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(&id)
    }

    #[inline]
    pub fn document_mut(&mut self, id: DocumentId) -> Option<&mut Document> {
        self.documents.get_mut(&id)
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
        let config = self.config();
        let (view, doc) = current_ref!(self);
        let cursor = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));
        if let Some(mut pos) = view.screen_coords_at_pos(doc, doc.text().slice(..), cursor) {
            let inner = view.inner_area();
            pos.col += inner.x as usize;
            pos.row += inner.y as usize;
            let cursorkind = config.cursor_shape.from_mode(doc.mode());
            (Some(pos), cursorkind)
        } else {
            (None, CursorKind::default())
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
