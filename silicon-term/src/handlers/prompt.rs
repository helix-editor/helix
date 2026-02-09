use silicon_event::register_hook;
use silicon_view::events::DocumentFocusLost;
use silicon_view::handlers::Handlers;

use crate::job::{self};
use crate::ui;

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |_event: &mut DocumentFocusLost<'_>| {
        job::dispatch_blocking(move |_, compositor| {
            if compositor.find::<ui::Prompt>().is_some() {
                compositor.remove_type::<ui::Prompt>();
            }
        });
        Ok(())
    });
}
