use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use helix_event::register_hook;
use helix_loader::workspace_trust::TrustStatus;
use helix_view::{events::DocumentDidOpen, handlers::Handlers, DocumentId};

use crate::{compositor::Compositor, job, ui};

const ID: &str = "workspace-trust-select";

pub(super) fn register_hooks(_handlers: &Handlers) {
    // Tracks which workspaces have already been prompted (or auto-dismissed) during this session.
    // Without this, every document opened in an untrusted workspace would re-dispatch the modal —
    // `deny_once` writes `Untrusted` to the trust cache but `restricted_for_doc` returns `true`
    // based purely on workspace state, so the cache alone is not enough to suppress re-prompts.
    // Inserts return false when the path was already present, which we use to short-circuit.
    let prompted: Arc<Mutex<HashSet<PathBuf>>> = Arc::default();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        let doc_id = event.doc;

        let (workspace, servers_to_load) = {
            let doc = doc!(event.editor, &doc_id);
            (doc.workspace_root().to_path_buf(), doc.servers_to_load())
        };

        // Stale: `.helix/` was edited since the user last ran `trust`. LSPs keep
        // running (binaries are unchanged), but local config is dropped.
        // Note: must use `status` (raw) not `query`: `query` collapses Stale to Untrusted via `demote_for_query`.
        if event.editor.workspace_trust.status(&workspace) == TrustStatus::Stale {
            event.editor.set_status(
                "Workspace `.helix/` config changed since `:workspace-trust`. \
                 Local config not loaded. Run `:workspace-trust` to re-allow.",
            );
            return Ok(());
        }

        if !event
            .editor
            .workspace_trust
            .restricted_for_doc(&workspace, servers_to_load)
        {
            return Ok(());
        }

        // Users who opt out of the modal still get the statusline `[⚠]` indicator and can act
        // explicitly via `:workspace-trust`.
        if !event.editor.workspace_trust.prompts_enabled() {
            return Ok(());
        }

        // First time we've seen this workspace this session — prompt once.
        if !prompted.lock().unwrap().insert(workspace.clone()) {
            return Ok(());
        }

        // Cache "denied for this session" so future trust queries treat the workspace as untrusted.
        event.editor.workspace_trust.deny_once(&workspace);
        let workspace = workspace.clone();
        job::dispatch_blocking(move |_editor, compositor| prompt(workspace, compositor));

        Ok(())
    });
}

fn prompt(workspace: PathBuf, compositor: &mut Compositor) {
    let select = select(workspace);
    compositor.replace_or_push(ID, select);
}

const TRUST_MESSAGE: &str = "Trust this workspace?

Trusted workspaces may load local Helix config files (`.helix/*`) and auto-start language servers. \
Both can execute arbitrary code. Only trust workspaces whose contents you have inspected.";

#[derive(Default, Clone, Copy, Debug)]
pub enum TrustChoice {
    #[default]
    Trust,
    Never,
}

fn select(workspace: PathBuf) -> ui::Select<TrustChoice> {
    ui::Select::new(
        TRUST_MESSAGE,
        [TrustChoice::Trust, TrustChoice::Never],
        (),
        move |editor, option, event| {
            if event != ui::PromptEvent::Validate {
                return;
            }
            match option {
                TrustChoice::Trust => {
                    editor.workspace_trust.trust(&workspace);
                    let documents: Vec<DocumentId> = editor.documents.keys().cloned().collect();
                    for document_id in documents.iter() {
                        editor.launch_language_servers(*document_id);
                    }
                    let _ = editor
                        .config_events
                        .0
                        .send(helix_view::editor::ConfigEvent::Refresh);
                }
                TrustChoice::Never => {
                    editor.workspace_trust.exclude(&workspace);
                    // Drop any workspace overrides that snuck into the live editor config before
                    // the user excluded the workspace.
                    let _ = editor
                        .config_events
                        .0
                        .send(helix_view::editor::ConfigEvent::Refresh);
                }
            }
        },
    )
}

impl crate::ui::menu::Item for TrustChoice {
    type Data = ();

    fn format(&self, _data: &Self::Data) -> tui::widgets::Row<'_> {
        match self {
            TrustChoice::Trust => "Trust",
            TrustChoice::Never => "Never",
        }
        .into()
    }
}
