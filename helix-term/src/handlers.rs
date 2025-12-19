use std::sync::Arc;

use arc_swap::ArcSwap;
use diagnostics::PullAllDocumentsDiagnosticHandler;
use helix_event::AsyncHook;

use crate::config::Config;
use crate::events;
use crate::handlers::auto_reload::PollHandler;
use crate::handlers::auto_save::AutoSaveHandler;
use crate::handlers::diagnostics::PullDiagnosticsHandler;
use crate::handlers::signature_help::SignatureHelpHandler;

pub use helix_view::handlers::{word_index, Handlers};

use self::document_colors::DocumentColorsHandler;

mod auto_reload;
mod auto_save;
pub mod completion;
pub mod diagnostics;
mod document_colors;
mod prompt;
mod signature_help;
mod snippet;

pub fn setup(config: Arc<ArcSwap<Config>>) -> Handlers {
    events::register();

    let event_tx = completion::CompletionHandler::new(config.clone()).spawn();
    let signature_hints = SignatureHelpHandler::new().spawn();
    let auto_save = AutoSaveHandler::new().spawn();
    let auto_reload = PollHandler::new().spawn();
    let document_colors = DocumentColorsHandler::default().spawn();
    let word_index = word_index::Handler::spawn();
    let pull_diagnostics = PullDiagnosticsHandler::default().spawn();
    let pull_all_documents_diagnostics = PullAllDocumentsDiagnosticHandler::default().spawn();

    let handlers = Handlers {
        completions: helix_view::handlers::completion::CompletionHandler::new(event_tx),
        signature_hints,
        auto_save,
        auto_reload,
        document_colors,
        word_index,
        pull_diagnostics,
        pull_all_documents_diagnostics,
    };

    helix_view::handlers::register_hooks(&handlers);
    completion::register_hooks(&handlers);
    signature_help::register_hooks(&handlers);
    auto_save::register_hooks(&handlers);
    diagnostics::register_hooks(&handlers);
    snippet::register_hooks(&handlers);
    document_colors::register_hooks(&handlers);
    prompt::register_hooks(&handlers);
    auto_reload::register_hooks(&handlers, &config.load().editor);
    handlers
}
