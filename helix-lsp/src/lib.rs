mod client;
pub mod jsonrpc;
mod transport;

pub use client::Client;
pub use futures_executor::block_on;
pub use jsonrpc::Call;
pub use lsp::{Position, Url};
pub use lsp_types as lsp;

use futures_util::stream::select_all::SelectAll;
use helix_core::syntax::{LanguageConfiguration, LanguageServerConfiguration};
use tokio::sync::mpsc::UnboundedReceiver;

use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};
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
    #[error("request timed out")]
    Timeout,
    #[error("server closed the stream")]
    StreamClosed,
    #[error("Unhandled")]
    Unhandled,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OffsetEncoding {
    /// UTF-8 code units aka bytes
    #[serde(rename = "utf-8")]
    Utf8,
    /// UTF-16 code units
    #[serde(rename = "utf-16")]
    Utf16,
}

pub mod util {
    use super::*;
    use helix_core::{diagnostic::NumberOrString, Range, Rope, Transaction};

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

        // TODO: add support for Diagnostic.data
        lsp::Diagnostic::new(
            range_to_lsp_range(doc, range, offset_encoding),
            severity,
            code,
            diag.source.clone(),
            diag.message.to_owned(),
            None,
            tags,
        )
    }

    /// Converts [`lsp::Position`] to a position in the document.
    ///
    /// Returns `None` if position exceeds document length or an operation overflows.
    pub fn lsp_pos_to_pos(
        doc: &Rope,
        pos: lsp::Position,
        offset_encoding: OffsetEncoding,
    ) -> Option<usize> {
        let pos_line = pos.line as usize;
        if pos_line > doc.len_lines() - 1 {
            return None;
        }

        match offset_encoding {
            OffsetEncoding::Utf8 => {
                let line = doc.line_to_char(pos_line);
                let pos = line.checked_add(pos.character as usize)?;
                if pos <= doc.len_chars() {
                    Some(pos)
                } else {
                    None
                }
            }
            OffsetEncoding::Utf16 => {
                let line = doc.line_to_char(pos_line);
                let line_start = doc.char_to_utf16_cu(line);
                let pos = line_start.checked_add(pos.character as usize)?;
                doc.try_utf16_cu_to_char(pos).ok()
            }
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
                let line_start = doc.line_to_char(line);
                let col = pos - line_start;

                lsp::Position::new(line as u32, col as u32)
            }
            OffsetEncoding::Utf16 => {
                let line = doc.char_to_line(pos);
                let line_start = doc.char_to_utf16_cu(doc.line_to_char(line));
                let col = doc.char_to_utf16_cu(pos) - line_start;

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
                    start_client(id, language_config, config, doc_path)?;
                self.incoming.push(UnboundedReceiverStream::new(incoming));

                let (_, old_client) = entry.insert((id, client.clone()));

                tokio::spawn(async move {
                    let _ = old_client.force_shutdown().await;
                });

                Ok(Some(client))
            }
        }
    }

    pub fn get(
        &mut self,
        language_config: &LanguageConfiguration,
        doc_path: Option<&std::path::PathBuf>,
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
                    start_client(id, language_config, config, doc_path)?;
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
) -> Result<NewClientResult> {
    let (client, incoming, initialize_notify) = Client::start(
        &ls_config.command,
        &ls_config.args,
        config.config.clone(),
        &config.roots,
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
        test_case!("", (0, 1) => None);
        test_case!("", (1, 0) => None);
        test_case!("\n\n", (0, 0) => Some(0));
        test_case!("\n\n", (1, 0) => Some(1));
        test_case!("\n\n", (1, 1) => Some(2));
        test_case!("\n\n", (2, 0) => Some(2));
        test_case!("\n\n", (3, 0) => None);
        test_case!("test\n\n\n\ncase", (4, 3) => Some(11));
        test_case!("test\n\n\n\ncase", (4, 4) => Some(12));
        test_case!("test\n\n\n\ncase", (4, 5) => None);
        test_case!("", (u32::MAX, u32::MAX) => None);
    }
}
