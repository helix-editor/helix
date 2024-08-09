use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use helix_core::syntax::LanguageServerFeature;
use helix_event::{register_hook, send_blocking};
use helix_lsp::lsp::{self, Diagnostic};
use helix_lsp::LanguageServerId;
use helix_view::document::Mode;
use helix_view::events::{DiagnosticsDidChange, DocumentDidChange};
use helix_view::handlers::diagnostics::DiagnosticEvent;
use helix_view::handlers::lsp::PullDiagnosticsEvent;
use helix_view::handlers::Handlers;
use helix_view::Editor;
use tokio::time::Instant;

use crate::events::OnModeSwitch;
use crate::job;

const TIMEOUT: u64 = 120;

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
        _: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        match event {
            PullDiagnosticsEvent::Trigger => {}
        }
        Some(Instant::now() + Duration::from_millis(TIMEOUT))
    }

    fn finish_debounce(&mut self) {
        job::dispatch_blocking(move |editor, _| pull_diagnostic_for_current_doc(editor))
    }
}

fn pull_diagnostic_for_current_doc(editor: &mut Editor) {
    let (_, doc) = current!(editor);

    for language_server in doc.language_servers_with_feature(LanguageServerFeature::PullDiagnostics)
    {
        let future = language_server
            .text_document_diagnostic(doc.identifier(), doc.previous_diagnostic_id.clone());

        let original_path = doc
            .path()
            .expect("safety: the file has a path if there is a running language server")
            .to_owned();

        let Some(future) = future else {
            return;
        };

        let server_id = language_server.id();

        tokio::spawn(async move {
            match future.await {
                Ok(res) => {
                    job::dispatch(move |editor, _| {
                        let parsed_response: Option<lsp::DocumentDiagnosticReport> =
                            match serde_json::from_value(res) {
                                Ok(result) => Some(result),
                                Err(_) => None,
                            };

                        show_pull_diagnostics(editor, parsed_response, server_id, original_path)
                    })
                    .await
                }
                Err(err) => log::error!("signature help request failed: {err}"),
            }
        });
    }
}

fn show_pull_diagnostics(
    editor: &mut Editor,
    response: Option<lsp::DocumentDiagnosticReport>,
    server_id: LanguageServerId,
    original_path: PathBuf,
) {
    let parse_diagnostic = |editor: &mut Editor,
                            path: PathBuf,
                            report: Vec<lsp::Diagnostic>,
                            result_id: Option<String>| {
        let uri = helix_core::Uri::try_from(path);
        let diagnostics: Vec<(Diagnostic, LanguageServerId)> =
            report.into_iter().map(|d| (d, server_id)).collect();

        if let Ok(uri) = uri {
            editor.add_diagnostics(diagnostics, server_id, uri, None, result_id);
        }
    };

    let handle_document_diagnostic_report_kind =
        |editor: &mut Editor,
         report: Option<HashMap<lsp::Url, lsp::DocumentDiagnosticReportKind>>| {
            for (url, report) in report.into_iter().flatten() {
                match report {
                    lsp::DocumentDiagnosticReportKind::Full(report) => {
                        let path = url.to_file_path().unwrap();
                        parse_diagnostic(editor, path, report.items, report.result_id);
                    }
                    lsp::DocumentDiagnosticReportKind::Unchanged(report) => {
                        let Some(doc) = editor.document_by_path_mut(url.path()) else {
                            return;
                        };
                        doc.previous_diagnostic_id = Some(report.result_id);
                    }
                }
            }
        };

    if let Some(response) = response {
        let doc = match editor.document_by_path_mut(&original_path) {
            Some(doc) => doc,
            None => return,
        };
        match response {
            lsp::DocumentDiagnosticReport::Full(report) => {
                // Original file diagnostic
                parse_diagnostic(
                    editor,
                    original_path,
                    report.full_document_diagnostic_report.items,
                    report.full_document_diagnostic_report.result_id,
                );

                // Related files diagnostic
                handle_document_diagnostic_report_kind(editor, report.related_documents);
            }
            lsp::DocumentDiagnosticReport::Unchanged(report) => {
                doc.previous_diagnostic_id =
                    Some(report.unchanged_document_diagnostic_report.result_id);
                handle_document_diagnostic_report_kind(editor, report.related_documents);
            }
        }
    }
}

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
        if event.doc.config.load().lsp.auto_signature_help {
            send_blocking(&tx, PullDiagnosticsEvent::Trigger);
        }
        Ok(())
    });
}
