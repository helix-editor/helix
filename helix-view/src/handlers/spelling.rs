//! Spell checking as a non-LSP diagnostic source.
//!
//! This is the editor-side state for the spell checker. The detection logic (the debounced hook,
//! dictionary loading and the word checking itself) lives in `helix-term`'s spelling handler, which
//! drives this state through [`SpellingEvent`]s and the editor's dictionaries.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use helix_core::{
    diagnostic::DiagnosticProvider, ChangeSet, Rope, SpellingLanguage, Tendril, Transaction,
};
use helix_event::{send_blocking, TaskController, TaskHandle};
use tokio::sync::mpsc::Sender;

use crate::{action::Action, DocumentId, Editor};

#[derive(Debug)]
pub struct SpellingHandler {
    pub event_tx: Sender<SpellingEvent>,
    /// In-flight full-document checks, keyed by document. Starting a new full check for a document
    /// cancels the previous one (incremental checks run synchronously and need no cancellation).
    pub requests: HashMap<DocumentId, TaskController>,
    /// Languages whose dictionary is currently being loaded, so the same one isn't loaded twice
    /// concurrently.
    pub loading_dictionaries: HashSet<SpellingLanguage>,
}

impl SpellingHandler {
    pub fn new(event_tx: Sender<SpellingEvent>) -> Self {
        Self {
            event_tx,
            requests: HashMap::new(),
            loading_dictionaries: HashSet::new(),
        }
    }

    /// Registers a new in-flight full check for `document`, cancelling any previous one, and
    /// returns a handle the background task uses to observe cancellation.
    pub fn open_request(&mut self, document: DocumentId) -> TaskHandle {
        let mut controller = TaskController::new();
        let handle = controller.restart();
        self.requests.insert(document, controller);
        handle
    }
}

#[derive(Debug)]
pub enum SpellingEvent {
    /// A dictionary finished loading; (re-)check the open documents that use it.
    DictionaryLoaded { language: SpellingLanguage },
    /// A document was opened; check it in full.
    DocumentOpened { doc: DocumentId },
    /// A document changed; re-check the regions around the change (or rescan, see the term-side
    /// handler). Carries the snapshot needed to recompute the affected regions off the main loop.
    DocumentChanged {
        doc: DocumentId,
        old_text: Rope,
        text: Rope,
        changes: ChangeSet,
        version: i32,
    },
}

/// Spelling actions sort after LSP code actions (which use a higher priority).
const SPELLING_ACTION_PRIORITY: u8 = 0;

/// Appends a word to the `language`'s personal dictionary file, creating it (and its parent
/// directory) if needed. The file is read back when that language's dictionary is loaded.
fn persist_to_personal_dictionary(language: SpellingLanguage, word: &str) -> std::io::Result<()> {
    use std::io::Write as _;

    let path = helix_loader::personal_dictionary_file(language.as_str());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{word}")
}

impl Editor {
    /// Code actions for the spelling diagnostics overlapping the primary selection: a replacement
    /// for each of the dictionary's suggestions, plus an "add to dictionary" action.
    pub fn spelling_actions(&self) -> Vec<Action> {
        let (view, doc) = current_ref!(self);
        // The dictionaries this document is checked against, in configuration order.
        let dictionaries: Vec<(SpellingLanguage, _)> = doc
            .spelling_languages
            .iter()
            .filter_map(|language| {
                Some((language.clone(), self.dictionaries.get(language)?.read()))
            })
            .collect();
        if dictionaries.is_empty() {
            return Vec::new();
        }
        let doc_id = doc.id();
        let view_id = view.id;
        let selection = doc.selection(view_id).primary();
        let text = doc.text();

        let mut suggestions = Vec::new();
        let mut actions = Vec::new();
        for diagnostic in doc.diagnostics() {
            if diagnostic.provider != DiagnosticProvider::Spelling {
                continue;
            }
            let range = diagnostic.range;
            if !selection.overlaps(&helix_core::Range::new(range.start, range.end)) {
                continue;
            }
            let word = Cow::<str>::from(text.slice(range.start..range.end)).into_owned();

            // Offer the suggestions from every dictionary, in order, without duplicates.
            suggestions.clear();
            for (_, dictionary) in &dictionaries {
                dictionary.suggest(&word, &mut suggestions);
            }
            suggestions.dedup();
            for suggestion in &suggestions {
                let suggestion = suggestion.clone();
                let title = format!("Replace '{word}' with '{suggestion}'");
                actions.push(Action::new(
                    title,
                    SPELLING_ACTION_PRIORITY,
                    move |editor| {
                        let doc = doc_mut!(editor, &doc_id);
                        let view = view_mut!(editor, view_id);
                        let transaction = Transaction::change(
                            doc.text(),
                            std::iter::once((
                                range.start,
                                range.end,
                                Some(Tendril::from(&*suggestion)),
                            )),
                        );
                        doc.apply(&transaction, view_id);
                        doc.append_changes_to_history(view);
                    },
                ));
            }

            // "Add to dictionary" targets one dictionary, so offer one action per language.
            for (language, _) in &dictionaries {
                let language = language.clone();
                let word = word.clone();
                let title = format!("Add '{word}' to dictionary '{language}'");
                actions.push(Action::new(
                    title,
                    SPELLING_ACTION_PRIORITY,
                    move |editor| {
                        let Some(dictionary) = editor.dictionaries.get(&language) else {
                            return;
                        };
                        if let Err(err) = dictionary.write().add(&word) {
                            log::error!(
                                "could not add '{word}' to dictionary '{language}': {err:?}"
                            );
                            return;
                        }
                        if let Err(err) = persist_to_personal_dictionary(language.clone(), &word) {
                            log::error!(
                                "could not persist '{word}' to the personal dictionary: {err}"
                            );
                        }
                        // The dictionary's contents changed; re-check the open documents using it.
                        send_blocking(
                            &editor.handlers.spelling.event_tx,
                            SpellingEvent::DictionaryLoaded {
                                language: language.clone(),
                            },
                        );
                    },
                ));
            }
        }

        actions
    }
}
