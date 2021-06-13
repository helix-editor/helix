use anyhow::{Context, Error};
use std::cell::Cell;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use helix_core::{
    chars::{char_is_linebreak, char_is_whitespace},
    syntax::{LanguageConfiguration, LOADER},
    ChangeSet, Diagnostic, LineEnding, Rope, Selection, State, Syntax, Transaction,
    DEFAULT_LINE_ENDING,
};

use crate::{DocumentId, ViewId};

use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    Normal,
    Select,
    Insert,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IndentStyle {
    Tabs,
    Spaces(u8),
}

/// Represents one of the valid Unicode line endings.
/// Also acts as an index into `LINE_ENDINGS`.
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum LineEnding {
    None = 0, // No line ending
    Crlf = 1, // CarriageReturn followed by LineFeed
    LF = 2,   // U+000A -- LineFeed
    VT = 3,   // U+000B -- VerticalTab
    FF = 4,   // U+000C -- FormFeed
    CR = 5,   // U+000D -- CarriageReturn
    Nel = 6,  // U+0085 -- NextLine
    LS = 7,   // U+2028 -- Line Separator
    PS = 8,   // U+2029 -- ParagraphSeparator
}

pub struct Document {
    // rope + selection
    pub(crate) id: DocumentId,
    text: Rope,
    pub(crate) selections: HashMap<ViewId, Selection>,

    path: Option<PathBuf>,

    /// Current editing mode.
    pub mode: Mode,
    pub restore_cursor: bool,

    /// Current indent style.
    pub indent_style: IndentStyle,

    syntax: Option<Syntax>,
    // /// Corresponding language scope name. Usually `source.<lang>`.
    pub(crate) language: Option<Arc<LanguageConfiguration>>,

    /// Pending changes since last history commit.
    changes: ChangeSet,
    /// State at last commit. Used for calculating reverts.
    old_state: Option<State>,
    /// Undo tree.
    // It can be used as a cell where we will take it out to get some parts of the history and put
    // it back as it separated from the edits. We could split out the parts manually but that will
    // be more troublesome.
    history: Cell<History>,
    last_saved_revision: usize,
    version: i32, // should be usize?

    diagnostics: Vec<Diagnostic>,
    language_server: Option<Arc<helix_lsp::Client>>,
    line_ending: LineEnding,
}

use std::fmt;
impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("id", &self.id)
            .field("text", &self.text)
            .field("selections", &self.selections)
            .field("path", &self.path)
            .field("mode", &self.mode)
            .field("restore_cursor", &self.restore_cursor)
            .field("syntax", &self.syntax)
            .field("language", &self.language)
            .field("changes", &self.changes)
            .field("old_state", &self.old_state)
            // .field("history", &self.history)
            .field("last_saved_revision", &self.last_saved_revision)
            .field("version", &self.version)
            .field("diagnostics", &self.diagnostics)
            // .field("language_server", &self.language_server)
            .finish()
    }
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

/// Normalize a path, removing things like `.` and `..`.
///
/// CAUTION: This does not resolve symlinks (unlike
/// [`std::fs::canonicalize`]). This may cause incorrect or surprising
/// behavior at times. This should be used carefully. Unfortunately,
/// [`std::fs::canonicalize`] can be hard to use correctly, since it can often
/// fail, or on Windows returns annoying device paths. This is a problem Cargo
/// needs to improve on.
/// Copied from cargo: https://github.com/rust-lang/cargo/blob/070e459c2d8b79c5b2ac5218064e7603329c92ae/crates/cargo-util/src/paths.rs#L81
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

// Returns the canonical, absolute form of a path with all intermediate components normalized.
//
// This function is used instead of `std::fs::canonicalize` because we don't want to verify
// here if the path exists, just normalize it's components.
pub fn canonicalize_path(path: &Path) -> std::io::Result<PathBuf> {
    std::env::current_dir().map(|current_dir| normalize_path(&current_dir.join(path)))
}

use helix_lsp::lsp;
use url::Url;

impl Document {
    pub fn new(text: Rope) -> Self {
        let changes = ChangeSet::new(&text);
        let old_state = None;

        Self {
            id: DocumentId::default(),
            path: None,
            text,
            selections: HashMap::default(),
            indent_style: IndentStyle::Spaces(4),
            mode: Mode::Normal,
            restore_cursor: false,
            syntax: None,
            language: None,
            changes,
            old_state,
            diagnostics: Vec::new(),
            version: 0,
            history: Cell::new(History::default()),
            last_saved_revision: 0,
            language_server: None,
            line_ending: DEFAULT_LINE_ENDING,
        }
    }

    // TODO: async fn?
    pub fn load(path: PathBuf) -> Result<Self, Error> {
        use std::{fs::File, io::BufReader};

        let doc = if !path.exists() {
            Rope::from("\n")
        } else {
            let file = File::open(&path).context(format!("unable to open {:?}", path))?;
            let mut doc = Rope::from_reader(BufReader::new(file))?;
            // add missing newline at the end of file
            if doc.len_bytes() == 0 || doc.byte(doc.len_bytes() - 1) != b'\n' {
                doc.insert_char(doc.len_chars(), '\n');
            }
            doc
        };

        // search for line endings
        let line_ending = auto_detect_line_ending(&doc).unwrap_or(DEFAULT_LINE_ENDING);

        let mut doc = Self::new(doc);
        // set the path and try detecting the language
        doc.set_path(&path)?;
        doc.detect_indent_style();
        doc.set_line_ending(line_ending);

        Ok(doc)
    }

    // TODO: remove view_id dependency here
    pub fn format(&mut self, view_id: ViewId) {
        if let Some(language_server) = self.language_server() {
            // TODO: await, no blocking
            let transaction = helix_lsp::block_on(
                language_server
                    .text_document_formatting(self.identifier(), lsp::FormattingOptions::default()),
            )
            .map(|edits| {
                helix_lsp::util::generate_transaction_from_edits(
                    self.text(),
                    edits,
                    language_server.offset_encoding(),
                )
            });

            if let Ok(transaction) = transaction {
                self.apply(&transaction, view_id);
                self.append_changes_to_history(view_id);
            }
        }
    }

    // TODO: do we need some way of ensuring two save operations on the same doc can't run at once?
    // or is that handled by the OS/async layer
    pub fn save(&mut self) -> impl Future<Output = Result<(), anyhow::Error>> {
        // we clone and move text + path into the future so that we asynchronously save the current
        // state without blocking any further edits.

        let text = self.text().clone();
        let path = self.path.clone().expect("Can't save with no path set!"); // TODO: handle no path
        let identifier = self.identifier();

        // TODO: mark changes up to now as saved

        let language_server = self.language_server.clone();

        // reset the modified flag
        let history = self.history.take();
        self.last_saved_revision = history.current_revision();
        self.history.set(history);

        async move {
            use tokio::{fs::File, io::AsyncWriteExt};
            if let Some(parent) = path.parent() {
                // TODO: display a prompt asking the user if the directories should be created
                if !parent.exists() {
                    return Err(Error::msg(
                        "can't save file, parent directory does not exist",
                    ));
                }
            }
            let mut file = File::create(path).await?;

            // write all the rope chunks to file
            for chunk in text.chunks() {
                file.write_all(chunk.as_bytes()).await?;
            }
            // TODO: flush?

            if let Some(language_server) = language_server {
                language_server
                    .text_document_did_save(identifier, &text)
                    .await?;
            }

            Ok(())
        }
    }

    fn detect_language(&mut self) {
        if let Some(path) = self.path() {
            let loader = LOADER.get().unwrap();
            let language_config = loader.language_config_for_file_name(path);
            let scopes = loader.scopes();
            self.set_language(language_config, scopes);
        }
    }

    fn detect_indent_style(&mut self) {
        // Build a histogram of the indentation *increases* between
        // subsequent lines, ignoring lines that are all whitespace.
        //
        // Index 0 is for tabs, the rest are 1-8 spaces.
        let histogram: [usize; 9] = {
            let mut histogram = [0; 9];
            let mut prev_line_is_tabs = false;
            let mut prev_line_leading_count = 0usize;

            // Loop through the lines, checking for and recording indentation
            // increases as we go.
            'outer: for line in self.text.lines().take(1000) {
                let mut c_iter = line.chars();

                // Is first character a tab or space?
                let is_tabs = match c_iter.next() {
                    Some('\t') => true,
                    Some(' ') => false,

                    // Ignore blank lines.
                    Some(c) if char_is_linebreak(c) => continue,

                    _ => {
                        prev_line_is_tabs = false;
                        prev_line_leading_count = 0;
                        continue;
                    }
                };

                // Count the line's total leading tab/space characters.
                let mut leading_count = 1;
                let mut count_is_done = false;
                for c in c_iter {
                    match c {
                        '\t' if is_tabs && !count_is_done => leading_count += 1,
                        ' ' if !is_tabs && !count_is_done => leading_count += 1,

                        // We stop counting if we hit whitespace that doesn't
                        // qualify as indent or doesn't match the leading
                        // whitespace, but we don't exit the loop yet because
                        // we still want to determine if the line is blank.
                        c if char_is_whitespace(c) => count_is_done = true,

                        // Ignore blank lines.
                        c if char_is_linebreak(c) => continue 'outer,

                        _ => break,
                    }

                    // Bound the worst-case execution time for weird text files.
                    if leading_count > 256 {
                        continue 'outer;
                    }
                }

                // If there was an increase in indentation over the previous
                // line, update the histogram with that increase.
                if (prev_line_is_tabs == is_tabs || prev_line_leading_count == 0)
                    && prev_line_leading_count < leading_count
                {
                    if is_tabs {
                        histogram[0] += 1;
                    } else {
                        let amount = leading_count - prev_line_leading_count;
                        if amount <= 8 {
                            histogram[amount] += 1;
                        }
                    }
                }

                // Store this line's leading whitespace info for use with
                // the next line.
                prev_line_is_tabs = is_tabs;
                prev_line_leading_count = leading_count;
            }

            // Give more weight to tabs, because their presence is a very
            // strong indicator.
            histogram[0] *= 2;

            histogram
        };

        // Find the most frequent indent, its frequency, and the frequency of
        // the next-most frequent indent.
        let indent = histogram
            .iter()
            .enumerate()
            .max_by_key(|kv| kv.1)
            .unwrap()
            .0;
        let indent_freq = histogram[indent];
        let indent_freq_2 = *histogram
            .iter()
            .enumerate()
            .filter(|kv| kv.0 != indent)
            .map(|kv| kv.1)
            .max()
            .unwrap();

        // Use the auto-detected result if we're confident enough in its
        // accuracy, based on some heuristics.  Otherwise fall back to
        // the language-based setting.
        if indent_freq >= 1 && (indent_freq_2 as f64 / indent_freq as f64) < 0.66 {
            // Use the auto-detected setting.
            self.indent_style = match indent {
                0 => IndentStyle::Tabs,
                _ => IndentStyle::Spaces(indent as u8),
            };
        } else {
            // Fall back to language-based setting.
            let indent = self
                .language
                .as_ref()
                .and_then(|config| config.indent.as_ref())
                .map_or("  ", |config| config.unit.as_str()); // fallback to 2 spaces

            self.indent_style = if indent.starts_with(' ') {
                IndentStyle::Spaces(indent.len() as u8)
            } else {
                IndentStyle::Tabs
            };
        }
    }

    pub fn set_path(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let path = canonicalize_path(path)?;

        // if parent doesn't exist we still want to open the document
        // and error out when document is saved
        self.path = Some(path);

        // try detecting the language based on filepath
        self.detect_language();

        Ok(())
    }

    pub fn set_language(
        &mut self,
        language_config: Option<Arc<helix_core::syntax::LanguageConfiguration>>,
        scopes: &[String],
    ) {
        if let Some(language_config) = language_config {
            if let Some(highlight_config) = language_config.highlight_config(scopes) {
                let syntax = Syntax::new(&self.text, highlight_config);
                self.syntax = Some(syntax);
                // TODO: config.configure(scopes) is now delayed, is that ok?
            }

            self.language = Some(language_config);
        } else {
            self.syntax = None;
            self.language = None;
        };
    }

    pub fn set_language2(&mut self, scope: &str) {
        let loader = LOADER.get().unwrap();
        let language_config = loader.language_config_for_scope(scope);
        let scopes = loader.scopes();

        self.set_language(language_config, scopes);
    }

    pub fn set_language_server(&mut self, language_server: Option<Arc<helix_lsp::Client>>) {
        self.language_server = language_server;
    }

    pub fn set_selection(&mut self, view_id: ViewId, selection: Selection) {
        // TODO: use a transaction?
        self.selections.insert(view_id, selection);
    }

    pub fn set_line_ending(&mut self, line_ending: LineEnding) {
        self.line_ending = line_ending;
    }

    fn _apply(&mut self, transaction: &Transaction, view_id: ViewId) -> bool {
        let old_doc = self.text().clone();

        let success = transaction.changes().apply(&mut self.text);

        if success {
            // update the selection: either take the selection specified in the transaction, or map the
            // current selection through changes.
            let selection = transaction
                .selection()
                .cloned()
                .unwrap_or_else(|| self.selection(view_id).clone().map(transaction.changes()));
            self.set_selection(view_id, selection);
        }

        if !transaction.changes().is_empty() {
            self.version += 1;

            // update tree-sitter syntax tree
            if let Some(syntax) = &mut self.syntax {
                // TODO: no unwrap
                syntax
                    .update(&old_doc, &self.text, transaction.changes())
                    .unwrap();
            }

            // map state.diagnostics over changes::map_pos too
            // NOTE: seems to do nothing since the language server resends diagnostics on each edit
            // for diagnostic in &mut self.diagnostics {
            //     use helix_core::Assoc;
            //     let changes = transaction.changes();
            //     diagnostic.range.start = changes.map_pos(diagnostic.range.start, Assoc::After);
            //     diagnostic.range.end = changes.map_pos(diagnostic.range.end, Assoc::After);
            //     diagnostic.line = self.text.char_to_line(diagnostic.range.start);
            // }

            // emit lsp notification
            if let Some(language_server) = &self.language_server {
                let notify = language_server.text_document_did_change(
                    self.versioned_identifier(),
                    &old_doc,
                    self.text(),
                    transaction.changes(),
                );

                if let Some(notify) = notify {
                    tokio::spawn(notify);
                } //.expect("failed to emit textDocument/didChange");
            }
        }
        success
    }

    pub fn apply(&mut self, transaction: &Transaction, view_id: ViewId) -> bool {
        // store the state just before any changes are made. This allows us to undo to the
        // state just before a transaction was applied.
        if self.changes.is_empty() && !transaction.changes().is_empty() {
            self.old_state = Some(State {
                doc: self.text.clone(),
                selection: self.selection(view_id).clone(),
            });
        }

        let success = self._apply(transaction, view_id);

        if !transaction.changes().is_empty() {
            // Compose this transaction with the previous one
            take_with(&mut self.changes, |changes| {
                changes.compose(transaction.changes().clone())
            });
        }
        success
    }

    pub fn undo(&mut self, view_id: ViewId) {
        let mut history = self.history.take();
        let success = if let Some(transaction) = history.undo() {
            self._apply(&transaction, view_id)
        } else {
            false
        };
        self.history.set(history);

        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text());
        }
    }

    pub fn redo(&mut self, view_id: ViewId) {
        let mut history = self.history.take();
        let success = if let Some(transaction) = history.redo() {
            self._apply(&transaction, view_id)
        } else {
            false
        };
        self.history.set(history);

        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text());
        }
    }

    pub fn earlier(&mut self, view_id: ViewId, uk: helix_core::history::UndoKind) {
        let txns = self.history.get_mut().earlier(uk);
        for txn in txns {
            self._apply(&txn, view_id);
        }
    }

    pub fn later(&mut self, view_id: ViewId, uk: helix_core::history::UndoKind) {
        let txns = self.history.get_mut().later(uk);
        for txn in txns {
            self._apply(&txn, view_id);
        }
    }

    pub fn append_changes_to_history(&mut self, view_id: ViewId) {
        if self.changes.is_empty() {
            return;
        }

        let new_changeset = ChangeSet::new(self.text());
        let changes = std::mem::replace(&mut self.changes, new_changeset);
        // Instead of doing this messy merge we could always commit, and based on transaction
        // annotations either add a new layer or compose into the previous one.
        let transaction =
            Transaction::from(changes).with_selection(self.selection(view_id).clone());

        // HAXX: we need to reconstruct the state as it was before the changes..
        let old_state = self.old_state.take().expect("no old_state available");

        let mut history = self.history.take();
        history.commit_revision(&transaction, &old_state);
        self.history.set(history);
    }

    #[inline]
    pub fn id(&self) -> DocumentId {
        self.id
    }

    #[inline]
    pub fn is_modified(&self) -> bool {
        let history = self.history.take();
        let current_revision = history.current_revision();
        self.history.set(history);
        current_revision != self.last_saved_revision || !self.changes.is_empty()
    }

    #[inline]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[inline]
    /// Corresponding language scope name. Usually `source.<lang>`.
    pub fn language(&self) -> Option<&str> {
        self.language
            .as_ref()
            .map(|language| language.scope.as_str())
    }

    #[inline]
    pub fn language_config(&self) -> Option<&LanguageConfiguration> {
        self.language.as_deref()
    }

    #[inline]
    /// Current document version, incremented at each change.
    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn language_server(&self) -> Option<&helix_lsp::Client> {
        self.language_server.as_deref()
    }

    /// Tree-sitter AST tree
    pub fn syntax(&self) -> Option<&Syntax> {
        self.syntax.as_ref()
    }

    /// Tab size in columns.
    pub fn tab_width(&self) -> usize {
        self.language
            .as_ref()
            .and_then(|config| config.indent.as_ref())
            .map_or(4, |config| config.tab_width) // fallback to 4 columns
    }

    /// Returns a string containing a single level of indentation.
    ///
    /// TODO: we might not need this function anymore, since the information
    /// is conveniently available in `Document::indent_style` now.
    pub fn indent_unit(&self) -> &'static str {
        match self.indent_style {
            IndentStyle::Tabs => "\t",
            IndentStyle::Spaces(1) => " ",
            IndentStyle::Spaces(2) => "  ",
            IndentStyle::Spaces(3) => "   ",
            IndentStyle::Spaces(4) => "    ",
            IndentStyle::Spaces(5) => "     ",
            IndentStyle::Spaces(6) => "      ",
            IndentStyle::Spaces(7) => "       ",
            IndentStyle::Spaces(8) => "        ",

            // Unsupported indentation style.  This should never happen,
            // but just in case fall back to two spaces.
            _ => "  ",
        }
    }

    #[inline]
    /// File path on disk.
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn url(&self) -> Option<Url> {
        self.path().map(|path| Url::from_file_path(path).unwrap())
    }

    pub fn text(&self) -> &Rope {
        &self.text
    }

    pub fn selection(&self, view_id: ViewId) -> &Selection {
        &self.selections[&view_id]
    }

    pub fn relative_path(&self) -> Option<&Path> {
        let cwdir = std::env::current_dir().expect("couldn't determine current directory");

        self.path
            .as_ref()
            .map(|path| path.strip_prefix(cwdir).unwrap_or(path))
    }

    // pub fn slice<R>(&self, range: R) -> RopeSlice where R: RangeBounds {
    //     self.state.doc.slice
    // }

    // transact(Fn) ?

    // -- LSP methods

    pub fn identifier(&self) -> lsp::TextDocumentIdentifier {
        lsp::TextDocumentIdentifier::new(self.url().unwrap())
    }

    pub fn versioned_identifier(&self) -> lsp::VersionedTextDocumentIdentifier {
        lsp::VersionedTextDocumentIdentifier::new(self.url().unwrap(), self.version)
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics = diagnostics;
    }

    pub fn line_ending(&self) -> &str {
        match self.line_ending {
            LineEnding::Crlf => "\u{000D}\u{000A}",
            LineEnding::LF => "\u{000A}",
            LineEnding::Nel => "\u{0085}",
            LineEnding::LS => "\u{2028}",
            LineEnding::CR => "\u{000D}",
            _ => panic!(
                "Unexpected line ending: {:?}, expected Crlf, LF, CR, Nel, or LS.",
                self.line_ending
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn changeset_to_changes() {
        use helix_lsp::{lsp, Client, OffsetEncoding};
        let text = Rope::from("hello");
        let mut doc = Document::new(text);
        let view = ViewId::default();
        doc.set_selection(view, Selection::single(5, 5));

        // insert

        let transaction = Transaction::insert(doc.text(), doc.selection(view), " world".into());
        let old_doc = doc.text().clone();
        doc.apply(&transaction, view);
        let changes = Client::changeset_to_changes(
            &old_doc,
            doc.text(),
            transaction.changes(),
            OffsetEncoding::Utf8,
        );

        assert_eq!(
            changes,
            &[lsp::TextDocumentContentChangeEvent {
                range: Some(lsp::Range::new(
                    lsp::Position::new(0, 5),
                    lsp::Position::new(0, 5)
                )),
                text: " world".into(),
                range_length: None,
            }]
        );

        // delete

        let transaction = transaction.invert(&old_doc);
        let old_doc = doc.text().clone();
        doc.apply(&transaction, view);
        let changes = Client::changeset_to_changes(
            &old_doc,
            doc.text(),
            transaction.changes(),
            OffsetEncoding::Utf8,
        );

        // line: 0-based.
        // col: 0-based, gaps between chars.
        // 0 1 2 3 4 5 6 7 8 9 0 1
        // |h|e|l|l|o| |w|o|r|l|d|
        //           -------------
        // (0, 5)-(0, 11)
        assert_eq!(
            changes,
            &[lsp::TextDocumentContentChangeEvent {
                range: Some(lsp::Range::new(
                    lsp::Position::new(0, 5),
                    lsp::Position::new(0, 11)
                )),
                text: "".into(),
                range_length: None,
            }]
        );

        // replace

        // also tests that changes are layered, positions depend on previous changes.

        doc.set_selection(view, Selection::single(0, 5));
        let transaction = Transaction::change(
            doc.text(),
            vec![(0, 2, Some("aei".into())), (3, 5, Some("ou".into()))].into_iter(),
        );
        // aeilou
        let old_doc = doc.text().clone();
        doc.apply(&transaction, view);
        let changes = Client::changeset_to_changes(
            &old_doc,
            doc.text(),
            transaction.changes(),
            OffsetEncoding::Utf8,
        );

        assert_eq!(
            changes,
            &[
                // 0 1 2 3 4 5
                // |h|e|l|l|o|
                // ----
                //
                // aeillo
                lsp::TextDocumentContentChangeEvent {
                    range: Some(lsp::Range::new(
                        lsp::Position::new(0, 0),
                        lsp::Position::new(0, 2)
                    )),
                    text: "aei".into(),
                    range_length: None,
                },
                // 0 1 2 3 4 5 6
                // |a|e|i|l|l|o|
                //         -----
                //
                // aeilou
                lsp::TextDocumentContentChangeEvent {
                    range: Some(lsp::Range::new(
                        lsp::Position::new(0, 4),
                        lsp::Position::new(0, 6)
                    )),
                    text: "ou".into(),
                    range_length: None,
                }
            ]
        );
    }
}
