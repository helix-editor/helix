mod client;
mod transport;

pub use client::Client;
pub use futures_executor::block_on;
pub use jsonrpc::Call;
pub use jsonrpc_core as jsonrpc;
pub use lsp::{Position, Url};
pub use lsp_types as lsp;

use futures_util::stream::select_all::SelectAll;
use helix_core::syntax::LanguageConfiguration;

use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
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
    #[error("LSP not defined")]
    LspNotDefined,
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
    use helix_core::{Range, Rope, Transaction};

    pub fn lsp_pos_to_pos(
        doc: &Rope,
        pos: lsp::Position,
        offset_encoding: OffsetEncoding,
    ) -> usize {
        match offset_encoding {
            OffsetEncoding::Utf8 => {
                let line = doc.line_to_char(pos.line as usize);
                line + pos.character as usize
            }
            OffsetEncoding::Utf16 => {
                let line = doc.line_to_char(pos.line as usize);
                let line_start = doc.char_to_utf16_cu(line);
                doc.utf16_cu_to_char(line_start + pos.character as usize)
            }
        }
    }
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

    pub fn range_to_lsp_range(
        doc: &Rope,
        range: Range,
        offset_encoding: OffsetEncoding,
    ) -> lsp::Range {
        let start = pos_to_lsp_pos(doc, range.from(), offset_encoding);
        let end = pos_to_lsp_pos(doc, range.to(), offset_encoding);

        lsp::Range::new(start, end)
    }

    pub fn generate_transaction_from_edits(
        doc: &Rope,
        edits: Vec<lsp::TextEdit>,
        offset_encoding: OffsetEncoding,
    ) -> Transaction {
        Transaction::change(
            doc,
            edits.into_iter().map(|edit| {
                // simplify "" into None for cleaner changesets
                let replacement = if !edit.new_text.is_empty() {
                    Some(edit.new_text.into())
                } else {
                    None
                };

                let start = lsp_pos_to_pos(doc, edit.range.start, offset_encoding);
                let end = lsp_pos_to_pos(doc, edit.range.end, offset_encoding);
                (start, end, replacement)
            }),
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Notification {
    PublishDiagnostics(lsp::PublishDiagnosticsParams),
    ShowMessage(lsp::ShowMessageParams),
    LogMessage(lsp::LogMessageParams),
}

impl Notification {
    pub fn parse(method: &str, params: jsonrpc::Params) -> Option<Notification> {
        use lsp::notification::Notification as _;

        let notification = match method {
            lsp::notification::PublishDiagnostics::METHOD => {
                let params: lsp::PublishDiagnosticsParams = params
                    .parse()
                    .expect("Failed to parse PublishDiagnostics params");

                // TODO: need to loop over diagnostics and distinguish them by URI
                Self::PublishDiagnostics(params)
            }

            lsp::notification::ShowMessage::METHOD => {
                let params: lsp::ShowMessageParams =
                    params.parse().expect("Failed to parse ShowMessage params");

                Self::ShowMessage(params)
            }
            lsp::notification::LogMessage::METHOD => {
                let params: lsp::LogMessageParams =
                    params.parse().expect("Failed to parse ShowMessage params");

                Self::LogMessage(params)
            }
            _ => {
                log::error!("unhandled LSP notification: {}", method);
                return None;
            }
        };

        Some(notification)
    }
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Registry {
    inner: HashMap<LanguageId, Arc<Client>>,

    pub incoming: SelectAll<UnboundedReceiverStream<Call>>,
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
            incoming: SelectAll::new(),
        }
    }

    pub fn get(&mut self, language_config: &LanguageConfiguration) -> Result<Arc<Client>> {
        if let Some(config) = &language_config.language_server {
            // avoid borrow issues
            let inner = &mut self.inner;
            let s_incoming = &mut self.incoming;

            match inner.entry(language_config.scope.clone()) {
                Entry::Occupied(language_server) => Ok(language_server.get().clone()),
                Entry::Vacant(entry) => {
                    // initialize a new client
                    let (mut client, incoming) = Client::start(&config.command, &config.args)?;
                    // TODO: run this async without blocking
                    futures_executor::block_on(client.initialize())?;
                    s_incoming.push(UnboundedReceiverStream::new(incoming));
                    let client = Arc::new(client);

                    entry.insert(client.clone());
                    Ok(client)
                }
            }
        } else {
            Err(Error::LspNotDefined)
        }
    }
}

// REGISTRY = HashMap<LanguageId, Lazy/OnceCell<Arc<RwLock<Client>>>
// spawn one server per language type, need to spawn one per workspace if server doesn't support
// workspaces
//
// could also be a client per root dir
//
// storing a copy of Option<Arc<RwLock<Client>>> on Document would make the LSP client easily
// accessible during edit/save callbacks
//
// the event loop needs to process all incoming streams, maybe we can just have that be a separate
// task that's continually running and store the state on the client, then use read lock to
// retrieve data during render
// -> PROBLEM: how do you trigger an update on the editor side when data updates?
//
// -> The data updates should pull all events until we run out so we don't frequently re-render
//
//
// v2:
//
// there should be a registry of lsp clients, one per language type (or workspace).
// the clients should lazy init on first access
// the client.initialize() should be called async and we buffer any requests until that completes
// there needs to be a way to process incoming lsp messages from all clients.
//  -> notifications need to be dispatched to wherever
//  -> requests need to generate a reply and travel back to the same lsp!
