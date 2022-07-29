use anyhow::{anyhow, bail, Context, Error};
use helix_core::auto_pairs::AutoPairs;
use helix_core::Range;
use serde::de::{self, Deserialize, Deserializer};
use serde::Serialize;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use helix_core::{
    encoding,
    history::{History, UndoKind},
    indent::{auto_detect_indent_style, IndentStyle},
    line_ending::auto_detect_line_ending,
    syntax::{self, LanguageConfiguration},
    ChangeSet, Diagnostic, LineEnding, Rope, RopeBuilder, Selection, State, Syntax, Transaction,
    DEFAULT_LINE_ENDING,
};
use helix_lsp::util::LspFormatting;

use crate::{DocumentId, Editor, ViewId};

/// 8kB of buffer space for encoding and decoding `Rope`s.
const BUF_SIZE: usize = 8192;

const DEFAULT_INDENT: IndentStyle = IndentStyle::Tabs;

pub const SCRATCH_BUFFER_NAME: &str = "[scratch]";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    Normal = 0,
    Select = 1,
    Insert = 2,
}

impl Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Normal => f.write_str("normal"),
            Mode::Select => f.write_str("select"),
            Mode::Insert => f.write_str("insert"),
        }
    }
}

impl FromStr for Mode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Mode::Normal),
            "select" => Ok(Mode::Select),
            "insert" => Ok(Mode::Insert),
            _ => bail!("Invalid mode '{}'", s),
        }
    }
}

// toml deserializer doesn't seem to recognize string as enum
impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl Serialize for Mode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

pub struct Document {
    pub(crate) id: DocumentId,
    text: Rope,
    selections: HashMap<ViewId, Selection>,

    path: Option<PathBuf>,
    encoding: &'static encoding::Encoding,

    /// Current editing mode.
    pub mode: Mode,
    pub restore_cursor: bool,

    /// Current indent style.
    pub indent_style: IndentStyle,

    /// The document's default line ending.
    pub line_ending: LineEnding,

    syntax: Option<Syntax>,
    /// Corresponding language scope name. Usually `source.<lang>`.
    pub(crate) language: Option<Arc<LanguageConfiguration>>,

    /// Pending changes since last history commit.
    changes: ChangeSet,
    /// State at last commit. Used for calculating reverts.
    old_state: Option<State>,
    /// Undo tree.
    // It can be used as a cell where we will take it out to get some parts of the history and put
    // it back as it separated from the edits. We could split out the parts manually but that will
    // be more troublesome.
    pub history: Cell<History>,

    pub savepoint: Option<Transaction>,

    last_saved_revision: usize,
    version: i32, // should be usize?
    pub(crate) modified_since_accessed: bool,

    diagnostics: Vec<Diagnostic>,
    language_server: Option<Arc<helix_lsp::Client>>,
}

use std::{fmt, mem};
impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("id", &self.id)
            .field("text", &self.text)
            .field("selections", &self.selections)
            .field("path", &self.path)
            .field("encoding", &self.encoding)
            .field("mode", &self.mode)
            .field("restore_cursor", &self.restore_cursor)
            .field("syntax", &self.syntax)
            .field("language", &self.language)
            .field("changes", &self.changes)
            .field("old_state", &self.old_state)
            // .field("history", &self.history)
            .field("last_saved_revision", &self.last_saved_revision)
            .field("version", &self.version)
            .field("modified_since_accessed", &self.modified_since_accessed)
            .field("diagnostics", &self.diagnostics)
            // .field("language_server", &self.language_server)
            .finish()
    }
}

// The documentation and implementation of this function should be up-to-date with
// its sibling function, `to_writer()`.
//
/// Decodes a stream of bytes into UTF-8, returning a `Rope` and the
/// encoding it was decoded as. The optional `encoding` parameter can
/// be used to override encoding auto-detection.
pub fn from_reader<R: std::io::Read + ?Sized>(
    reader: &mut R,
    encoding: Option<&'static encoding::Encoding>,
) -> Result<(Rope, &'static encoding::Encoding), Error> {
    // These two buffers are 8192 bytes in size each and are used as
    // intermediaries during the decoding process. Text read into `buf`
    // from `reader` is decoded into `buf_out` as UTF-8. Once either
    // `buf_out` is full or the end of the reader was reached, the
    // contents are appended to `builder`.
    let mut buf = [0u8; BUF_SIZE];
    let mut buf_out = [0u8; BUF_SIZE];
    let mut builder = RopeBuilder::new();

    // By default, the encoding of the text is auto-detected via the
    // `chardetng` crate which requires sample data from the reader.
    // As a manual override to this auto-detection is possible, the
    // same data is read into `buf` to ensure symmetry in the upcoming
    // loop.
    let (encoding, mut decoder, mut slice, mut is_empty) = {
        let read = reader.read(&mut buf)?;
        let is_empty = read == 0;
        let encoding = encoding.unwrap_or_else(|| {
            let mut encoding_detector = chardetng::EncodingDetector::new();
            encoding_detector.feed(&buf, is_empty);
            encoding_detector.guess(None, true)
        });
        let decoder = encoding.new_decoder();

        // If the amount of bytes read from the reader is less than
        // `buf.len()`, it is undesirable to read the bytes afterwards.
        let slice = &buf[..read];
        (encoding, decoder, slice, is_empty)
    };

    // `RopeBuilder::append()` expects a `&str`, so this is the "real"
    // output buffer. When decoding, the number of bytes in the output
    // buffer will often exceed the number of bytes in the input buffer.
    // The `result` returned by `decode_to_str()` will state whether or
    // not that happened. The contents of `buf_str` is appended to
    // `builder` and it is reused for the next iteration of the decoding
    // loop.
    //
    // As it is possible to read less than the buffer's maximum from `read()`
    // even when the end of the reader has yet to be reached, the end of
    // the reader is determined only when a `read()` call returns `0`.
    //
    // SAFETY: `buf_out` is a zero-initialized array, thus it will always
    // contain valid UTF-8.
    let buf_str = unsafe { std::str::from_utf8_unchecked_mut(&mut buf_out[..]) };
    let mut total_written = 0usize;
    loop {
        let mut total_read = 0usize;

        // An inner loop is necessary as it is possible that the input buffer
        // may not be completely decoded on the first `decode_to_str()` call
        // which would happen in cases where the output buffer is filled to
        // capacity.
        loop {
            let (result, read, written, ..) = decoder.decode_to_str(
                &slice[total_read..],
                &mut buf_str[total_written..],
                is_empty,
            );

            // These variables act as the read and write cursors of `buf` and `buf_str` respectively.
            // They are necessary in case the output buffer fills before decoding of the entire input
            // loop is complete. Otherwise, the loop would endlessly iterate over the same `buf` and
            // the data inside the output buffer would be overwritten.
            total_read += read;
            total_written += written;
            match result {
                encoding::CoderResult::InputEmpty => {
                    debug_assert_eq!(slice.len(), total_read);
                    break;
                }
                encoding::CoderResult::OutputFull => {
                    debug_assert!(slice.len() > total_read);
                    builder.append(&buf_str[..total_written]);
                    total_written = 0;
                }
            }
        }
        // Once the end of the stream is reached, the output buffer is
        // flushed and the loop terminates.
        if is_empty {
            debug_assert_eq!(reader.read(&mut buf)?, 0);
            builder.append(&buf_str[..total_written]);
            break;
        }

        // Once the previous input has been processed and decoded, the next set of
        // data is fetched from the reader. The end of the reader is determined to
        // be when exactly `0` bytes were read from the reader, as per the invariants
        // of the `Read` trait.
        let read = reader.read(&mut buf)?;
        slice = &buf[..read];
        is_empty = read == 0;
    }
    let rope = builder.finish();
    Ok((rope, encoding))
}

// The documentation and implementation of this function should be up-to-date with
// its sibling function, `from_reader()`.
//
/// Encodes the text inside `rope` into the given `encoding` and writes the
/// encoded output into `writer.` As a `Rope` can only contain valid UTF-8,
/// replacement characters may appear in the encoded text.
pub async fn to_writer<'a, W: tokio::io::AsyncWriteExt + Unpin + ?Sized>(
    writer: &'a mut W,
    encoding: &'static encoding::Encoding,
    rope: &'a Rope,
) -> Result<(), Error> {
    // Text inside a `Rope` is stored as non-contiguous blocks of data called
    // chunks. The absolute size of each chunk is unknown, thus it is impossible
    // to predict the end of the chunk iterator ahead of time. Instead, it is
    // determined by filtering the iterator to remove all empty chunks and then
    // appending an empty chunk to it. This is valuable for detecting when all
    // chunks in the `Rope` have been iterated over in the subsequent loop.
    let iter = rope
        .chunks()
        .filter(|c| !c.is_empty())
        .chain(std::iter::once(""));
    let mut buf = [0u8; BUF_SIZE];
    let mut encoder = encoding.new_encoder();
    let mut total_written = 0usize;
    for chunk in iter {
        let is_empty = chunk.is_empty();
        let mut total_read = 0usize;

        // An inner loop is necessary as it is possible that the input buffer
        // may not be completely encoded on the first `encode_from_utf8()` call
        // which would happen in cases where the output buffer is filled to
        // capacity.
        loop {
            let (result, read, written, ..) =
                encoder.encode_from_utf8(&chunk[total_read..], &mut buf[total_written..], is_empty);

            // These variables act as the read and write cursors of `chunk` and `buf` respectively.
            // They are necessary in case the output buffer fills before encoding of the entire input
            // loop is complete. Otherwise, the loop would endlessly iterate over the same `chunk` and
            // the data inside the output buffer would be overwritten.
            total_read += read;
            total_written += written;
            match result {
                encoding::CoderResult::InputEmpty => {
                    debug_assert_eq!(chunk.len(), total_read);
                    debug_assert!(buf.len() >= total_written);
                    break;
                }
                encoding::CoderResult::OutputFull => {
                    debug_assert!(chunk.len() > total_read);
                    writer.write_all(&buf[..total_written]).await?;
                    total_written = 0;
                }
            }
        }

        // Once the end of the iterator is reached, the output buffer is
        // flushed and the outer loop terminates.
        if is_empty {
            writer.write_all(&buf[..total_written]).await?;
            writer.flush().await?;
            break;
        }
    }
    Ok(())
}

fn take_with<T, F>(mut_ref: &mut T, f: F)
where
    T: Default,
    F: FnOnce(T) -> T,
{
    *mut_ref = f(mem::take(mut_ref));
}

use helix_lsp::lsp;
use url::Url;

impl Document {
    pub fn from(text: Rope, encoding: Option<&'static encoding::Encoding>) -> Self {
        let encoding = encoding.unwrap_or(encoding::UTF_8);
        let changes = ChangeSet::new(&text);
        let old_state = None;

        Self {
            id: DocumentId::default(),
            path: None,
            encoding,
            text,
            selections: HashMap::default(),
            indent_style: DEFAULT_INDENT,
            line_ending: DEFAULT_LINE_ENDING,
            mode: Mode::Normal,
            restore_cursor: false,
            syntax: None,
            language: None,
            changes,
            old_state,
            diagnostics: Vec::new(),
            version: 0,
            history: Cell::new(History::default()),
            savepoint: None,
            last_saved_revision: 0,
            modified_since_accessed: false,
            language_server: None,
        }
    }

    // TODO: async fn?
    /// Create a new document from `path`. Encoding is auto-detected, but it can be manually
    /// overwritten with the `encoding` parameter.
    pub fn open(
        path: &Path,
        encoding: Option<&'static encoding::Encoding>,
        config_loader: Option<Arc<syntax::Loader>>,
    ) -> Result<Self, Error> {
        // Open the file if it exists, otherwise assume it is a new file (and thus empty).
        let (rope, encoding) = if path.exists() {
            let mut file =
                std::fs::File::open(path).context(format!("unable to open {:?}", path))?;
            from_reader(&mut file, encoding)?
        } else {
            let encoding = encoding.unwrap_or(encoding::UTF_8);
            (Rope::from(DEFAULT_LINE_ENDING.as_str()), encoding)
        };

        let mut doc = Self::from(rope, Some(encoding));

        // set the path and try detecting the language
        doc.set_path(Some(path))?;
        if let Some(loader) = config_loader {
            doc.detect_language(loader);
        }

        doc.detect_indent_and_line_ending();

        Ok(doc)
    }

    /// The same as [`format`], but only returns formatting changes if auto-formatting
    /// is configured.
    pub fn auto_format(&self) -> Option<impl Future<Output = LspFormatting> + 'static> {
        if self.language_config()?.auto_format {
            self.format()
        } else {
            None
        }
    }

    /// If supported, returns the changes that should be applied to this document in order
    /// to format it nicely.
    pub fn format(&self) -> Option<impl Future<Output = LspFormatting> + 'static> {
        let language_server = self.language_server()?;
        let text = self.text.clone();
        let offset_encoding = language_server.offset_encoding();

        let request = language_server.text_document_formatting(
            self.identifier(),
            lsp::FormattingOptions {
                tab_size: self.tab_width() as u32,
                insert_spaces: matches!(self.indent_style, IndentStyle::Spaces(_)),
                ..Default::default()
            },
            None,
        )?;

        let fut = async move {
            let edits = request.await.unwrap_or_else(|e| {
                log::warn!("LSP formatting failed: {}", e);
                Default::default()
            });
            LspFormatting {
                doc: text,
                edits,
                offset_encoding,
            }
        };
        Some(fut)
    }

    pub fn save(&mut self, force: bool) -> impl Future<Output = Result<(), anyhow::Error>> {
        self.save_impl::<futures_util::future::Ready<_>>(None, force)
    }

    pub fn format_and_save(
        &mut self,
        formatting: Option<impl Future<Output = LspFormatting>>,
        force: bool,
    ) -> impl Future<Output = anyhow::Result<()>> {
        self.save_impl(formatting, force)
    }

    // TODO: do we need some way of ensuring two save operations on the same doc can't run at once?
    // or is that handled by the OS/async layer
    /// The `Document`'s text is encoded according to its encoding and written to the file located
    /// at its `path()`.
    ///
    /// If `formatting` is present, it supplies some changes that we apply to the text before saving.
    fn save_impl<F: Future<Output = LspFormatting>>(
        &mut self,
        formatting: Option<F>,
        force: bool,
    ) -> impl Future<Output = Result<(), anyhow::Error>> {
        // we clone and move text + path into the future so that we asynchronously save the current
        // state without blocking any further edits.

        let mut text = self.text().clone();
        let path = self.path.clone().expect("Can't save with no path set!");
        let identifier = self.identifier();

        let language_server = self.language_server.clone();

        // mark changes up to now as saved
        self.reset_modified();

        let encoding = self.encoding;

        // We encode the file according to the `Document`'s encoding.
        async move {
            use tokio::fs::File;
            if let Some(parent) = path.parent() {
                // TODO: display a prompt asking the user if the directories should be created
                if !parent.exists() {
                    if force {
                        std::fs::DirBuilder::new().recursive(true).create(parent)?;
                    } else {
                        bail!("can't save file, parent directory does not exist");
                    }
                }
            }

            if let Some(fmt) = formatting {
                let success = Transaction::from(fmt.await).changes().apply(&mut text);
                if !success {
                    // This shouldn't happen, because the transaction changes were generated
                    // from the same text we're saving.
                    log::error!("failed to apply format changes before saving");
                }
            }

            let mut file = File::create(path).await?;
            to_writer(&mut file, encoding, &text).await?;

            if let Some(language_server) = language_server {
                if !language_server.is_initialized() {
                    return Ok(());
                }
                if let Some(notification) =
                    language_server.text_document_did_save(identifier, &text)
                {
                    notification.await?;
                }
            }

            Ok(())
        }
    }

    /// Detect the programming language based on the file type.
    pub fn detect_language(&mut self, config_loader: Arc<syntax::Loader>) {
        if let Some(path) = &self.path {
            let language_config = config_loader
                .language_config_for_file_name(path)
                .or_else(|| config_loader.language_config_for_shebang(self.text()));
            self.set_language(language_config, Some(config_loader));
        }
    }

    /// Detect the indentation used in the file, or otherwise defaults to the language indentation
    /// configured in `languages.toml`, with a fallback to 4 space indentation if it isn't
    /// specified. Line ending is likewise auto-detected, and will fallback to the default OS
    /// line ending.
    pub fn detect_indent_and_line_ending(&mut self) {
        self.indent_style = auto_detect_indent_style(&self.text).unwrap_or_else(|| {
            self.language_config()
                .and_then(|config| config.indent.as_ref())
                .map_or(DEFAULT_INDENT, |config| IndentStyle::from_str(&config.unit))
        });
        self.line_ending = auto_detect_line_ending(&self.text).unwrap_or(DEFAULT_LINE_ENDING);
    }

    /// Reload the document from its path.
    pub fn reload(&mut self, view_id: ViewId) -> Result<(), Error> {
        let encoding = &self.encoding;
        let path = self.path().filter(|path| path.exists());

        // If there is no path or the path no longer exists.
        if path.is_none() {
            bail!("can't find file to reload from");
        }

        let mut file = std::fs::File::open(path.unwrap())?;
        let (rope, ..) = from_reader(&mut file, Some(encoding))?;

        // Calculate the difference between the buffer and source text, and apply it.
        // This is not considered a modification of the contents of the file regardless
        // of the encoding.
        let transaction = helix_core::diff::compare_ropes(self.text(), &rope);
        self.apply(&transaction, view_id);
        self.append_changes_to_history(view_id);
        self.reset_modified();

        self.detect_indent_and_line_ending();

        Ok(())
    }

    /// Sets the [`Document`]'s encoding with the encoding correspondent to `label`.
    pub fn set_encoding(&mut self, label: &str) -> Result<(), Error> {
        self.encoding = encoding::Encoding::for_label(label.as_bytes())
            .ok_or_else(|| anyhow!("unknown encoding"))?;
        Ok(())
    }

    /// Returns the [`Document`]'s current encoding.
    pub fn encoding(&self) -> &'static encoding::Encoding {
        self.encoding
    }

    pub fn set_path(&mut self, path: Option<&Path>) -> Result<(), std::io::Error> {
        let path = path
            .map(helix_core::path::get_canonicalized_path)
            .transpose()?;

        // if parent doesn't exist we still want to open the document
        // and error out when document is saved
        self.path = path;

        Ok(())
    }

    /// Set the programming language for the file and load associated data (e.g. highlighting)
    /// if it exists.
    pub fn set_language(
        &mut self,
        language_config: Option<Arc<helix_core::syntax::LanguageConfiguration>>,
        loader: Option<Arc<helix_core::syntax::Loader>>,
    ) {
        if let (Some(language_config), Some(loader)) = (language_config, loader) {
            if let Some(highlight_config) = language_config.highlight_config(&loader.scopes()) {
                let syntax = Syntax::new(&self.text, highlight_config, loader);
                self.syntax = Some(syntax);
            }

            self.language = Some(language_config);
        } else {
            self.syntax = None;
            self.language = None;
        };
    }

    /// Set the programming language for the file if you know the name (scope) but don't have the
    /// [`syntax::LanguageConfiguration`] for it.
    pub fn set_language2(&mut self, scope: &str, config_loader: Arc<syntax::Loader>) {
        let language_config = config_loader.language_config_for_scope(scope);

        self.set_language(language_config, Some(config_loader));
    }

    /// Set the programming language for the file if you know the language but don't have the
    /// [`syntax::LanguageConfiguration`] for it.
    pub fn set_language_by_language_id(
        &mut self,
        language_id: &str,
        config_loader: Arc<syntax::Loader>,
    ) {
        let language_config = config_loader.language_config_for_language_id(language_id);
        self.set_language(language_config, Some(config_loader));
    }

    /// Set the LSP.
    pub fn set_language_server(&mut self, language_server: Option<Arc<helix_lsp::Client>>) {
        self.language_server = language_server;
    }

    /// Select text within the [`Document`].
    pub fn set_selection(&mut self, view_id: ViewId, selection: Selection) {
        // TODO: use a transaction?
        self.selections
            .insert(view_id, selection.ensure_invariants(self.text().slice(..)));
    }

    /// Find the origin selection of the text in a document, i.e. where
    /// a single cursor would go if it were on the first grapheme. If
    /// the text is empty, returns (0, 0).
    pub fn origin(&self) -> Range {
        if self.text().len_chars() == 0 {
            return Range::new(0, 0);
        }

        Range::new(0, 1).grapheme_aligned(self.text().slice(..))
    }

    /// Reset the view's selection on this document to the
    /// [origin](Document::origin) cursor.
    pub fn reset_selection(&mut self, view_id: ViewId) {
        let origin = self.origin();
        self.set_selection(view_id, Selection::single(origin.anchor, origin.head));
    }

    /// Initializes a new selection for the given view if it does not
    /// already have one.
    pub fn ensure_view_init(&mut self, view_id: ViewId) {
        if self.selections.get(&view_id).is_none() {
            self.reset_selection(view_id);
        }
    }

    /// Remove a view's selection from this document.
    pub fn remove_view(&mut self, view_id: ViewId) {
        self.selections.remove(&view_id);
    }

    /// Apply a [`Transaction`] to the [`Document`] to change its text.
    fn apply_impl(&mut self, transaction: &Transaction, view_id: ViewId) -> bool {
        let old_doc = self.text().clone();

        let success = transaction.changes().apply(&mut self.text);

        if success {
            for selection in self.selections.values_mut() {
                *selection = selection
                    .clone()
                    // Map through changes
                    .map(transaction.changes())
                    // Ensure all selections across all views still adhere to invariants.
                    .ensure_invariants(self.text.slice(..));
            }

            // if specified, the current selection should instead be replaced by transaction.selection
            if let Some(selection) = transaction.selection() {
                self.selections.insert(
                    view_id,
                    selection.clone().ensure_invariants(self.text.slice(..)),
                );
            }

            self.modified_since_accessed = true;
        }

        if !transaction.changes().is_empty() {
            self.version += 1;

            // generate revert to savepoint
            if self.savepoint.is_some() {
                take_with(&mut self.savepoint, |prev_revert| {
                    let revert = transaction.invert(&old_doc);
                    Some(revert.compose(prev_revert.unwrap()))
                });
            }

            // update tree-sitter syntax tree
            if let Some(syntax) = &mut self.syntax {
                // TODO: no unwrap
                syntax
                    .update(&old_doc, &self.text, transaction.changes())
                    .unwrap();
            }

            // map state.diagnostics over changes::map_pos too
            for diagnostic in &mut self.diagnostics {
                use helix_core::Assoc;
                let changes = transaction.changes();
                diagnostic.range.start = changes.map_pos(diagnostic.range.start, Assoc::After);
                diagnostic.range.end = changes.map_pos(diagnostic.range.end, Assoc::After);
                diagnostic.line = self.text.char_to_line(diagnostic.range.start);
            }

            // emit lsp notification
            if let Some(language_server) = self.language_server() {
                let notify = language_server.text_document_did_change(
                    self.versioned_identifier(),
                    &old_doc,
                    self.text(),
                    transaction.changes(),
                );

                if let Some(notify) = notify {
                    tokio::spawn(notify);
                }
            }
        }
        success
    }

    /// Apply a [`Transaction`] to the [`Document`] to change its text.
    pub fn apply(&mut self, transaction: &Transaction, view_id: ViewId) -> bool {
        // store the state just before any changes are made. This allows us to undo to the
        // state just before a transaction was applied.
        if self.changes.is_empty() && !transaction.changes().is_empty() {
            self.old_state = Some(State {
                doc: self.text.clone(),
                selection: self.selection(view_id).clone(),
            });
        }

        let success = self.apply_impl(transaction, view_id);

        if !transaction.changes().is_empty() {
            // Compose this transaction with the previous one
            take_with(&mut self.changes, |changes| {
                changes.compose(transaction.changes().clone())
            });
        }
        success
    }

    fn undo_redo_impl(&mut self, view_id: ViewId, undo: bool) -> bool {
        let mut history = self.history.take();
        let txn = if undo { history.undo() } else { history.redo() };
        let success = if let Some(txn) = txn {
            self.apply_impl(txn, view_id)
        } else {
            false
        };
        self.history.set(history);

        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text());
        }
        success
    }

    /// Undo the last modification to the [`Document`]. Returns whether the undo was successful.
    pub fn undo(&mut self, view_id: ViewId) -> bool {
        self.undo_redo_impl(view_id, true)
    }

    /// Redo the last modification to the [`Document`]. Returns whether the redo was successful.
    pub fn redo(&mut self, view_id: ViewId) -> bool {
        self.undo_redo_impl(view_id, false)
    }

    pub fn savepoint(&mut self) {
        self.savepoint = Some(Transaction::new(self.text()));
    }

    pub fn restore(&mut self, view_id: ViewId) {
        if let Some(revert) = self.savepoint.take() {
            self.apply(&revert, view_id);
        }
    }

    fn earlier_later_impl(&mut self, view_id: ViewId, uk: UndoKind, earlier: bool) -> bool {
        let txns = if earlier {
            self.history.get_mut().earlier(uk)
        } else {
            self.history.get_mut().later(uk)
        };
        let mut success = false;
        for txn in txns {
            if self.apply_impl(&txn, view_id) {
                success = true;
            }
        }
        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text());
        }
        success
    }

    /// Undo modifications to the [`Document`] according to `uk`.
    pub fn earlier(&mut self, view_id: ViewId, uk: UndoKind) -> bool {
        self.earlier_later_impl(view_id, uk, true)
    }

    /// Redo modifications to the [`Document`] according to `uk`.
    pub fn later(&mut self, view_id: ViewId, uk: UndoKind) -> bool {
        self.earlier_later_impl(view_id, uk, false)
    }

    /// Commit pending changes to history
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

    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// If there are unsaved modifications.
    pub fn is_modified(&self) -> bool {
        let history = self.history.take();
        let current_revision = history.current_revision();
        self.history.set(history);
        current_revision != self.last_saved_revision || !self.changes.is_empty()
    }

    /// Save modifications to history, and so [`Self::is_modified`] will return false.
    pub fn reset_modified(&mut self) {
        let history = self.history.take();
        let current_revision = history.current_revision();
        self.history.set(history);
        self.last_saved_revision = current_revision;
    }

    /// Current editing mode for the [`Document`].
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Corresponding language scope name. Usually `source.<lang>`.
    pub fn language(&self) -> Option<&str> {
        self.language
            .as_ref()
            .map(|language| language.scope.as_str())
    }

    /// Language ID for the document. Either the `language-id` from the
    /// `language-server` configuration, or the document language if no
    /// `language-id` has been specified.
    pub fn language_id(&self) -> Option<&str> {
        self.language_config()?
            .language_server
            .as_ref()?
            .language_id
            .as_deref()
            .or_else(|| Some(self.language()?.rsplit_once('.')?.1))
    }

    /// Corresponding [`LanguageConfiguration`].
    pub fn language_config(&self) -> Option<&LanguageConfiguration> {
        self.language.as_deref()
    }

    /// Current document version, incremented at each change.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Language server if it has been initialized.
    pub fn language_server(&self) -> Option<&helix_lsp::Client> {
        let server = self.language_server.as_deref()?;
        server.is_initialized().then(|| server)
    }

    #[inline]
    /// Tree-sitter AST tree
    pub fn syntax(&self) -> Option<&Syntax> {
        self.syntax.as_ref()
    }

    /// Tab size in columns.
    pub fn tab_width(&self) -> usize {
        self.language_config()
            .and_then(|config| config.indent.as_ref())
            .map_or(4, |config| config.tab_width) // fallback to 4 columns
    }

    /// Returns a string containing a single level of indentation.
    ///
    /// TODO: we might not need this function anymore, since the information
    /// is conveniently available in `Document::indent_style` now.
    pub fn indent_unit(&self) -> &'static str {
        self.indent_style.as_str()
    }

    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    #[inline]
    /// File path on disk.
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    /// File path as a URL.
    pub fn url(&self) -> Option<Url> {
        Url::from_file_path(self.path()?).ok()
    }

    #[inline]
    pub fn text(&self) -> &Rope {
        &self.text
    }

    #[inline]
    pub fn selection(&self, view_id: ViewId) -> &Selection {
        &self.selections[&view_id]
    }

    pub fn selections(&self) -> &HashMap<ViewId, Selection> {
        &self.selections
    }

    pub fn relative_path(&self) -> Option<PathBuf> {
        self.path
            .as_deref()
            .map(helix_core::path::get_relative_path)
    }

    // transact(Fn) ?

    // -- LSP methods

    #[inline]
    pub fn identifier(&self) -> lsp::TextDocumentIdentifier {
        lsp::TextDocumentIdentifier::new(self.url().unwrap())
    }

    pub fn versioned_identifier(&self) -> lsp::VersionedTextDocumentIdentifier {
        lsp::VersionedTextDocumentIdentifier::new(self.url().unwrap(), self.version)
    }

    pub fn position(
        &self,
        view_id: ViewId,
        offset_encoding: helix_lsp::OffsetEncoding,
    ) -> lsp::Position {
        let text = self.text();

        helix_lsp::util::pos_to_lsp_pos(
            text,
            self.selection(view_id).primary().cursor(text.slice(..)),
            offset_encoding,
        )
    }

    #[inline]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics = diagnostics;
        self.diagnostics
            .sort_unstable_by_key(|diagnostic| diagnostic.range);
    }

    /// Get the document's auto pairs. If the document has a recognized
    /// language config with auto pairs configured, returns that;
    /// otherwise, falls back to the global auto pairs config. If the global
    /// config is false, then ignore language settings.
    pub fn auto_pairs<'a>(&'a self, editor: &'a Editor) -> Option<&'a AutoPairs> {
        let global_config = (editor.auto_pairs).as_ref();

        // NOTE: If the user specifies the global auto pairs config as false, then
        //       we want to disable it globally regardless of language settings
        #[allow(clippy::question_mark)]
        {
            if global_config.is_none() {
                return None;
            }
        }

        match &self.language {
            Some(lang) => lang.as_ref().auto_pairs.as_ref().or(global_config),
            None => global_config,
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        let text = Rope::from(DEFAULT_LINE_ENDING.as_str());
        Self::from(text, None)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn changeset_to_changes_ignore_line_endings() {
        use helix_lsp::{lsp, Client, OffsetEncoding};
        let text = Rope::from("hello\r\nworld");
        let mut doc = Document::from(text, None);
        let view = ViewId::default();
        doc.set_selection(view, Selection::single(0, 0));

        let transaction =
            Transaction::change(doc.text(), vec![(5, 7, Some("\n".into()))].into_iter());
        let old_doc = doc.text().clone();
        doc.apply(&transaction, view);
        let changes = Client::changeset_to_changes(
            &old_doc,
            doc.text(),
            transaction.changes(),
            OffsetEncoding::Utf8,
        );

        assert_eq!(doc.text(), "hello\nworld");

        assert_eq!(
            changes,
            &[lsp::TextDocumentContentChangeEvent {
                range: Some(lsp::Range::new(
                    lsp::Position::new(0, 5),
                    lsp::Position::new(1, 0)
                )),
                text: "\n".into(),
                range_length: None,
            }]
        );
    }

    #[test]
    fn changeset_to_changes() {
        use helix_lsp::{lsp, Client, OffsetEncoding};
        let text = Rope::from("hello");
        let mut doc = Document::from(text, None);
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

    #[test]
    fn test_line_ending() {
        assert_eq!(
            Document::default().text().to_string(),
            DEFAULT_LINE_ENDING.as_str()
        );
    }

    macro_rules! test_decode {
        ($label:expr, $label_override:expr) => {
            let encoding = encoding::Encoding::for_label($label_override.as_bytes()).unwrap();
            let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/encoding");
            let path = base_path.join(format!("{}_in.txt", $label));
            let ref_path = base_path.join(format!("{}_in_ref.txt", $label));
            assert!(path.exists());
            assert!(ref_path.exists());

            let mut file = std::fs::File::open(path).unwrap();
            let text = from_reader(&mut file, Some(encoding))
                .unwrap()
                .0
                .to_string();
            let expectation = std::fs::read_to_string(ref_path).unwrap();
            assert_eq!(text[..], expectation[..]);
        };
    }

    macro_rules! test_encode {
        ($label:expr, $label_override:expr) => {
            let encoding = encoding::Encoding::for_label($label_override.as_bytes()).unwrap();
            let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/encoding");
            let path = base_path.join(format!("{}_out.txt", $label));
            let ref_path = base_path.join(format!("{}_out_ref.txt", $label));
            assert!(path.exists());
            assert!(ref_path.exists());

            let text = Rope::from_str(&std::fs::read_to_string(path).unwrap());
            let mut buf: Vec<u8> = Vec::new();
            helix_lsp::block_on(to_writer(&mut buf, encoding, &text)).unwrap();

            let expectation = std::fs::read(ref_path).unwrap();
            assert_eq!(buf, expectation);
        };
    }

    macro_rules! test_decode_fn {
        ($name:ident, $label:expr, $label_override:expr) => {
            #[test]
            fn $name() {
                test_decode!($label, $label_override);
            }
        };
        ($name:ident, $label:expr) => {
            #[test]
            fn $name() {
                test_decode!($label, $label);
            }
        };
    }

    macro_rules! test_encode_fn {
        ($name:ident, $label:expr, $label_override:expr) => {
            #[test]
            fn $name() {
                test_encode!($label, $label_override);
            }
        };
        ($name:ident, $label:expr) => {
            #[test]
            fn $name() {
                test_encode!($label, $label);
            }
        };
    }

    test_decode_fn!(test_big5_decode, "big5");
    test_encode_fn!(test_big5_encode, "big5");
    test_decode_fn!(test_euc_kr_decode, "euc_kr", "EUC-KR");
    test_encode_fn!(test_euc_kr_encode, "euc_kr", "EUC-KR");
    test_decode_fn!(test_gb18030_decode, "gb18030");
    test_encode_fn!(test_gb18030_encode, "gb18030");
    test_decode_fn!(test_iso_2022_jp_decode, "iso_2022_jp", "ISO-2022-JP");
    test_encode_fn!(test_iso_2022_jp_encode, "iso_2022_jp", "ISO-2022-JP");
    test_decode_fn!(test_jis0208_decode, "jis0208", "EUC-JP");
    test_encode_fn!(test_jis0208_encode, "jis0208", "EUC-JP");
    test_decode_fn!(test_jis0212_decode, "jis0212", "EUC-JP");
    test_decode_fn!(test_shift_jis_decode, "shift_jis");
    test_encode_fn!(test_shift_jis_encode, "shift_jis");
}
