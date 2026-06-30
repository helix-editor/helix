//! Editor-level state for the GitHub Copilot integration.
//!
//! The heavy lifting (the protocol) lives in [`helix_lsp::copilot`]. This module
//! only holds the running client handle and a little bit of session state so the
//! editor, commands and rendering code can reach it. The active inline
//! suggestion itself is stored per-[`Document`](crate::Document) as
//! [`CopilotCompletion`](crate::document::CopilotCompletion) so it can be
//! rendered as ghost text alongside the other text annotations.

use std::sync::Arc;

use helix_lsp::copilot::Client;

use crate::DocumentId;

/// Tracks the lifecycle of the Copilot connection for the editor.
#[derive(Default)]
pub struct Copilot {
    /// The running, initialized Copilot client, if one has been started.
    client: Option<Arc<Client>>,
    /// Runtime on/off toggle (`:copilot-toggle`). Initialized from
    /// `editor.copilot.enable`.
    pub enabled: bool,
    /// Whether the user is known to be signed in. Best-effort: updated after
    /// `checkStatus`/`signIn` round-trips.
    pub signed_in: bool,
    /// Documents that have been `didOpen`ed with the Copilot server, so we can
    /// send `didChange` afterwards.
    opened: std::collections::HashSet<DocumentId>,
}

impl Copilot {
    pub fn new(enabled: bool) -> Self {
        Self {
            client: None,
            enabled,
            signed_in: false,
            opened: std::collections::HashSet::new(),
        }
    }

    /// The running client handle, if Copilot has been started.
    pub fn client(&self) -> Option<Arc<Client>> {
        self.client.clone()
    }

    /// Whether a client has been started and initialized.
    pub fn is_running(&self) -> bool {
        self.client.is_some()
    }

    /// Store the started client handle.
    pub fn set_client(&mut self, client: Arc<Client>) {
        self.client = Some(client);
    }

    /// Drop the client handle (e.g. on `:copilot-toggle` off or shutdown).
    pub fn take_client(&mut self) -> Option<Arc<Client>> {
        self.opened.clear();
        self.client.take()
    }

    /// Whether the given document has already been opened with the server.
    pub fn is_open(&self, doc: DocumentId) -> bool {
        self.opened.contains(&doc)
    }

    /// Record that the given document has been opened with the server.
    pub fn mark_open(&mut self, doc: DocumentId) {
        self.opened.insert(doc);
    }

    /// Forget that a document was opened (e.g. when it is closed).
    pub fn forget(&mut self, doc: DocumentId) {
        self.opened.remove(&doc);
    }
}
