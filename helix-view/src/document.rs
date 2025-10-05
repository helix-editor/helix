use anyhow::{anyhow, bail, Error};
use arc_swap::access::DynAccess;
use arc_swap::ArcSwap;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use helix_core::auto_pairs::AutoPairs;
use helix_core::chars::char_is_word;
use helix_core::command_line::Token;
use helix_core::diagnostic::DiagnosticProvider;
use helix_core::doc_formatter::TextFormat;
use helix_core::encoding::Encoding;
use helix_core::snippets::{ActiveSnippet, SnippetRenderCtx};
use helix_core::syntax::config::LanguageServerFeature;
use helix_core::text_annotations::{InlineAnnotation, Overlay};
use helix_event::TaskController;
use helix_lsp::util::lsp_pos_to_pos;
use helix_stdx::faccess::{copy_metadata, readonly};
use helix_vcs::{DiffHandle, DiffProviderRegistry};
use once_cell::sync::OnceCell;
use thiserror;

use ::parking_lot::Mutex;
use serde::de::{self, Deserialize, Deserializer};
use serde::Serialize;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Weak};
use std::time::SystemTime;

use helix_core::{
    editor_config::EditorConfig,
    encoding,
    history::{History, State, UndoKind},
    indent::{auto_detect_indent_style, IndentStyle},
    line_ending::auto_detect_line_ending,
    syntax::{self, config::LanguageConfiguration},
    ChangeSet, Diagnostic, LineEnding, Range, Rope, RopeBuilder, Selection, Syntax, Transaction,
};

use crate::{
    editor::Config,
    events::{DocumentDidChange, SelectionDidChange},
    expansion,
    view::ViewPosition,
    DocumentId, Editor, Theme, View, ViewId,
};

/// 8kB of buffer space for encoding and decoding `Rope`s.
const BUF_SIZE: usize = 8192;

const DEFAULT_INDENT: IndentStyle = IndentStyle::Tabs;
const DEFAULT_TAB_WIDTH: usize = 4;

pub const DEFAULT_LANGUAGE_NAME: &str = "text";

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
/// A snapshot of the text of a document that we want to write out to disk
#[derive(Debug, Clone)]
pub struct DocumentSavedEvent {
    pub revision: usize,
    pub save_time: SystemTime,
    pub doc_id: DocumentId,
    pub path: PathBuf,
    pub text: Rope,
}

pub type DocumentSavedEventResult = Result<DocumentSavedEvent, anyhow::Error>;
pub type DocumentSavedEventFuture = BoxFuture<'static, DocumentSavedEventResult>;

#[derive(Debug)]
pub struct SavePoint {
    /// The view this savepoint is associated with
    pub view: ViewId,
    revert: Mutex<Transaction>,
}

#[derive(Debug, thiserror::Error)]
pub enum DocumentOpenError {
    #[error("path must be a regular file, symlink, or directory")]
    IrregularFile,
    #[error(transparent)]
    IoError(#[from] io::Error),
}

pub struct Document {
    pub(crate) id: DocumentId,
    text: Rope,
    selections: HashMap<ViewId, Selection>,
    view_data: HashMap<ViewId, ViewData>,
    pub active_snippet: Option<ActiveSnippet>,

    /// Inlay hints annotations for the document, by view.
    ///
    /// To know if they're up-to-date, check the `id` field in `DocumentInlayHints`.
    pub(crate) inlay_hints: HashMap<ViewId, DocumentInlayHints>,
    pub(crate) jump_labels: HashMap<ViewId, Vec<Overlay>>,
    /// Set to `true` when the document is updated, reset to `false` on the next inlay hints
    /// update from the LSP
    pub inlay_hints_oudated: bool,

    path: Option<PathBuf>,
    relative_path: OnceCell<Option<PathBuf>>,
    encoding: &'static encoding::Encoding,
    has_bom: bool,

    pub restore_cursor: bool,

    /// Current indent style.
    pub indent_style: IndentStyle,
    editor_config: EditorConfig,

    /// The document's default line ending.
    pub line_ending: LineEnding,

    pub syntax: Option<Syntax>,
    /// Corresponding language scope name. Usually `source.<lang>`.
    pub language: Option<Arc<LanguageConfiguration>>,

    /// Pending changes since last history commit.
    changes: ChangeSet,
    /// State at last commit. Used for calculating reverts.
    old_state: Option<State>,
    /// Undo tree.
    // It can be used as a cell where we will take it out to get some parts of the history and put
    // it back as it separated from the edits. We could split out the parts manually but that will
    // be more troublesome.
    pub history: Cell<History>,
    pub config: Arc<dyn DynAccess<Config>>,

    savepoints: Vec<Weak<SavePoint>>,

    // Last time we wrote to the file. This will carry the time the file was last opened if there
    // were no saves.
    pub last_saved_time: SystemTime,

    last_saved_revision: usize,
    version: i32, // should be usize?
    pub(crate) modified_since_accessed: bool,

    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) language_servers: HashMap<LanguageServerName, Arc<Client>>,

    pub diff_handle: Option<DiffHandle>,
    version_control_head: Option<Arc<ArcSwap<Box<str>>>>,

    // when document was used for most-recent-used buffer picker
    pub focused_at: std::time::Instant,

    pub readonly: bool,

    pub previous_diagnostic_id: Option<String>,

    /// Annotations for LSP document color swatches
    pub color_swatches: Option<DocumentColorSwatches>,
    // NOTE: ideally this would live on the handler for color swatches. This is blocked on a
    // large refactor that would make `&mut Editor` available on the `DocumentDidChange` event.
    pub color_swatch_controller: TaskController,
    pub pull_diagnostic_controller: TaskController,

    // NOTE: this field should eventually go away - we should use the Editor's syn_loader instead
    // of storing a copy on every doc. Then we can remove the surrounding `Arc` and use the
    // `ArcSwap` directly.
    syn_loader: Arc<ArcSwap<syntax::Loader>>,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentColorSwatches {
    pub color_swatches: Vec<InlineAnnotation>,
    pub colors: Vec<syntax::Highlight>,
    pub color_swatches_padding: Vec<InlineAnnotation>,
}

/// Inlay hints for a single `(Document, View)` combo.
///
/// There are `*_inlay_hints` field for each kind of hints an LSP can send since we offer the
/// option to style theme differently in the theme according to the (currently supported) kinds
/// (`type`, `parameter` and the rest).
///
/// Inlay hints are always `InlineAnnotation`s, not overlays or line-ones: LSP may choose to place
/// them anywhere in the text and will sometime offer config options to move them where the user
/// wants them but it shouldn't be Helix who decides that so we use the most precise positioning.
///
/// The padding for inlay hints needs to be stored separately for before and after (the LSP spec
/// uses 'left' and 'right' but not all text is left to right so let's be correct) padding because
/// the 'before' padding must be added to a layer *before* the regular inlay hints and the 'after'
/// padding comes ... after.
#[derive(Debug, Clone)]
pub struct DocumentInlayHints {
    /// Identifier for the inlay hints stored in this structure. To be checked to know if they have
    /// to be recomputed on idle or not.
    pub id: DocumentInlayHintsId,

    /// Inlay hints of `TYPE` kind, if any.
    pub type_inlay_hints: Vec<InlineAnnotation>,

    /// Inlay hints of `PARAMETER` kind, if any.
    pub parameter_inlay_hints: Vec<InlineAnnotation>,

    /// Inlay hints that are neither `TYPE` nor `PARAMETER`.
    ///
    /// LSPs are not required to associate a kind to their inlay hints, for example Rust-Analyzer
    /// currently never does (February 2023) and the LSP spec may add new kinds in the future that
    /// we want to display even if we don't have some special highlighting for them.
    pub other_inlay_hints: Vec<InlineAnnotation>,

    /// Inlay hint padding. When creating the final `TextAnnotations`, the `before` padding must be
    /// added first, then the regular inlay hints, then the `after` padding.
    pub padding_before_inlay_hints: Vec<InlineAnnotation>,
    pub padding_after_inlay_hints: Vec<InlineAnnotation>,
}

impl DocumentInlayHints {
    /// Generate an empty list of inlay hints with the given ID.
    pub fn empty_with_id(id: DocumentInlayHintsId) -> Self {
        Self {
            id,
            type_inlay_hints: Vec::new(),
            parameter_inlay_hints: Vec::new(),
            other_inlay_hints: Vec::new(),
            padding_before_inlay_hints: Vec::new(),
            padding_after_inlay_hints: Vec::new(),
        }
    }
}

/// Associated with a [`Document`] and [`ViewId`], uniquely identifies the state of inlay hints for
/// for that document and view: if this changed since the last save, the inlay hints for the view
/// should be recomputed.
///
/// We can't store the `ViewOffset` instead of the first and last asked-for lines because if
/// softwrapping changes, the `ViewOffset` may not change while the displayed lines will.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct DocumentInlayHintsId {
    /// First line for which the inlay hints were requested.
    pub first_line: usize,
    /// Last line for which the inlay hints were requested.
    pub last_line: usize,
}

use std::{fmt, mem};
impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("id", &self.id)
            .field("text", &self.text)
            .field("selections", &self.selections)
            .field("inlay_hints_oudated", &self.inlay_hints_oudated)
            .field("text_annotations", &self.inlay_hints)
            .field("view_data", &self.view_data)
            .field("path", &self.path)
            .field("encoding", &self.encoding)
            .field("restore_cursor", &self.restore_cursor)
            .field("syntax", &self.syntax)
            .field("language", &self.language)
            .field("changes", &self.changes)
            .field("old_state", &self.old_state)
            // .field("history", &self.history)
            .field("last_saved_time", &self.last_saved_time)
            .field("last_saved_revision", &self.last_saved_revision)
            .field("version", &self.version)
            .field("modified_since_accessed", &self.modified_since_accessed)
            .field("diagnostics", &self.diagnostics)
            // .field("language_server", &self.language_server)
            .finish()
    }
}

impl fmt::Debug for DocumentInlayHintsId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Much more agreable to read when debugging
        f.debug_struct("DocumentInlayHintsId")
            .field("lines", &(self.first_line..self.last_line))
            .finish()
    }
}

impl Editor {
    pub(crate) fn clear_doc_relative_paths(&mut self) {
        for doc in self.documents_mut() {
            doc.relative_path.take();
        }
    }
}

enum Encoder {
    Utf16Be,
    Utf16Le,
    EncodingRs(encoding::Encoder),
}

impl Encoder {
    fn from_encoding(encoding: &'static encoding::Encoding) -> Self {
        if encoding == encoding::UTF_16BE {
            Self::Utf16Be
        } else if encoding == encoding::UTF_16LE {
            Self::Utf16Le
        } else {
            Self::EncodingRs(encoding.new_encoder())
        }
    }

    fn encode_from_utf8(
        &mut self,
        src: &str,
        dst: &mut [u8],
        is_empty: bool,
    ) -> (encoding::CoderResult, usize, usize) {
        if src.is_empty() {
            return (encoding::CoderResult::InputEmpty, 0, 0);
        }
        let mut write_to_buf = |convert: fn(u16) -> [u8; 2]| {
            let to_write = src.char_indices().map(|(indice, char)| {
                let mut encoded: [u16; 2] = [0, 0];
                (
                    indice,
                    char.encode_utf16(&mut encoded)
                        .iter_mut()
                        .flat_map(|char| convert(*char))
                        .collect::<Vec<u8>>(),
                )
            });

            let mut total_written = 0usize;

            for (indice, utf16_bytes) in to_write {
                let character_size = utf16_bytes.len();

                if dst.len() <= (total_written + character_size) {
                    return (encoding::CoderResult::OutputFull, indice, total_written);
                }

                for character in utf16_bytes {
                    dst[total_written] = character;
                    total_written += 1;
                }
            }

            (encoding::CoderResult::InputEmpty, src.len(), total_written)
        };

        match self {
            Self::Utf16Be => write_to_buf(u16::to_be_bytes),
            Self::Utf16Le => write_to_buf(u16::to_le_bytes),
            Self::EncodingRs(encoder) => {
                let (code_result, read, written, ..) = encoder.encode_from_utf8(src, dst, is_empty);

                (code_result, read, written)
            }
        }
    }
}

// Apply BOM if encoding permit it, return the number of bytes written at the start of buf
fn apply_bom(encoding: &'static encoding::Encoding, buf: &mut [u8; BUF_SIZE]) -> usize {
    if encoding == encoding::UTF_8 {
        buf[0] = 0xef;
        buf[1] = 0xbb;
        buf[2] = 0xbf;
        3
    } else if encoding == encoding::UTF_16BE {
        buf[0] = 0xfe;
        buf[1] = 0xff;
        2
    } else if encoding == encoding::UTF_16LE {
        buf[0] = 0xff;
        buf[1] = 0xfe;
        2
    } else {
        0
    }
}

// The documentation and implementation of this function should be up-to-date with
// its sibling function, `to_writer()`.
//
/// Decodes a stream of bytes into UTF-8, returning a `Rope` and the
/// encoding it was decoded as with BOM information. The optional `encoding`
/// parameter can be used to override encoding auto-detection.
pub fn from_reader<R: std::io::Read + ?Sized>(
    reader: &mut R,
    encoding: Option<&'static Encoding>,
) -> Result<(Rope, &'static Encoding, bool), io::Error> {
    // These two buffers are 8192 bytes in size each and are used as
    // intermediaries during the decoding process. Text read into `buf`
    // from `reader` is decoded into `buf_out` as UTF-8. Once either
    // `buf_out` is full or the end of the reader was reached, the
    // contents are appended to `builder`.
    let mut buf = [0u8; BUF_SIZE];
    let mut buf_out = [0u8; BUF_SIZE];
    let mut builder = RopeBuilder::new();

    let (encoding, has_bom, mut decoder, read) =
        read_and_detect_encoding(reader, encoding, &mut buf)?;

    let mut slice = &buf[..read];
    let mut is_empty = read == 0;

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
    Ok((rope, encoding, has_bom))
}

pub fn read_to_string<R: std::io::Read + ?Sized>(
    reader: &mut R,
    encoding: Option<&'static Encoding>,
) -> Result<(String, &'static Encoding, bool), Error> {
    let mut buf = [0u8; BUF_SIZE];

    let (encoding, has_bom, mut decoder, read) =
        read_and_detect_encoding(reader, encoding, &mut buf)?;

    let mut slice = &buf[..read];
    let mut is_empty = read == 0;
    let mut buf_string = String::with_capacity(buf.len());

    loop {
        let mut total_read = 0usize;

        loop {
            let (result, read, ..) =
                decoder.decode_to_string(&slice[total_read..], &mut buf_string, is_empty);

            total_read += read;

            match result {
                encoding::CoderResult::InputEmpty => {
                    debug_assert_eq!(slice.len(), total_read);
                    break;
                }
                encoding::CoderResult::OutputFull => {
                    debug_assert!(slice.len() > total_read);
                    buf_string.reserve(buf.len())
                }
            }
        }

        if is_empty {
            debug_assert_eq!(reader.read(&mut buf)?, 0);
            break;
        }

        let read = reader.read(&mut buf)?;
        slice = &buf[..read];
        is_empty = read == 0;
    }
    Ok((buf_string, encoding, has_bom))
}

/// Reads the first chunk from a Reader into the given buffer
/// and detects the encoding.
///
/// By default, the encoding of the text is auto-detected by
/// `encoding_rs` for_bom, and if it fails, from `chardetng`
/// crate which requires sample data from the reader.
/// As a manual override to this auto-detection is possible, the
/// same data is read into `buf` to ensure symmetry in the upcoming
/// loop.
fn read_and_detect_encoding<R: std::io::Read + ?Sized>(
    reader: &mut R,
    encoding: Option<&'static Encoding>,
    buf: &mut [u8],
) -> Result<(&'static Encoding, bool, encoding::Decoder, usize), io::Error> {
    let read = reader.read(buf)?;
    let is_empty = read == 0;
    let (encoding, has_bom) = encoding
        .map(|encoding| (encoding, false))
        .or_else(|| encoding::Encoding::for_bom(buf).map(|(encoding, _bom_size)| (encoding, true)))
        .unwrap_or_else(|| {
            let mut encoding_detector = chardetng::EncodingDetector::new();
            encoding_detector.feed(buf, is_empty);
            (encoding_detector.guess(None, true), false)
        });
    let decoder = encoding.new_decoder();

    Ok((encoding, has_bom, decoder, read))
}

// The documentation and implementation of this function should be up-to-date with
// its sibling function, `from_reader()`.
//
/// Encodes the text inside `rope` into the given `encoding` and writes the
/// encoded output into `writer.` As a `Rope` can only contain valid UTF-8,
/// replacement characters may appear in the encoded text.
pub async fn to_writer<'a, W: tokio::io::AsyncWriteExt + Unpin + ?Sized>(
    writer: &'a mut W,
    encoding_with_bom_info: (&'static Encoding, bool),
    rope: &'a Rope,
) -> Result<(), Error> {
    // Text inside a `Rope` is stored as non-contiguous blocks of data called
    // chunks. The absolute size of each chunk is unknown, thus it is impossible
    // to predict the end of the chunk iterator ahead of time. Instead, it is
    // determined by filtering the iterator to remove all empty chunks and then
    // appending an empty chunk to it. This is valuable for detecting when all
    // chunks in the `Rope` have been iterated over in the subsequent loop.
    let (encoding, has_bom) = encoding_with_bom_info;

    let iter = rope
        .chunks()
        .filter(|c| !c.is_empty())
        .chain(std::iter::once(""));
    let mut buf = [0u8; BUF_SIZE];

    let mut total_written = if has_bom {
        apply_bom(encoding, &mut buf)
    } else {
        0
    };

    let mut encoder = Encoder::from_encoding(encoding);

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

use helix_lsp::{lsp, Client, LanguageServerId, LanguageServerName};
use url::Url;

impl Document {
    pub fn from(
        text: Rope,
        encoding_with_bom_info: Option<(&'static Encoding, bool)>,
        config: Arc<dyn DynAccess<Config>>,
        syn_loader: Arc<ArcSwap<syntax::Loader>>,
    ) -> Self {
        let (encoding, has_bom) = encoding_with_bom_info.unwrap_or((encoding::UTF_8, false));
        let line_ending = config.load().default_line_ending.into();
        let changes = ChangeSet::new(text.slice(..));
        let old_state = None;

        Self {
            id: DocumentId::default(),
            active_snippet: None,
            path: None,
            relative_path: OnceCell::new(),
            encoding,
            has_bom,
            text,
            selections: HashMap::default(),
            inlay_hints: HashMap::default(),
            inlay_hints_oudated: false,
            view_data: Default::default(),
            indent_style: DEFAULT_INDENT,
            editor_config: EditorConfig::default(),
            line_ending,
            restore_cursor: false,
            syntax: None,
            language: None,
            changes,
            old_state,
            diagnostics: Vec::new(),
            version: 0,
            history: Cell::new(History::default()),
            savepoints: Vec::new(),
            last_saved_time: SystemTime::now(),
            last_saved_revision: 0,
            modified_since_accessed: false,
            language_servers: HashMap::new(),
            diff_handle: None,
            config,
            version_control_head: None,
            focused_at: std::time::Instant::now(),
            readonly: false,
            jump_labels: HashMap::new(),
            color_swatches: None,
            color_swatch_controller: TaskController::new(),
            syn_loader,
            previous_diagnostic_id: None,
            pull_diagnostic_controller: TaskController::new(),
        }
    }

    pub fn default(
        config: Arc<dyn DynAccess<Config>>,
        syn_loader: Arc<ArcSwap<syntax::Loader>>,
    ) -> Self {
        let line_ending: LineEnding = config.load().default_line_ending.into();
        let text = Rope::from(line_ending.as_str());
        Self::from(text, None, config, syn_loader)
    }

    // TODO: async fn?
    /// Create a new document from `path`. Encoding is auto-detected, but it can be manually
    /// overwritten with the `encoding` parameter.
    pub fn open(
        path: &Path,
        mut encoding: Option<&'static Encoding>,
        detect_language: bool,
        config: Arc<dyn DynAccess<Config>>,
        syn_loader: Arc<ArcSwap<syntax::Loader>>,
    ) -> Result<Self, DocumentOpenError> {
        // If the path is not a regular file (e.g.: /dev/random) it should not be opened.
        if path.metadata().is_ok_and(|metadata| !metadata.is_file()) {
            return Err(DocumentOpenError::IrregularFile);
        }

        let editor_config = if config.load().editor_config {
            EditorConfig::find(path)
        } else {
            EditorConfig::default()
        };
        encoding = encoding.or(editor_config.encoding);

        // Open the file if it exists, otherwise assume it is a new file (and thus empty).
        let (rope, encoding, has_bom) = if path.exists() {
            let mut file = std::fs::File::open(path)?;
            from_reader(&mut file, encoding)?
        } else {
            let line_ending = editor_config
                .line_ending
                .unwrap_or_else(|| config.load().default_line_ending.into());
            let encoding = encoding.unwrap_or(encoding::UTF_8);
            (Rope::from(line_ending.as_str()), encoding, false)
        };

        let loader = syn_loader.load();
        let mut doc = Self::from(rope, Some((encoding, has_bom)), config, syn_loader);

        // set the path and try detecting the language
        doc.set_path(Some(path));
        if detect_language {
            doc.detect_language(&loader);
        }

        doc.editor_config = editor_config;
        doc.detect_indent_and_line_ending();

        Ok(doc)
    }

    /// The same as [`format`], but only returns formatting changes if auto-formatting
    /// is configured.
    pub fn auto_format(
        &self,
        editor: &Editor,
    ) -> Option<BoxFuture<'static, Result<Transaction, FormatterError>>> {
        if self.language_config()?.auto_format {
            self.format(editor)
        } else {
            None
        }
    }

    /// If supported, returns the changes that should be applied to this document in order
    /// to format it nicely.
    // We can't use anyhow::Result here since the output of the future has to be
    // clonable to be used as shared future. So use a custom error type.
    pub fn format(
        &self,
        editor: &Editor,
    ) -> Option<BoxFuture<'static, Result<Transaction, FormatterError>>> {
        if let Some((fmt_cmd, fmt_args)) = self
            .language_config()
            .and_then(|c| c.formatter.as_ref())
            .and_then(|formatter| {
                Some((
                    helix_stdx::env::which(&formatter.command).ok()?,
                    &formatter.args,
                ))
            })
        {
            log::debug!(
                "formatting '{}' with command '{}', args {fmt_args:?}",
                self.display_name(),
                fmt_cmd.display(),
            );
            use std::process::Stdio;
            let text = self.text().clone();

            let mut process = tokio::process::Command::new(&fmt_cmd);

            if let Some(doc_dir) = self.path.as_ref().and_then(|path| path.parent()) {
                process.current_dir(doc_dir);
            }

            let args = match fmt_args
                .iter()
                .map(|content| expansion::expand(editor, Token::expand(content)))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(args) => args,
                Err(err) => {
                    log::error!("Failed to expand formatter arguments: {err}");
                    return None;
                }
            };

            process
                .args(args.iter().map(AsRef::as_ref))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let formatting_future = async move {
                let mut process = process
                    .spawn()
                    .map_err(|e| FormatterError::SpawningFailed {
                        command: fmt_cmd.to_string_lossy().into(),
                        error: e.kind(),
                    })?;

                let mut stdin = process.stdin.take().ok_or(FormatterError::BrokenStdin)?;
                let input_text = text.clone();
                let input_task = tokio::spawn(async move {
                    to_writer(&mut stdin, (encoding::UTF_8, false), &input_text).await
                    // Note that `stdin` is dropped here, causing the pipe to close. This can
                    // avoid a deadlock with `wait_with_output` below if the process is waiting on
                    // stdin to close before exiting.
                });
                let (input_result, output_result) = tokio::join! {
                    input_task,
                    process.wait_with_output(),
                };
                let _ = input_result.map_err(|_| FormatterError::BrokenStdin)?;
                let output = output_result.map_err(|_| FormatterError::WaitForOutputFailed)?;

                if !output.status.success() {
                    if !output.stderr.is_empty() {
                        let err = String::from_utf8_lossy(&output.stderr).to_string();
                        log::error!("Formatter error: {}", err);
                        return Err(FormatterError::NonZeroExitStatus(Some(err)));
                    }

                    return Err(FormatterError::NonZeroExitStatus(None));
                } else if !output.stderr.is_empty() {
                    log::debug!(
                        "Formatter printed to stderr: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }

                let str = std::str::from_utf8(&output.stdout)
                    .map_err(|_| FormatterError::InvalidUtf8Output)?;

                Ok(helix_core::diff::compare_ropes(&text, &Rope::from(str)))
            };
            return Some(formatting_future.boxed());
        };

        let text = self.text.clone();
        // finds first language server that supports formatting and then formats
        let language_server = self
            .language_servers_with_feature(LanguageServerFeature::Format)
            .next()?;
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
            let edits = request
                .await
                .unwrap_or_else(|e| {
                    log::warn!("LSP formatting failed: {}", e);
                    Default::default()
                })
                .unwrap_or_default();
            Ok(helix_lsp::util::generate_transaction_from_edits(
                &text,
                edits,
                offset_encoding,
            ))
        };
        Some(fut.boxed())
    }

    pub fn save<P: Into<PathBuf>>(
        &mut self,
        path: Option<P>,
        force: bool,
    ) -> Result<
        impl Future<Output = Result<DocumentSavedEvent, anyhow::Error>> + 'static + Send,
        anyhow::Error,
    > {
        let path = path.map(|path| path.into());
        self.save_impl(path, force)

        // futures_util::future::Ready<_>,
    }

    /// The `Document`'s text is encoded according to its encoding and written to the file located
    /// at its `path()`.
    fn save_impl(
        &mut self,
        path: Option<PathBuf>,
        force: bool,
    ) -> Result<
        impl Future<Output = Result<DocumentSavedEvent, anyhow::Error>> + 'static + Send,
        anyhow::Error,
    > {
        log::debug!(
            "submitting save of doc '{:?}'",
            self.path().map(|path| path.to_string_lossy())
        );

        // we clone and move text + path into the future so that we asynchronously save the current
        // state without blocking any further edits.
        let text = self.text().clone();

        let path = match path {
            Some(path) => helix_stdx::path::canonicalize(path),
            None => {
                if self.path.is_none() {
                    bail!("Can't save with no path set!");
                }
                self.path.as_ref().unwrap().clone()
            }
        };

        let identifier = self.path().map(|_| self.identifier());
        let language_servers: Vec<_> = self.language_servers.values().cloned().collect();

        // mark changes up to now as saved
        let current_rev = self.get_current_revision();
        let doc_id = self.id();
        let atomic_save = self.config.load().atomic_save;

        let encoding_with_bom_info = (self.encoding, self.has_bom);
        let last_saved_time = self.last_saved_time;

        // We encode the file according to the `Document`'s encoding.
        let future = async move {
            use tokio::fs;
            if let Some(parent) = path.parent() {
                // TODO: display a prompt asking the user if the directories should be created
                if !parent.exists() {
                    if force {
                        std::fs::DirBuilder::new().recursive(true).create(parent)?;
                    } else {
                        bail!("can't save file, parent directory does not exist (use :w! to create it)");
                    }
                }
            }

            // Protect against overwriting changes made externally
            if !force {
                if let Ok(metadata) = fs::metadata(&path).await {
                    if let Ok(mtime) = metadata.modified() {
                        if last_saved_time < mtime {
                            bail!("file modified by an external process, use :w! to overwrite");
                        }
                    }
                }
            }
            let write_path = tokio::fs::read_link(&path)
                .await
                .ok()
                .and_then(|p| {
                    if p.is_relative() {
                        path.parent().map(|parent| parent.join(p))
                    } else {
                        Some(p)
                    }
                })
                .unwrap_or_else(|| path.clone());

            if readonly(&write_path) {
                bail!(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Path is read only"
                ));
            }

            // Assume it is a hardlink to prevent data loss if the metadata cant be read (e.g. on certain Windows configurations)
            let is_hardlink = helix_stdx::faccess::hardlink_count(&write_path).unwrap_or(2) > 1;
            let backup = if path.exists() && atomic_save {
                let path_ = write_path.clone();
                // hacks: we use tempfile to handle the complex task of creating
                // non clobbered temporary path for us we don't want
                // the whole automatically delete path on drop thing
                // since the path doesn't exist yet, we just want
                // the path
                tokio::task::spawn_blocking(move || -> Option<PathBuf> {
                    let mut builder = tempfile::Builder::new();
                    builder.prefix(path_.file_name()?).suffix(".bck");

                    let backup_path = if is_hardlink {
                        builder
                            .make_in(path_.parent()?, |backup| std::fs::copy(&path_, backup))
                            .ok()?
                            .into_temp_path()
                    } else {
                        builder
                            .make_in(path_.parent()?, |backup| std::fs::rename(&path_, backup))
                            .ok()?
                            .into_temp_path()
                    };

                    backup_path.keep().ok()
                })
                .await
                .ok()
                .flatten()
            } else {
                None
            };

            let write_result: anyhow::Result<_> = async {
                let mut dst = tokio::fs::File::create(&write_path).await?;
                to_writer(&mut dst, encoding_with_bom_info, &text).await?;
                dst.sync_all().await?;
                Ok(())
            }
            .await;

            let save_time = match fs::metadata(&write_path).await {
                Ok(metadata) => metadata.modified().map_or(SystemTime::now(), |mtime| mtime),
                Err(_) => SystemTime::now(),
            };

            if let Some(backup) = backup {
                if is_hardlink {
                    let mut delete = true;
                    if write_result.is_err() {
                        // Restore backup
                        let _ = tokio::fs::copy(&backup, &write_path).await.map_err(|e| {
                            delete = false;
                            log::error!("Failed to restore backup on write failure: {e}")
                        });
                    }

                    if delete {
                        // Delete backup
                        let _ = tokio::fs::remove_file(backup)
                            .await
                            .map_err(|e| log::error!("Failed to remove backup file on write: {e}"));
                    }
                } else if write_result.is_err() {
                    // restore backup
                    let _ = tokio::fs::rename(&backup, &write_path)
                        .await
                        .map_err(|e| log::error!("Failed to restore backup on write failure: {e}"));
                } else {
                    // copy metadata and delete backup
                    let _ = tokio::task::spawn_blocking(move || {
                        let _ = copy_metadata(&backup, &write_path)
                            .map_err(|e| log::error!("Failed to copy metadata on write: {e}"));
                        let _ = std::fs::remove_file(backup)
                            .map_err(|e| log::error!("Failed to remove backup file on write: {e}"));
                    })
                    .await;
                }
            }

            write_result?;

            let event = DocumentSavedEvent {
                revision: current_rev,
                save_time,
                doc_id,
                path,
                text: text.clone(),
            };

            for language_server in language_servers {
                if !language_server.is_initialized() {
                    continue;
                }
                if let Some(id) = identifier.clone() {
                    language_server.text_document_did_save(id, &text);
                }
            }

            Ok(event)
        };

        Ok(future)
    }

    /// Detect the programming language based on the file type.
    pub fn detect_language(&mut self, loader: &syntax::Loader) {
        self.set_language(self.detect_language_config(loader), loader);
    }

    /// Detect the programming language based on the file type.
    pub fn detect_language_config(
        &self,
        loader: &syntax::Loader,
    ) -> Option<Arc<syntax::config::LanguageConfiguration>> {
        let language = loader
            .language_for_filename(self.path.as_ref()?)
            .or_else(|| loader.language_for_shebang(self.text().slice(..)))?;

        Some(loader.language(language).config().clone())
    }

    /// Detect the indentation used in the file, or otherwise defaults to the language indentation
    /// configured in `languages.toml`, with a fallback to tabs if it isn't specified. Line ending
    /// is likewise auto-detected, and will remain unchanged if no line endings were detected.
    pub fn detect_indent_and_line_ending(&mut self) {
        self.indent_style = if let Some(indent_style) = self.editor_config.indent_style {
            indent_style
        } else {
            auto_detect_indent_style(&self.text).unwrap_or_else(|| {
                self.language_config()
                    .and_then(|config| config.indent.as_ref())
                    .map_or(DEFAULT_INDENT, |config| IndentStyle::from_str(&config.unit))
            })
        };
        if let Some(line_ending) = self
            .editor_config
            .line_ending
            .or_else(|| auto_detect_line_ending(&self.text))
        {
            self.line_ending = line_ending;
        }
    }

    pub fn detect_editor_config(&mut self) {
        if self.config.load().editor_config {
            if let Some(path) = self.path.as_ref() {
                self.editor_config = EditorConfig::find(path);
            }
        }
    }

    pub fn pickup_last_saved_time(&mut self) {
        self.last_saved_time = match self.path() {
            Some(path) => match path.metadata() {
                Ok(metadata) => match metadata.modified() {
                    Ok(mtime) => mtime,
                    Err(err) => {
                        log::debug!("Could not fetch file system's mtime, falling back to current system time: {}", err);
                        SystemTime::now()
                    }
                },
                Err(err) => {
                    log::debug!("Could not fetch file system's mtime, falling back to current system time: {}", err);
                    SystemTime::now()
                }
            },
            None => SystemTime::now(),
        };
    }

    // Detect if the file is readonly and change the readonly field if necessary (unix only)
    pub fn detect_readonly(&mut self) {
        // Allows setting the flag for files the user cannot modify, like root files
        self.readonly = match &self.path {
            None => false,
            Some(p) => readonly(p),
        };
    }

    /// Reload the document from its path.
    pub fn reload(
        &mut self,
        view: &mut View,
        provider_registry: &DiffProviderRegistry,
    ) -> Result<(), Error> {
        let encoding = self.encoding;
        let path = match self.path() {
            None => return Ok(()),
            Some(path) => match path.exists() {
                true => path.to_owned(),
                false => bail!("can't find file to reload from {:?}", self.display_name()),
            },
        };

        // Once we have a valid path we check if its readonly status has changed
        self.detect_readonly();

        let mut file = std::fs::File::open(&path)?;
        let (rope, ..) = from_reader(&mut file, Some(encoding))?;

        // Calculate the difference between the buffer and source text, and apply it.
        // This is not considered a modification of the contents of the file regardless
        // of the encoding.
        let transaction = helix_core::diff::compare_ropes(self.text(), &rope);
        self.apply(&transaction, view.id);
        self.append_changes_to_history(view);
        self.reset_modified();
        self.pickup_last_saved_time();
        self.detect_indent_and_line_ending();

        match provider_registry.get_diff_base(&path) {
            Some(diff_base) => self.set_diff_base(diff_base),
            None => self.diff_handle = None,
        }

        self.version_control_head = provider_registry.get_current_head_name(&path);

        Ok(())
    }

    /// Sets the [`Document`]'s encoding with the encoding correspondent to `label`.
    pub fn set_encoding(&mut self, label: &str) -> Result<(), Error> {
        let encoding =
            Encoding::for_label(label.as_bytes()).ok_or_else(|| anyhow!("unknown encoding"))?;

        self.encoding = encoding;

        Ok(())
    }

    /// Returns the [`Document`]'s current encoding.
    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }

    /// sets the document path without sending events to various
    /// observers (like LSP), in most cases `Editor::set_doc_path`
    /// should be used instead
    pub fn set_path(&mut self, path: Option<&Path>) {
        let path = path.map(helix_stdx::path::canonicalize);

        // `take` to remove any prior relative path that may have existed.
        // This will get set in `relative_path()`.
        self.relative_path.take();

        // if parent doesn't exist we still want to open the document
        // and error out when document is saved
        self.path = path;

        self.detect_readonly();
        self.pickup_last_saved_time();
    }

    /// Set the programming language for the file and load associated data (e.g. highlighting)
    /// if it exists.
    pub fn set_language(
        &mut self,
        language_config: Option<Arc<syntax::config::LanguageConfiguration>>,
        loader: &syntax::Loader,
    ) {
        self.language = language_config;
        self.syntax = self.language.as_ref().and_then(|config| {
            Syntax::new(self.text.slice(..), config.language(), loader)
                .map_err(|err| {
                    // `NoRootConfig` means that there was an issue loading the language/syntax
                    // config for the root language of the document. An error must have already
                    // been logged by `LanguageData::syntax_config`.
                    if err != syntax::HighlighterError::NoRootConfig {
                        log::warn!("Error building syntax for '{}': {err}", self.display_name());
                    }
                })
                .ok()
        });
    }

    /// Set the programming language for the file if you know the language but don't have the
    /// [`syntax::config::LanguageConfiguration`] for it.
    pub fn set_language_by_language_id(
        &mut self,
        language_id: &str,
        loader: &syntax::Loader,
    ) -> anyhow::Result<()> {
        let language = loader
            .language_for_name(language_id)
            .ok_or_else(|| anyhow!("invalid language id: {}", language_id))?;
        let config = loader.language(language).config().clone();
        self.set_language(Some(config), loader);
        Ok(())
    }

    /// Select text within the [`Document`].
    pub fn set_selection(&mut self, view_id: ViewId, selection: Selection) {
        // TODO: use a transaction?
        self.selections
            .insert(view_id, selection.ensure_invariants(self.text().slice(..)));
        helix_event::dispatch(SelectionDidChange {
            doc: self,
            view: view_id,
        })
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

    /// Initializes a new selection and view_data for the given view
    /// if it does not already have them.
    pub fn ensure_view_init(&mut self, view_id: ViewId) {
        if !self.selections.contains_key(&view_id) {
            self.reset_selection(view_id);
        }

        self.view_data_mut(view_id);
    }

    /// Mark document as recent used for MRU sorting
    pub fn mark_as_focused(&mut self) {
        self.focused_at = std::time::Instant::now();
    }

    /// Remove a view's selection and inlay hints from this document.
    pub fn remove_view(&mut self, view_id: ViewId) {
        self.selections.remove(&view_id);
        self.inlay_hints.remove(&view_id);
        self.jump_labels.remove(&view_id);
    }

    /// Apply a [`Transaction`] to the [`Document`] to change its text.
    fn apply_impl(
        &mut self,
        transaction: &Transaction,
        view_id: ViewId,
        emit_lsp_notification: bool,
    ) -> bool {
        use helix_core::Assoc;

        let old_doc = self.text().clone();
        let changes = transaction.changes();
        if !changes.apply(&mut self.text) {
            return false;
        }

        if changes.is_empty() {
            if let Some(selection) = transaction.selection() {
                self.selections.insert(
                    view_id,
                    selection.clone().ensure_invariants(self.text.slice(..)),
                );
                helix_event::dispatch(SelectionDidChange {
                    doc: self,
                    view: view_id,
                });
            }
            return true;
        }

        self.modified_since_accessed = true;
        self.version += 1;

        for selection in self.selections.values_mut() {
            *selection = selection
                .clone()
                // Map through changes
                .map(transaction.changes())
                // Ensure all selections across all views still adhere to invariants.
                .ensure_invariants(self.text.slice(..));
        }

        for view_data in self.view_data.values_mut() {
            view_data.view_position.anchor = transaction
                .changes()
                .map_pos(view_data.view_position.anchor, Assoc::Before);
        }

        // generate revert to savepoint
        if !self.savepoints.is_empty() {
            let revert = transaction.invert(&old_doc);
            self.savepoints
                .retain_mut(|save_point| match save_point.upgrade() {
                    Some(savepoint) => {
                        let mut revert_to_savepoint = savepoint.revert.lock();
                        *revert_to_savepoint =
                            revert.clone().compose(mem::take(&mut revert_to_savepoint));
                        true
                    }
                    None => false,
                })
        }

        // update tree-sitter syntax tree
        if let Some(syntax) = &mut self.syntax {
            let loader = self.syn_loader.load();
            if let Err(err) = syntax.update(
                old_doc.slice(..),
                self.text.slice(..),
                transaction.changes(),
                &loader,
            ) {
                log::error!("TS parser failed, disabling TS for the current buffer: {err}");
                self.syntax = None;
            }
        }

        // TODO: all of that should likely just be hooks
        // start computing the diff in parallel
        if let Some(diff_handle) = &self.diff_handle {
            diff_handle.update_document(self.text.clone(), false);
        }

        // map diagnostics over changes too
        changes.update_positions(self.diagnostics.iter_mut().map(|diagnostic| {
            let assoc = if diagnostic.starts_at_word {
                Assoc::BeforeWord
            } else {
                Assoc::After
            };
            (&mut diagnostic.range.start, assoc)
        }));
        changes.update_positions(self.diagnostics.iter_mut().filter_map(|diagnostic| {
            if diagnostic.zero_width {
                // for zero width diagnostics treat the diagnostic as a point
                // rather than a range
                return None;
            }
            let assoc = if diagnostic.ends_at_word {
                Assoc::AfterWord
            } else {
                Assoc::Before
            };
            Some((&mut diagnostic.range.end, assoc))
        }));
        self.diagnostics.retain_mut(|diagnostic| {
            if diagnostic.zero_width {
                diagnostic.range.end = diagnostic.range.start
            } else if diagnostic.range.start >= diagnostic.range.end {
                return false;
            }
            diagnostic.line = self.text.char_to_line(diagnostic.range.start);
            true
        });

        self.diagnostics.sort_by_key(|diagnostic| {
            (
                diagnostic.range,
                diagnostic.severity,
                diagnostic.provider.clone(),
            )
        });

        // Update the inlay hint annotations' positions, helping ensure they are displayed in the proper place
        let apply_inlay_hint_changes = |annotations: &mut Vec<InlineAnnotation>| {
            changes.update_positions(
                annotations
                    .iter_mut()
                    .map(|annotation| (&mut annotation.char_idx, Assoc::After)),
            );
        };

        self.inlay_hints_oudated = true;
        for text_annotation in self.inlay_hints.values_mut() {
            let DocumentInlayHints {
                id: _,
                type_inlay_hints,
                parameter_inlay_hints,
                other_inlay_hints,
                padding_before_inlay_hints,
                padding_after_inlay_hints,
            } = text_annotation;

            apply_inlay_hint_changes(padding_before_inlay_hints);
            apply_inlay_hint_changes(type_inlay_hints);
            apply_inlay_hint_changes(parameter_inlay_hints);
            apply_inlay_hint_changes(other_inlay_hints);
            apply_inlay_hint_changes(padding_after_inlay_hints);
        }

        helix_event::dispatch(DocumentDidChange {
            doc: self,
            view: view_id,
            old_text: &old_doc,
            changes,
            ghost_transaction: !emit_lsp_notification,
        });

        // if specified, the current selection should instead be replaced by transaction.selection
        if let Some(selection) = transaction.selection() {
            self.selections.insert(
                view_id,
                selection.clone().ensure_invariants(self.text.slice(..)),
            );
            helix_event::dispatch(SelectionDidChange {
                doc: self,
                view: view_id,
            });
        }

        true
    }

    fn apply_inner(
        &mut self,
        transaction: &Transaction,
        view_id: ViewId,
        emit_lsp_notification: bool,
    ) -> bool {
        // store the state just before any changes are made. This allows us to undo to the
        // state just before a transaction was applied.
        if self.changes.is_empty() && !transaction.changes().is_empty() {
            self.old_state = Some(State {
                doc: self.text.clone(),
                selection: self.selection(view_id).clone(),
            });
        }

        let success = self.apply_impl(transaction, view_id, emit_lsp_notification);

        if !transaction.changes().is_empty() {
            // Compose this transaction with the previous one
            take_with(&mut self.changes, |changes| {
                changes.compose(transaction.changes().clone())
            });
        }
        success
    }
    /// Apply a [`Transaction`] to the [`Document`] to change its text.
    pub fn apply(&mut self, transaction: &Transaction, view_id: ViewId) -> bool {
        self.apply_inner(transaction, view_id, true)
    }

    /// Apply a [`Transaction`] to the [`Document`] to change its text
    /// without notifying the language servers. This is useful for temporary transactions
    /// that must not influence the server.
    pub fn apply_temporary(&mut self, transaction: &Transaction, view_id: ViewId) -> bool {
        self.apply_inner(transaction, view_id, false)
    }

    fn undo_redo_impl(&mut self, view: &mut View, undo: bool) -> bool {
        if undo {
            self.append_changes_to_history(view);
        } else if !self.changes.is_empty() {
            return false;
        }
        let mut history = self.history.take();
        let txn = if undo { history.undo() } else { history.redo() };
        let success = if let Some(txn) = txn {
            self.apply_impl(txn, view.id, true)
        } else {
            false
        };
        self.history.set(history);

        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text().slice(..));
            // Sync with changes with the jumplist selections.
            view.sync_changes(self);
        }
        success
    }

    /// Undo the last modification to the [`Document`]. Returns whether the undo was successful.
    pub fn undo(&mut self, view: &mut View) -> bool {
        self.undo_redo_impl(view, true)
    }

    /// Redo the last modification to the [`Document`]. Returns whether the redo was successful.
    pub fn redo(&mut self, view: &mut View) -> bool {
        self.undo_redo_impl(view, false)
    }

    /// Creates a reference counted snapshot (called savpepoint) of the document.
    ///
    /// The snapshot will remain valid (and updated) idenfinitly as long as ereferences to it exist.
    /// Restoring the snapshot will restore the selection and the contents of the document to
    /// the state it had when this function was called.
    pub fn savepoint(&mut self, view: &View) -> Arc<SavePoint> {
        let revert = Transaction::new(self.text()).with_selection(self.selection(view.id).clone());
        // check if there is already an existing (identical) savepoint around
        if let Some(savepoint) = self
            .savepoints
            .iter()
            .rev()
            .find_map(|savepoint| savepoint.upgrade())
        {
            let transaction = savepoint.revert.lock();
            if savepoint.view == view.id
                && transaction.changes().is_empty()
                && transaction.selection() == revert.selection()
            {
                drop(transaction);
                return savepoint;
            }
        }
        let savepoint = Arc::new(SavePoint {
            view: view.id,
            revert: Mutex::new(revert),
        });
        self.savepoints.push(Arc::downgrade(&savepoint));
        savepoint
    }

    pub fn restore(&mut self, view: &mut View, savepoint: &SavePoint, emit_lsp_notification: bool) {
        assert_eq!(
            savepoint.view, view.id,
            "Savepoint must not be used with a different view!"
        );
        // search and remove savepoint using a ptr comparison
        // this avoids a deadlock as we need to lock the mutex
        let savepoint_idx = self
            .savepoints
            .iter()
            .position(|savepoint_ref| std::ptr::eq(savepoint_ref.as_ptr(), savepoint))
            .expect("Savepoint must belong to this document");

        let savepoint_ref = self.savepoints.remove(savepoint_idx);
        let mut revert = savepoint.revert.lock();
        self.apply_inner(&revert, view.id, emit_lsp_notification);
        *revert = Transaction::new(self.text()).with_selection(self.selection(view.id).clone());
        self.savepoints.push(savepoint_ref)
    }

    fn earlier_later_impl(&mut self, view: &mut View, uk: UndoKind, earlier: bool) -> bool {
        if earlier {
            self.append_changes_to_history(view);
        } else if !self.changes.is_empty() {
            return false;
        }
        let txns = if earlier {
            self.history.get_mut().earlier(uk)
        } else {
            self.history.get_mut().later(uk)
        };
        let mut success = false;
        for txn in txns {
            if self.apply_impl(&txn, view.id, true) {
                success = true;
            }
        }
        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text().slice(..));
            // Sync with changes with the jumplist selections.
            view.sync_changes(self);
        }
        success
    }

    /// Undo modifications to the [`Document`] according to `uk`.
    pub fn earlier(&mut self, view: &mut View, uk: UndoKind) -> bool {
        self.earlier_later_impl(view, uk, true)
    }

    /// Redo modifications to the [`Document`] according to `uk`.
    pub fn later(&mut self, view: &mut View, uk: UndoKind) -> bool {
        self.earlier_later_impl(view, uk, false)
    }

    /// Commit pending changes to history
    pub fn append_changes_to_history(&mut self, view: &mut View) {
        if self.changes.is_empty() {
            return;
        }

        let new_changeset = ChangeSet::new(self.text().slice(..));
        let changes = std::mem::replace(&mut self.changes, new_changeset);
        // Instead of doing this messy merge we could always commit, and based on transaction
        // annotations either add a new layer or compose into the previous one.
        let transaction =
            Transaction::from(changes).with_selection(self.selection(view.id).clone());

        // HAXX: we need to reconstruct the state as it was before the changes..
        let old_state = self.old_state.take().expect("no old_state available");

        let mut history = self.history.take();
        history.commit_revision(&transaction, &old_state);
        self.history.set(history);

        // Update jumplist entries in the view.
        view.apply(&transaction, self);
    }

    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// If there are unsaved modifications.
    pub fn is_modified(&self) -> bool {
        let history = self.history.take();
        let current_revision = history.current_revision();
        self.history.set(history);
        log::debug!(
            "id {} modified - last saved: {}, current: {}",
            self.id,
            self.last_saved_revision,
            current_revision
        );
        current_revision != self.last_saved_revision || !self.changes.is_empty()
    }

    /// Save modifications to history, and so [`Self::is_modified`] will return false.
    pub fn reset_modified(&mut self) {
        let history = self.history.take();
        let current_revision = history.current_revision();
        self.history.set(history);
        self.last_saved_revision = current_revision;
    }

    /// Set the document's latest saved revision to the given one.
    pub fn set_last_saved_revision(&mut self, rev: usize, save_time: SystemTime) {
        log::debug!(
            "doc {} revision updated {} -> {}",
            self.id,
            self.last_saved_revision,
            rev
        );
        self.last_saved_revision = rev;
        self.last_saved_time = save_time;
    }

    /// Get the document's latest saved revision.
    pub fn get_last_saved_revision(&mut self) -> usize {
        self.last_saved_revision
    }

    /// Get the current revision number
    pub fn get_current_revision(&mut self) -> usize {
        let history = self.history.take();
        let current_revision = history.current_revision();
        self.history.set(history);
        current_revision
    }

    /// Corresponding language scope name. Usually `source.<lang>`.
    pub fn language_scope(&self) -> Option<&str> {
        self.language
            .as_ref()
            .map(|language| language.scope.as_str())
    }

    /// Language name for the document. Corresponds to the `name` key in
    /// `languages.toml` configuration.
    pub fn language_name(&self) -> Option<&str> {
        self.language
            .as_ref()
            .map(|language| language.language_id.as_str())
    }

    /// Language ID for the document. Either the `language-id`,
    /// or the document language name if no `language-id` has been specified.
    pub fn language_id(&self) -> Option<&str> {
        self.language_config()?
            .language_server_language_id
            .as_deref()
            .or_else(|| self.language_name())
    }

    /// Corresponding [`LanguageConfiguration`].
    pub fn language_config(&self) -> Option<&LanguageConfiguration> {
        self.language.as_deref()
    }

    /// Current document version, incremented at each change.
    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn word_completion_enabled(&self) -> bool {
        self.language_config()
            .and_then(|lang_config| lang_config.word_completion.and_then(|c| c.enable))
            .unwrap_or_else(|| self.config.load().word_completion.enable)
    }

    pub fn path_completion_enabled(&self) -> bool {
        self.language_config()
            .and_then(|lang_config| lang_config.path_completion)
            .unwrap_or_else(|| self.config.load().path_completion)
    }

    /// maintains the order as configured in the language_servers TOML array
    pub fn language_servers(&self) -> impl Iterator<Item = &helix_lsp::Client> {
        self.language_config().into_iter().flat_map(move |config| {
            config.language_servers.iter().filter_map(move |features| {
                let ls = &**self.language_servers.get(&features.name)?;
                if ls.is_initialized() {
                    Some(ls)
                } else {
                    None
                }
            })
        })
    }

    pub fn remove_language_server_by_name(&mut self, name: &str) -> Option<Arc<Client>> {
        self.language_servers.remove(name)
    }

    pub fn language_servers_with_feature(
        &self,
        feature: LanguageServerFeature,
    ) -> impl Iterator<Item = &helix_lsp::Client> {
        self.language_config().into_iter().flat_map(move |config| {
            config.language_servers.iter().filter_map(move |features| {
                let ls = &**self.language_servers.get(&features.name)?;
                if ls.is_initialized()
                    && ls.supports_feature(feature)
                    && features.has_feature(feature)
                {
                    Some(ls)
                } else {
                    None
                }
            })
        })
    }

    pub fn supports_language_server(&self, id: LanguageServerId) -> bool {
        self.language_servers().any(|l| l.id() == id)
    }

    pub fn diff_handle(&self) -> Option<&DiffHandle> {
        self.diff_handle.as_ref()
    }

    /// Intialize/updates the differ for this document with a new base.
    pub fn set_diff_base(&mut self, diff_base: Vec<u8>) {
        if let Ok((diff_base, ..)) = from_reader(&mut diff_base.as_slice(), Some(self.encoding)) {
            if let Some(differ) = &self.diff_handle {
                differ.update_diff_base(diff_base);
                return;
            }
            self.diff_handle = Some(DiffHandle::new(diff_base, self.text.clone()))
        } else {
            self.diff_handle = None;
        }
    }

    pub fn version_control_head(&self) -> Option<Arc<Box<str>>> {
        self.version_control_head.as_ref().map(|a| a.load_full())
    }

    pub fn set_version_control_head(
        &mut self,
        version_control_head: Option<Arc<ArcSwap<Box<str>>>>,
    ) {
        self.version_control_head = version_control_head;
    }

    #[inline]
    /// Tree-sitter AST tree
    pub fn syntax(&self) -> Option<&Syntax> {
        self.syntax.as_ref()
    }

    /// The width that the tab character is rendered at
    pub fn tab_width(&self) -> usize {
        self.editor_config
            .tab_width
            .map(|n| n.get() as usize)
            .unwrap_or_else(|| {
                self.language_config()
                    .and_then(|config| config.indent.as_ref())
                    .map_or(DEFAULT_TAB_WIDTH, |config| config.tab_width)
            })
    }

    // The width (in spaces) of a level of indentation.
    pub fn indent_width(&self) -> usize {
        self.indent_style.indent_width(self.tab_width())
    }

    /// Whether the document should have a trailing line ending appended on save.
    pub fn insert_final_newline(&self) -> bool {
        self.editor_config
            .insert_final_newline
            .unwrap_or_else(|| self.config.load().insert_final_newline)
    }

    /// Whether the document should trim whitespace preceding line endings on save.
    pub fn trim_trailing_whitespace(&self) -> bool {
        self.editor_config
            .trim_trailing_whitespace
            .unwrap_or_else(|| self.config.load().trim_trailing_whitespace)
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

    pub fn uri(&self) -> Option<helix_core::Uri> {
        Some(self.path()?.clone().into())
    }

    #[inline]
    pub fn text(&self) -> &Rope {
        &self.text
    }

    #[inline]
    pub fn selection(&self, view_id: ViewId) -> &Selection {
        &self.selections[&view_id]
    }

    #[inline]
    pub fn selections(&self) -> &HashMap<ViewId, Selection> {
        &self.selections
    }

    fn view_data(&self, view_id: ViewId) -> &ViewData {
        self.view_data
            .get(&view_id)
            .expect("This should only be called after ensure_view_init")
    }

    fn view_data_mut(&mut self, view_id: ViewId) -> &mut ViewData {
        self.view_data.entry(view_id).or_default()
    }

    pub(crate) fn get_view_offset(&self, view_id: ViewId) -> Option<ViewPosition> {
        Some(self.view_data.get(&view_id)?.view_position)
    }

    pub fn view_offset(&self, view_id: ViewId) -> ViewPosition {
        self.view_data(view_id).view_position
    }

    pub fn set_view_offset(&mut self, view_id: ViewId, new_offset: ViewPosition) {
        self.view_data_mut(view_id).view_position = new_offset;
    }

    pub fn relative_path(&self) -> Option<&Path> {
        self.relative_path
            .get_or_init(|| {
                self.path
                    .as_ref()
                    .map(|path| helix_stdx::path::get_relative_path(path).to_path_buf())
            })
            .as_deref()
    }

    pub fn display_name(&self) -> Cow<'_, str> {
        self.relative_path()
            .map_or_else(|| SCRATCH_BUFFER_NAME.into(), |path| path.to_string_lossy())
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

    pub fn lsp_diagnostic_to_diagnostic(
        text: &Rope,
        language_config: Option<&LanguageConfiguration>,
        diagnostic: &helix_lsp::lsp::Diagnostic,
        provider: DiagnosticProvider,
        offset_encoding: helix_lsp::OffsetEncoding,
    ) -> Option<Diagnostic> {
        use helix_core::diagnostic::{Range, Severity::*};

        // TODO: convert inside server
        let start =
            if let Some(start) = lsp_pos_to_pos(text, diagnostic.range.start, offset_encoding) {
                start
            } else {
                log::warn!("lsp position out of bounds - {:?}", diagnostic);
                return None;
            };

        let end = if let Some(end) = lsp_pos_to_pos(text, diagnostic.range.end, offset_encoding) {
            end
        } else {
            log::warn!("lsp position out of bounds - {:?}", diagnostic);
            return None;
        };

        let severity = diagnostic.severity.and_then(|severity| match severity {
            lsp::DiagnosticSeverity::ERROR => Some(Error),
            lsp::DiagnosticSeverity::WARNING => Some(Warning),
            lsp::DiagnosticSeverity::INFORMATION => Some(Info),
            lsp::DiagnosticSeverity::HINT => Some(Hint),
            severity => {
                log::error!("unrecognized diagnostic severity: {:?}", severity);
                None
            }
        });

        if let Some(lang_conf) = language_config {
            if let Some(severity) = severity {
                if severity < lang_conf.diagnostic_severity {
                    return None;
                }
            }
        };
        use helix_core::diagnostic::{DiagnosticTag, NumberOrString};

        let code = match diagnostic.code.clone() {
            Some(x) => match x {
                lsp::NumberOrString::Number(x) => Some(NumberOrString::Number(x)),
                lsp::NumberOrString::String(x) => Some(NumberOrString::String(x)),
            },
            None => None,
        };

        let tags = if let Some(tags) = &diagnostic.tags {
            let new_tags = tags
                .iter()
                .filter_map(|tag| match *tag {
                    lsp::DiagnosticTag::DEPRECATED => Some(DiagnosticTag::Deprecated),
                    lsp::DiagnosticTag::UNNECESSARY => Some(DiagnosticTag::Unnecessary),
                    _ => None,
                })
                .collect();

            new_tags
        } else {
            Vec::new()
        };

        let ends_at_word =
            start != end && end != 0 && text.get_char(end - 1).is_some_and(char_is_word);
        let starts_at_word = start != end && text.get_char(start).is_some_and(char_is_word);

        Some(Diagnostic {
            range: Range { start, end },
            ends_at_word,
            starts_at_word,
            zero_width: start == end,
            line: diagnostic.range.start.line as usize,
            message: diagnostic.message.clone(),
            severity,
            code,
            tags,
            source: diagnostic.source.clone(),
            data: diagnostic.data.clone(),
            provider,
        })
    }

    #[inline]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn replace_diagnostics(
        &mut self,
        diagnostics: impl IntoIterator<Item = Diagnostic>,
        unchanged_sources: &[String],
        provider: Option<&DiagnosticProvider>,
    ) {
        if unchanged_sources.is_empty() {
            if let Some(provider) = provider {
                self.diagnostics
                    .retain(|diagnostic| &diagnostic.provider != provider);
            } else {
                self.diagnostics.clear();
            }
        } else {
            self.diagnostics.retain(|d| {
                if provider.is_some_and(|provider| provider != &d.provider) {
                    return true;
                }

                if let Some(source) = &d.source {
                    unchanged_sources.contains(source)
                } else {
                    false
                }
            });
        }
        self.diagnostics.extend(diagnostics);
        self.diagnostics.sort_by_key(|diagnostic| {
            (
                diagnostic.range,
                diagnostic.severity,
                diagnostic.provider.clone(),
            )
        });
    }

    /// clears diagnostics for a given language server id if set, otherwise all diagnostics are cleared
    pub fn clear_diagnostics_for_language_server(&mut self, id: LanguageServerId) {
        self.diagnostics
            .retain(|d| d.provider.language_server_id() != Some(id));
    }

    /// Get the document's auto pairs. If the document has a recognized
    /// language config with auto pairs configured, returns that;
    /// otherwise, falls back to the global auto pairs config. If the global
    /// config is false, then ignore language settings.
    pub fn auto_pairs<'a>(
        &'a self,
        editor: &'a Editor,
        loader: &'a syntax::Loader,
        view: &View,
    ) -> Option<&'a AutoPairs> {
        let global_config = (editor.auto_pairs).as_ref();

        // NOTE: If the user specifies the global auto pairs config as false, then
        //       we want to disable it globally regardless of language settings
        #[allow(clippy::question_mark)]
        {
            if global_config.is_none() {
                return None;
            }
        }

        self.syntax
            .as_ref()
            .and_then(|syntax| {
                let selection = self.selection(view.id).primary();
                let (start, end) = selection.into_byte_range(self.text().slice(..));
                let layer = syntax.layer_for_byte_range(start as u32, end as u32);

                let lang_config = loader.language(syntax.layer(layer).language).config();
                lang_config.auto_pairs.as_ref()
            })
            .or(global_config)
    }

    pub fn snippet_ctx(&self) -> SnippetRenderCtx {
        SnippetRenderCtx {
            // TODO snippet variable resolution
            resolve_var: Box::new(|_| None),
            tab_width: self.tab_width(),
            indent_style: self.indent_style,
            line_ending: self.line_ending.as_str(),
        }
    }

    pub fn text_width(&self) -> usize {
        self.editor_config
            .max_line_length
            .map(|n| n.get() as usize)
            .or_else(|| self.language_config().and_then(|config| config.text_width))
            .unwrap_or_else(|| self.config.load().text_width)
    }

    pub fn text_format(&self, mut viewport_width: u16, theme: Option<&Theme>) -> TextFormat {
        let config = self.config.load();
        let text_width = self.text_width();
        let mut soft_wrap_at_text_width = self
            .language_config()
            .and_then(|config| {
                config
                    .soft_wrap
                    .as_ref()
                    .and_then(|soft_wrap| soft_wrap.wrap_at_text_width)
            })
            .or(config.soft_wrap.wrap_at_text_width)
            .unwrap_or(false);
        if soft_wrap_at_text_width {
            // if the viewport is smaller than the specified
            // width then this setting has no effcet
            if text_width >= viewport_width as usize {
                soft_wrap_at_text_width = false;
            } else {
                viewport_width = text_width as u16;
            }
        }
        let config = self.config.load();
        let editor_soft_wrap = &config.soft_wrap;
        let language_soft_wrap = self
            .language
            .as_ref()
            .and_then(|config| config.soft_wrap.as_ref());
        let enable_soft_wrap = language_soft_wrap
            .and_then(|soft_wrap| soft_wrap.enable)
            .or(editor_soft_wrap.enable)
            .unwrap_or(false);
        let max_wrap = language_soft_wrap
            .and_then(|soft_wrap| soft_wrap.max_wrap)
            .or(config.soft_wrap.max_wrap)
            .unwrap_or(20);
        let max_indent_retain = language_soft_wrap
            .and_then(|soft_wrap| soft_wrap.max_indent_retain)
            .or(editor_soft_wrap.max_indent_retain)
            .unwrap_or(40);
        let wrap_indicator = language_soft_wrap
            .and_then(|soft_wrap| soft_wrap.wrap_indicator.clone())
            .or_else(|| config.soft_wrap.wrap_indicator.clone())
            .unwrap_or_else(|| "↪ ".into());
        let tab_width = self.tab_width() as u16;
        TextFormat {
            soft_wrap: enable_soft_wrap && viewport_width > 10,
            tab_width,
            max_wrap: max_wrap.min(viewport_width / 4),
            max_indent_retain: max_indent_retain.min(viewport_width * 2 / 5),
            // avoid spinning forever when the window manager
            // sets the size to something tiny
            viewport_width,
            wrap_indicator: wrap_indicator.into_boxed_str(),
            wrap_indicator_highlight: theme
                .and_then(|theme| theme.find_highlight("ui.virtual.wrap")),
            soft_wrap_at_text_width,
        }
    }

    /// Set the inlay hints for this document and `view_id`.
    pub fn set_inlay_hints(&mut self, view_id: ViewId, inlay_hints: DocumentInlayHints) {
        self.inlay_hints.insert(view_id, inlay_hints);
    }

    pub fn set_jump_labels(&mut self, view_id: ViewId, labels: Vec<Overlay>) {
        self.jump_labels.insert(view_id, labels);
    }

    pub fn remove_jump_labels(&mut self, view_id: ViewId) {
        self.jump_labels.remove(&view_id);
    }

    /// Get the inlay hints for this document and `view_id`.
    pub fn inlay_hints(&self, view_id: ViewId) -> Option<&DocumentInlayHints> {
        self.inlay_hints.get(&view_id)
    }

    /// Completely removes all the inlay hints saved for the document, dropping them to free memory
    /// (since it often means inlay hints have been fully deactivated).
    pub fn reset_all_inlay_hints(&mut self) {
        self.inlay_hints = Default::default();
    }

    pub fn has_language_server_with_feature(&self, feature: LanguageServerFeature) -> bool {
        self.language_servers_with_feature(feature).next().is_some()
    }
}

#[derive(Debug, Default)]
pub struct ViewData {
    view_position: ViewPosition,
}

#[derive(Clone, Debug)]
pub enum FormatterError {
    SpawningFailed {
        command: String,
        error: std::io::ErrorKind,
    },
    BrokenStdin,
    WaitForOutputFailed,
    InvalidUtf8Output,
    NonZeroExitStatus(Option<String>),
}

impl std::error::Error for FormatterError {}

impl Display for FormatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawningFailed { command, error } => {
                write!(f, "Failed to spawn formatter {}: {:?}", command, error)
            }
            Self::BrokenStdin => write!(f, "Could not write to formatter stdin"),
            Self::WaitForOutputFailed => write!(f, "Waiting for formatter output failed"),
            Self::InvalidUtf8Output => write!(f, "Invalid UTF-8 formatter output"),
            Self::NonZeroExitStatus(Some(output)) => write!(f, "Formatter error: {}", output),
            Self::NonZeroExitStatus(None) => {
                write!(f, "Formatter exited with non zero exit status")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use arc_swap::ArcSwap;

    use super::*;

    #[test]
    fn changeset_to_changes_ignore_line_endings() {
        use helix_lsp::{lsp, Client, OffsetEncoding};
        let text = Rope::from("hello\r\nworld");
        let mut doc = Document::from(
            text,
            None,
            Arc::new(ArcSwap::new(Arc::new(Config::default()))),
            Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
        );
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
        let mut doc = Document::from(
            text,
            None,
            Arc::new(ArcSwap::new(Arc::new(Config::default()))),
            Arc::new(ArcSwap::from_pointee(syntax::Loader::default())),
        );
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
            Document::default(
                Arc::new(ArcSwap::new(Arc::new(Config::default()))),
                Arc::new(ArcSwap::from_pointee(syntax::Loader::default()))
            )
            .text()
            .to_string(),
            helix_core::NATIVE_LINE_ENDING.as_str()
        );
    }

    macro_rules! decode {
        ($name:ident, $label:expr, $label_override:expr) => {
            #[test]
            fn $name() {
                let encoding = encoding::Encoding::for_label($label_override.as_bytes()).unwrap();
                let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/encoding");
                let path = base_path.join(format!("{}_in.txt", $label));
                let ref_path = base_path.join(format!("{}_in_ref.txt", $label));
                assert!(path.exists());
                assert!(ref_path.exists());

                let mut file = std::fs::File::open(path).unwrap();
                let text = from_reader(&mut file, Some(encoding.into()))
                    .unwrap()
                    .0
                    .to_string();
                let expectation = std::fs::read_to_string(ref_path).unwrap();
                assert_eq!(text[..], expectation[..]);
            }
        };
        ($name:ident, $label:expr) => {
            decode!($name, $label, $label);
        };
    }

    macro_rules! encode {
        ($name:ident, $label:expr, $label_override:expr) => {
            #[test]
            fn $name() {
                let encoding = encoding::Encoding::for_label($label_override.as_bytes()).unwrap();
                let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/encoding");
                let path = base_path.join(format!("{}_out.txt", $label));
                let ref_path = base_path.join(format!("{}_out_ref.txt", $label));
                assert!(path.exists());
                assert!(ref_path.exists());

                let text = Rope::from_str(&std::fs::read_to_string(path).unwrap());
                let mut buf: Vec<u8> = Vec::new();
                helix_lsp::block_on(to_writer(&mut buf, (encoding, false), &text)).unwrap();

                let expectation = std::fs::read(ref_path).unwrap();
                assert_eq!(buf, expectation);
            }
        };
        ($name:ident, $label:expr) => {
            encode!($name, $label, $label);
        };
    }

    decode!(big5_decode, "big5");
    encode!(big5_encode, "big5");
    decode!(euc_kr_decode, "euc_kr", "EUC-KR");
    encode!(euc_kr_encode, "euc_kr", "EUC-KR");
    decode!(gb18030_decode, "gb18030");
    encode!(gb18030_encode, "gb18030");
    decode!(iso_2022_jp_decode, "iso_2022_jp", "ISO-2022-JP");
    encode!(iso_2022_jp_encode, "iso_2022_jp", "ISO-2022-JP");
    decode!(jis0208_decode, "jis0208", "EUC-JP");
    encode!(jis0208_encode, "jis0208", "EUC-JP");
    decode!(jis0212_decode, "jis0212", "EUC-JP");
    decode!(shift_jis_decode, "shift_jis");
    encode!(shift_jis_encode, "shift_jis");
}
