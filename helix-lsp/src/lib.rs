mod client;
mod select_all;
mod transport;

pub use jsonrpc_core as jsonrpc;
pub use lsp_types as lsp;

pub use client::Client;
pub use lsp::{Position, Url};

use helix_core::syntax::LanguageConfiguration;

use thiserror::Error;

use std::{collections::HashMap, sync::Arc};

#[derive(Error, Debug)]
pub enum Error {
    #[error("protocol error: {0}")]
    Rpc(#[from] jsonrpc::Error),
    #[error("failed to parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("request timed out")]
    Timeout,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub mod util {
    use super::*;
    use helix_core::{Range, Rope, RopeSlice, Transaction};

    pub fn lsp_pos_to_pos(doc: RopeSlice, pos: lsp::Position) -> usize {
        let line = doc.line_to_char(pos.line as usize);
        let line_start = doc.char_to_utf16_cu(line);
        doc.utf16_cu_to_char(line_start + pos.character as usize)
    }
    pub fn pos_to_lsp_pos(doc: RopeSlice, pos: usize) -> lsp::Position {
        let line = doc.char_to_line(pos);
        let line_start = doc.char_to_utf16_cu(doc.line_to_char(line));
        let col = doc.char_to_utf16_cu(pos) - line_start;

        lsp::Position::new(line as u32, col as u32)
    }

    pub fn range_to_lsp_range(doc: RopeSlice, range: Range) -> lsp::Range {
        let start = pos_to_lsp_pos(doc, range.from());
        let end = pos_to_lsp_pos(doc, range.to());

        lsp::Range::new(start, end)
    }

    pub fn generate_transaction_from_edits(doc: &Rope, edits: Vec<lsp::TextEdit>) -> Transaction {
        let text = doc.slice(..); // would be unnecessary if Transaction::change took Rope | RopeSlice
        Transaction::change(
            doc,
            edits.into_iter().map(|edit| {
                // simplify "" into None for cleaner changesets
                let replacement = if !edit.new_text.is_empty() {
                    Some(edit.new_text.into())
                } else {
                    None
                };

                let start = lsp_pos_to_pos(text, edit.range.start);
                let end = lsp_pos_to_pos(text, edit.range.end);
                (start, end, replacement)
            }),
        )
    }

    // apply_insert_replace_edit
}

#[derive(Debug, PartialEq, Clone)]
pub enum Notification {
    PublishDiagnostics(lsp::PublishDiagnosticsParams),
}

impl Notification {
    pub fn parse(method: &str, params: jsonrpc::Params) -> Notification {
        use lsp::notification::Notification as _;

        match method {
            lsp::notification::PublishDiagnostics::METHOD => {
                let params: lsp::PublishDiagnosticsParams = params
                    .parse()
                    .expect("Failed to parse PublishDiagnostics params");

                // TODO: need to loop over diagnostics and distinguish them by URI
                Notification::PublishDiagnostics(params)
            }
            _ => unimplemented!("unhandled notification: {}", method),
        }
    }
}

pub use jsonrpc::Call;

type LanguageId = String;

use crate::select_all::SelectAll;
use smol::channel::Receiver;

pub struct Registry {
    inner: HashMap<LanguageId, Arc<Client>>,

    pub incoming: SelectAll<Receiver<Call>>,
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

    pub fn get(
        &mut self,
        language_config: &LanguageConfiguration,
        ex: &smol::Executor,
    ) -> Option<Arc<Client>> {
        // TODO: propagate the error
        if let Some(config) = &language_config.language_server_config {
            // avoid borrow issues
            let inner = &mut self.inner;
            let s_incoming = &self.incoming;

            let language_server = inner
                .entry(language_config.scope.clone()) // can't use entry with Borrow keys: https://github.com/rust-lang/rfcs/pull/1769
                .or_insert_with(|| {
                    // TODO: lookup defaults for id (name, args)

                    // initialize a new client
                    let (mut client, incoming) = Client::start(&ex, &config.command, &config.args);
                    // TODO: run this async without blocking
                    smol::block_on(client.initialize()).unwrap();

                    s_incoming.push(incoming);

                    Arc::new(client)
                })
                .clone();

            return Some(language_server);
        }

        None
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
