use anyhow::Error;
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
            changes,
            old_state,
            diagnostics: Vec::new(),
            version: 0,
            history: History::default(),
        }
    }

    // TODO: passing scopes here is awkward
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
        };

        // canonicalize path to absolute value
        doc.path = Some(std::fs::canonicalize(path)?);

        Ok(doc)
    }

    pub fn set_language(&mut self, scope: &str, scopes: &[String]) {
        if let Some(language_config) = LOADER.language_config_for_scope(scope) {
            let highlight_config = language_config.highlight_config(scopes).unwrap().unwrap();
            // TODO: config.configure(scopes) is now delayed, is that ok?

            let syntax = Syntax::new(&self.state.doc, highlight_config.clone());

            self.syntax = Some(syntax);
        };
    }

    pub fn apply(&mut self, transaction: &Transaction) -> bool {
        let old_doc = self.text().clone();

        let success = transaction.apply(&mut self.state);

        if !transaction.changes().is_empty() {
            // Compose this transaction with the previous one
            take_with(&mut self.changes, |changes| {
                changes.compose(transaction.changes().clone()).unwrap()
            });

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

    // pub fn slice<R>(&self, range: R) -> RopeSlice where R: RangeBounds {
    //     self.state.doc.slice
    // }
}
