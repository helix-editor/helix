mod client;
mod transport;

pub use jsonrpc_core as jsonrpc;
pub use lsp_types as lsp;

pub use once_cell::sync::{Lazy, OnceCell};

pub use client::Client;
pub use lsp::{Position, Url};

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

    pub fn lsp_pos_to_pos(doc: &helix_core::RopeSlice, pos: lsp::Position) -> usize {
        let line = doc.line_to_char(pos.line as usize);
        let line_start = doc.char_to_utf16_cu(line);
        doc.utf16_cu_to_char(pos.character as usize + line_start)
    }
    pub fn pos_to_lsp_pos(doc: &helix_core::RopeSlice, pos: usize) -> lsp::Position {
        let line = doc.char_to_line(pos);
        let line_start = doc.char_to_utf16_cu(line);
        let col = doc.char_to_utf16_cu(pos) - line_start;

        lsp::Position::new(line as u64, col as u64)
    }
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

pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::init);

pub struct Registry {
    inner: HashMap<LanguageId, OnceCell<Arc<Client>>>,
}

impl Registry {
    pub fn init() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn get(&self, id: &str, ex: &smol::Executor) -> Option<Arc<Client>> {
        // TODO: use get_or_try_init and propagate the error
        self.inner
            .get(id)
            .map(|cell| {
                cell.get_or_init(|| {
                    // TODO: lookup defaults for id (name, args)

                    // initialize a new client
                    let client = Client::start(&ex, "rust-analyzer", &[]);
                    // TODO: also call initialize().await()
                    Arc::new(client)
                })
            })
            .cloned()
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
