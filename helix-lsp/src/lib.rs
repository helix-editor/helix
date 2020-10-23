mod client;
mod transport;

pub use jsonrpc_core as jsonrpc;
pub use lsp_types as lsp;

pub use client::Client;
pub use lsp::{Position, Url};

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
