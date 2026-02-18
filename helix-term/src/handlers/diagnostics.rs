use futures_util::stream::FuturesUnordered;
use std::collections::HashSet;
use std::mem;
use std::time::Duration;
use tokio::time::Instant;
use tokio_stream::StreamExt;

use helix_core::diagnostic::DiagnosticProvider;
use helix_core::syntax::config::LanguageServerFeature;
use helix_core::Uri;
use helix_event::{cancelable_future, register_hook, send_blocking};
use helix_lsp::{lsp, LanguageServerId};
use helix_view::document::Mode;
use helix_view::events::{
    DiagnosticsDidChange, DocumentDidChange, DocumentDidOpen, LanguageServerInitialized,
};
use helix_view::handlers::diagnostics::DiagnosticEvent;
use helix_view::handlers::lsp::{PullAllDocumentsDiagnosticsEvent, PullDiagnosticsEvent};
use helix_view::handlers::Handlers;
use helix_view::{DocumentId, Editor};

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
    let tx_all_documents = handlers.pull_all_documents_diagnostics.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event
            .doc
            .has_language_server_with_feature(LanguageServerFeature::PullDiagnostics)
            && !event.ghost_transaction
        {
            // Cancel the ongoing request, if present.
            event.doc.pull_diagnostic_controller.cancel();
            let document_id = event.doc.id();
            send_blocking(&tx, PullDiagnosticsEvent { document_id });

            let inter_file_dependencies_language_servers = event
                .doc
                .language_servers_with_feature(LanguageServerFeature::PullDiagnostics)
                .filter(|language_server| {
                    language_server
                        .capabilities()
                        .diagnostic_provider
                        .as_ref()
                        .is_some_and(|diagnostic_provider| match diagnostic_provider {
                            lsp::DiagnosticServerCapabilities::Options(options) => {
                                options.inter_file_dependencies
                            }

                            lsp::DiagnosticServerCapabilities::RegistrationOptions(options) => {
                                options.diagnostic_options.inter_file_dependencies
                            }
                        })
                })
                .map(|language_server| language_server.id())
                .collect();

            send_blocking(
                &tx_all_documents,
                PullAllDocumentsDiagnosticsEvent {
                    language_servers: inter_file_dependencies_language_servers,
                },
            );
        }
        Ok(())
    });

    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        request_document_diagnostics(event.editor, event.doc);

        Ok(())
    });

    register_hook!(move |event: &mut LanguageServerInitialized<'_>| {
        let doc_ids: Vec<_> = event.editor.documents.keys().copied().collect();

        for doc_id in doc_ids {
            request_document_diagnostics(event.editor, doc_id);
        }

        Ok(())
    });
}

#[derive(Debug, Default)]
pub(super) struct PullDiagnosticsHandler {
    document_ids: HashSet<DocumentId>,
}

impl helix_event::AsyncHook for PullDiagnosticsHandler {
    type Event = PullDiagnosticsEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        self.document_ids.insert(event.document_id);
        Some(Instant::now() + Duration::from_millis(250))
    }

    fn finish_debounce(&mut self) {
        let document_ids = mem::take(&mut self.document_ids);
        job::dispatch_blocking(move |editor, _| {
            for document_id in document_ids {
                request_document_diagnostics(editor, document_id);
            }
        })
    }
}

#[derive(Debug, Default)]
pub(super) struct PullAllDocumentsDiagnosticHandler {
    language_servers: HashSet<LanguageServerId>,
}

impl helix_event::AsyncHook for PullAllDocumentsDiagnosticHandler {
    type Event = PullAllDocumentsDiagnosticsEvent;

    fn handle_event(
        &mut self,
        event: Self::Event,
        _timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        self.language_servers.extend(&event.language_servers);
        Some(Instant::now() + Duration::from_secs(1))
    }

    fn finish_debounce(&mut self) {
        let language_servers = mem::take(&mut self.language_servers);
        job::dispatch_blocking(move |editor, _| {
            let documents: Vec<_> = editor.documents.keys().copied().collect();

            for document in documents {
                request_document_diagnostics_for_language_severs(
                    editor,
                    document,
                    language_servers.clone(),
                );
            }
        })
    }
}

fn request_document_diagnostics_for_language_severs(
    editor: &mut Editor,
    doc_id: DocumentId,
    language_servers: HashSet<LanguageServerId>,
) {
    let Some(doc) = editor.document_mut(doc_id) else {
        return;
    };

    let cancel = doc.pull_diagnostic_controller.restart();

    let mut futures: FuturesUnordered<_> = language_servers
        .iter()
        .filter_map(|x| doc.language_servers().find(|y| &y.id() == x))
        .filter_map(|language_server| {
            let future = language_server
                .text_document_diagnostic(doc.identifier(), doc.previous_diagnostic_id.clone())?;

            let identifier = language_server
                .capabilities()
                .diagnostic_provider
                .as_ref()
                .and_then(|diagnostic_provider| match diagnostic_provider {
                    lsp::DiagnosticServerCapabilities::Options(options) => {
                        options.identifier.clone()
                    }
                    lsp::DiagnosticServerCapabilities::RegistrationOptions(options) => {
                        options.diagnostic_options.identifier.clone()
                    }
                });

            let language_server_id = language_server.id();
            let provider = DiagnosticProvider::Lsp {
                server_id: language_server_id,
                identifier,
            };
            let uri = doc.uri()?;

            Some(async move {
                let result = future.await;

                (result, provider, uri)
            })
        })
        .collect();

    if futures.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let mut retry_language_servers = HashSet::new();
        loop {
            match cancelable_future(futures.next(), &cancel).await {
                Some(Some((Ok(result), provider, uri))) => {
                    job::dispatch(move |editor, _| {
                        handle_pull_diagnostics_response(editor, result, provider, uri, doc_id);
                    })
                    .await;
                }
                Some(Some((Err(err), DiagnosticProvider::Lsp { server_id, .. }, _))) => {
                    let parsed_cancellation_data = if let helix_lsp::Error::Rpc(error) = err {
                        error.data.and_then(|data| {
                            serde_json::from_value::<lsp::DiagnosticServerCancellationData>(data)
                                .ok()
                        })
                    } else {
                        log::error!("Pull diagnostic request failed: {err}");
                        continue;
                    };
                    if parsed_cancellation_data.is_some_and(|data| data.retrigger_request) {
                        retry_language_servers.insert(server_id);
                    }
                }
                Some(None) => break,
                // The request was cancelled.
                None => return,
            }
        }

        if !retry_language_servers.is_empty() {
            tokio::time::sleep(Duration::from_millis(500)).await;

            job::dispatch(move |editor, _| {
                request_document_diagnostics_for_language_severs(
                    editor,
                    doc_id,
                    retry_language_servers,
                );
            })
            .await;
        }
    });
}

pub fn request_document_diagnostics(editor: &mut Editor, doc_id: DocumentId) {
    let Some(doc) = editor.document(doc_id) else {
        return;
    };

    let language_servers = doc
        .language_servers_with_feature(LanguageServerFeature::PullDiagnostics)
        .map(|language_servers| language_servers.id())
        .collect();

    request_document_diagnostics_for_language_severs(editor, doc_id, language_servers);
}

fn handle_pull_diagnostics_response(
    editor: &mut Editor,
    result: lsp::DocumentDiagnosticReportResult,
    provider: DiagnosticProvider,
    uri: Uri,
    document_id: DocumentId,
) {
    match result {
        lsp::DocumentDiagnosticReportResult::Report(report) => {
            let result_id = match report {
                lsp::DocumentDiagnosticReport::Full(report) => {
                    editor.handle_lsp_diagnostics(
                        &provider,
                        uri,
                        None,
                        report.full_document_diagnostic_report.items,
                    );

                    report.full_document_diagnostic_report.result_id
                }
                lsp::DocumentDiagnosticReport::Unchanged(report) => {
                    Some(report.unchanged_document_diagnostic_report.result_id)
                }
            };

            if let Some(doc) = editor.document_mut(document_id) {
                doc.previous_diagnostic_id = result_id;
            };
        }
        lsp::DocumentDiagnosticReportResult::Partial(_) => {}
    };
}
