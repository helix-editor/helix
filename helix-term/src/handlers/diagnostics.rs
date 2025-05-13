use helix_event::{register_hook, send_blocking};
use helix_view::document::Mode;
use helix_view::events::DiagnosticsDidChange;
use helix_view::handlers::diagnostics::DiagnosticEvent;
use helix_view::handlers::Handlers;

use crate::events::OnModeSwitch;

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DiagnosticsDidChange<'_>| {
        for (_, c) in event.editor.clients.iter() {
            if c.mode != Mode::Insert {
                for (view, _) in c.tree.views(&event.editor.views) {
                    send_blocking(&view.diagnostics_handler.events, DiagnosticEvent::Refresh)
                }
            }
        }
        Ok(())
    });
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        let ids = client!(event.cx.editor, event.cx.client_id)
            .tree
            .views(&event.cx.editor.views)
            .map(|(v, _)| v.id)
            .collect::<Vec<_>>();
        for id in ids {
            view_mut!(event.cx.editor, id).diagnostics_handler.active =
                event.new_mode != Mode::Insert;
        }
        Ok(())
    });
}
