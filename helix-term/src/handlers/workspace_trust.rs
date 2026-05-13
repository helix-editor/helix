use helix_event::register_hook;
use helix_loader::workspace_trust::{
    cache_non_trust_in_current_workspace, quick_query_workspace_with_explicit_untrust, TrustType,
    WorkspaceTrust,
};
use helix_view::{events::DocumentDidOpen, handlers::Handlers, DocumentId};

use crate::{compositor::Compositor, job, ui};

const ID: &str = "workspace-trust-select";

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |_event: &mut DocumentDidOpen<'_>| {
        if quick_query_workspace_with_explicit_untrust(TrustType::Select).is_none() {
            job::dispatch_blocking(|_editor, compositor| prompt(compositor));
        }
        Ok(())
    });
}

pub fn prompt(compositor: &mut Compositor) {
    let select = select();
    compositor.replace_or_push(ID, select);
}

const TRUST_MESSAGE: &str = "Trust this workspace?

Trusted workspaces can load local Helix config files and automatically start language servers, both of which may execute arbitrary code. Only trust workspaces you know are safe.";

#[derive(Default, Clone, Copy, Debug)]
pub enum TrustUntrustStatus {
    DenyAlways,
    #[default]
    DenyOnce,
    AllowAlways,
}

fn select() -> ui::Select<TrustUntrustStatus> {
    ui::Select::new(
        TRUST_MESSAGE,
        [
            TrustUntrustStatus::DenyOnce,
            TrustUntrustStatus::DenyAlways,
            TrustUntrustStatus::AllowAlways,
        ],
        (),
        move |editor, option, event| {
            if event == ui::PromptEvent::Validate {
                let mut trust = WorkspaceTrust::load(true);
                match option {
                    TrustUntrustStatus::DenyAlways => {
                        trust.exclude_workspace();
                    }
                    TrustUntrustStatus::DenyOnce => {
                        cache_non_trust_in_current_workspace();
                    }
                    TrustUntrustStatus::AllowAlways => {
                        trust.trust_workspace();

                        let documents: Vec<DocumentId> = editor.documents.keys().cloned().collect();
                        for document_id in documents.iter() {
                            editor.launch_language_servers(*document_id);
                        }

                        let _ = editor
                            .config_events
                            .0
                            .send(helix_view::editor::ConfigEvent::Refresh);
                    }
                }
            }
        },
    )
}

impl crate::ui::menu::Item for TrustUntrustStatus {
    type Data = ();

    fn format(&self, _data: &Self::Data) -> tui::widgets::Row<'_> {
        match self {
            TrustUntrustStatus::DenyAlways => "Never",
            TrustUntrustStatus::DenyOnce => "Not now",
            TrustUntrustStatus::AllowAlways => "Always",
        }
        .into()
    }
}
