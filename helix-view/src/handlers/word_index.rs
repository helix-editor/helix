//! Indexing of words from open buffers.
//!
//! This provides an eventually consistent set of words used in any open buffers. This set is
//! later used for lexical completion.

use std::{borrow::Cow, iter, sync::Arc, time::Duration};

use hashbrown::HashMap;
use helix_core::{
    chars::char_is_word, diff::compare_ropes, fuzzy::fuzzy_match, ChangeSet, Rope, RopeSlice,
};
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
    /// Whether `changes` is stale and must be recomputed from `old_text`/`text` before use.
    ///
    /// Set when this change is the result of coalescing several observed changes (see
    /// [`Hook::handle_event`]). The observed changesets cannot be reliably chained, so the
    /// coalesced changeset is recomputed once, lazily, at [`Hook::finish_debounce`].
    dirty: bool,
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

/// Sends an event to the coordinator task (lossy).
///
/// The coordinator stops when its [`Handler`] is dropped. A closed channel means that index is no
/// longer in use, so dropping the event is harmless.
fn send(coordinator: &mpsc::UnboundedSender<Event>, event: Event) {
    let _ = coordinator.send(event);
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
                    // There is already a change waiting for this document. Coalesce: keep the
                    // original `old_text` and advance to the latest `text`.
                    //
                    // We deliberately do NOT chain the observed changesets here. The index skips
                    // ghost transactions (see `register_hooks`), so a ghost edit may have altered
                    // the document between these two observed changes, which may leave them
                    // non-contiguous in ways a cheap check can't catch. Instead, mark the change
                    // dirty and recompute the changeset from the real ropes at `finish_debounce`,
                    // i.e.  once per debounce window rather than once per keystroke.
                    pending_change.text = change.text;
                    pending_change.dirty = true;
                    Some(Instant::now() + DEBOUNCE)
                } else if !is_changeset_significant(&change.changes) {
                    // The change is small: debounce so a burst of edits coalesces before the
                    // index updates.
                    self.changes.insert(doc, change);
                    Some(Instant::now() + DEBOUNCE)
                } else {
                    // The change is large: update the index immediately rather than waiting out
                    // the debounce.
                    send(&self.coordinator, Event::Update(doc, change));
                    timeout
                }
            }
            Event::Delete(doc, text) => {
                // If there are pending changes that haven't been indexed since the last debounce,
                // forget them and delete the old text.
                if let Some(change) = self.changes.remove(&doc) {
                    send(&self.coordinator, Event::Delete(doc, change.old_text));
                } else {
                    send(&self.coordinator, Event::Delete(doc, text));
                }
                timeout
            }
            Event::Clear => unreachable!("clear is sent to the worker directly"),
        }
    }

    fn finish_debounce(&mut self) {
        for (doc, mut change) in self.changes.drain() {
            // A coalesced change carries a stale changeset; recompute it from the real endpoints.
            // This diff is valid regardless of any ghost edits skipped between observed changes.
            if change.dirty {
                change.changes = compare_ropes(&change.old_text, &change.text)
                    .changes()
                    .clone();
                change.dirty = false;
            }
            send(&self.coordinator, Event::Update(doc, change));
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
            send(&coordinator, Event::Insert(doc.text().clone()));
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
                        dirty: false,
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
            send(&coordinator, Event::Clear);
        }

        // The feature has been turned on. Index open documents.
        if !event.old.word_completion.enable && event.new.word_completion.enable {
            for doc in event.editor.documents() {
                if doc.word_completion_enabled() {
                    send(&coordinator, Event::Insert(doc.text().clone()));
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
    use super::*;
    use hashbrown::{HashMap, HashSet};
    use quickcheck::{Arbitrary, Gen};

    impl WordIndex {
        fn words(&self) -> HashSet<String> {
            let inner = self.inner.read();
            inner.words().map(|w| w.to_string()).collect()
        }

        /// The full reference-counted word multiset. Unlike [`WordIndex::words`] this keeps the
        /// counts, which the incremental update path must hold exactly in step with a fresh index:
        /// a word is only freed once its count falls to zero, so any drift leaks stale words.
        fn counts(&self) -> HashMap<String, u32> {
            let inner = self.inner.read();
            inner
                .words
                .iter()
                .map(|(w, c)| (w.to_string(), *c))
                .collect()
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

    fn build_change(old: &str, new: &str) -> Change {
        let old_text = Rope::from_str(old);
        let text = Rope::from_str(new);
        let changes = compare_ropes(&old_text, &text).changes().clone();
        Change {
            old_text,
            text,
            changes,
            dirty: false,
        }
    }

    /// Drives a [`Hook`] through a sequence of observed `(old_text, new_text)` changes to a single
    /// document, flushes the debounce, and returns the change the hook hands to the indexer (with
    /// its changeset recomputed if the observed changes were coalesced). Returns `None` if no
    /// change was emitted.
    fn coalesce<'a>(observations: impl IntoIterator<Item = (&'a str, &'a str)>) -> Option<Change> {
        let (coordinator, mut rx) = mpsc::unbounded_channel();
        let mut hook = Hook {
            changes: HashMap::default(),
            coordinator,
        };
        let doc = DocumentId::default();
        for (old, new) in observations {
            hook.handle_event(Event::Update(doc, build_change(old, new)), None);
        }
        hook.finish_debounce();
        match rx.try_recv() {
            Ok(Event::Update(_, change)) => Some(change),
            _ => None,
        }
    }

    #[track_caller]
    fn assert_coalesces(observations: &[(&str, &str)]) {
        let change = coalesce(observations.iter().copied())
            .expect("a coalesced change should have been emitted");
        let first = observations.first().unwrap();
        let last = observations.last().unwrap();

        // The coalesced change spans from the first text observed to the last.
        assert_eq!(change.old_text, Rope::from_str(first.0));
        assert_eq!(change.text, Rope::from_str(last.1));

        // Its changeset must faithfully map `old_text` -> `text`; `update_document` relies on this
        // to locate the windows it reindexes.
        let mut applied = change.old_text.clone();
        assert!(change.changes.apply(&mut applied));
        assert_eq!(applied, change.text);

        // Reindexing must converge onto exactly the word multiset of the final text (words and
        // refcounts) matching a fresh index built from scratch.
        let mut cancel = TaskController::new();
        let handle = cancel.restart();
        let index = WordIndex::default();
        index.add_document(&change.old_text, &handle);
        index.update_document(&change.old_text, &change.text, &change.changes, &handle);

        let fresh = WordIndex::default();
        fresh.add_document(&change.text, &handle);

        assert_eq!(index.counts(), fresh.counts());
    }

    #[test]
    fn hook_coalesces_contiguous_changes() {
        // Two real edits that chain cleanly: the text after the first edit is the text before the
        // second, so the changesets compose directly.
        assert_coalesces(&[
            ("the quick brown fox", "the slowish brown fox"),
            ("the slowish brown fox", "the slowish green fox"),
        ]);
    }

    #[test]
    fn hook_coalesces_across_skipped_ghost_edit() {
        // A ghost transaction edited the document between the two real edits and was not reported
        // to the index, so the text before the second edit no longer matches the text after the
        // first. Here it is longer, which would make a chained `ChangeSet::compose` panic on its
        // `len_after == len` precondition.
        assert_coalesces(&[
            ("the quick brown fox", "the quick brown foxes"),
            ("the quick brown foxes jumped over", "the lazy brown foxes"),
        ]);
    }

    #[test]
    fn hook_coalesces_across_length_preserving_ghost_edit() {
        // The subtle case: a ghost edit changes content without changing length, in a region the
        // next real edit leaves untouched. The two observed changes then have matching lengths
        // but different contents, so a length check alone cannot tell they are non-contiguous.
        let p = ".".repeat(60);
        assert_coalesces(&[
            (&format!("aaa{p}ggg"), &format!("bbb{p}ggg")),
            (&format!("bbb{p}hhh"), &format!("ccc{p}hhh")),
        ]);
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

    /// Grapheme-cluster edge cases which the prose corpora underrepresent: combining marks at
    /// word starts and seams, a skin-tone emoji, a ZWJ family sequence, a Devanagari conjunct,
    /// and a lone Hangul syllable.
    const SPICE: &[&str] = &[
        "a\u{0301}",              // base + combining acute
        "\u{0301}lead",           // leading combining mark
        "mid\u{0308}dle",         // combining diaeresis mid-word
        "👍🏽",                     // emoji + skin-tone modifier
        "👨\u{200D}👩\u{200D}👧", // ZWJ family sequence
        "क्ष",                     // Devanagari conjunct (virama)
        "한",                     // Hangul syllable
    ];

    /// Picks a chunk of text: usually a random char span sliced out of one of the multilingual
    /// corpora (real grapheme complexity for free, and slicing on char boundaries deliberately
    /// frays some clusters), occasionally an adversarial cluster from [`SPICE`].
    fn sample_text(g: &mut Gen) -> String {
        if usize::arbitrary(g) % 4 == 0 {
            return g.choose(SPICE).unwrap().to_string();
        }
        let (_, corpus) = g.choose(CORPORA).unwrap();
        let chars: Vec<char> = corpus.chars().collect();
        if chars.is_empty() {
            return String::new();
        }
        let span = usize::arbitrary(g) % 200;
        let start = usize::arbitrary(g) % chars.len();
        let end = (start + span).min(chars.len());
        chars[start..end].iter().collect()
    }

    /// Applies one random splice to `text`: delete a random char range and insert a fresh chunk.
    fn random_edit(g: &mut Gen, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let del_start = usize::arbitrary(g) % (len + 1);
        let del_len = usize::arbitrary(g) % (len - del_start + 1);
        let mut out: String = chars[..del_start].iter().collect();
        out.push_str(&sample_text(g));
        out.extend(&chars[del_start + del_len..]);
        out
    }

    /// An obvious, independent reimplementation of [`words`]: walk grapheme clusters, accumulate
    /// a run of consecutive word-character clusters, and emit it when it ends if it satisfies the
    /// length bounds. Used as an oracle for the optimized single-pass extractor.
    fn reference_words(text: &str) -> Vec<String> {
        let rope = Rope::from_str(text);
        let mut out = Vec::new();
        let mut run = String::new();
        let mut clusters = 0usize;
        let flush = |run: &mut String, clusters: &mut usize, out: &mut Vec<String>| {
            if *clusters >= MIN_WORD_GRAPHEMES && run.chars().count() <= MAX_WORD_LEN {
                out.push(run.clone());
            }
            run.clear();
            *clusters = 0;
        };
        for cluster in rope.slice(..).graphemes() {
            if cluster.chars().next().is_some_and(char_is_word) {
                run.extend(cluster.chars());
                clusters += 1;
            } else {
                flush(&mut run, &mut clusters, &mut out);
            }
        }
        flush(&mut run, &mut clusters, &mut out);
        out
    }

    #[derive(Clone, Debug)]
    struct SampledText(String);

    impl Arbitrary for SampledText {
        fn arbitrary(g: &mut Gen) -> Self {
            SampledText(sample_text(g))
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(self.0.shrink().map(SampledText))
        }
    }

    #[derive(Clone, Debug)]
    struct EditPair {
        old: String,
        new: String,
    }

    impl Arbitrary for EditPair {
        fn arbitrary(g: &mut Gen) -> Self {
            let old = sample_text(g);
            let new = random_edit(g, &old);
            EditPair { old, new }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(
                (self.old.clone(), self.new.clone())
                    .shrink()
                    .map(|(old, new)| EditPair { old, new }),
            )
        }
    }

    /// A sequence of `(old_text, new_text)` changes as the hook observes them. Some steps leave
    /// a gap (the next change's `old_text` differs from the previous `new_text`) modelling an
    /// unobserved ghost edit applied to the document between two real ones.
    #[derive(Clone, Debug)]
    struct Observations(Vec<(String, String)>);

    impl Arbitrary for Observations {
        fn arbitrary(g: &mut Gen) -> Self {
            let steps = usize::arbitrary(g) % 5 + 1;
            let mut current = sample_text(g);
            let mut observations = Vec::with_capacity(steps);
            for _ in 0..steps {
                let observed_old = if bool::arbitrary(g) {
                    // A ghost edit perturbed the document before this real edit, so the observed
                    // `old_text` no longer matches the previous `new_text`.
                    random_edit(g, &current)
                } else {
                    current.clone()
                };
                let new = random_edit(g, &observed_old);
                observations.push((observed_old, new.clone()));
                current = new;
            }
            Observations(observations)
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            Box::new(self.0.shrink().map(Observations))
        }
    }

    quickcheck::quickcheck! {
        /// The optimized single-pass extractor agrees with the obvious reference on arbitrary
        /// (multilingual, cluster-frayed) text.
        fn prop_words_match_reference(text: SampledText) -> bool {
            let rope = Rope::from_str(&text.0);
            let got: Vec<String> = words(rope.slice(..)).map(|w| w.to_string()).collect();
            got == reference_words(&text.0)
        }

        /// Incrementally updating across one edit lands on exactly the same word multiset (words
        /// and refcounts) as reindexing the new text from scratch.
        fn prop_update_document_matches_fresh(pair: EditPair) -> bool {
            let old = Rope::from_str(&pair.old);
            let new = Rope::from_str(&pair.new);
            let changes = compare_ropes(&old, &new).changes().clone();

            let mut cancel = TaskController::new();
            let handle = cancel.restart();
            let incremental = WordIndex::default();
            incremental.add_document(&old, &handle);
            incremental.update_document(&old, &new, &changes, &handle);

            let fresh = WordIndex::default();
            fresh.add_document(&new, &handle);

            incremental.counts() == fresh.counts()
        }

        /// Coalescing a sequence of observed changes (gaps and all) and flushing yields a change
        /// that maps its `old_text` to its `text` and drives the index to the same multiset as a
        /// fresh index of the final text.
        fn prop_hook_coalescing_matches_fresh(obs: Observations) -> bool {
            let Some(change) = coalesce(obs.0.iter().map(|(o, n)| (o.as_str(), n.as_str()))) else {
                return true;
            };

            let mut applied = change.old_text.clone();
            if !change.changes.apply(&mut applied) || applied != change.text {
                return false;
            }

            let mut cancel = TaskController::new();
            let handle = cancel.restart();
            let incremental = WordIndex::default();
            incremental.add_document(&change.old_text, &handle);
            incremental.update_document(&change.old_text, &change.text, &change.changes, &handle);

            let fresh = WordIndex::default();
            fresh.add_document(&change.text, &handle);

            incremental.counts() == fresh.counts()
        }
    }
}
