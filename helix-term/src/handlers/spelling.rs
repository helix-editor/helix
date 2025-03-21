use std::{borrow::Cow, collections::HashSet, future::Future, sync::Arc, time::Duration};

use anyhow::Result;
use helix_core::{Rope, SpellingLanguage};
use helix_event::{cancelable_future, register_hook, send_blocking};
use helix_stdx::rope::{Regex, RopeSliceExt as _};
use helix_view::{
    diagnostic::DiagnosticProvider,
    editor::Severity,
    events::{DocumentDidChange, DocumentDidOpen},
    handlers::{spelling::SpellingEvent, Handlers},
    Diagnostic, Dictionary, DocumentId, Editor,
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use tokio::time::Instant;

use crate::job;

const PROVIDER: DiagnosticProvider = DiagnosticProvider::Spelling;

#[derive(Debug, Default)]
pub(super) struct SpellingHandler {
    changed_docs: HashSet<DocumentId>,
}

impl helix_event::AsyncHook for SpellingHandler {
    type Event = SpellingEvent;

    fn handle_event(&mut self, event: Self::Event, timeout: Option<Instant>) -> Option<Instant> {
        match event {
            SpellingEvent::DictionaryLoaded { language } => {
                job::dispatch_blocking(move |editor, _compositor| {
                    let docs: Vec<_> = editor
                        .documents
                        .iter()
                        .filter_map(|(&doc_id, doc)| {
                            (doc.spelling_language() == Some(language)).then_some(doc_id)
                        })
                        .collect();
                    for doc in docs {
                        check_document(editor, doc);
                    }
                });
                timeout
            }
            SpellingEvent::DocumentOpened { doc } => {
                job::dispatch_blocking(move |editor, _compositor| {
                    check_document(editor, doc);
                });
                timeout
            }
            SpellingEvent::DocumentChanged { doc } => {
                self.changed_docs.insert(doc);
                Some(Instant::now() + Duration::from_secs(3))
            }
        }
    }

    fn finish_debounce(&mut self) {
        let docs = std::mem::take(&mut self.changed_docs);
        job::dispatch_blocking(move |editor, _compositor| {
            for doc in docs {
                check_document(editor, doc);
            }
        });
    }
}

fn check_document(editor: &mut Editor, doc_id: DocumentId) {
    let Some(doc) = editor.documents.get(&doc_id) else {
        return;
    };
    let Some(language) = doc.spelling_language() else {
        return;
    };
    let Some(dictionary) = editor.dictionaries.get(&language).cloned() else {
        if editor
            .handlers
            .spelling
            .loading_dictionaries
            .insert(language)
        {
            load_dictionary(language);
        }
        return;
    };

    let uri = doc.uri();
    let future = check_text(dictionary, doc.text().clone());
    let cancel = editor.handlers.spelling.open_request(doc_id);

    tokio::spawn(async move {
        match cancelable_future(future, cancel).await {
            Some(Ok(diagnostics)) => {
                job::dispatch_blocking(move |editor, _compositor| {
                    editor.handlers.spelling.requests.remove(&doc_id);
                    editor.handle_diagnostics(&PROVIDER, uri, None, diagnostics);
                });
            }
            Some(Err(err)) => log::error!("spelling background job failed: {err}"),
            None => (),
        }
    });
}

fn load_dictionary(language: SpellingLanguage) {
    tokio::task::spawn_blocking(move || {
        let aff = std::fs::read_to_string(helix_loader::runtime_file(format!(
            "dictionaries/{language}/{language}.aff"
        )))
        .unwrap();
        let dic = std::fs::read_to_string(helix_loader::runtime_file(format!(
            "dictionaries/{language}/{language}.dic"
        )))
        .unwrap();

        let mut dictionary = Dictionary::new(&aff, &dic).unwrap();
        // TODO: personal dictionaries should be namespaced under runtime directories under the
        // language.
        if let Ok(file) = std::fs::File::open(helix_loader::personal_dictionary_file()) {
            use std::io::{BufRead as _, BufReader};
            let reader = BufReader::with_capacity(8 * 1024, file);
            for line in reader.lines() {
                let line = line.unwrap();
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                dictionary.add(line).unwrap();
            }
        }

        job::dispatch_blocking(move |editor, _compositor| {
            let was_removed = editor
                .handlers
                .spelling
                .loading_dictionaries
                .remove(&language);
            // Other processes should respect that a dictionary is loading and not change
            // `loading_dictionaries`. So this should always be true.
            debug_assert!(was_removed);
            editor
                .dictionaries
                .insert(language, Arc::new(RwLock::new(dictionary)));
            send_blocking(
                &editor.handlers.spelling.event_tx,
                SpellingEvent::DictionaryLoaded { language },
            );
        })
    });
}

fn check_text(
    dictionary: Arc<RwLock<Dictionary>>,
    text: Rope,
) -> impl Future<Output = Result<Vec<Diagnostic>, tokio::task::JoinError>> {
    tokio::task::spawn_blocking(move || {
        static WORDS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[0-9A-Z]*(['-]?[a-z]+)*"#).unwrap());

        let dict = dictionary.read();
        let text = text.slice(..);
        let mut diagnostics = Vec::new();
        for match_ in WORDS.find_iter(text.regex_input()) {
            let word = Cow::from(text.byte_slice(match_.range()));
            if !dict.check(&word) {
                diagnostics.push(Diagnostic {
                    range: helix_view::Range::Document(helix_stdx::Range {
                        start: text.byte_to_char(match_.start()),
                        end: text.byte_to_char(match_.end()),
                    }),
                    message: format!("Possible spelling issue '{word}'"),
                    severity: Some(Severity::Error),
                    code: None,
                    provider: PROVIDER,
                    tags: Default::default(),
                    source: None,
                    data: None,
                });
            }
        }
        diagnostics
    })
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.spelling.event_tx.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        let doc = doc!(event.editor, &event.doc);
        if doc.spelling_language().is_some() {
            send_blocking(&tx, SpellingEvent::DocumentOpened { doc: event.doc });
        }
        Ok(())
    });

    let tx = handlers.spelling.event_tx.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if event.doc.spelling_language().is_some() {
            send_blocking(
                &tx,
                SpellingEvent::DocumentChanged {
                    doc: event.doc.id(),
                },
            );
        }
        Ok(())
    });
}
