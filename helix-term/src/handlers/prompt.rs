use helix_event::register_hook;
use helix_view::events::DocumentFocusLost;
use helix_view::handlers::Handlers;

use crate::job::{self};
use crate::ui;

#[allow(unused_variables)]
pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentFocusLost<'_>| {
        job::dispatch_blocking(move |_, compositor| {
            if compositor.find::<ui::Prompt>().is_some() {
                compositor.remove_type::<ui::Prompt>();
            }
        });
        Ok(())
    });
}
