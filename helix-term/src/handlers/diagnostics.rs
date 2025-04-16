use std::time::Duration;

use helix_core::diagnostic::DiagnosticProvider;
use helix_core::syntax::config::LanguageServerFeature;
use helix_core::Uri;
use helix_event::{register_hook, send_blocking};
use helix_lsp::lsp;
use helix_view::document::Mode;
use helix_view::events::{
    DiagnosticsDidChange, DocumentDidChange, DocumentDidOpen, LanguageServerInitialized,
};
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
        let doc = doc!(event.editor, &event.doc);
        for language_server in
            doc.language_servers_with_feature(LanguageServerFeature::PullDiagnostics)
        {
            pull_diagnostics_for_document(doc, language_server);
        }

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        let language_server = event.editor.language_server_by_id(event.server_id).unwrap();
        if language_server.supports_feature(LanguageServerFeature::PullDiagnostics) {
            for doc in event
                .editor
                .documents()
                .filter(|doc| doc.supports_language_server(event.server_id))
            {
                pull_diagnostics_for_document(doc, language_server);
            }
        }

        Ok(())
    });
}

#[derive(Debug)]
pub(super) struct PullDiagnosticsHandler {
    no_inter_file_dependency_timeout: Option<tokio::time::Instant>,
}

impl PullDiagnosticsHandler {
    pub fn new() -> PullDiagnosticsHandler {
        PullDiagnosticsHandler {
            no_inter_file_dependency_timeout: None,
        }
    }
}

const TIMEOUT: Duration = Duration::from_millis(500);
const TIMEOUT_NO_INTER_FILE_DEPENDENCY: Duration = Duration::from_millis(125);

impl helix_event::AsyncHook for PullDiagnosticsHandler {
    type Event = PullDiagnosticsEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        if timeout.is_none() {
            dispatch_pull_diagnostic_for_document(event.document_id, false);
            self.no_inter_file_dependency_timeout = Some(Instant::now());
        }

        if self
            .no_inter_file_dependency_timeout
            .is_some_and(|nifd_timeout| {
                nifd_timeout.duration_since(Instant::now()) > TIMEOUT_NO_INTER_FILE_DEPENDENCY
            })
        {
            dispatch_pull_diagnostic_for_document(event.document_id, true);
            self.no_inter_file_dependency_timeout = Some(Instant::now());
        };

        Some(Instant::now() + TIMEOUT)
    }

    fn finish_debounce(&mut self) {
        dispatch_pull_diagnostic_for_open_documents();
    }
}

fn dispatch_pull_diagnostic_for_document(
    document_id: DocumentId,
    exclude_language_servers_without_inter_file_dependency: bool,
) {
    job::dispatch_blocking(move |editor, _| {
        let Some(doc) = editor.document(document_id) else {
            return;
        };

        let language_servers = doc
            .language_servers_with_feature(LanguageServerFeature::PullDiagnostics)
            .filter(|ls| ls.is_initialized())
            .filter(|ls| {
                if !exclude_language_servers_without_inter_file_dependency {
                    return true;
                };
                ls.capabilities()
                    .diagnostic_provider
                    .as_ref()
                    .is_some_and(|dp| match dp {
                        lsp::DiagnosticServerCapabilities::Options(options) => {
                            options.inter_file_dependencies
                        }
                        lsp::DiagnosticServerCapabilities::RegistrationOptions(options) => {
                            options.diagnostic_options.inter_file_dependencies
                        }
                    })
            });

        for language_server in language_servers {
            pull_diagnostics_for_document(doc, language_server);
        }
    })
}

fn dispatch_pull_diagnostic_for_open_documents() {
    job::dispatch_blocking(move |editor, _| {
        let documents = editor.documents.values();

        for document in documents {
            let language_servers = document
                .language_servers_with_feature(LanguageServerFeature::PullDiagnostics)
                .filter(|ls| ls.is_initialized());

            for language_server in language_servers {
                pull_diagnostics_for_document(document, language_server);
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

    let language_server_id = language_server.id();
    let provider = DiagnosticProvider::Lsp {
        server_id: language_server_id,
        identifier,
    };
    let document_id = doc.id();

    tokio::spawn(async move {
        match future.await {
            Ok(result) => {
                job::dispatch(move |editor, _| {
                    handle_pull_diagnostics_response(editor, result, provider, uri, document_id)
                })
                .await
            }
            Err(err) => {
                let parsed_cancellation_data = if let helix_lsp::Error::Rpc(error) = err {
                    error.data.and_then(|data| {
                        serde_json::from_value::<lsp::DiagnosticServerCancellationData>(data).ok()
                    })
                } else {
                    log::error!("Pull diagnostic request failed: {err}");
                    return;
                };

                if let Some(parsed_cancellation_data) = parsed_cancellation_data {
                    if parsed_cancellation_data.retrigger_request {
                        tokio::time::sleep(Duration::from_millis(500)).await;

                        job::dispatch(move |editor, _| {
                            if let (Some(doc), Some(language_server)) = (
                                editor.document(document_id),
                                editor.language_server_by_id(language_server_id),
                            ) {
                                pull_diagnostics_for_document(doc, language_server);
                            }
                        })
                        .await;
                    }
                }
            }
        }
    });
}

fn handle_pull_diagnostics_response(
    editor: &mut Editor,
    result: lsp::DocumentDiagnosticReportResult,
    provider: DiagnosticProvider,
    uri: Uri,
    document_id: DocumentId,
) {
    let related_documents = match result {
        lsp::DocumentDiagnosticReportResult::Report(report) => {
            let (result_id, related_documents) = match report {
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

            related_documents
        }
        lsp::DocumentDiagnosticReportResult::Partial(report) => report.related_documents,
    };

    for (url, report) in related_documents.into_iter().flatten() {
        let result_id = match report {
            lsp::DocumentDiagnosticReportKind::Full(report) => {
                let Ok(uri) = Uri::try_from(&url) else {
                    continue;
                };

                editor.handle_lsp_diagnostics(&provider, uri, None, report.items);
                report.result_id
            }
            lsp::DocumentDiagnosticReportKind::Unchanged(report) => Some(report.result_id),
        };

        if let Some(doc) = editor.document_by_path_mut(url.path()) {
            doc.previous_diagnostic_id = result_id;
        }
    }
}
