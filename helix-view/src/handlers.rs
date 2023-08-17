use std::sync::Arc;

use helix_event::send_blocking;
use tokio::sync::mpsc::Sender;

use crate::handlers::lsp::SignatureHelpInvoked;
use crate::Editor;

pub mod dap;
pub mod lsp;

pub struct Handlers {}
