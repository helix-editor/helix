use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use futures_util::{future::BoxFuture, FutureExt as _};
use helix_core::{SpellingLanguage, Tendril, Transaction};
use helix_event::{TaskController, TaskHandle};
use tokio::sync::mpsc::Sender;

use crate::{diagnostic::DiagnosticProvider, Action, DocumentId, Editor};

const ACTION_PRIORITY: u8 = 0;

#[derive(Debug)]
pub struct SpellingHandler {
    pub event_tx: Sender<SpellingEvent>,
    pub requests: HashMap<DocumentId, TaskController>,
    pub loading_dictionaries: HashSet<SpellingLanguage>,
}

impl SpellingHandler {
    pub fn new(event_tx: Sender<SpellingEvent>) -> Self {
        Self {
            event_tx,
            requests: Default::default(),
            loading_dictionaries: Default::default(),
        }
    }

    pub fn open_request(&mut self, document: DocumentId) -> TaskHandle {
        let mut controller = TaskController::new();
        let handle = controller.restart();
        self.requests.insert(document, controller);
        handle
    }
}

#[derive(Debug)]
pub enum SpellingEvent {
    /*
    DictionaryUpdated {
        word: String,
        language: SpellingLanguage,
    },
    */
    DictionaryLoaded { language: SpellingLanguage },
    DocumentOpened { doc: DocumentId },
    DocumentChanged { doc: DocumentId },
}

impl Editor {
    pub(crate) fn spelling_actions(
        &self,
    ) -> Option<BoxFuture<'static, anyhow::Result<Vec<Action>>>> {
        let (view, doc) = current_ref!(self);
        let doc_id = doc.id();
        let view_id = view.id;
        let language = doc.spelling_language()?;
        // TODO: consider fixes for all selections?
        let range = doc.selection(view_id).primary();
        let text = doc.text().clone();
        let dictionary = self.dictionaries.get(&language)?.clone();
        // TODO: can do this faster with partition_point + take_while
        let selected_diagnostics: Vec<_> = doc
            .diagnostics()
            .iter()
            .filter(|d| {
                range.overlaps(&helix_core::Range::new(d.range.start, d.range.end))
                    && d.inner.provider == DiagnosticProvider::Spelling
            })
            .map(|d| d.range)
            .collect();

        let future = tokio::task::spawn_blocking(move || {
            let text = text.slice(..);
            let dictionary = dictionary.read();
            let mut suggest_buffer = Vec::new();
            selected_diagnostics
            .into_iter()
            .flat_map(|range| {
                suggest_buffer.clear();
                let word = Cow::from(text.slice(range.start..range.end));
                dictionary.suggest(&word, &mut suggest_buffer);

                let mut actions = Vec::with_capacity(suggest_buffer.len() + 1);
                actions.extend(
                    suggest_buffer.drain(..).map(|suggestion| {
                        Action::new(
                            format!("Replace '{word}' with '{suggestion}'"),
                            ACTION_PRIORITY,
                            move |editor| {
                                let doc = doc_mut!(editor, &doc_id);
                                let view = view_mut!(editor, view_id);
                                let transaction = Transaction::change(
                                    doc.text(),
                                    [(range.start, range.end, Some(Tendril::from(suggestion.as_str())))].into_iter(),
                                );
                                doc.apply(&transaction, view_id);
                                doc.append_changes_to_history(view);
                                // TODO: get rid of the diagnostic for this word.
                            },
                        )
                    })
                );
                let word = word.to_string();
                actions.push(Action::new(
                    format!("Add '{word}' to dictionary '{language}'"),
                    ACTION_PRIORITY,
                    move |editor| {
                        let Some(dictionary) = editor.dictionaries.get(&language) else {
                            log::error!("Failed to add '{word}' to dictionary '{language}' because the dictionary does not exist");
                            return;
                        };
                        // TODO: fire an event?
                        let mut dictionary = dictionary.write();
                        if let Err(err) = dictionary.add(&word) {
                            log::error!("Failed to add '{word}' to dictionary '{language}': {err}");
                        }
                    }
                ));
                actions
            })
            .collect()
        });
        Some(async move { Ok(future.await?) }.boxed())
    }
}
