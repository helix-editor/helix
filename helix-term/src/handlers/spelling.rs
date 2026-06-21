//! Spell checking as a non-LSP diagnostic source.
//!
//! Checking a word is cheap, so the work is split by where it is cheapest to run. A small edit is
//! re-checked incrementally and synchronously on the main loop: only the regions around the change
//! (`ChangeSet::changed_ranges`) are re-tokenized and spliced in with `splice_diagnostics`. A
//! full check (document open, dictionary load, or a large or fragmented edit) runs off the main
//! thread and replaces the spelling diagnostics wholesale, discarding its result if the document
//! changed while it ran.

use std::{
    borrow::Cow, collections::HashMap, future::Future, ops::Range, sync::Arc, time::Duration,
};

use helix_core::{
    chars::char_is_word,
    diagnostic::{Diagnostic, DiagnosticProvider, Range as DiagnosticRange, Severity},
    diff::compare_ropes,
    syntax::{config::SpellingFilter, Loader},
    ChangeSet, Operation, Rope, RopeSlice, SpellingLanguage, Syntax,
};
use helix_event::{cancelable_future, register_hook, send_blocking, AsyncHook};
use helix_stdx::rope::{Regex, RopeSliceExt as _};
use helix_view::{
    events::{DocumentDidChange, DocumentDidClose, DocumentDidOpen},
    handlers::{spelling::SpellingEvent, Handlers},
    Dictionary, DocumentId, Editor,
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use tokio::time::Instant;

use crate::job;

const PROVIDER: DiagnosticProvider = DiagnosticProvider::Spelling;

/// How long to wait after the last change before re-checking.
const DEBOUNCE: Duration = Duration::from_secs(1);
/// Char padding around each edit when re-checking incrementally. At least the longest word, so an
/// edit in the middle of a word re-checks the whole word and its neighbours.
const WINDOW_PADDING: usize = 50;
/// A change touching more than this many chars is re-scanned fully rather than incrementally.
const LARGE_EDIT_CHARS: usize = 1000;
/// A change with more separate edits than this (e.g. a multi-cursor edit) would blanket the
/// document in re-check windows; it is cheaper to rescan wholesale once.
// TODO: a more precise "windowed coverage exceeds X% of the document" metric, tuned against
// benchmarks, would supersede this coarse op count.
const MANY_EDIT_OPS: usize = 64;

#[derive(Debug)]
struct Change {
    old_text: Rope,
    text: Rope,
    changes: ChangeSet,
    version: i32,
    /// Whether `changes` is stale (the result of coalescing several observed changes) and must be
    /// recomputed from `old_text`/`text` at [`SpellingHook::finish_debounce`]. See the word index
    /// for why coalesced changesets cannot be chained directly.
    dirty: bool,
}

#[derive(Debug, Default)]
pub(super) struct SpellingHook {
    changes: HashMap<DocumentId, Change>,
}

impl AsyncHook for SpellingHook {
    type Event = SpellingEvent;

    fn handle_event(&mut self, event: Self::Event, timeout: Option<Instant>) -> Option<Instant> {
        match event {
            SpellingEvent::DictionaryLoaded { language } => {
                job::dispatch_blocking(move |editor, _| {
                    let docs: Vec<_> = editor
                        .documents()
                        .filter(|doc| doc.spelling_languages.contains(&language))
                        .map(|doc| doc.id())
                        .collect();
                    for doc in docs {
                        check_document(editor, doc);
                    }
                });
                timeout
            }
            SpellingEvent::DocumentOpened { doc } => {
                job::dispatch_blocking(move |editor, _| check_document(editor, doc));
                timeout
            }
            SpellingEvent::DocumentChanged {
                doc,
                old_text,
                text,
                changes,
                version,
            } => {
                if let Some(pending) = self.changes.get_mut(&doc) {
                    // Coalesce with the pending change: keep the original `old_text`, advance to the
                    // latest `text`/`version`, and recompute the changeset lazily (see `Change`).
                    pending.text = text;
                    pending.version = version;
                    pending.dirty = true;
                } else {
                    self.changes.insert(
                        doc,
                        Change {
                            old_text,
                            text,
                            changes,
                            version,
                            dirty: false,
                        },
                    );
                }
                Some(Instant::now() + DEBOUNCE)
            }
        }
    }

    fn finish_debounce(&mut self) {
        for (doc, mut change) in self.changes.drain() {
            if change.dirty {
                change.changes = compare_ropes(&change.old_text, &change.text)
                    .changes()
                    .clone();
            }
            let changes = change.changes;
            let version = change.version;
            job::dispatch_blocking(move |editor, _| {
                recheck_document(editor, doc, changes, version);
            });
        }
    }
}

/// Re-checks a document incrementally around `changes`, falling back to a full rescan when the
/// document has moved on since the snapshot (`version`) or the change is too large/fragmented to
/// be worth doing incrementally.
fn recheck_document(editor: &mut Editor, doc_id: DocumentId, changes: ChangeSet, version: i32) {
    let Some(doc) = editor.documents.get(&doc_id) else {
        return;
    };
    if doc.spelling_languages.is_empty() {
        return;
    }
    let languages = doc.spelling_languages.clone();
    let stale = doc.version() != version;
    let text = doc.text().clone();
    let filter = SpellingFilter::new(
        &editor.config().spelling.merged(
            doc.language_config()
                .and_then(|config| config.spelling.as_ref()),
        ),
    );
    let Some(dictionaries) = lookup_dictionaries(editor, &languages) else {
        return;
    };

    if stale || needs_full_scan(&changes) {
        check_document(editor, doc_id);
        return;
    }

    // Not stale, so the changed ranges line up with `text`. Collect the regions to re-check,
    // merging overlapping windows so no word is checked (or emitted) twice.
    let mut regions: Vec<Range<usize>> = Vec::new();
    for (_, new_range) in changes.changed_ranges(WINDOW_PADDING) {
        match regions.last_mut() {
            Some(last) if new_range.start <= last.end => last.end = last.end.max(new_range.end),
            _ => regions.push(new_range),
        }
    }

    // Restrict each window to the parts worth checking per the syntax tree. The splice below still
    // clears the whole window, so a word that left a checked region (e.g. a comment edited into
    // code) loses its diagnostic.
    let loader = editor.syn_loader.load_full();
    let doc = editor.documents.get(&doc_id).unwrap();
    let check_regions: Vec<Range<usize>> = regions
        .iter()
        .flat_map(|window| {
            spell_check_regions(doc.syntax(), &loader, text.slice(..), window.clone())
        })
        .collect();

    let diagnostics = {
        let guards: Vec<_> = dictionaries
            .iter()
            .map(|dictionary| dictionary.read())
            .collect();
        let dictionaries: Vec<&Dictionary> = guards.iter().map(|guard| &**guard).collect();
        let mut diagnostics = Vec::new();
        for region in check_regions {
            check_region(
                &dictionaries,
                &filter,
                text.slice(..),
                region,
                &mut diagnostics,
            );
        }
        diagnostics
    };

    let doc = editor.documents.get_mut(&doc_id).unwrap();
    doc.splice_diagnostics(diagnostics, &regions, &PROVIDER);
    helix_event::dispatch(helix_view::events::DiagnosticsDidChange {
        editor,
        doc: doc_id,
    });
}

/// Checks an entire document off the main loop and replaces its spelling diagnostics wholesale.
fn check_document(editor: &mut Editor, doc_id: DocumentId) {
    let Some(doc) = editor.documents.get(&doc_id) else {
        return;
    };
    if doc.spelling_languages.is_empty() {
        return;
    }
    let languages = doc.spelling_languages.clone();
    let version = doc.version();
    let text = doc.text().clone();
    // Cloning the syntax bumps a few refcounts on its (persistent) trees; cheap enough to snapshot
    // for the off-thread check.
    let syntax = doc.syntax().cloned();
    let filter = SpellingFilter::new(
        &editor.config().spelling.merged(
            doc.language_config()
                .and_then(|config| config.spelling.as_ref()),
        ),
    );
    let loader = editor.syn_loader.load_full();
    let Some(dictionaries) = lookup_dictionaries(editor, &languages) else {
        return;
    };

    let cancel = editor.handlers.spelling.open_request(doc_id);
    let future = check_text(dictionaries, text, syntax, loader, filter);

    tokio::spawn(async move {
        match cancelable_future(future, cancel).await {
            Some(Ok(diagnostics)) => {
                job::dispatch_blocking(move |editor, _| {
                    editor.handlers.spelling.requests.remove(&doc_id);
                    let Some(doc) = editor.documents.get_mut(&doc_id) else {
                        return;
                    };
                    // A newer edit landed while we were checking; it has queued its own re-check.
                    if doc.version() != version {
                        return;
                    }
                    doc.replace_diagnostics(diagnostics, &[], Some(&PROVIDER));
                    helix_event::dispatch(helix_view::events::DiagnosticsDidChange {
                        editor,
                        doc: doc_id,
                    });
                });
            }
            Some(Err(err)) => log::error!("spell check task panicked: {err}"),
            None => (),
        }
    });
}

/// Returns the dictionaries for `languages`, or `None` if any are not loaded yet (after kicking off
/// the missing loads). Checking waits until all are present so a not-yet-loaded dictionary can't
/// cause false positives; the load completion re-checks via [`SpellingEvent::DictionaryLoaded`].
fn lookup_dictionaries(
    editor: &mut Editor,
    languages: &[SpellingLanguage],
) -> Option<Vec<Arc<RwLock<Dictionary>>>> {
    let mut dictionaries = Vec::with_capacity(languages.len());
    let mut missing = false;
    for language in languages {
        // Call through for every language so all missing loads are kicked off, not just the first.
        match lookup_dictionary(editor, language.clone()) {
            Some(dictionary) => dictionaries.push(dictionary),
            None => missing = true,
        }
    }
    (!missing).then_some(dictionaries)
}

/// Returns the dictionary for `language`, kicking off an async load (once) if it isn't loaded yet.
fn lookup_dictionary(
    editor: &mut Editor,
    language: SpellingLanguage,
) -> Option<Arc<RwLock<Dictionary>>> {
    if let Some(dictionary) = editor.dictionaries.get(&language) {
        return Some(dictionary.clone());
    }
    if editor
        .handlers
        .spelling
        .loading_dictionaries
        .insert(language.clone())
    {
        load_dictionary(language);
    }
    None
}

fn load_dictionary(language: SpellingLanguage) {
    tokio::task::spawn_blocking(move || {
        let load = || -> anyhow::Result<Dictionary> {
            let aff = std::fs::read_to_string(helix_loader::runtime_file(format!(
                "dictionaries/{language}/{language}.aff"
            )))?;
            let dic = std::fs::read_to_string(helix_loader::runtime_file(format!(
                "dictionaries/{language}/{language}.dic"
            )))?;
            let mut dictionary = Dictionary::new(&aff, &dic)
                .map_err(|err| anyhow::anyhow!("could not parse dictionary: {err:?}"))?;

            // Append this language's personal dictionary, skipping entries spellbook rejects rather
            // than failing the whole load.
            if let Ok(file) =
                std::fs::File::open(helix_loader::personal_dictionary_file(language.as_str()))
            {
                use std::io::{BufRead as _, BufReader};
                for line in BufReader::new(file).lines() {
                    let word = line?;
                    let word = word.trim();
                    if word.is_empty() {
                        continue;
                    }
                    if let Err(err) = dictionary.add(word) {
                        log::warn!("ignoring personal dictionary entry {word:?}: {err:?}");
                    }
                }
            }

            Ok(dictionary)
        };

        match load() {
            Ok(dictionary) => job::dispatch_blocking(move |editor, _| {
                editor
                    .handlers
                    .spelling
                    .loading_dictionaries
                    .remove(&language);
                editor
                    .dictionaries
                    .insert(language.clone(), Arc::new(RwLock::new(dictionary)));
                send_blocking(
                    &editor.handlers.spelling.event_tx,
                    SpellingEvent::DictionaryLoaded { language },
                );
            }),
            Err(err) => {
                log::error!("could not load spelling dictionary '{language}': {err}");
                // Allow a later check to retry the load.
                job::dispatch_blocking(move |editor, _| {
                    editor
                        .handlers
                        .spelling
                        .loading_dictionaries
                        .remove(&language);
                });
            }
        }
    });
}

fn check_text(
    dictionaries: Vec<Arc<RwLock<Dictionary>>>,
    text: Rope,
    syntax: Option<Syntax>,
    loader: Arc<Loader>,
    filter: SpellingFilter,
) -> impl Future<Output = Result<Vec<Diagnostic>, tokio::task::JoinError>> {
    tokio::task::spawn_blocking(move || {
        let guards: Vec<_> = dictionaries
            .iter()
            .map(|dictionary| dictionary.read())
            .collect();
        let dictionaries: Vec<&Dictionary> = guards.iter().map(|guard| &**guard).collect();
        let mut diagnostics = Vec::new();
        for region in spell_check_regions(
            syntax.as_ref(),
            &loader,
            text.slice(..),
            0..text.len_chars(),
        ) {
            check_region(
                &dictionaries,
                &filter,
                text.slice(..),
                region,
                &mut diagnostics,
            );
        }
        diagnostics
    })
}

/// The char ranges within `region` to spell-check. With a syntax tree, checking is restricted to
/// the natural-language regions selected by each layer's `spellcheck.scm` query (comments, prose,
/// ...); without a tree (plain text), the whole `region` is checked.
//
// `Syntax::spell_regions` works in byte offsets (tree-sitter's native unit) while the spelling
// diagnostics, like all diagnostics, are in char offsets, so we convert at this boundary. The
// conversions go away once diagnostics move to byte offsets.
fn spell_check_regions(
    syntax: Option<&Syntax>,
    loader: &Loader,
    text: RopeSlice,
    region: Range<usize>,
) -> Vec<Range<usize>> {
    let Some(syntax) = syntax else {
        return vec![region];
    };
    let bytes = text.char_to_byte(region.start)..text.char_to_byte(region.end);
    syntax
        .spell_regions(text, loader, bytes)
        .into_iter()
        .map(|region| text.byte_to_char(region.start)..text.byte_to_char(region.end))
        .collect()
}

/// Tokenizes the `region` (a char range) of `text` and appends a diagnostic for each word that
/// every dictionary rejects (a word known to any one of them is accepted). Match offsets from
/// `regex_input_at` are absolute byte offsets in `text`.
fn check_region(
    dictionaries: &[&Dictionary],
    filter: &SpellingFilter,
    text: RopeSlice,
    region: Range<usize>,
    out: &mut Vec<Diagnostic>,
) {
    static WORDS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[0-9A-Z]*(['-]?[a-z]+)*").unwrap());
    // URLs and email addresses tokenize into word-like fragments (host and path segments) that
    // aren't real words, so skip any word overlapping one. These match the source text rather than
    // individual tokens, which is why they can't be expressed as ordinary `ignore-regexes`.
    static IGNORE_SPANS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://\S+|[\w.+-]+@[A-Za-z0-9-]+\.[\w.-]+").unwrap()
    });

    let ignore_spans: Vec<Range<usize>> = IGNORE_SPANS
        .find_iter(text.regex_input_at(region.clone()))
        .map(|m| m.range())
        .collect();

    for m in WORDS.find_iter(text.regex_input_at(region)) {
        if m.range().is_empty() {
            continue;
        }
        // Skip words that fall inside a URL or email address.
        if ignore_spans
            .iter()
            .any(|span| m.start() < span.end && span.start < m.end())
        {
            continue;
        }
        let word = Cow::from(text.byte_slice(m.range()));
        if word.is_empty() || filter.ignores(&word) {
            continue;
        }
        if !dictionaries
            .iter()
            .any(|dictionary| dictionary.check(&word))
        {
            let start = text.byte_to_char(m.start());
            let end = text.byte_to_char(m.end());
            out.push(spelling_diagnostic(text, start, end, &word));
        }
    }
}

fn spelling_diagnostic(text: RopeSlice, start: usize, end: usize, word: &str) -> Diagnostic {
    // Mirror `lsp_diagnostic_to_diagnostic` so edit-mapping associations behave the same.
    let ends_at_word = start != end && end != 0 && text.get_char(end - 1).is_some_and(char_is_word);
    let starts_at_word = start != end && text.get_char(start).is_some_and(char_is_word);
    Diagnostic {
        range: DiagnosticRange { start, end },
        ends_at_word,
        starts_at_word,
        zero_width: start == end,
        line: text.char_to_line(start),
        message: format!("Possible spelling mistake: '{word}'"),
        severity: Some(Severity::Hint),
        code: None,
        provider: PROVIDER,
        tags: Vec::new(),
        source: Some(Cow::Borrowed("spelling")),
        data: None,
    }
}

/// Whether a change should be re-scanned wholesale instead of incrementally: a large edit, or one
/// fragmented across many sites (e.g. multi-cursor) whose padded windows would blanket the doc.
fn needs_full_scan(changes: &ChangeSet) -> bool {
    let mut edited_chars = 0;
    let mut edit_ops = 0;
    for op in changes.changes() {
        if !matches!(op, Operation::Retain(_)) {
            edited_chars += op.len_chars();
            edit_ops += 1;
        }
    }
    edited_chars > LARGE_EDIT_CHARS || edit_ops > MANY_EDIT_OPS
}

pub(super) fn register_hooks(handlers: &Handlers) {
    let tx = handlers.spelling.event_tx.clone();
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        let doc = doc!(event.editor, &event.doc);
        if !doc.spelling_languages.is_empty() {
            send_blocking(&tx, SpellingEvent::DocumentOpened { doc: event.doc });
        }
        Ok(())
    });

    let tx = handlers.spelling.event_tx.clone();
    register_hook!(move |event: &mut DocumentDidChange<'_>| {
        // Mirror the word index: ignore synthetic edits so they don't churn the diagnostics.
        if !event.ghost_transaction && !event.doc.spelling_languages.is_empty() {
            send_blocking(
                &tx,
                SpellingEvent::DocumentChanged {
                    doc: event.doc.id(),
                    old_text: event.old_text.clone(),
                    text: event.doc.text().clone(),
                    changes: event.changes.clone(),
                    version: event.doc.version(),
                },
            );
        }
        Ok(())
    });

    register_hook!(move |event: &mut DocumentDidClose<'_>| {
        // Cancel any in-flight full check for the closed document.
        event
            .editor
            .handlers
            .spelling
            .requests
            .remove(&event.doc.id());
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_core::{syntax::config::SpellingConfig, Rope};

    /// The `en_US` dictionary vendored under `runtime/dictionaries/`.
    fn en_us() -> Dictionary {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../runtime/dictionaries/en_US");
        let aff = std::fs::read_to_string(format!("{dir}/en_US.aff")).unwrap();
        let dic = std::fs::read_to_string(format!("{dir}/en_US.dic")).unwrap();
        Dictionary::new(&aff, &dic).unwrap()
    }

    /// A filter that skips nothing (the default config).
    fn no_filter() -> SpellingFilter {
        SpellingFilter::new(&SpellingConfig::default())
    }

    fn check(text: &str, region: Range<usize>) -> Vec<Diagnostic> {
        let rope = Rope::from_str(text);
        let mut out = Vec::new();
        check_region(&[&en_us()], &no_filter(), rope.slice(..), region, &mut out);
        out
    }

    /// A throwaway dictionary containing exactly `words`, for testing multi-dictionary checks.
    fn mini_dictionary(words: &[&str]) -> Dictionary {
        let dic = format!("{}\n{}\n", words.len(), words.join("\n"));
        Dictionary::new("SET UTF-8\n", &dic).unwrap()
    }

    #[test]
    fn a_word_known_to_any_dictionary_is_accepted() {
        // "wrld" is not in en_US, so en_US alone flags it.
        let rope = Rope::from_str("wrld");
        let mut out = Vec::new();
        check_region(&[&en_us()], &no_filter(), rope.slice(..), 0..4, &mut out);
        assert_eq!(out.len(), 1, "{out:?}");

        // A second dictionary that knows "wrld" makes the OR accept it.
        let custom = mini_dictionary(&["wrld"]);
        let mut out = Vec::new();
        check_region(
            &[&en_us(), &custom],
            &no_filter(),
            rope.slice(..),
            0..4,
            &mut out,
        );
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn flags_only_the_misspelled_word() {
        let diagnostics = check("the quik brown fox", 0..18);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        let d = &diagnostics[0];
        assert_eq!((d.range.start, d.range.end), (4, 8));
        assert_eq!(d.provider, DiagnosticProvider::Spelling);
        assert_eq!(d.severity, Some(Severity::Hint));
        assert!(d.starts_at_word && d.ends_at_word && !d.zero_width);
    }

    #[test]
    fn region_scopes_the_scan() {
        // The same misspelling appears twice; only the one inside the region is reported.
        let diagnostics = check("quik brown quik", 11..15);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(
            (diagnostics[0].range.start, diagnostics[0].range.end),
            (11, 15)
        );
    }

    #[test]
    fn offsets_are_char_indices_across_multibyte_text() {
        // A 4-byte emoji precedes the misspelling: the diagnostic range must be in chars (2..6),
        // not bytes (5..9), exercising the byte→char conversion.
        let diagnostics = check("🚀 quik", 0..6);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(
            (diagnostics[0].range.start, diagnostics[0].range.end),
            (2, 6)
        );
    }

    #[test]
    fn skips_words_inside_urls_and_emails() {
        // Only the prose misspelling "teh" is flagged; the misspelled-looking host/path fragments
        // ("barbaz", "exampel") inside the URL and email are skipped.
        let text = "teh https://github.com/foo/barbaz me@exampel.org";
        let diagnostics = check(text, 0..text.len());
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].message.contains("teh"), "{diagnostics:?}");
    }

    #[test]
    fn filter_skips_allowlisted_short_and_ignored_words() {
        let config = SpellingConfig {
            words: vec!["Helix".into()],
            ignore_regexes: vec!["^[A-Z0-9_]+$".into()],
            min_word_length: Some(3),
            ..Default::default()
        };
        let filter = SpellingFilter::new(&config);
        let rope = Rope::from_str("Helix HE teh ABC123");
        let mut out = Vec::new();
        check_region(
            &[&en_us()],
            &filter,
            rope.slice(..),
            0..rope.len_chars(),
            &mut out,
        );
        // "Helix" is allowlisted (case-insensitively), "HE" is too short, and "ABC123" matches the
        // ignore regex; only the genuine misspelling "teh" survives.
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(
            rope.slice(out[0].range.start..out[0].range.end).to_string(),
            "teh"
        );
    }

    /// Runs the full-document check path's logic (region selection + tokenization) against a real
    /// syntax tree, the way `check_text` does off-thread.
    fn check_scoped(language: &str, text: &str) -> Vec<Diagnostic> {
        let loader = helix_core::config::default_lang_loader();
        let rope = Rope::from_str(text);
        let language = loader.language_for_name(language).unwrap();
        let syntax = Syntax::new(rope.slice(..), language, &loader).unwrap();
        let dictionary = en_us();
        let mut out = Vec::new();
        for region in
            spell_check_regions(Some(&syntax), &loader, rope.slice(..), 0..rope.len_chars())
        {
            check_region(
                &[&dictionary],
                &no_filter(),
                rope.slice(..),
                region,
                &mut out,
            );
        }
        out
    }

    #[test]
    fn syntax_scoping_checks_comments_not_code() {
        // `teh` in the comment is a misspelling; the identically misspelled identifier `teh_value`
        // is code and must not be flagged.
        let diagnostics = check_scoped("rust", "// teh bug\nlet teh_value = 1;\n");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].message.contains("teh"), "{diagnostics:?}");
        assert_eq!(diagnostics[0].range.start, 3, "the comment occurrence");
    }
}
