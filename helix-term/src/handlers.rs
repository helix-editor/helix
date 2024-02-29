use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_event::AsyncHook;

use crate::config::Config;
use crate::events;
use crate::handlers::completion::CompletionHandler;
use crate::handlers::signature_help::SignatureHelpHandler;
use crate::handlers::auto_save::AutoSaveHandler;

pub use completion::trigger_auto_completion;
pub use helix_view::handlers::lsp::SignatureHelpInvoked;
pub use helix_view::handlers::Handlers;

mod completion;
mod signature_help;
mod auto_save;

pub fn setup(config: Arc<ArcSwap<Config>>) -> Handlers {
    events::register();

    let completions = CompletionHandler::new(config).spawn();
    let signature_hints = SignatureHelpHandler::new().spawn();
    let auto_save = AutoSaveHandler::new().spawn();

    let handlers = Handlers {
        completions,
        signature_hints,
        auto_save,
    };

    completion::register_hooks(&handlers);
    signature_help::register_hooks(&handlers);
    auto_save::register_hooks(&handlers);
    handlers
}
