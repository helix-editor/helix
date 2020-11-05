use anyhow::Error;
use std::future::Future;
use std::path::PathBuf;

use helix_core::{
    syntax::LOADER, ChangeSet, Diagnostic, History, Position, Range, Rope, RopeSlice, Selection,
    State, Syntax, Transaction,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    Normal,
    Insert,
    Goto,
}

pub struct Document {
    pub state: State, // rope + selection
    /// File path on disk.
    pub path: Option<PathBuf>,

    /// Current editing mode.
    pub mode: Mode,
    pub restore_cursor: bool,

    /// Tree-sitter AST tree
    pub syntax: Option<Syntax>,
    /// Corresponding language scope name. Usually `source.<lang>`.
    pub language: Option<String>,

    /// Pending changes since last history commit.
    pub changes: ChangeSet,
    pub old_state: State,
    pub history: History,
    pub version: i64, // should be usize?

    pub diagnostics: Vec<Diagnostic>,
}

/// Like std::mem::replace() except it allows the replacement value to be mapped from the
/// original value.
fn take_with<T, F>(mut_ref: &mut T, closure: F)
where
    F: FnOnce(T) -> T,
{
    use std::{panic, ptr};

    unsafe {
        let old_t = ptr::read(mut_ref);
        let new_t = panic::catch_unwind(panic::AssertUnwindSafe(|| closure(old_t)))
            .unwrap_or_else(|_| ::std::process::abort());
        ptr::write(mut_ref, new_t);
    }
}

use url::Url;

impl Document {
    fn new(state: State) -> Self {
        let changes = ChangeSet::new(&state.doc);
        let old_state = state.clone();

        Self {
            path: None,
            state,
            mode: Mode::Normal,
            restore_cursor: false,
            syntax: None,
            language: None,
            changes,
            old_state,
            diagnostics: Vec::new(),
            version: 0,
            history: History::default(),
        }
    }

    // TODO: passing scopes here is awkward
    // TODO: async fn?
    pub fn load(path: PathBuf, scopes: &[String]) -> Result<Self, Error> {
        use std::{env, fs::File, io::BufReader};
        let _current_dir = env::current_dir()?;

        let doc = Rope::from_reader(BufReader::new(File::open(path.clone())?))?;

        // TODO: create if not found

        let mut doc = Self::new(State::new(doc));

        if let Some(language_config) = LOADER.language_config_for_file_name(path.as_path()) {
            let highlight_config = language_config.highlight_config(scopes).unwrap().unwrap();
            // TODO: config.configure(scopes) is now delayed, is that ok?

            let syntax = Syntax::new(&doc.state.doc, highlight_config.clone());

            doc.syntax = Some(syntax);
            // TODO: maybe just keep an Arc<> pointer to the language_config?
            doc.language = Some(language_config.scope().to_string());

            // TODO: this ties lsp support to tree-sitter enabled languages for now. Language
            // config should use Option<HighlightConfig> to let us have non-tree-sitter configs.

            // TODO: circular dep: view <-> lsp
            // helix_lsp::REGISTRY;
            // view should probably depend on lsp
        };

        // canonicalize path to absolute value
        doc.path = Some(std::fs::canonicalize(path)?);

        Ok(doc)
    }

    // TODO: do we need some way of ensuring two save operations on the same doc can't run at once?
    // or is that handled by the OS/async layer
    pub fn save(&self) -> impl Future<Output = Result<(), anyhow::Error>> {
        // we clone and move text + path into the future so that we asynchronously save the current
        // state without blocking any further edits.

        let text = self.text().clone();
        let path = self.path.clone().expect("Can't save with no path set!"); // TODO: handle no path

        // TODO: mark changes up to now as saved
        // TODO: mark dirty false

        async move {
            use smol::{fs::File, prelude::*};
            let mut file = File::create(path).await?;

            // write all the rope chunks to file
            for chunk in text.chunks() {
                file.write_all(chunk.as_bytes()).await?;
            }
            // TODO: flush?

            Ok(())
        } // and_then(// lsp.send_text_saved_notification())
    }

    pub fn set_language(&mut self, scope: &str, scopes: &[String]) {
        if let Some(language_config) = LOADER.language_config_for_scope(scope) {
            let highlight_config = language_config.highlight_config(scopes).unwrap().unwrap();
            // TODO: config.configure(scopes) is now delayed, is that ok?

            let syntax = Syntax::new(&self.state.doc, highlight_config.clone());

            self.syntax = Some(syntax);
        };
    }

    pub fn set_selection(&mut self, selection: Selection) {
        // TODO: use a transaction?
        self.state.selection = selection;
    }

    pub fn apply(&mut self, transaction: &Transaction) -> bool {
        let old_doc = self.text().clone();

        let success = transaction.apply(&mut self.state);

        if !transaction.changes().is_empty() {
            // Compose this transaction with the previous one
            take_with(&mut self.changes, |changes| {
                changes.compose(transaction.changes().clone()).unwrap()
            });

            // TODO: when composing, replace transaction.selection too

            // update tree-sitter syntax tree
            if let Some(syntax) = &mut self.syntax {
                // TODO: no unwrap
                syntax
                    .update(&old_doc, &self.state.doc, transaction.changes())
                    .unwrap();
            }

            // TODO: map state.diagnostics over changes::map_pos too
        }
        success
    }

    #[inline]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[inline]
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn url(&self) -> Option<Url> {
        self.path().map(|path| Url::from_file_path(path).unwrap())
    }

    pub fn text(&self) -> &Rope {
        &self.state.doc
    }

    pub fn selection(&self) -> &Selection {
        &self.state.selection
    }

    // pub fn slice<R>(&self, range: R) -> RopeSlice where R: RangeBounds {
    //     self.state.doc.slice
    // }

    // TODO: transact(Fn) ?
}
