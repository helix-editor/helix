mod client;
pub mod file_event;
mod file_operations;
pub mod jsonrpc;
pub mod snippet;
mod transport;

pub use client::Client;
pub use futures_executor::block_on;
pub use jsonrpc::Call;
pub use lsp::{Position, Url};
pub use lsp_types as lsp;

use futures_util::stream::select_all::SelectAll;
use helix_core::syntax::{
    LanguageConfiguration, LanguageServerConfiguration, LanguageServerFeatures,
};
use helix_stdx::path;
use tokio::sync::mpsc::UnboundedReceiver;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use thiserror::Error;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub type Result<T> = core::result::Result<T, Error>;
pub type LanguageServerName = String;

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
    ExecutableNotFound(#[from] helix_stdx::env::ExecutableNotFoundError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
            // If it extends past the end, truncate it to the end. This is because the
            // way the LSP describes the range including the last newline is by
            // specifying a line number after what we would call the last line.
            log::warn!("LSP position {pos:?} out of range assuming EOF");
            return Some(doc.len_chars());
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
        // This means that while the line must be in bounds the `character`
        // must be capped to the end of the line.
        // Note that the end of the line here is **before** the line terminator
        // so we must use `line_end_char_index` instead of `doc.line_to_char(pos_line + 1)`
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
        mut range: lsp::Range,
        offset_encoding: OffsetEncoding,
    ) -> Option<Range> {
        // This is sort of an edgecase. It's not clear from the spec how to deal with
        // ranges where end < start. They don't make much sense but vscode simply caps start to end
        // and because it's not specified quite a few LS rely on this as a result (for example the TS server)
        if range.start > range.end {
            log::error!(
                "Invalid LSP range start {:?} > end {:?}, using an empty range at the end instead",
                range.start,
                range.end
            );
            range.start = range.end;
        }
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

        let (transaction, mut selection) = Transaction::change_by_selection_ignore_overlapping(
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

        // Don't normalize to avoid merging/reording selections which would
        // break the association between tabstops and selections. Most ranges
        // will be replaced by tabstops anyways and the final selection will be
        // normalized anyways
        selection = selection.map_no_normalize(changes);
        let mut mapped_selection = SmallVec::with_capacity(selection.len());
        let mut mapped_primary_idx = 0;
        let primary_range = selection.primary();
        for (range, (tabstop_anchor, tabstops)) in selection.into_iter().zip(selection_tabstops) {
            if range == primary_range {
                mapped_primary_idx = mapped_selection.len()
            }

            let tabstops = tabstops.first().filter(|tabstops| !tabstops.is_empty());
            let Some(tabstops) = tabstops else {
                // no tabstop normal mapping
                mapped_selection.push(range);
                continue;
            };

            // expand the selection to cover the tabstop to retain the helix selection semantic
            // the tabstop closest to the range simply replaces `head` while anchor remains in place
            // the remaining tabstops receive their own single-width cursor
            if range.head < range.anchor {
                let last_idx = tabstops.len() - 1;
                let last_tabstop = tabstop_anchor + tabstops[last_idx].0;

                // if selection is forward but was moved to the right it is
                // contained entirely in the replacement text, just do a point
                // selection (fallback below)
                if range.anchor > last_tabstop {
                    let range = Range::new(range.anchor, last_tabstop);
                    mapped_selection.push(range);
                    let rem_tabstops = tabstops[..last_idx]
                        .iter()
                        .map(|tabstop| Range::point(tabstop_anchor + tabstop.0));
                    mapped_selection.extend(rem_tabstops);
                    continue;
                }
            } else {
                let first_tabstop = tabstop_anchor + tabstops[0].0;

                // if selection is forward but was moved to the right it is
                // contained entirely in the replacement text, just do a point
                // selection (fallback below)
                if range.anchor < first_tabstop {
                    // we can't properly compute the the next grapheme
                    // here because the transaction hasn't been applied yet
                    // that is not a problem because the range gets grapheme aligned anyway
                    // tough so just adding one will always cause head to be grapheme
                    // aligned correctly when applied to the document
                    let range = Range::new(range.anchor, first_tabstop + 1);
                    mapped_selection.push(range);
                    let rem_tabstops = tabstops[1..]
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
    UnregisterCapability(lsp::UnregistrationParams),
    ShowDocument(lsp::ShowDocumentParams),
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
            lsp::request::UnregisterCapability::METHOD => {
                let params: lsp::UnregistrationParams = params.parse()?;
                Self::UnregisterCapability(params)
            }
            lsp::request::ShowDocument::METHOD => {
                let params: lsp::ShowDocumentParams = params.parse()?;
                Self::ShowDocument(params)
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
    inner: HashMap<LanguageServerName, Vec<Arc<Client>>>,
    syn_loader: Arc<helix_core::syntax::Loader>,
    counter: usize,
    pub incoming: SelectAll<UnboundedReceiverStream<(usize, Call)>>,
    pub file_event_handler: file_event::Handler,
}

impl Registry {
    pub fn new(syn_loader: Arc<helix_core::syntax::Loader>) -> Self {
        Self {
            inner: HashMap::new(),
            syn_loader,
            counter: 0,
            incoming: SelectAll::new(),
            file_event_handler: file_event::Handler::new(),
        }
    }

    pub fn get_by_id(&self, id: usize) -> Option<&Client> {
        self.inner
            .values()
            .flatten()
            .find(|client| client.id() == id)
            .map(|client| &**client)
    }

    pub fn remove_by_id(&mut self, id: usize) {
        self.file_event_handler.remove_client(id);
        self.inner.retain(|_, language_servers| {
            language_servers.retain(|ls| id != ls.id());
            !language_servers.is_empty()
        });
    }

    fn start_client(
        &mut self,
        name: String,
        ls_config: &LanguageConfiguration,
        doc_path: Option<&std::path::PathBuf>,
        root_dirs: &[PathBuf],
        enable_snippets: bool,
    ) -> Result<Arc<Client>> {
        let config = self
            .syn_loader
            .language_server_configs()
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("Language server '{name}' not defined"))?;
        let id = self.counter;
        self.counter += 1;
        let NewClient(client, incoming) = start_client(
            id,
            name,
            ls_config,
            config,
            doc_path,
            root_dirs,
            enable_snippets,
        )?;
        self.incoming.push(UnboundedReceiverStream::new(incoming));
        Ok(client)
    }

    /// If this method is called, all documents that have a reference to language servers used by the language config have to refresh their language servers,
    /// as it could be that language servers of these documents were stopped by this method.
    /// See helix_view::editor::Editor::refresh_language_servers
    pub fn restart(
        &mut self,
        language_config: &LanguageConfiguration,
        doc_path: Option<&std::path::PathBuf>,
        root_dirs: &[PathBuf],
        enable_snippets: bool,
    ) -> Result<Vec<Arc<Client>>> {
        language_config
            .language_servers
            .iter()
            .filter_map(|LanguageServerFeatures { name, .. }| {
                if self.inner.contains_key(name) {
                    let client = match self.start_client(
                        name.clone(),
                        language_config,
                        doc_path,
                        root_dirs,
                        enable_snippets,
                    ) {
                        Ok(client) => client,
                        error => return Some(error),
                    };
                    let old_clients = self
                        .inner
                        .insert(name.clone(), vec![client.clone()])
                        .unwrap();

                    for old_client in old_clients {
                        self.file_event_handler.remove_client(old_client.id());
                        tokio::spawn(async move {
                            let _ = old_client.force_shutdown().await;
                        });
                    }

                    Some(Ok(client))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn stop(&mut self, name: &str) {
        if let Some(clients) = self.inner.remove(name) {
            for client in clients {
                self.file_event_handler.remove_client(client.id());
                tokio::spawn(async move {
                    let _ = client.force_shutdown().await;
                });
            }
        }
    }

    pub fn get<'a>(
        &'a mut self,
        language_config: &'a LanguageConfiguration,
        doc_path: Option<&'a std::path::PathBuf>,
        root_dirs: &'a [PathBuf],
        enable_snippets: bool,
    ) -> impl Iterator<Item = (LanguageServerName, Result<Arc<Client>>)> + 'a {
        language_config.language_servers.iter().map(
            move |LanguageServerFeatures { name, .. }| {
                if let Some(clients) = self.inner.get(name) {
                    if let Some((_, client)) = clients.iter().enumerate().find(|(i, client)| {
                        client.try_add_doc(&language_config.roots, root_dirs, doc_path, *i == 0)
                    }) {
                        return (name.to_owned(), Ok(client.clone()));
                    }
                }
                match self.start_client(
                    name.clone(),
                    language_config,
                    doc_path,
                    root_dirs,
                    enable_snippets,
                ) {
                    Ok(client) => {
                        self.inner
                            .entry(name.to_owned())
                            .or_default()
                            .push(client.clone());
                        (name.clone(), Ok(client))
                    }
                    Err(err) => (name.to_owned(), Err(err)),
                }
            },
        )
    }

    pub fn iter_clients(&self) -> impl Iterator<Item = &Arc<Client>> {
        self.inner.values().flatten()
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

struct NewClient(Arc<Client>, UnboundedReceiver<(usize, Call)>);

/// start_client takes both a LanguageConfiguration and a LanguageServerConfiguration to ensure that
/// it is only called when it makes sense.
fn start_client(
    id: usize,
    name: String,
    config: &LanguageConfiguration,
    ls_config: &LanguageServerConfiguration,
    doc_path: Option<&std::path::PathBuf>,
    root_dirs: &[PathBuf],
    enable_snippets: bool,
) -> Result<NewClient> {
    let (client, incoming, initialize_notify) = Client::start(
        &ls_config.command,
        &ls_config.args,
        ls_config.config.clone(),
        ls_config.environment.clone(),
        &config.roots,
        config.workspace_lsp_roots.as_deref().unwrap_or(root_dirs),
        id,
        name,
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
                    .initialize(enable_snippets)
                    .map_ok(|response| response.capabilities)
            })
            .await;

        if let Err(e) = value {
            log::error!("failed to initialize language server: {}", e);
            return;
        }

        // next up, notify<initialized>
        let notification_result = _client
            .notify::<lsp::notification::Initialized>(lsp::InitializedParams {})
            .await;

        if let Err(e) = notification_result {
            log::error!(
                "failed to notify language server of its initialization: {}",
                e
            );
            return;
        }

        initialize_notify.notify_one();
    });

    Ok(NewClient(client, incoming))
}

/// Find an LSP workspace of a file using the following mechanism:
/// * if the file is outside `workspace` return `None`
/// * start at `file` and search the file tree upward
/// * stop the search at the first `root_dirs` entry that contains `file`
/// * if no `root_dirs` matches `file` stop at workspace
/// * Returns the top most directory that contains a `root_marker`
/// * If no root marker and we stopped at a `root_dirs` entry, return the directory we stopped at
/// * If we stopped at `workspace` instead and `workspace_is_cwd == false` return `None`
/// * If we stopped at `workspace` instead and `workspace_is_cwd == true` return `workspace`
pub fn find_lsp_workspace(
    file: &str,
    root_markers: &[String],
    root_dirs: &[PathBuf],
    workspace: &Path,
    workspace_is_cwd: bool,
) -> Option<PathBuf> {
    let file = std::path::Path::new(file);
    let mut file = if file.is_absolute() {
        file.to_path_buf()
    } else {
        let current_dir = helix_stdx::env::current_working_dir();
        current_dir.join(file)
    };
    file = path::normalize(&file);

    if !file.starts_with(workspace) {
        return None;
    }

    let mut top_marker = None;
    for ancestor in file.ancestors() {
        if root_markers
            .iter()
            .any(|marker| ancestor.join(marker).exists())
        {
            top_marker = Some(ancestor);
        }

        if root_dirs
            .iter()
            .any(|root_dir| path::normalize(workspace.join(root_dir)) == ancestor)
        {
            // if the worskapce is the cwd do not search any higher for workspaces
            // but specify
            return Some(top_marker.unwrap_or(workspace).to_owned());
        }
        if ancestor == workspace {
            // if the workspace is the CWD, let the LSP decide what the workspace
            // is
            return top_marker
                .or_else(|| (!workspace_is_cwd).then_some(workspace))
                .map(Path::to_owned);
        }
    }

    debug_assert!(false, "workspace must be an ancestor of <file>");
    None
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
        test_case!("", (1, 0) => Some(0));
        test_case!("\n\n", (0, 0) => Some(0));
        test_case!("\n\n", (1, 0) => Some(1));
        test_case!("\n\n", (1, 1) => Some(1));
        test_case!("\n\n", (2, 0) => Some(2));
        test_case!("\n\n", (3, 0) => Some(2));
        test_case!("test\n\n\n\ncase", (4, 3) => Some(11));
        test_case!("test\n\n\n\ncase", (4, 4) => Some(12));
        test_case!("test\n\n\n\ncase", (4, 5) => Some(12));
        test_case!("", (u32::MAX, u32::MAX) => Some(0));
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
