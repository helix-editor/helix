use std::time::Duration;

use helix_core::diagnostic::DiagnosticProvider;
use helix_core::syntax::LanguageServerFeature;
use helix_core::Uri;
use helix_event::{register_hook, send_blocking};
use helix_lsp::lsp;
use helix_view::document::Mode;
use helix_view::events::{DiagnosticsDidChange, DocumentDidChange, DocumentDidOpen};
use helix_view::handlers::diagnostics::DiagnosticEvent;
use helix_view::handlers::lsp::PullDiagnosticsEvent;
use helix_view::handlers::Handlers;
use helix_view::{DocumentId, Editor};
use tokio::time::Instant;

use crate::events::OnModeSwitch;
use crate::job;

pub(super) fn register_hooks(handlers: &Handlers) {
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

    let tx = handlers.pull_diagnostics.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event
            .doc
            .has_language_server_with_feature(LanguageServerFeature::PullDiagnostics)
        {
            let document_id = event.doc.id();
            send_blocking(&tx, PullDiagnosticsEvent { document_id });
        }
        Ok(())
    });

    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        for language_server in event
            .doc
            .language_servers_with_feature(LanguageServerFeature::PullDiagnostics)
        {
            pull_diagnostics_for_document(event.doc, language_server);
        }

        Ok(())
    });
}

#[derive(Debug)]
pub(super) struct PullDiagnosticsHandler {}

impl PullDiagnosticsHandler {
    pub fn new() -> PullDiagnosticsHandler {
        PullDiagnosticsHandler {}
    }
}

impl helix_event::AsyncHook for PullDiagnosticsHandler {
    type Event = PullDiagnosticsEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        existing_debounce: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        if existing_debounce.is_none() {
            dispatch_pull_diagnostic_for_document(event.document_id);
        }

        Some(Instant::now() + Duration::from_millis(500))
    }

    fn finish_debounce(&mut self) {
        dispatch_pull_diagnostic_for_open_documents();
    }
}

fn dispatch_pull_diagnostic_for_document(document_id: DocumentId) {
    job::dispatch_blocking(move |editor, _| {
        let Some(doc) = editor.document(document_id) else {
            return;
        };

        let language_servers =
            doc.language_servers_with_feature(LanguageServerFeature::PullDiagnostics);

        for language_server in language_servers {
            pull_diagnostics_for_document(doc, language_server);
        }
    })
}

fn dispatch_pull_diagnostic_for_open_documents() {
    job::dispatch_blocking(move |editor, _| {
        let document_ids = editor.tree.views().map(|(view, _)| view.doc);

        let language_servers = editor
            .language_servers
            .iter_clients()
            .filter(|ls| ls.supports_feature(LanguageServerFeature::PullDiagnostics))
            .collect::<Vec<_>>();

        for document_id in document_ids {
            let Some(doc) = editor.document(document_id) else {
                continue;
            };
            for language_server in language_servers.iter() {
                pull_diagnostics_for_document(doc, language_server);
            }
        }
    })
}

pub fn pull_diagnostics_for_document(
    doc: &helix_view::Document,
    language_server: &helix_lsp::Client,
) {
    let Some(future) = language_server
        .text_document_diagnostic(doc.identifier(), doc.previous_diagnostic_id.clone())
    else {
        return;
    };

    let Some(uri) = doc.uri() else {
        return;
    };

    let identifier = language_server
        .capabilities()
        .diagnostic_provider
        .as_ref()
        .and_then(|diagnostic_provider| match diagnostic_provider {
            lsp::DiagnosticServerCapabilities::Options(options) => options.identifier.clone(),
            lsp::DiagnosticServerCapabilities::RegistrationOptions(options) => {
                options.diagnostic_options.identifier.clone()
            }
        });

    let provider = DiagnosticProvider::from_server_and_identifier(language_server.id(), identifier);
    let document_id = doc.id();

    tokio::spawn(async move {
        match future.await {
            Ok(res) => {
                job::dispatch(move |editor, _| {
                    let response = match serde_json::from_value(res) {
                        Ok(result) => result,
                        Err(_) => return,
                    };

                    handle_pull_diagnostics_response(editor, response, provider, uri, document_id)
                })
                .await
            }
            Err(err) => log::error!("Pull diagnostic request failed: {err}"),
        }
    });
}

fn handle_pull_diagnostics_response(
    editor: &mut Editor,
    response: lsp::DocumentDiagnosticReport,
    provider: DiagnosticProvider,
    uri: Uri,
    document_id: DocumentId,
) {
    let (result_id, related_documents) = match response {
        lsp::DocumentDiagnosticReport::Full(report) => {
            editor.handle_lsp_diagnostics(
                &provider,
                uri,
                None,
                report.full_document_diagnostic_report.items,
            );

            (
                report.full_document_diagnostic_report.result_id,
                report.related_documents,
            )
        }
        lsp::DocumentDiagnosticReport::Unchanged(report) => (
            Some(report.unchanged_document_diagnostic_report.result_id),
            report.related_documents,
        ),
    };

    if let Some(doc) = editor.document_mut(document_id) {
        doc.previous_diagnostic_id = result_id;
    };

    for (url, report) in related_documents.into_iter().flatten() {
        let result_id = match report {
            lsp::DocumentDiagnosticReportKind::Full(report) => {
                let Ok(uri) = Uri::try_from(url) else {
                    continue;
                };

                editor.handle_lsp_diagnostics(&provider, uri, None, report.items);
                report.result_id
            }
            lsp::DocumentDiagnosticReportKind::Unchanged(report) => Some(report.result_id),
        };

        if let Some(doc) = editor.document_mut(document_id) {
            doc.previous_diagnostic_id = result_id;
        }
    }
}
