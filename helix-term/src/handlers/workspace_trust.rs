use std::{collections::HashSet, path::PathBuf};

use helix_event::register_hook;
use helix_loader::workspace_trust::{
    quick_query_workspace_with_explicit_untrust, workspace_needs_trust, TrustUntrustStatus,
    WorkspaceTrust,
};
use helix_view::{
    editor::WorkspaceTrustLevelConfig, events::DocumentDidOpen, handlers::Handlers, DocumentId,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::{compositor::Compositor, job, ui};

const ID: &str = "workspace-trust-select";

/// A set of canonicalized workspace paths which have been prompted for trust at runtime.
static PROMPTED_WORKSPACES: Lazy<Mutex<HashSet<PathBuf>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if workspace_needs_trust()
            && quick_query_workspace_with_explicit_untrust(
                event.editor.config.load().workspace_trust_level == WorkspaceTrustLevelConfig::All,
            ) == TrustUntrustStatus::DenyOnce
        {
            let (workspace, _) = helix_loader::find_workspace();
            job::dispatch_blocking(|_editor, compositor| prompt(workspace, compositor));
        }
        Ok(())
    });
}

pub fn prompt(path: PathBuf, compositor: &mut Compositor) {
    let mut workspaces = PROMPTED_WORKSPACES.lock();
    if workspaces.contains(&path) {
        return;
    } else {
        workspaces.insert(path.clone());
    }
    let select = select();
    compositor.replace_or_push(ID, select);
}

const TRUST_MESSAGE: &str = "Trust this workspace?

Trusted workspaces may load local config files and auto-start language servers. Config and language servers can execute arbitrary code. Only trust workspaces which you know contain harmless config and code.

This prompt can be disabled by setting `editor.workspace-trust-level` to \"lsp\" to always allow starting language servers, or \"all\" to always allow all insecure features.";

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
                        // Do nothing
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
