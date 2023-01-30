mod client;
pub mod jsonrpc;
pub mod snippet;
mod transport;

pub use client::Client;
pub use futures_executor::block_on;
pub use jsonrpc::Call;
pub use lsp::{Position, Url};
pub use lsp_types as lsp;

use futures_util::stream::select_all::SelectAll;
use helix_core::{
    find_workspace,
    syntax::{LanguageConfiguration, LanguageServerConfiguration},
};
use tokio::sync::mpsc::UnboundedReceiver;

use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use thiserror::Error;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub type Result<T> = core::result::Result<T, Error>;
type LanguageId = String;

#[derive(Error, Debug)]
pub enum Error {
    #[error("protocol error: {0}")]
    Rpc(#[from] jsonrpc::Error),
    #[error("failed to parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("request {0} timed out")]
    Timeout(jsonrpc::Id),
    #[error("server closed the stream")]
    StreamClosed,
    #[error("Unhandled")]
    Unhandled,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone, Copy, Debug, Default)]
pub enum OffsetEncoding {
    /// UTF-8 code units aka bytes
    Utf8,
    /// UTF-32 code units aka chars
    Utf32,
    /// UTF-16 code units
    #[default]
    Utf16,
}

pub mod util {
    use super::*;
    use helix_core::line_ending::{line_end_byte_index, line_end_char_index};
    use helix_core::{chars, RopeSlice, SmallVec};
    use helix_core::{diagnostic::NumberOrString, Range, Rope, Selection, Tendril, Transaction};

    /// Converts a diagnostic in the document to [`lsp::Diagnostic`].
    ///
    /// Panics when [`pos_to_lsp_pos`] would for an invalid range on the diagnostic.
    pub fn diagnostic_to_lsp_diagnostic(
        doc: &Rope,
        diag: &helix_core::diagnostic::Diagnostic,
        offset_encoding: OffsetEncoding,
    ) -> lsp::Diagnostic {
        use helix_core::diagnostic::Severity::*;

        let range = Range::new(diag.range.start, diag.range.end);
        let severity = diag.severity.map(|s| match s {
            Hint => lsp::DiagnosticSeverity::HINT,
            Info => lsp::DiagnosticSeverity::INFORMATION,
            Warning => lsp::DiagnosticSeverity::WARNING,
            Error => lsp::DiagnosticSeverity::ERROR,
        });

        let code = match diag.code.clone() {
            Some(x) => match x {
                NumberOrString::Number(x) => Some(lsp::NumberOrString::Number(x)),
                NumberOrString::String(x) => Some(lsp::NumberOrString::String(x)),
            },
            None => None,
        };

        let new_tags: Vec<_> = diag
            .tags
            .iter()
            .map(|tag| match tag {
                helix_core::diagnostic::DiagnosticTag::Unnecessary => {
                    lsp::DiagnosticTag::UNNECESSARY
                }
                helix_core::diagnostic::DiagnosticTag::Deprecated => lsp::DiagnosticTag::DEPRECATED,
            })
            .collect();

        let tags = if !new_tags.is_empty() {
            Some(new_tags)
        } else {
            None
        };

        lsp::Diagnostic {
            range: range_to_lsp_range(doc, range, offset_encoding),
            severity,
            code,
            source: diag.source.clone(),
            message: diag.message.to_owned(),
            related_information: None,
            tags,
            data: diag.data.to_owned(),
            ..Default::default()
        }
    }

    /// Converts [`lsp::Position`] to a position in the document.
    ///
    /// Returns `None` if position.line is out of bounds or an overflow occurs
    pub fn lsp_pos_to_pos(
        doc: &Rope,
        pos: lsp::Position,
        offset_encoding: OffsetEncoding,
    ) -> Option<usize> {
        let pos_line = pos.line as usize;
        if pos_line > doc.len_lines() - 1 {
            return None;
        }

        // We need to be careful here to fully comply ith the LSP spec.
        // Two relevant quotes from the spec:
        //
        // https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position
        // > If the character value is greater than the line length it defaults back
        // >  to the line length.
        //
        // https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocuments
        // > To ensure that both client and server split the string into the same
        // > line representation the protocol specifies the following end-of-line sequences:
        // > ‘\n’, ‘\r\n’ and ‘\r’. Positions are line end character agnostic.
        // > So you can not specify a position that denotes \r|\n or \n| where | represents the character offset.
        //
        // This means that while the line must be in bounds the `charater`
        // must be capped to the end of the line.
        // Note that the end of the line here is **before** the line terminator
        // so we must use `line_end_char_index` istead of `doc.line_to_char(pos_line + 1)`
        //
        // FIXME: Helix does not fully comply with the LSP spec for line terminators.
        // The LSP standard requires that line terminators are ['\n', '\r\n', '\r'].
        // Without the unicode-linebreak feature disabled, the `\r` terminator is not handled by helix.
        // With the unicode-linebreak feature, helix recognizes multiple extra line break chars
        // which means that positions will be decoded/encoded incorrectly in their presence

        let line = match offset_encoding {
            OffsetEncoding::Utf8 => {
                let line_start = doc.line_to_byte(pos_line);
                let line_end = line_end_byte_index(&doc.slice(..), pos_line);
                line_start..line_end
            }
            OffsetEncoding::Utf16 => {
                // TODO directly translate line index to char-idx
                // ropey can do this just as easily as utf-8 byte translation
                // but the functions are just missing.
                // Translate to char first and then utf-16 as a workaround
                let line_start = doc.line_to_char(pos_line);
                let line_end = line_end_char_index(&doc.slice(..), pos_line);
                doc.char_to_utf16_cu(line_start)..doc.char_to_utf16_cu(line_end)
            }
            OffsetEncoding::Utf32 => {
                let line_start = doc.line_to_char(pos_line);
                let line_end = line_end_char_index(&doc.slice(..), pos_line);
                line_start..line_end
            }
        };

        // The LSP spec demands that the offset is capped to the end of the line
        let pos = line
            .start
            .checked_add(pos.character as usize)
            .unwrap_or(line.end)
            .min(line.end);

        match offset_encoding {
            OffsetEncoding::Utf8 => doc.try_byte_to_char(pos).ok(),
            OffsetEncoding::Utf16 => doc.try_utf16_cu_to_char(pos).ok(),
            OffsetEncoding::Utf32 => Some(pos),
        }
    }

    /// Converts position in the document to [`lsp::Position`].
    ///
    /// Panics when `pos` is out of `doc` bounds or operation overflows.
    pub fn pos_to_lsp_pos(
        doc: &Rope,
        pos: usize,
        offset_encoding: OffsetEncoding,
    ) -> lsp::Position {
        match offset_encoding {
            OffsetEncoding::Utf8 => {
                let line = doc.char_to_line(pos);
                let line_start = doc.line_to_byte(line);
                let col = doc.char_to_byte(pos) - line_start;

                lsp::Position::new(line as u32, col as u32)
            }
            OffsetEncoding::Utf16 => {
                let line = doc.char_to_line(pos);
                let line_start = doc.char_to_utf16_cu(doc.line_to_char(line));
                let col = doc.char_to_utf16_cu(pos) - line_start;

                lsp::Position::new(line as u32, col as u32)
            }
            OffsetEncoding::Utf32 => {
                let line = doc.char_to_line(pos);
                let line_start = doc.line_to_char(line);
                let col = pos - line_start;

                lsp::Position::new(line as u32, col as u32)
            }
        }
    }

    /// Converts a range in the document to [`lsp::Range`].
    pub fn range_to_lsp_range(
        doc: &Rope,
        range: Range,
        offset_encoding: OffsetEncoding,
    ) -> lsp::Range {
        let start = pos_to_lsp_pos(doc, range.from(), offset_encoding);
        let end = pos_to_lsp_pos(doc, range.to(), offset_encoding);

        lsp::Range::new(start, end)
    }

    pub fn lsp_range_to_range(
        doc: &Rope,
        range: lsp::Range,
        offset_encoding: OffsetEncoding,
    ) -> Option<Range> {
        let start = lsp_pos_to_pos(doc, range.start, offset_encoding)?;
        let end = lsp_pos_to_pos(doc, range.end, offset_encoding)?;

        Some(Range::new(start, end))
    }

    /// If the LS did not provide a range for the completion or the range of the
    /// primary cursor can not be used for the secondary cursor, this function
    /// can be used to find the completion range for a cursor
    fn find_completion_range(text: RopeSlice, replace_mode: bool, cursor: usize) -> (usize, usize) {
        let start = cursor
            - text
                .chars_at(cursor)
                .reversed()
                .take_while(|ch| chars::char_is_word(*ch))
                .count();
        let mut end = cursor;
        if replace_mode {
            end += text
                .chars_at(cursor)
                .skip(1)
                .take_while(|ch| chars::char_is_word(*ch))
                .count();
        }
        (start, end)
    }
    fn completion_range(
        text: RopeSlice,
        edit_offset: Option<(i128, i128)>,
        replace_mode: bool,
        cursor: usize,
    ) -> Option<(usize, usize)> {
        let res = match edit_offset {
            Some((start_offset, end_offset)) => {
                let start_offset = cursor as i128 + start_offset;
                if start_offset < 0 {
                    return None;
                }
                let end_offset = cursor as i128 + end_offset;
                if end_offset > text.len_chars() as i128 {
                    return None;
                }
                (start_offset as usize, end_offset as usize)
            }
            None => find_completion_range(text, replace_mode, cursor),
        };
        Some(res)
    }

    /// Creates a [Transaction] from the [lsp::TextEdit] in a completion response.
    /// The transaction applies the edit to all cursors.
    pub fn generate_transaction_from_completion_edit(
        doc: &Rope,
        selection: &Selection,
        edit_offset: Option<(i128, i128)>,
        replace_mode: bool,
        new_text: String,
    ) -> Transaction {
        let replacement: Option<Tendril> = if new_text.is_empty() {
            None
        } else {
            Some(new_text.into())
        };

        let text = doc.slice(..);
        let (removed_start, removed_end) = completion_range(
            text,
            edit_offset,
            replace_mode,
            selection.primary().cursor(text),
        )
        .expect("transaction must be valid for primary selection");
        let removed_text = text.slice(removed_start..removed_end);

        let (transaction, mut selection) = Transaction::change_by_selection_ignore_overlapping(
            doc,
            selection,
            |range| {
                let cursor = range.cursor(text);
                completion_range(text, edit_offset, replace_mode, cursor)
                    .filter(|(start, end)| text.slice(start..end) == removed_text)
                    .unwrap_or_else(|| find_completion_range(text, replace_mode, cursor))
            },
            |_, _| replacement.clone(),
        );
        if transaction.changes().is_empty() {
            return transaction;
        }
        selection = selection.map(transaction.changes());
        transaction.with_selection(selection)
    }

    /// Creates a [Transaction] from the [snippet::Snippet] in a completion response.
    /// The transaction applies the edit to all cursors.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_transaction_from_snippet(
        doc: &Rope,
        selection: &Selection,
        edit_offset: Option<(i128, i128)>,
        replace_mode: bool,
        snippet: snippet::Snippet,
        line_ending: &str,
        include_placeholder: bool,
        tab_width: usize,
        indent_width: usize,
    ) -> Transaction {
        let text = doc.slice(..);

        let mut off = 0i128;
        let mut mapped_doc = doc.clone();
        let mut selection_tabstops: SmallVec<[_; 1]> = SmallVec::new();
        let (removed_start, removed_end) = completion_range(
            text,
            edit_offset,
            replace_mode,
            selection.primary().cursor(text),
        )
        .expect("transaction must be valid for primary selection");
        let removed_text = text.slice(removed_start..removed_end);

        let (transaction, selection) = Transaction::change_by_selection_ignore_overlapping(
            doc,
            selection,
            |range| {
                let cursor = range.cursor(text);
                completion_range(text, edit_offset, replace_mode, cursor)
                    .filter(|(start, end)| text.slice(start..end) == removed_text)
                    .unwrap_or_else(|| find_completion_range(text, replace_mode, cursor))
            },
            |replacement_start, replacement_end| {
                let mapped_replacement_start = (replacement_start as i128 + off) as usize;
                let mapped_replacement_end = (replacement_end as i128 + off) as usize;

                let line_idx = mapped_doc.char_to_line(mapped_replacement_start);
                let indent_level = helix_core::indent::indent_level_for_line(
                    mapped_doc.line(line_idx),
                    tab_width,
                    indent_width,
                ) * indent_width;

                let newline_with_offset = format!(
                    "{line_ending}{blank:indent_level$}",
                    line_ending = line_ending,
                    blank = ""
                );

                let (replacement, tabstops) =
                    snippet::render(&snippet, &newline_with_offset, include_placeholder);
                selection_tabstops.push((mapped_replacement_start, tabstops));
                mapped_doc.remove(mapped_replacement_start..mapped_replacement_end);
                mapped_doc.insert(mapped_replacement_start, &replacement);
                off +=
                    replacement_start as i128 - replacement_end as i128 + replacement.len() as i128;

                Some(replacement)
            },
        );

        let changes = transaction.changes();
        if changes.is_empty() {
            return transaction;
        }

        let mut mapped_selection = SmallVec::with_capacity(selection.len());
        let mut mapped_primary_idx = 0;
        let primary_range = selection.primary();
        for (range, (tabstop_anchor, tabstops)) in selection.into_iter().zip(selection_tabstops) {
            if range == primary_range {
                mapped_primary_idx = mapped_selection.len()
            }

            let range = range.map(changes);
            let tabstops = tabstops.first().filter(|tabstops| !tabstops.is_empty());
            let Some(tabstops) = tabstops else{
                // no tabstop normal mapping
                mapped_selection.push(range);
                continue;
            };

            // expand the selection to cover the tabstop to retain the helix selection semantic
            // the tabstop closest to the range simply replaces `head` while anchor remains in place
            // the remaining tabstops receive their own single-width cursor
            if range.head < range.anchor {
                let first_tabstop = tabstop_anchor + tabstops[0].1;

                // if selection is forward but was moved to the right it is
                // contained entirely in the replacement text, just do a point
                // selection (fallback below)
                if range.anchor >= first_tabstop {
                    let range = Range::new(range.anchor, first_tabstop);
                    mapped_selection.push(range);
                    let rem_tabstops = tabstops[1..]
                        .iter()
                        .map(|tabstop| Range::point(tabstop_anchor + tabstop.1));
                    mapped_selection.extend(rem_tabstops);
                    continue;
                }
            } else {
                let last_idx = tabstops.len() - 1;
                let last_tabstop = tabstop_anchor + tabstops[last_idx].1;

                // if selection is forward but was moved to the right it is
                // contained entirely in the replacement text, just do a point
                // selection (fallback below)
                if range.anchor <= last_tabstop {
                    // we can't properly compute the the next grapheme
                    // here because the transaction hasn't been applied yet
                    // that is not a problem because the range gets grapheme aligned anyway
                    // tough so just adding one will always cause head to be grapheme
                    // aligned correctly when applied to the document
                    let range = Range::new(range.anchor, last_tabstop + 1);
                    mapped_selection.push(range);
                    let rem_tabstops = tabstops[..last_idx]
                        .iter()
                        .map(|tabstop| Range::point(tabstop_anchor + tabstop.0));
                    mapped_selection.extend(rem_tabstops);
                    continue;
                }
            };

            let tabstops = tabstops
                .iter()
                .map(|tabstop| Range::point(tabstop_anchor + tabstop.0));
            mapped_selection.extend(tabstops);
        }

        transaction.with_selection(Selection::new(mapped_selection, mapped_primary_idx))
    }

    pub fn generate_transaction_from_edits(
        doc: &Rope,
        mut edits: Vec<lsp::TextEdit>,
        offset_encoding: OffsetEncoding,
    ) -> Transaction {
        // Sort edits by start range, since some LSPs (Omnisharp) send them
        // in reverse order.
        edits.sort_unstable_by_key(|edit| edit.range.start);

        // Generate a diff if the edit is a full document replacement.
        #[allow(clippy::collapsible_if)]
        if edits.len() == 1 {
            let is_document_replacement = edits.first().and_then(|edit| {
                let start = lsp_pos_to_pos(doc, edit.range.start, offset_encoding)?;
                let end = lsp_pos_to_pos(doc, edit.range.end, offset_encoding)?;
                Some(start..end)
            }) == Some(0..doc.len_chars());
            if is_document_replacement {
                let new_text = Rope::from(edits.pop().unwrap().new_text);
                return helix_core::diff::compare_ropes(doc, &new_text);
            }
        }

        Transaction::change(
            doc,
            edits.into_iter().map(|edit| {
                // simplify "" into None for cleaner changesets
                let replacement = if !edit.new_text.is_empty() {
                    Some(edit.new_text.into())
                } else {
                    None
                };

                let start =
                    if let Some(start) = lsp_pos_to_pos(doc, edit.range.start, offset_encoding) {
                        start
                    } else {
                        return (0, 0, None);
                    };
                let end = if let Some(end) = lsp_pos_to_pos(doc, edit.range.end, offset_encoding) {
                    end
                } else {
                    return (0, 0, None);
                };
                (start, end, replacement)
            }),
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MethodCall {
    WorkDoneProgressCreate(lsp::WorkDoneProgressCreateParams),
    ApplyWorkspaceEdit(lsp::ApplyWorkspaceEditParams),
    WorkspaceFolders,
    WorkspaceConfiguration(lsp::ConfigurationParams),
    RegisterCapability(lsp::RegistrationParams),
}

impl MethodCall {
    pub fn parse(method: &str, params: jsonrpc::Params) -> Result<MethodCall> {
        use lsp::request::Request;
        let request = match method {
            lsp::request::WorkDoneProgressCreate::METHOD => {
                let params: lsp::WorkDoneProgressCreateParams = params.parse()?;
                Self::WorkDoneProgressCreate(params)
            }
            lsp::request::ApplyWorkspaceEdit::METHOD => {
                let params: lsp::ApplyWorkspaceEditParams = params.parse()?;
                Self::ApplyWorkspaceEdit(params)
            }
            lsp::request::WorkspaceFoldersRequest::METHOD => Self::WorkspaceFolders,
            lsp::request::WorkspaceConfiguration::METHOD => {
                let params: lsp::ConfigurationParams = params.parse()?;
                Self::WorkspaceConfiguration(params)
            }
            lsp::request::RegisterCapability::METHOD => {
                let params: lsp::RegistrationParams = params.parse()?;
                Self::RegisterCapability(params)
            }
            _ => {
                return Err(Error::Unhandled);
            }
        };
        Ok(request)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Notification {
    // we inject this notification to signal the LSP is ready
    Initialized,
    // and this notification to signal that the LSP exited
    Exit,
    PublishDiagnostics(lsp::PublishDiagnosticsParams),
    ShowMessage(lsp::ShowMessageParams),
    LogMessage(lsp::LogMessageParams),
    ProgressMessage(lsp::ProgressParams),
}

impl Notification {
    pub fn parse(method: &str, params: jsonrpc::Params) -> Result<Notification> {
        use lsp::notification::Notification as _;

        let notification = match method {
            lsp::notification::Initialized::METHOD => Self::Initialized,
            lsp::notification::Exit::METHOD => Self::Exit,
            lsp::notification::PublishDiagnostics::METHOD => {
                let params: lsp::PublishDiagnosticsParams = params.parse()?;
                Self::PublishDiagnostics(params)
            }

            lsp::notification::ShowMessage::METHOD => {
                let params: lsp::ShowMessageParams = params.parse()?;
                Self::ShowMessage(params)
            }
            lsp::notification::LogMessage::METHOD => {
                let params: lsp::LogMessageParams = params.parse()?;
                Self::LogMessage(params)
            }
            lsp::notification::Progress::METHOD => {
                let params: lsp::ProgressParams = params.parse()?;
                Self::ProgressMessage(params)
            }
            _ => {
                return Err(Error::Unhandled);
            }
        };

        Ok(notification)
    }
}

#[derive(Debug)]
pub struct Registry {
    inner: HashMap<LanguageId, (usize, Arc<Client>)>,

    counter: AtomicUsize,
    pub incoming: SelectAll<UnboundedReceiverStream<(usize, Call)>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            counter: AtomicUsize::new(0),
            incoming: SelectAll::new(),
        }
    }

    pub fn get_by_id(&self, id: usize) -> Option<&Client> {
        self.inner
            .values()
            .find(|(client_id, _)| client_id == &id)
            .map(|(_, client)| client.as_ref())
    }

    pub fn remove_by_id(&mut self, id: usize) {
        self.inner.retain(|_, (client_id, _)| client_id != &id)
    }

    pub fn restart(
        &mut self,
        language_config: &LanguageConfiguration,
        doc_path: Option<&std::path::PathBuf>,
        root_dirs: &[PathBuf],
    ) -> Result<Option<Arc<Client>>> {
        let config = match &language_config.language_server {
            Some(config) => config,
            None => return Ok(None),
        };

        let scope = language_config.scope.clone();

        match self.inner.entry(scope) {
            Entry::Vacant(_) => Ok(None),
            Entry::Occupied(mut entry) => {
                // initialize a new client
                let id = self.counter.fetch_add(1, Ordering::Relaxed);

                let NewClientResult(client, incoming) =
                    start_client(id, language_config, config, doc_path, root_dirs)?;
                self.incoming.push(UnboundedReceiverStream::new(incoming));

                let (_, old_client) = entry.insert((id, client.clone()));

                tokio::spawn(async move {
                    let _ = old_client.force_shutdown().await;
                });

                Ok(Some(client))
            }
        }
    }

    pub fn stop(&mut self, language_config: &LanguageConfiguration) {
        let scope = language_config.scope.clone();

        if let Some((_, client)) = self.inner.remove(&scope) {
            tokio::spawn(async move {
                let _ = client.force_shutdown().await;
            });
        }
    }

    pub fn get(
        &mut self,
        language_config: &LanguageConfiguration,
        doc_path: Option<&std::path::PathBuf>,
        root_dirs: &[PathBuf],
    ) -> Result<Option<Arc<Client>>> {
        let config = match &language_config.language_server {
            Some(config) => config,
            None => return Ok(None),
        };

        match self.inner.entry(language_config.scope.clone()) {
            Entry::Occupied(entry) => Ok(Some(entry.get().1.clone())),
            Entry::Vacant(entry) => {
                // initialize a new client
                let id = self.counter.fetch_add(1, Ordering::Relaxed);

                let NewClientResult(client, incoming) =
                    start_client(id, language_config, config, doc_path, root_dirs)?;
                self.incoming.push(UnboundedReceiverStream::new(incoming));

                entry.insert((id, client.clone()));
                Ok(Some(client))
            }
        }
    }

    pub fn iter_clients(&self) -> impl Iterator<Item = &Arc<Client>> {
        self.inner.values().map(|(_, client)| client)
    }
}

#[derive(Debug)]
pub enum ProgressStatus {
    Created,
    Started(lsp::WorkDoneProgress),
}

impl ProgressStatus {
    pub fn progress(&self) -> Option<&lsp::WorkDoneProgress> {
        match &self {
            ProgressStatus::Created => None,
            ProgressStatus::Started(progress) => Some(progress),
        }
    }
}

#[derive(Default, Debug)]
/// Acts as a container for progress reported by language servers. Each server
/// has a unique id assigned at creation through [`Registry`]. This id is then used
/// to store the progress in this map.
pub struct LspProgressMap(HashMap<usize, HashMap<lsp::ProgressToken, ProgressStatus>>);

impl LspProgressMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a map of all tokens corresponding to the language server with `id`.
    pub fn progress_map(&self, id: usize) -> Option<&HashMap<lsp::ProgressToken, ProgressStatus>> {
        self.0.get(&id)
    }

    pub fn is_progressing(&self, id: usize) -> bool {
        self.0.get(&id).map(|it| !it.is_empty()).unwrap_or_default()
    }

    /// Returns last progress status for a given server with `id` and `token`.
    pub fn progress(&self, id: usize, token: &lsp::ProgressToken) -> Option<&ProgressStatus> {
        self.0.get(&id).and_then(|values| values.get(token))
    }

    /// Checks if progress `token` for server with `id` is created.
    pub fn is_created(&mut self, id: usize, token: &lsp::ProgressToken) -> bool {
        self.0
            .get(&id)
            .map(|values| values.get(token).is_some())
            .unwrap_or_default()
    }

    pub fn create(&mut self, id: usize, token: lsp::ProgressToken) {
        self.0
            .entry(id)
            .or_default()
            .insert(token, ProgressStatus::Created);
    }

    /// Ends the progress by removing the `token` from server with `id`, if removed returns the value.
    pub fn end_progress(
        &mut self,
        id: usize,
        token: &lsp::ProgressToken,
    ) -> Option<ProgressStatus> {
        self.0.get_mut(&id).and_then(|vals| vals.remove(token))
    }

    /// Updates the progress of `token` for server with `id` to `status`, returns the value replaced or `None`.
    pub fn update(
        &mut self,
        id: usize,
        token: lsp::ProgressToken,
        status: lsp::WorkDoneProgress,
    ) -> Option<ProgressStatus> {
        self.0
            .entry(id)
            .or_default()
            .insert(token, ProgressStatus::Started(status))
    }
}

struct NewClientResult(Arc<Client>, UnboundedReceiver<(usize, Call)>);

/// start_client takes both a LanguageConfiguration and a LanguageServerConfiguration to ensure that
/// it is only called when it makes sense.
fn start_client(
    id: usize,
    config: &LanguageConfiguration,
    ls_config: &LanguageServerConfiguration,
    doc_path: Option<&std::path::PathBuf>,
    root_dirs: &[PathBuf],
) -> Result<NewClientResult> {
    let (client, incoming, initialize_notify) = Client::start(
        &ls_config.command,
        &ls_config.args,
        config.config.clone(),
        ls_config.environment.clone(),
        &config.roots,
        config.workspace_lsp_roots.as_deref().unwrap_or(root_dirs),
        id,
        ls_config.timeout,
        doc_path,
    )?;

    let client = Arc::new(client);

    // Initialize the client asynchronously
    let _client = client.clone();
    tokio::spawn(async move {
        use futures_util::TryFutureExt;
        let value = _client
            .capabilities
            .get_or_try_init(|| {
                _client
                    .initialize()
                    .map_ok(|response| response.capabilities)
            })
            .await;

        if let Err(e) = value {
            log::error!("failed to initialize language server: {}", e);
            return;
        }

        // next up, notify<initialized>
        _client
            .notify::<lsp::notification::Initialized>(lsp::InitializedParams {})
            .await
            .unwrap();

        initialize_notify.notify_one();
    });

    Ok(NewClientResult(client, incoming))
}

/// Find an LSP root of a file using the following mechansim:
/// * start at `file` (either an absolute path or relative to CWD)
/// * find the top most directory containing a root_marker
/// * inside the current workspace
///     * stop the search at the first root_dir that contains `file` or the workspace (obtained from `helix_core::find_workspace`)
///     * root_dirs only apply inside the workspace. For files outside of the workspace they are ignored
/// * outside the current workspace: keep searching to the top of the file hiearchy
pub fn find_root(file: &str, root_markers: &[String], root_dirs: &[PathBuf]) -> PathBuf {
    let file = std::path::Path::new(file);
    let workspace = find_workspace();
    let file = if file.is_absolute() {
        file.to_path_buf()
    } else {
        let current_dir = std::env::current_dir().expect("unable to determine current directory");
        current_dir.join(file)
    };

    let inside_workspace = file.strip_prefix(&workspace).is_ok();

    let mut top_marker = None;
    for ancestor in file.ancestors() {
        if root_markers
            .iter()
            .any(|marker| ancestor.join(marker).exists())
        {
            top_marker = Some(ancestor);
        }

        if inside_workspace
            && (ancestor == workspace
                || root_dirs
                    .iter()
                    .any(|root_dir| root_dir == ancestor.strip_prefix(&workspace).unwrap()))
        {
            return top_marker.unwrap_or(ancestor).to_owned();
        }
    }

    // If no root was found use the workspace as a fallback
    workspace
}

#[cfg(test)]
mod tests {
    use super::{lsp, util::*, OffsetEncoding};
    use helix_core::Rope;

    #[test]
    fn converts_lsp_pos_to_pos() {
        macro_rules! test_case {
            ($doc:expr, ($x:expr, $y:expr) => $want:expr) => {
                let doc = Rope::from($doc);
                let pos = lsp::Position::new($x, $y);
                assert_eq!($want, lsp_pos_to_pos(&doc, pos, OffsetEncoding::Utf16));
                assert_eq!($want, lsp_pos_to_pos(&doc, pos, OffsetEncoding::Utf8))
            };
        }

        test_case!("", (0, 0) => Some(0));
        test_case!("", (0, 1) => Some(0));
        test_case!("", (1, 0) => None);
        test_case!("\n\n", (0, 0) => Some(0));
        test_case!("\n\n", (1, 0) => Some(1));
        test_case!("\n\n", (1, 1) => Some(1));
        test_case!("\n\n", (2, 0) => Some(2));
        test_case!("\n\n", (3, 0) => None);
        test_case!("test\n\n\n\ncase", (4, 3) => Some(11));
        test_case!("test\n\n\n\ncase", (4, 4) => Some(12));
        test_case!("test\n\n\n\ncase", (4, 5) => Some(12));
        test_case!("", (u32::MAX, u32::MAX) => None);
    }

    #[test]
    fn emoji_format_gh_4791() {
        use lsp_types::{Position, Range, TextEdit};

        let edits = vec![
            TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 1,
                    },
                    end: Position {
                        line: 1,
                        character: 0,
                    },
                },
                new_text: "\n  ".to_string(),
            },
            TextEdit {
                range: Range {
                    start: Position {
                        line: 1,
                        character: 7,
                    },
                    end: Position {
                        line: 2,
                        character: 0,
                    },
                },
                new_text: "\n  ".to_string(),
            },
        ];

        let mut source = Rope::from_str("[\n\"🇺🇸\",\n\"🎄\",\n]");

        let transaction = generate_transaction_from_edits(&source, edits, OffsetEncoding::Utf8);
        assert!(transaction.apply(&mut source));
    }
}
