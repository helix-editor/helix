use helix_event::{register_hook, send_blocking};
use helix_view::document::Mode;
use helix_view::events::DiagnosticsDidChange;
use helix_view::handlers::diagnostics::DiagnosticEvent;
use helix_view::handlers::Handlers;

use crate::events::OnModeSwitch;

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DiagnosticsDidChange<'_>| {
        if event.editor.mode != Mode::Insert {
            for (view, _) in event.editor.tree.views_mut() {
                send_blocking(&view.diagnostics_handler.events, DiagnosticEvent::Refresh)
            }
        }
        Ok(())
    });
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        for (view, _) in event.cx.editor.tree.views_mut() {
            view.diagnostics_handler.active = event.new_mode != Mode::Insert;
        }
        Ok(())
    });
}
