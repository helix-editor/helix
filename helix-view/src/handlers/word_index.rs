//! Indexing of words from open buffers.
//!
//! This provides an eventually consistent set of words used in any open buffers. This set is
//! later used for lexical completion.

use std::{borrow::Cow, iter, mem, sync::Arc, time::Duration};

use foldhash::HashMap;
use helix_core::{chars::char_is_word, fuzzy::fuzzy_match, ChangeSet, Rope, RopeSlice};
use helix_event::{register_hook, AsyncHook, TaskController, TaskHandle};
use helix_stdx::rope::RopeSliceExt as _;
use parking_lot::RwLock;
use tokio::{sync::mpsc, time::Instant};

use crate::{
    events::{ConfigDidChange, DocumentDidChange, DocumentDidClose, DocumentDidOpen},
    DocumentId,
};

use super::Handlers;

#[derive(Debug)]
struct Change {
    old_text: Rope,
    text: Rope,
    changes: ChangeSet,
}

#[derive(Debug)]
enum Event {
    Insert(Rope),
    Update(DocumentId, Change),
    Delete(DocumentId, Rope),
    /// Clear the entire word index.
    /// This is used to clear memory when the feature is turned off.
    Clear,
}

#[derive(Debug)]
pub struct Handler {
    pub(super) index: WordIndex,
    /// A sender into an async hook which debounces updates to the index.
    hook: mpsc::Sender<Event>,
    /// A sender to a tokio task which coordinates the indexing of documents.
    ///
    /// See [WordIndex::run]. A supervisor-like task is in charge of spawning tasks to update the
    /// index. This ensures that consecutive edits to a document trigger the correct order of
    /// insertions and deletions into the word set.
    coordinator: mpsc::UnboundedSender<Event>,
    /// Cancels in-flight indexing when the handler is dropped.
    ///
    /// Indexing a large document runs on a blocking task which cannot be preempted. Without this,
    /// dropping the tokio runtime on shutdown would block until that task finishes, keeping the
    /// process alive and unresponsive. The indexing task holds a [TaskHandle] from this
    /// controller and checks it periodically.
    _cancel: TaskController,
}

impl Handler {
    pub fn spawn() -> Self {
        let index = WordIndex::default();
        let (tx, rx) = mpsc::unbounded_channel();
        let mut cancel = TaskController::new();
        tokio::spawn(index.clone().run(rx, cancel.restart()));
        Self {
            hook: Hook {
                changes: HashMap::default(),
                coordinator: tx.clone(),
            }
            .spawn(),
            index,
            coordinator: tx,
            _cancel: cancel,
        }
    }
}

#[derive(Debug)]
struct Hook {
    changes: HashMap<DocumentId, Change>,
    coordinator: mpsc::UnboundedSender<Event>,
}

const DEBOUNCE: Duration = Duration::from_secs(1);

impl AsyncHook for Hook {
    type Event = Event;

    fn handle_event(&mut self, event: Self::Event, timeout: Option<Instant>) -> Option<Instant> {
        match event {
            Event::Insert(_) => unreachable!("inserts are sent to the worker directly"),
            Event::Update(doc, change) => {
                if let Some(pending_change) = self.changes.get_mut(&doc) {
                    // If there is already a change waiting for this document, merge the two
                    // changes together by composing the changesets and saving the new `text`.
                    pending_change.changes =
                        mem::take(&mut pending_change.changes).compose(change.changes);
                    pending_change.text = change.text;
                    Some(Instant::now() + DEBOUNCE)
                } else if !is_changeset_significant(&change.changes) {
                    // If the changeset is fairly large, debounce before updating the index.
                    self.changes.insert(doc, change);
                    Some(Instant::now() + DEBOUNCE)
                } else {
                    // Otherwise if the change is small, queue the update to the index immediately.
                    self.coordinator.send(Event::Update(doc, change)).unwrap();
                    timeout
                }
            }
            Event::Delete(doc, text) => {
                // If there are pending changes that haven't been indexed since the last debounce,
                // forget them and delete the old text.
                if let Some(change) = self.changes.remove(&doc) {
                    self.coordinator
                        .send(Event::Delete(doc, change.old_text))
                        .unwrap();
                } else {
                    self.coordinator.send(Event::Delete(doc, text)).unwrap();
                }
                timeout
            }
            Event::Clear => unreachable!("clear is sent to the worker directly"),
        }
    }

    fn finish_debounce(&mut self) {
        for (doc, change) in self.changes.drain() {
            self.coordinator.send(Event::Update(doc, change)).unwrap();
        }
    }
}

/// Minimum number of grapheme clusters required to include a word in the index
const MIN_WORD_GRAPHEMES: usize = 3;
/// Maximum word length allowed (in chars)
const MAX_WORD_LEN: usize = 50;
/// Number of words to index between checks of the cancellation handle.
const CANCEL_CHECK_INTERVAL: usize = 4096;

type Word = kstring::KString;

#[derive(Debug, Default)]
struct WordIndexInner {
    /// Reference counted storage for words.
    ///
    /// Words are very likely to be reused many times. Instead of storing duplicates we keep a
    /// reference count of times a word is used. When the reference count drops to zero the word
    /// is removed from the index.
    words: HashMap<Word, u32>,
}

impl WordIndexInner {
    fn words(&self) -> impl Iterator<Item = &Word> {
        self.words.keys()
    }

    fn insert(&mut self, word: RopeSlice) {
        let word: Cow<str> = word.into();
        if let Some(rc) = self.words.get_mut(word.as_ref()) {
            *rc = rc.saturating_add(1);
        } else {
            let word = match word {
                Cow::Owned(s) => Word::from_string(s),
                Cow::Borrowed(s) => Word::from_ref(s),
            };
            self.words.insert(word, 1);
        }
    }

    fn remove(&mut self, word: RopeSlice) {
        let word: Cow<str> = word.into();
        match self.words.get_mut(word.as_ref()) {
            Some(1) => {
                self.words.remove(word.as_ref());
            }
            Some(n) => *n -= 1,
            None => (),
        }
    }

    fn clear(&mut self) {
        std::mem::take(&mut self.words);
    }
}

#[derive(Debug, Default, Clone)]
pub struct WordIndex {
    inner: Arc<RwLock<WordIndexInner>>,
}

impl WordIndex {
    pub fn matches(&self, pattern: &str) -> Vec<String> {
        let inner = self.inner.read();
        let mut matches = fuzzy_match(pattern, inner.words(), false);
        matches.sort_unstable_by_key(|(_, score)| *score);
        matches
            .into_iter()
            .map(|(word, _)| word.to_string())
            .collect()
    }

    fn add_document(&self, text: &Rope, cancel: &TaskHandle) {
        let mut inner = self.inner.write();
        for (i, word) in words(text.slice(..)).enumerate() {
            if i % CANCEL_CHECK_INTERVAL == 0 && cancel.is_canceled() {
                return;
            }
            inner.insert(word);
        }
    }

    fn update_document(
        &self,
        old_text: &Rope,
        text: &Rope,
        changes: &ChangeSet,
        cancel: &TaskHandle,
    ) {
        let mut inner = self.inner.write();
        // A single changed window can span the whole document, so the check is driven by a count
        // of the words processed rather than the number of windows.
        let mut since_check = 0;
        for (old_window, new_window) in changed_windows(old_text.slice(..), text.slice(..), changes)
        {
            for word in words(new_window) {
                inner.insert(word);
                since_check += 1;
            }
            for word in words(old_window) {
                inner.remove(word);
                since_check += 1;
            }
            if since_check >= CANCEL_CHECK_INTERVAL {
                if cancel.is_canceled() {
                    return;
                }
                since_check = 0;
            }
        }
    }

    fn remove_document(&self, text: &Rope, cancel: &TaskHandle) {
        let mut inner = self.inner.write();
        for (i, word) in words(text.slice(..)).enumerate() {
            if i % CANCEL_CHECK_INTERVAL == 0 && cancel.is_canceled() {
                return;
            }
            inner.remove(word);
        }
    }

    fn clear(&self) {
        let mut inner = self.inner.write();
        inner.clear();
    }

    /// Coordinate the indexing of documents.
    ///
    /// This task wraps a MPSC queue and spawns blocking tasks which update the index. Updates
    /// are applied one-by-one to ensure that changes to the index are **serialized**:
    /// updates to each document must be applied in-order.
    async fn run(self, mut events: mpsc::UnboundedReceiver<Event>, cancel: TaskHandle) {
        while let Some(event) = events.recv().await {
            if cancel.is_canceled() {
                return;
            }
            let this = self.clone();
            let cancel = cancel.clone();
            tokio::task::spawn_blocking(move || match event {
                Event::Insert(text) => {
                    this.add_document(&text, &cancel);
                }
                Event::Update(
                    _doc,
                    Change {
                        old_text,
                        text,
                        changes,
                        ..
                    },
                ) => {
                    this.update_document(&old_text, &text, &changes, &cancel);
                }
                Event::Delete(_doc, text) => {
                    this.remove_document(&text, &cancel);
                }
                Event::Clear => {
                    this.clear();
                }
            })
            .await
            .unwrap();
        }
    }
}

/// Extracts indexable words from a rope slice.
///
/// A word is a run of grapheme clusters whose first character is a
/// w[word character][char_is_word], spanning at least [`MIN_WORD_GRAPHEMES`] clusters and at
/// most [`MAX_WORD_LEN`] chars. All other text is skipped.
///
/// This is a single forward pass over the text's grapheme clusters: each cluster is visited once.
/// and the only rope position ever sought is the start of an emitted word, which keeps
/// extraction roughly linear in the length of the text.
fn words(text: RopeSlice) -> impl Iterator<Item = RopeSlice> {
    let mut graphemes = text.grapheme_indices();
    // The in-progress word run: the byte offset of its first cluster, its length in chars, and the
    // number of graphemes it spans. `graphemes_len == 0` means we are between words.
    let mut start_byte = 0;
    let mut char_len = 0;
    let mut graphemes_len = 0;

    // Yields `text[start_byte..end_byte]` if that run satisfies the length bounds.
    let qualify = move |start_byte, end_byte, char_len, graphemes_len| {
        (graphemes_len >= MIN_WORD_GRAPHEMES && char_len <= MAX_WORD_LEN)
            .then(|| text.byte_slice(start_byte..end_byte))
    };

    iter::from_fn(move || {
        loop {
            let Some((byte_idx, grapheme)) = graphemes.next() else {
                // Flush a word that runs up to the end of the text.
                let word = qualify(start_byte, text.len_bytes(), char_len, graphemes_len);
                graphemes_len = 0;
                return word;
            };

            if grapheme.chars().next().is_some_and(char_is_word) {
                if graphemes_len == 0 {
                    start_byte = byte_idx;
                    char_len = 0;
                }
                graphemes_len += 1;
                char_len += grapheme.len_chars();
            } else if graphemes_len != 0 {
                // A non-word cluster ends the current run; `byte_idx` is one past the run's end.
                let word = qualify(start_byte, byte_idx, char_len, graphemes_len);
                graphemes_len = 0;
                if word.is_some() {
                    return word;
                }
            }
        }
    })
}

/// Finds areas of the old and new texts around each operation in `changes`.
///
/// The window is larger than the changed area and can encompass multiple insert/delete operations
/// if they are grouped closely together.
///
/// The ranges of the old and new text should usually be of different sizes. For example a
/// deletion of "foo" surrounded by large retain sections would give a longer window into the
/// `old_text` and shorter window of `new_text`. Vice-versa for an insertion. A full replacement
/// of a word though would give two slices of the same size.
fn changed_windows<'a>(
    old_text: RopeSlice<'a>,
    new_text: RopeSlice<'a>,
    changes: &'a ChangeSet,
) -> impl Iterator<Item = (RopeSlice<'a>, RopeSlice<'a>)> {
    use helix_core::Operation::*;

    let mut operations = changes.changes().iter().peekable();
    let mut old_pos = 0;
    let mut new_pos = 0;
    iter::from_fn(move || loop {
        let operation = operations.next()?;
        let old_start = old_pos;
        let new_start = new_pos;
        let len = operation.len_chars();
        match operation {
            Retain(_) => {
                old_pos += len;
                new_pos += len;
                continue;
            }
            Insert(_) => new_pos += len,
            Delete(_) => old_pos += len,
        }

        // Scan ahead until a `Retain` is found which would end a window.
        while let Some(o) = operations.next_if(|op| !matches!(op, Retain(n) if *n > MAX_WORD_LEN)) {
            let len = o.len_chars();
            match o {
                Retain(_) => {
                    old_pos += len;
                    new_pos += len;
                }
                Delete(_) => old_pos += len,
                Insert(_) => new_pos += len,
            }
        }

        let old_window = old_start.saturating_sub(MAX_WORD_LEN)
            ..(old_pos + MAX_WORD_LEN).min(old_text.len_chars());
        let new_window = new_start.saturating_sub(MAX_WORD_LEN)
            ..(new_pos + MAX_WORD_LEN).min(new_text.len_chars());

        return Some((old_text.slice(old_window), new_text.slice(new_window)));
    })
}

/// Estimates whether a changeset is significant or small.
fn is_changeset_significant(changes: &ChangeSet) -> bool {
    use helix_core::Operation::*;

    let mut diff = 0;
    for operation in changes.changes() {
        match operation {
            Retain(_) => continue,
            Delete(_) | Insert(_) => diff += operation.len_chars(),
        }
    }

    // This is arbitrary and could be tuned further:
    diff > 1_000
}

pub(crate) fn register_hooks(handlers: &Handlers) {
    let coordinator = handlers.word_index.coordinator.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        let doc = doc!(event.editor, &event.doc);
        if doc.word_completion_enabled() {
            coordinator.send(Event::Insert(doc.text().clone())).unwrap();
        }
        Ok(())
    });

    let tx = handlers.word_index.hook.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        if !event.ghost_transaction && event.doc.word_completion_enabled() {
            helix_event::send_blocking(
                &tx,
                Event::Update(
                    event.doc.id(),
                    Change {
                        old_text: event.old_text.clone(),
                        text: event.doc.text().clone(),
                        changes: event.changes.clone(),
                    },
                ),
            );
        }
        Ok(())
    });

    let tx = handlers.word_index.hook.clone();
    register_hook!(move |event: &mut DocumentDidClose<'_>| {
        if event.doc.word_completion_enabled() {
            helix_event::send_blocking(
                &tx,
                Event::Delete(event.doc.id(), event.doc.text().clone()),
            );
        }
        Ok(())
    });

    let coordinator = handlers.word_index.coordinator.clone();
    register_hook!(move |event: &mut ConfigDidChange<'_>| {
        // The feature has been turned off. Clear the index and reclaim any used memory.
        if event.old.word_completion.enable && !event.new.word_completion.enable {
            coordinator.send(Event::Clear).unwrap();
        }

        // The feature has been turned on. Index open documents.
        if !event.old.word_completion.enable && event.new.word_completion.enable {
            for doc in event.editor.documents() {
                if doc.word_completion_enabled() {
                    coordinator.send(Event::Insert(doc.text().clone())).unwrap();
                }
            }
        }

        Ok(())
    });
}

// See `benches/word_index.rs`.
#[cfg(feature = "bench")]
pub mod bench {
    use helix_core::{Rope, RopeSlice};

    pub use super::WordIndex;

    pub fn add_document(index: &WordIndex, text: &Rope) {
        let mut cancel = helix_event::TaskController::new();
        index.add_document(text, &cancel.restart());
    }

    pub fn words(text: RopeSlice) -> impl Iterator<Item = RopeSlice> {
        super::words(text)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use helix_core::diff::compare_ropes;

    impl WordIndex {
        fn words(&self) -> HashSet<String> {
            let inner = self.inner.read();
            inner.words().map(|w| w.to_string()).collect()
        }
    }

    #[track_caller]
    fn assert_words<I: ToString, T: IntoIterator<Item = I>>(text: &str, expected: T) {
        let text = Rope::from_str(text);
        let index = WordIndex::default();
        let mut cancel = TaskController::new();
        index.add_document(&text, &cancel.restart());
        let actual = index.words();
        let expected: HashSet<_> = expected.into_iter().map(|i| i.to_string()).collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn parse() {
        assert_words("one two three", ["one", "two", "three"]);
        assert_words("a foo c", ["foo"]);
    }

    #[track_caller]
    fn assert_diff<S, R, I>(before: &str, after: &str, expect_removed: R, expect_inserted: I)
    where
        S: ToString,
        R: IntoIterator<Item = S>,
        I: IntoIterator<Item = S>,
    {
        let before = Rope::from_str(before);
        let after = Rope::from_str(after);
        let diff = compare_ropes(&before, &after);
        let expect_removed: HashSet<_> =
            expect_removed.into_iter().map(|i| i.to_string()).collect();
        let expect_inserted: HashSet<_> =
            expect_inserted.into_iter().map(|i| i.to_string()).collect();

        let index = WordIndex::default();
        let mut cancel = TaskController::new();
        let handle = cancel.restart();
        index.add_document(&before, &handle);
        let words_before = index.words();
        index.update_document(&before, &after, diff.changes(), &handle);
        let words_after = index.words();

        let actual_removed = words_before.difference(&words_after).cloned().collect();
        let actual_inserted = words_after.difference(&words_before).cloned().collect();

        eprintln!("\"{before}\" {words_before:?} => \"{after}\" {words_after:?}");
        assert_eq!(
            expect_removed, actual_removed,
            "expected {expect_removed:?} to be removed, instead {actual_removed:?} was"
        );
        assert_eq!(
            expect_inserted, actual_inserted,
            "expected {expect_inserted:?} to be inserted, instead {actual_inserted:?} was"
        );
    }

    #[test]
    fn diff() {
        assert_diff("one two three", "one five three", ["two"], ["five"]);
        assert_diff("one two three", "one to three", ["two"], []);
        assert_diff("one two three", "one three", ["two"], []);
        assert_diff("one two three", "one t{o three", ["two"], []);
        assert_diff("one foo three", "one fooo three", ["foo"], ["fooo"]);
    }

    const CORPORA: &[(&str, &str)] = &[
        ("arabic", include_str!("../../benches/texts/arabic.txt")),
        ("english", include_str!("../../benches/texts/english.txt")),
        ("hindi", include_str!("../../benches/texts/hindi.txt")),
        ("japanese", include_str!("../../benches/texts/japanese.txt")),
        ("korean", include_str!("../../benches/texts/korean.txt")),
        ("mandarin", include_str!("../../benches/texts/mandarin.txt")),
        ("russian", include_str!("../../benches/texts/russian.txt")),
        (
            "source_code",
            include_str!("../../benches/texts/source_code.txt"),
        ),
    ];

    #[track_caller]
    fn assert_extracts(text: &str, expected: &[&str]) {
        let rope = Rope::from_str(text);
        let got: Vec<String> = words(rope.slice(..)).map(|w| w.to_string()).collect();
        assert_eq!(got, expected, "extracting words from {text:?}");
    }

    /// `words` categorizes whole grapheme clusters, so a word boundary never falls inside a
    /// cluster. A non-word combining mark stays attached to the word character it modifies.
    #[test]
    fn extract_respects_grapheme_clusters() {
        // Whitespace and punctuation separate words. Runs under MIN_WORD_GRAPHEMES are dropped.
        assert_extracts("a foo c", &["foo"]);
        assert_extracts(
            "foo.bar.baz qux::quux",
            &["foo", "bar", "baz", "qux", "quux"],
        );
        assert_extracts("snake_case_id CamelCase", &["snake_case_id", "CamelCase"]);
        // Precomposed and decomposed accents both keep the whole word: the combining mark rides
        // along with the letter it attaches to rather than truncating the word before it.
        assert_extracts("naïve café résumé", &["naïve", "café", "résumé"]);
        assert_extracts(
            "nai\u{0308}ve cafe\u{0301}",
            &["nai\u{0308}ve", "cafe\u{0301}"],
        );
        // A Devanagari conjunct is joined by a virama (itself a non-word char) yet stays one word.
        assert_extracts("ज्ञानकोश है", &["ज्ञानकोश"]);
        // Hangul syllables are word characters.
        assert_extracts("한국어 위키백과", &["한국어", "위키백과"]);
        // An emoji cluster is not.
        assert_extracts("emoji 👍🏽 word", &["emoji", "word"]);
    }

    /// Every word extracted from the corpora must satisfy the documented invariants.
    #[test]
    fn extract_corpora_invariants() {
        for (name, text) in CORPORA {
            let rope = Rope::from_str(text);
            let mut count = 0;
            for word in words(rope.slice(..)) {
                count += 1;
                let clusters = word.graphemes().count();
                assert!(
                    clusters >= MIN_WORD_GRAPHEMES,
                    "{name}: {word:?} spans only {clusters} grapheme cluster(s)"
                );
                assert!(
                    word.len_chars() <= MAX_WORD_LEN,
                    "{name}: {word:?} is {} chars long",
                    word.len_chars()
                );
                assert!(
                    word.chars().next().is_some_and(char_is_word),
                    "{name}: {word:?} starts with a non-word character"
                );
            }
            assert!(count > 0, "{name}: expected to extract some words");
        }
    }
}
