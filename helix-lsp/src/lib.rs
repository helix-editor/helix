mod client;
mod transport;

use jsonrpc_core as jsonrpc;
use lsp_types as lsp;

pub use client::Client;
pub use lsp::{Position, Url};

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
}

/// A type representing all possible values sent from the server to the client.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
enum Message {
    /// A regular JSON-RPC request output (single response).
    Output(jsonrpc::Output),
    /// A notification.
    Notification(jsonrpc::Notification),
    /// A JSON-RPC request
    Call(jsonrpc::Call),
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
