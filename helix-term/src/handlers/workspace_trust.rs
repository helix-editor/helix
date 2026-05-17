use helix_core::config::user_lang_config;
use helix_event::register_hook;
use helix_loader::workspace_trust::TrustType;
use helix_stdx::env::which;
use helix_view::{events::DocumentDidOpen, handlers::Handlers, DocumentId};

use crate::{compositor::Compositor, job, ui};

const ID: &str = "workspace-trust-select";

pub(super) fn register_hooks(_handlers: &Handlers) {
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        let doc_id = event.doc;
        let wst = &event.editor.workspace_trust;
        let lang = if let Some(doc) = event.editor.document(doc_id) {
            doc.language_config()
        } else {
            None
        };

        let language_servers_to_load = if let Some(lang) = lang {
            if lang.language_servers.is_empty() {
                false
            } else if let Ok(config) = user_lang_config(wst) {
                lang.language_servers.iter().any(|a| {
                    if let Some(val) = config.language_server.get(&a.name) {
                        which(val.command.as_str()).is_ok()
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        } else {
            false
        };

        if wst
            .query_status_with_explicit_untrust(TrustType::Select {
                language_servers_to_load,
            })
            .is_none()
        {
            if event.editor.config().workspace_trust.selector {
                job::dispatch_blocking(|_editor, compositor| prompt(compositor));
            } else {
                event.editor.set_status(
                "Current workspace is not trusted. Run `:workspace-trust` to enable all features.",
            );
            }
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
                let trust = &editor.workspace_trust;
                match option {
                    TrustUntrustStatus::DenyAlways => {
                        trust.exclude_workspace();
                    }
                    TrustUntrustStatus::DenyOnce => {
                        trust.cache_non_trust_in_current_workspace();
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
