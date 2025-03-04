use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_event::AsyncHook;

use crate::config::Config;
use crate::events;
use crate::handlers::auto_save::AutoSaveHandler;
use crate::handlers::inlay_hints::InlayHintHandler;
use crate::handlers::signature_help::SignatureHelpHandler;

pub use helix_view::handlers::Handlers;

mod auto_save;
pub mod completion;
mod diagnostics;
mod inlay_hints;
mod signature_help;
mod snippet;

pub fn setup(config: Arc<ArcSwap<Config>>) -> Handlers {
    events::register();

    let completion_sender = completion::CompletionHandler::new(config).spawn();
    let signature_hints = SignatureHelpHandler::new().spawn();
    let inlay_hint_sender = InlayHintHandler::new().spawn();
    let auto_save = AutoSaveHandler::new().spawn();

    let handlers = Handlers {
        completions: helix_view::handlers::completion::CompletionHandler::new(completion_sender),
        signature_hints,
        inlay_hints: helix_view::handlers::lsp::InlayHintHandler::new(inlay_hint_sender),
        auto_save,
    };

    helix_view::handlers::register_hooks(&handlers);
    completion::register_hooks(&handlers);
    signature_help::register_hooks(&handlers);
    auto_save::register_hooks(&handlers);
    diagnostics::register_hooks(&handlers);
    snippet::register_hooks(&handlers);
    inlay_hints::register_hooks(&handlers);
    handlers
}
