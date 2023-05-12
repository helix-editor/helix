use anyhow::{anyhow, bail, Context, Error};
use arc_swap::access::DynAccess;
use arc_swap::ArcSwap;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use helix_core::auto_pairs::AutoPairs;
use helix_core::doc_formatter::TextFormat;
use helix_core::encoding::Encoding;
use helix_core::syntax::Highlight;
use helix_core::text_annotations::{InlineAnnotation, TextAnnotations};
use helix_core::Range;
use helix_vcs::{DiffHandle, DiffProviderRegistry};

use ::parking_lot::Mutex;
use serde::de::{self, Deserialize, Deserializer};
use serde::Serialize;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, Weak};
use std::time::SystemTime;

use helix_core::{
    encoding,
    history::{History, State, UndoKind},
    indent::{auto_detect_indent_style, IndentStyle},
    line_ending::auto_detect_line_ending,
    syntax::{self, LanguageConfiguration},
    ChangeSet, Diagnostic, LineEnding, Rope, RopeBuilder, Selection, Syntax, Transaction,
    DEFAULT_LINE_ENDING,
};

use crate::editor::{Config, RedrawHandle};
use crate::{DocumentId, Editor, Theme, View, ViewId};

/// 8kB of buffer space for encoding and decoding `Rope`s.
const BUF_SIZE: usize = 8192;

const DEFAULT_INDENT: IndentStyle = IndentStyle::Tabs;

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

pub struct Document {
    pub(crate) id: DocumentId,
    text: Rope,
    selections: HashMap<ViewId, Selection>,

    /// Inlay hints annotations for the document, by view.
    ///
    /// To know if they're up-to-date, check the `id` field in `DocumentInlayHints`.
    pub(crate) inlay_hints: HashMap<ViewId, DocumentInlayHints>,
    /// Set to `true` when the document is updated, reset to `false` on the next inlay hints
    /// update from the LSP
    pub inlay_hints_oudated: bool,

    path: Option<PathBuf>,
    encoding: &'static encoding::Encoding,
    has_bom: bool,

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
    pub config: Arc<dyn DynAccess<Config>>,

    savepoints: Vec<Weak<SavePoint>>,

    // Last time we wrote to the file. This will carry the time the file was last opened if there
    // were no saves.
    last_saved_time: SystemTime,

    last_saved_revision: usize,
    version: i32, // should be usize?
    pub(crate) modified_since_accessed: bool,

    diagnostics: Vec<Diagnostic>,
    language_server: Option<Arc<helix_lsp::Client>>,

    diff_handle: Option<DiffHandle>,
    version_control_head: Option<Arc<ArcSwap<Box<str>>>>,

    // when document was used for most-recent-used buffer picker
    pub focused_at: std::time::Instant,
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
    pub type_inlay_hints: Rc<[InlineAnnotation]>,

    /// Inlay hints of `PARAMETER` kind, if any.
    pub parameter_inlay_hints: Rc<[InlineAnnotation]>,

    /// Inlay hints that are neither `TYPE` nor `PARAMETER`.
    ///
    /// LSPs are not required to associate a kind to their inlay hints, for example Rust-Analyzer
    /// currently never does (February 2023) and the LSP spec may add new kinds in the future that
    /// we want to display even if we don't have some special highlighting for them.
    pub other_inlay_hints: Rc<[InlineAnnotation]>,

    /// Inlay hint padding. When creating the final `TextAnnotations`, the `before` padding must be
    /// added first, then the regular inlay hints, then the `after` padding.
    pub padding_before_inlay_hints: Rc<[InlineAnnotation]>,
    pub padding_after_inlay_hints: Rc<[InlineAnnotation]>,
}

impl DocumentInlayHints {
    /// Generate an empty list of inlay hints with the given ID.
    pub fn empty_with_id(id: DocumentInlayHintsId) -> Self {
        Self {
            id,
            type_inlay_hints: Rc::new([]),
            parameter_inlay_hints: Rc::new([]),
            other_inlay_hints: Rc::new([]),
            padding_before_inlay_hints: Rc::new([]),
            padding_after_inlay_hints: Rc::new([]),
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
) -> Result<(Rope, &'static Encoding, bool), Error> {
    // These two buffers are 8192 bytes in size each and are used as
    // intermediaries during the decoding process. Text read into `buf`
    // from `reader` is decoded into `buf_out` as UTF-8. Once either
    // `buf_out` is full or the end of the reader was reached, the
    // contents are appended to `builder`.
    let mut buf = [0u8; BUF_SIZE];
    let mut buf_out = [0u8; BUF_SIZE];
    let mut builder = RopeBuilder::new();

    // By default, the encoding of the text is auto-detected by
    // `encoding_rs` for_bom, and if it fails, from `chardetng`
    // crate which requires sample data from the reader.
    // As a manual override to this auto-detection is possible, the
    // same data is read into `buf` to ensure symmetry in the upcoming
    // loop.
    let (encoding, has_bom, mut decoder, mut slice, mut is_empty) = {
        let read = reader.read(&mut buf)?;
        let is_empty = read == 0;
        let (encoding, has_bom) = encoding
            .map(|encoding| (encoding, false))
            .or_else(|| {
                encoding::Encoding::for_bom(&buf).map(|(encoding, _bom_size)| (encoding, true))
            })
            .unwrap_or_else(|| {
                let mut encoding_detector = chardetng::EncodingDetector::new();
                encoding_detector.feed(&buf, is_empty);
                (encoding_detector.guess(None, true), false)
            });

        let decoder = encoding.new_decoder();

        // If the amount of bytes read from the reader is less than
        // `buf.len()`, it is undesirable to read the bytes afterwards.
        let slice = &buf[..read];
        (encoding, has_bom, decoder, slice, is_empty)
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
    Ok((rope, encoding, has_bom))
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

use helix_lsp::lsp;
use url::Url;

impl Document {
    pub fn from(
        text: Rope,
        encoding_with_bom_info: Option<(&'static Encoding, bool)>,
        config: Arc<dyn DynAccess<Config>>,
    ) -> Self {
        let (encoding, has_bom) = encoding_with_bom_info.unwrap_or((encoding::UTF_8, false));
        let changes = ChangeSet::new(&text);
        let old_state = None;

        Self {
            id: DocumentId::default(),
            path: None,
            encoding,
            has_bom,
            text,
            selections: HashMap::default(),
            inlay_hints: HashMap::default(),
            inlay_hints_oudated: false,
            indent_style: DEFAULT_INDENT,
            line_ending: DEFAULT_LINE_ENDING,
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
            language_server: None,
            diff_handle: None,
            config,
            version_control_head: None,
            focused_at: std::time::Instant::now(),
        }
    }
    pub fn default(config: Arc<dyn DynAccess<Config>>) -> Self {
        let text = Rope::from(DEFAULT_LINE_ENDING.as_str());
        Self::from(text, None, config)
    }
    // TODO: async fn?
    /// Create a new document from `path`. Encoding is auto-detected, but it can be manually
    /// overwritten with the `encoding` parameter.
    pub fn open(
        path: &Path,
        encoding: Option<&'static Encoding>,
        config_loader: Option<Arc<syntax::Loader>>,
        config: Arc<dyn DynAccess<Config>>,
    ) -> Result<Self, Error> {
        // Open the file if it exists, otherwise assume it is a new file (and thus empty).
        let (rope, encoding, has_bom) = if path.exists() {
            let mut file =
                std::fs::File::open(path).context(format!("unable to open {:?}", path))?;
            from_reader(&mut file, encoding)?
        } else {
            let encoding = encoding.unwrap_or(encoding::UTF_8);
            (Rope::from(DEFAULT_LINE_ENDING.as_str()), encoding, false)
        };

        let mut doc = Self::from(rope, Some((encoding, has_bom)), config);

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
    pub fn auto_format(&self) -> Option<BoxFuture<'static, Result<Transaction, FormatterError>>> {
        if self.language_config()?.auto_format {
            self.format()
        } else {
            None
        }
    }

    /// If supported, returns the changes that should be applied to this document in order
    /// to format it nicely.
    // We can't use anyhow::Result here since the output of the future has to be
    // clonable to be used as shared future. So use a custom error type.
    pub fn format(&self) -> Option<BoxFuture<'static, Result<Transaction, FormatterError>>> {
        if let Some(formatter) = self
            .language_config()
            .and_then(|c| c.formatter.clone())
            .filter(|formatter| which::which(&formatter.command).is_ok())
        {
            use std::process::Stdio;
            let text = self.text().clone();
            let mut process = tokio::process::Command::new(&formatter.command);
            process
                .args(&formatter.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let formatting_future = async move {
                let mut process = process
                    .spawn()
                    .map_err(|e| FormatterError::SpawningFailed {
                        command: formatter.command.clone(),
                        error: e.kind(),
                    })?;
                {
                    let mut stdin = process.stdin.take().ok_or(FormatterError::BrokenStdin)?;
                    to_writer(&mut stdin, (encoding::UTF_8, false), &text)
                        .await
                        .map_err(|_| FormatterError::BrokenStdin)?;
                }

                let output = process
                    .wait_with_output()
                    .await
                    .map_err(|_| FormatterError::WaitForOutputFailed)?;

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
                        String::from_utf8_lossy(&output.stderr).to_string()
                    );
                }

                let str = std::str::from_utf8(&output.stdout)
                    .map_err(|_| FormatterError::InvalidUtf8Output)?;

                Ok(helix_core::diff::compare_ropes(&text, &Rope::from(str)))
            };
            return Some(formatting_future.boxed());
        };

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
            Some(path) => helix_core::path::get_canonicalized_path(&path)?,
            None => {
                if self.path.is_none() {
                    bail!("Can't save with no path set!");
                }

                self.path.as_ref().unwrap().clone()
            }
        };

        let identifier = self.path().map(|_| self.identifier());
        let language_server = self.language_server.clone();

        // mark changes up to now as saved
        let current_rev = self.get_current_revision();
        let doc_id = self.id();

        let encoding_with_bom_info = (self.encoding, self.has_bom);
        let last_saved_time = self.last_saved_time;

        // We encode the file according to the `Document`'s encoding.
        let future = async move {
            use tokio::{fs, fs::File};
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

            let mut file = File::create(&path).await?;
            to_writer(&mut file, encoding_with_bom_info, &text).await?;

            let event = DocumentSavedEvent {
                revision: current_rev,
                doc_id,
                path,
                text: text.clone(),
            };

            if let Some(language_server) = language_server {
                if !language_server.is_initialized() {
                    return Ok(event);
                }

                if let Some(identifier) = identifier {
                    if let Some(notification) =
                        language_server.text_document_did_save(identifier, &text)
                    {
                        notification.await?;
                    }
                }
            }

            Ok(event)
        };

        Ok(future)
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
    /// configured in `languages.toml`, with a fallback to tabs if it isn't specified. Line ending
    /// is likewise auto-detected, and will fallback to the default OS line ending.
    pub fn detect_indent_and_line_ending(&mut self) {
        self.indent_style = auto_detect_indent_style(&self.text).unwrap_or_else(|| {
            self.language_config()
                .and_then(|config| config.indent.as_ref())
                .map_or(DEFAULT_INDENT, |config| IndentStyle::from_str(&config.unit))
        });
        self.line_ending = auto_detect_line_ending(&self.text).unwrap_or(DEFAULT_LINE_ENDING);
    }

    /// Reload the document from its path.
    pub fn reload(
        &mut self,
        view: &mut View,
        provider_registry: &DiffProviderRegistry,
        redraw_handle: RedrawHandle,
    ) -> Result<(), Error> {
        let encoding = self.encoding;
        let path = self
            .path()
            .filter(|path| path.exists())
            .ok_or_else(|| anyhow!("can't find file to reload from {:?}", self.display_name()))?
            .to_owned();

        let mut file = std::fs::File::open(&path)?;
        let (rope, ..) = from_reader(&mut file, Some(encoding))?;

        // Calculate the difference between the buffer and source text, and apply it.
        // This is not considered a modification of the contents of the file regardless
        // of the encoding.
        let transaction = helix_core::diff::compare_ropes(self.text(), &rope);
        self.apply(&transaction, view.id);
        self.append_changes_to_history(view);
        self.reset_modified();

        self.last_saved_time = SystemTime::now();

        self.detect_indent_and_line_ending();

        match provider_registry.get_diff_base(&path) {
            Some(diff_base) => self.set_diff_base(diff_base, redraw_handle),
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
    ) -> anyhow::Result<()> {
        let language_config = config_loader
            .language_config_for_language_id(language_id)
            .ok_or_else(|| anyhow!("invalid language id: {}", language_id))?;
        self.set_language(Some(language_config), Some(config_loader));
        Ok(())
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

    /// Mark document as recent used for MRU sorting
    pub fn mark_as_focused(&mut self) {
        self.focused_at = std::time::Instant::now();
    }

    /// Remove a view's selection and inlay hints from this document.
    pub fn remove_view(&mut self, view_id: ViewId) {
        self.selections.remove(&view_id);
        self.inlay_hints.remove(&view_id);
    }

    /// Apply a [`Transaction`] to the [`Document`] to change its text.
    fn apply_impl(&mut self, transaction: &Transaction, view_id: ViewId) -> bool {
        use helix_core::Assoc;

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
            // start computing the diff in parallel
            if let Some(diff_handle) = &self.diff_handle {
                diff_handle.update_document(self.text.clone(), false);
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
                // TODO: no unwrap
                syntax
                    .update(&old_doc, &self.text, transaction.changes())
                    .unwrap();
            }

            let changes = transaction.changes();

            // map state.diagnostics over changes::map_pos too
            for diagnostic in &mut self.diagnostics {
                diagnostic.range.start = changes.map_pos(diagnostic.range.start, Assoc::After);
                diagnostic.range.end = changes.map_pos(diagnostic.range.end, Assoc::After);
                diagnostic.line = self.text.char_to_line(diagnostic.range.start);
            }
            self.diagnostics
                .sort_unstable_by_key(|diagnostic| diagnostic.range);

            // Update the inlay hint annotations' positions, helping ensure they are displayed in the proper place
            let apply_inlay_hint_changes = |annotations: &mut Rc<[InlineAnnotation]>| {
                if let Some(data) = Rc::get_mut(annotations) {
                    for inline in data.iter_mut() {
                        inline.char_idx = changes.map_pos(inline.char_idx, Assoc::After);
                    }
                }
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

            // emit lsp notification
            if let Some(language_server) = self.language_server() {
                let notify = language_server.text_document_did_change(
                    self.versioned_identifier(),
                    &old_doc,
                    self.text(),
                    changes,
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

    fn undo_redo_impl(&mut self, view: &mut View, undo: bool) -> bool {
        let mut history = self.history.take();
        let txn = if undo { history.undo() } else { history.redo() };
        let success = if let Some(txn) = txn {
            self.apply_impl(txn, view.id)
        } else {
            false
        };
        self.history.set(history);

        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text());
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
        let savepoint = Arc::new(SavePoint {
            view: view.id,
            revert: Mutex::new(revert),
        });
        self.savepoints.push(Arc::downgrade(&savepoint));
        savepoint
    }

    pub fn restore(&mut self, view: &mut View, savepoint: &SavePoint) {
        assert_eq!(
            savepoint.view, view.id,
            "Savepoint must not be used with a different view!"
        );
        // search and remove savepoint using a ptr comparison
        // this avoids a deadlock as we need to lock the mutex
        let savepoint_idx = self
            .savepoints
            .iter()
            .position(|savepoint_ref| savepoint_ref.as_ptr() == savepoint as *const _)
            .expect("Savepoint must belong to this document");

        let savepoint_ref = self.savepoints.remove(savepoint_idx);
        let mut revert = savepoint.revert.lock();
        self.apply(&revert, view.id);
        *revert = Transaction::new(self.text()).with_selection(self.selection(view.id).clone());
        self.savepoints.push(savepoint_ref)
    }

    fn earlier_later_impl(&mut self, view: &mut View, uk: UndoKind, earlier: bool) -> bool {
        let txns = if earlier {
            self.history.get_mut().earlier(uk)
        } else {
            self.history.get_mut().later(uk)
        };
        let mut success = false;
        for txn in txns {
            if self.apply_impl(&txn, view.id) {
                success = true;
            }
        }
        if success {
            // reset changeset to fix len
            self.changes = ChangeSet::new(self.text());
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

        let new_changeset = ChangeSet::new(self.text());
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
    pub fn set_last_saved_revision(&mut self, rev: usize) {
        log::debug!(
            "doc {} revision updated {} -> {}",
            self.id,
            self.last_saved_revision,
            rev
        );
        self.last_saved_revision = rev;
        self.last_saved_time = SystemTime::now();
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

    /// Language ID for the document. Either the `language-id` from the
    /// `language-server` configuration, or the document language if no
    /// `language-id` has been specified.
    pub fn language_id(&self) -> Option<&str> {
        let language_config = self.language.as_deref()?;

        language_config
            .language_server
            .as_ref()?
            .language_id
            .as_deref()
            .or(Some(language_config.language_id.as_str()))
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
        server.is_initialized().then_some(server)
    }

    pub fn diff_handle(&self) -> Option<&DiffHandle> {
        self.diff_handle.as_ref()
    }

    /// Intialize/updates the differ for this document with a new base.
    pub fn set_diff_base(&mut self, diff_base: Vec<u8>, redraw_handle: RedrawHandle) {
        if let Ok((diff_base, ..)) = from_reader(&mut diff_base.as_slice(), Some(self.encoding)) {
            if let Some(differ) = &self.diff_handle {
                differ.update_diff_base(diff_base);
                return;
            }
            self.diff_handle = Some(DiffHandle::new(diff_base, self.text.clone(), redraw_handle))
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
        self.language_config()
            .and_then(|config| config.indent.as_ref())
            .map_or(4, |config| config.tab_width) // fallback to 4 columns
    }

    // The width (in spaces) of a level of indentation.
    pub fn indent_width(&self) -> usize {
        self.indent_style.indent_width(self.tab_width())
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

    #[inline]
    pub fn selections(&self) -> &HashMap<ViewId, Selection> {
        &self.selections
    }

    pub fn relative_path(&self) -> Option<PathBuf> {
        self.path
            .as_deref()
            .map(helix_core::path::get_relative_path)
    }

    pub fn display_name(&self) -> Cow<'static, str> {
        self.relative_path()
            .map(|path| path.to_string_lossy().to_string().into())
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into())
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

    pub fn text_format(&self, mut viewport_width: u16, theme: Option<&Theme>) -> TextFormat {
        let config = self.config.load();
        let text_width = self
            .language_config()
            .and_then(|config| config.text_width)
            .unwrap_or(config.text_width);
        let soft_wrap_at_text_width = self
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
            // We increase max_line_len by 1 because softwrap considers the newline character
            // as part of the line length while the "typical" expectation is that this is not the case.
            // In particular other commands like :reflow do not count the line terminator.
            // This is technically inconsistent for the last line as that line never has a line terminator
            // but having the last visual line exceed the width by 1 seems like a rare edge case.
            viewport_width = viewport_width.min(text_width as u16 + 1)
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
                .and_then(|theme| theme.find_scope_index("ui.virtual.wrap"))
                .map(Highlight),
        }
    }

    /// Get the text annotations that apply to the whole document, those that do not apply to any
    /// specific view.
    pub fn text_annotations(&self, _theme: Option<&Theme>) -> TextAnnotations {
        TextAnnotations::default()
    }

    /// Set the inlay hints for this document and `view_id`.
    pub fn set_inlay_hints(&mut self, view_id: ViewId, inlay_hints: DocumentInlayHints) {
        self.inlay_hints.insert(view_id, inlay_hints);
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
    DiskReloadError(String),
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
            Self::DiskReloadError(error) => write!(f, "Error reloading file from disk: {}", error),
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
            Document::default(Arc::new(ArcSwap::new(Arc::new(Config::default()))))
                .text()
                .to_string(),
            DEFAULT_LINE_ENDING.as_str()
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
