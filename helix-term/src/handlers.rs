use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_event::AsyncHook;


use crate::config::Config;
use crate::events;
use crate::handlers::completion::CompletionHandler;
use crate::handlers::signature_help::SignatureHelpHandler;
use crate::handlers::copilot::CopilotHandler;

pub use completion::trigger_auto_completion;
pub use helix_view::handlers::Handlers;


mod completion;
mod copilot;
mod signature_help;

pub fn setup(config: Arc<ArcSwap<Config>>, enable_copilot: bool) -> Handlers {
    events::register();

    let completions = CompletionHandler::new(config).spawn();
    let signature_hints = SignatureHelpHandler::new().spawn();
    let copilot = if enable_copilot {
        Some(CopilotHandler::new().spawn())
    } else {
        None
    };

    let handlers = Handlers {
        completions,
        signature_hints,
        copilot,
    };
    completion::register_hooks(&handlers);
    signature_help::register_hooks(&handlers);
    copilot::try_register_hooks(&handlers);
    handlers
}
