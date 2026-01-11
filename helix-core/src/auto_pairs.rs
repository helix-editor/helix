//! When typing the opening character of one of the possible pairs defined below,
//! this module provides the functionality to insert the paired closing character.

use crate::{graphemes, movement::Direction, Range, Rope, Selection, Tendril, Transaction};
use bitflags::bitflags;
use std::collections::HashMap;

use smallvec::SmallVec;

// Heavily based on https://github.com/codemirror/closebrackets/
pub const DEFAULT_PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('{', '}'),
    ('[', ']'),
    ('\'', '\''),
    ('"', '"'),
    ('`', '`'),
];

bitflags! {
    /// Context mask for where auto-pairing is allowed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ContextMask: u8 {
        /// Auto-pair in regular code context
        const CODE    = 0b0000_0001;
        /// Auto-pair inside string literals
        const STRING  = 0b0000_0010;
        /// Auto-pair inside comments
        const COMMENT = 0b0000_0100;
        /// Auto-pair inside regex literals
        const REGEX   = 0b0000_1000;
        /// Auto-pair in all contexts
        const ALL     = Self::CODE.bits() | Self::STRING.bits() | Self::COMMENT.bits() | Self::REGEX.bits();
    }
}

/// Classification of bracket pair types for features like rainbow brackets, surround, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BracketKind {
    /// Parentheses, braces, square brackets: (), {}, []
    #[default]
    Bracket,
    /// Quotes: ', ", `
    Quote,
    /// Template/markup delimiters: {% %}, <!-- -->, etc.
    Delimiter,
    /// User-defined custom pair
    Custom,
}

/// The syntactic context at a position in the document.
///
/// Used to determine whether auto-pairing should be allowed based on
/// the `allowed_contexts` field of a `BracketPair`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BracketContext {
    /// Regular code (not inside string, comment, or regex)
    #[default]
    Code,
    /// Inside a string literal
    String,
    /// Inside a comment
    Comment,
    /// Inside a regex literal
    Regex,
    /// Context could not be determined (treat as Code)
    Unknown,
}

impl BracketContext {
    /// Convert this context to the corresponding ContextMask flag.
    pub fn to_mask(self) -> ContextMask {
        match self {
            BracketContext::Code | BracketContext::Unknown => ContextMask::CODE,
            BracketContext::String => ContextMask::STRING,
            BracketContext::Comment => ContextMask::COMMENT,
            BracketContext::Regex => ContextMask::REGEX,
        }
    }
}

/// Represents a multi-character bracket pair configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketPair {
    /// What the user types to trigger pairing (usually == open)
    pub trigger: String,
    /// Inserted on the left
    pub open: String,
    /// Inserted on the right
    pub close: String,
    /// Classification for features (surround, highlighting, etc.)
    pub kind: BracketKind,
    /// Where auto-pairing is allowed (code, string, comment, regex)
    pub allowed_contexts: ContextMask,
    /// Whether this pair participates in surround commands
    pub surround: bool,
    /// Cached char length of trigger
    trigger_len: usize,
    /// Cached char length of open
    open_len: usize,
    /// Cached char length of close
    close_len: usize,
}

impl BracketPair {
    /// Create a new bracket pair with default settings.
    pub fn new(open: impl Into<String>, close: impl Into<String>) -> Self {
        let open = open.into();
        let close = close.into();
        let trigger = open.clone();
        let trigger_len = trigger.chars().count();
        let open_len = open.chars().count();
        let close_len = close.chars().count();
        Self {
            trigger,
            open,
            close,
            kind: BracketKind::Bracket,
            allowed_contexts: ContextMask::CODE,
            surround: true,
            trigger_len,
            open_len,
            close_len,
        }
    }

    /// Create a bracket pair with a specific kind.
    pub fn with_kind(mut self, kind: BracketKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the allowed contexts for this pair.
    pub fn with_contexts(mut self, contexts: ContextMask) -> Self {
        self.allowed_contexts = contexts;
        self
    }

    /// Set whether this pair participates in surround commands.
    pub fn with_surround(mut self, surround: bool) -> Self {
        self.surround = surround;
        self
    }

    /// Set a custom trigger (different from open).
    pub fn with_trigger(mut self, trigger: impl Into<String>) -> Self {
        self.trigger = trigger.into();
        self.trigger_len = self.trigger.chars().count();
        self
    }

    /// Returns true if open == close (symmetric pair like quotes).
    pub fn same(&self) -> bool {
        self.open == self.close
    }

    /// Returns the first character of the trigger.
    pub fn trigger_first_char(&self) -> Option<char> {
        self.trigger.chars().next()
    }

    /// Returns the first character of the close string.
    pub fn close_first_char(&self) -> Option<char> {
        self.close.chars().next()
    }

    /// Returns the cached char length of the trigger.
    pub fn trigger_len(&self) -> usize {
        self.trigger_len
    }

    /// Returns the cached char length of the open string.
    pub fn open_len(&self) -> usize {
        self.open_len
    }

    /// Returns the cached char length of the close string.
    pub fn close_len(&self) -> usize {
        self.close_len
    }

    /// Check if this pair should auto-close at the given position.
    pub fn should_close(&self, doc: &Rope, range: &Range) -> bool {
        Self::next_is_not_alpha(doc, range)
            && (!self.same() || Self::prev_is_not_alpha(doc, range))
    }

    fn next_is_not_alpha(doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        doc.get_char(cursor).map_or(true, |c| !c.is_alphanumeric())
    }

    fn prev_is_not_alpha(doc: &Rope, range: &Range) -> bool {
        let cursor = range.cursor(doc.slice(..));
        prev_char(doc, cursor).map_or(true, |c| !c.is_alphanumeric())
    }
}

impl From<(char, char)> for BracketPair {
    fn from((open, close): (char, char)) -> Self {
        let kind = if open == close {
            BracketKind::Quote
        } else {
            BracketKind::Bracket
        };
        BracketPair::new(open.to_string(), close.to_string()).with_kind(kind)
    }
}

impl From<&(char, char)> for BracketPair {
    fn from(&(open, close): &(char, char)) -> Self {
        (open, close).into()
    }
}

impl From<(&str, &str)> for BracketPair {
    fn from((open, close): (&str, &str)) -> Self {
        let kind = if open == close {
            BracketKind::Quote
        } else if open.len() > 1 || close.len() > 1 {
            BracketKind::Delimiter
        } else {
            BracketKind::Bracket
        };
        BracketPair::new(open, close).with_kind(kind)
    }
}

/// Fast-lookup container for bracket pairs.
#[derive(Debug, Clone, Default)]
pub struct BracketSet {
    pairs: Vec<BracketPair>,
    first_char_index: HashMap<char, Vec<usize>>,
    close_char_index: HashMap<char, Vec<usize>>,
    max_trigger_len: usize,
}

impl BracketSet {
    /// Create a new BracketSet from a list of pairs.
    pub fn new(pairs: Vec<BracketPair>) -> Self {
        let mut first_char_index: HashMap<char, Vec<usize>> = HashMap::new();
        let mut close_char_index: HashMap<char, Vec<usize>> = HashMap::new();
        let mut max_trigger_len = 0;

        for (i, pair) in pairs.iter().enumerate() {
            if let Some(ch) = pair.trigger_first_char() {
                first_char_index.entry(ch).or_default().push(i);
            }
            if let Some(ch) = pair.close_first_char() {
                close_char_index.entry(ch).or_default().push(i);
            }
            max_trigger_len = max_trigger_len.max(pair.trigger.len());
        }

        Self {
            pairs,
            first_char_index,
            close_char_index,
            max_trigger_len,
        }
    }

    /// Create a BracketSet from default single-char pairs.
    pub fn from_default_pairs() -> Self {
        let pairs: Vec<BracketPair> = DEFAULT_PAIRS.iter().map(|p| p.into()).collect();
        Self::new(pairs)
    }

    /// Get all pairs.
    pub fn pairs(&self) -> &[BracketPair] {
        &self.pairs
    }

    /// Get pairs whose trigger starts with the given character.
    pub fn candidates_for_trigger(&self, ch: char) -> impl Iterator<Item = &BracketPair> {
        self.first_char_index
            .get(&ch)
            .into_iter()
            .flatten()
            .map(|&i| &self.pairs[i])
    }

    /// Get pairs whose close starts with the given character.
    pub fn candidates_for_close(&self, ch: char) -> impl Iterator<Item = &BracketPair> {
        self.close_char_index
            .get(&ch)
            .into_iter()
            .flatten()
            .map(|&i| &self.pairs[i])
    }

    /// Get pairs that participate in surround commands.
    pub fn surround_pairs(&self) -> impl Iterator<Item = &BracketPair> {
        self.pairs.iter().filter(|p| p.surround)
    }

    /// Look up a surround pair by a single character.
    ///
    /// Searches surround-enabled pairs for one where the character matches
    /// either the first character of open or close. Returns the (open, close)
    /// strings if found.
    ///
    /// Prefers single-char pairs over multi-char pairs when the char matches
    /// the first character of multiple pairs.
    pub fn get_pair_by_char(&self, ch: char) -> Option<(&str, &str)> {
        self.surround_pairs()
            .filter(|p| {
                let open_first = p.open.chars().next();
                let close_first = p.close.chars().next();
                open_first == Some(ch) || close_first == Some(ch)
            })
            .min_by_key(|p| p.open.len())
            .map(|p| (p.open.as_str(), p.close.as_str()))
    }

    /// Get surround strings for a character, with fallback.
    ///
    /// If the character matches a known surround pair, returns (open, close).
    /// Otherwise, returns the character as both open and close (symmetric pair).
    ///
    /// This mirrors the behavior of `match_brackets::get_pair()` but works
    /// with the `BracketSet` configuration.
    pub fn get_surround_strings(&self, ch: char) -> (String, String) {
        match self.get_pair_by_char(ch) {
            Some((open, close)) => (open.to_string(), close.to_string()),
            None => (ch.to_string(), ch.to_string()),
        }
    }

    /// Get the maximum trigger length.
    pub fn max_trigger_len(&self) -> usize {
        self.max_trigger_len
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Get the number of pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }
}

/// Detect the longest matching trigger at the cursor position.
///
/// For overlapping symmetric pairs like `"` and `"""`, this function applies
/// a heuristic: if the next character after the cursor is the same as what
/// we just typed, we filter out multi-char symmetric pairs. This allows the
/// single-char pair's skip logic to handle closing sequences one character
/// at a time (e.g., stepping through `"""` as three individual `"` skips).
pub fn detect_trigger_at<'a>(
    doc: &Rope,
    cursor_char: usize,
    last_typed: char,
    set: &'a BracketSet,
) -> Option<&'a BracketPair> {
    let mut candidates: Vec<_> = set
        .pairs()
        .iter()
        .filter(|pair| pair.trigger.ends_with(last_typed))
        .collect();

    if candidates.is_empty() {
        return None;
    }

    if candidates.iter().all(|p| p.trigger.len() == 1) {
        return candidates.into_iter().next();
    }

    // For overlapping symmetric triggers like " and """, we need special handling.
    // Count consecutive same-chars before cursor once, then use it for filtering.
    let next_char = doc.get_char(cursor_char);
    let consecutive_count = {
        let mut count = 0;
        let mut pos = cursor_char;
        while pos > 0 {
            if doc.get_char(pos - 1) == Some(last_typed) {
                count += 1;
                pos -= 1;
            } else {
                break;
            }
        }
        count
    };

    // Find the longest multi-char symmetric trigger for this char
    let max_symmetric_trigger_len = candidates
        .iter()
        .filter(|p| p.same() && p.trigger_len() > 1)
        .map(|p| p.trigger_len())
        .max()
        .unwrap_or(0);

    candidates.retain(|pair| {
        if !pair.same() {
            return true;
        }
        let trigger_len = pair.trigger_len();

        // If next char is the same as what we typed, filter out multi-char
        // symmetric pairs - let single-char pair handle skip logic.
        // But keep single-char pairs for skip behavior.
        if trigger_len > 1 && next_char == Some(last_typed) {
            return false;
        }

        // For multi-char symmetric pairs, only match if preceding count is exactly trigger_len - 1
        // (i.e., we're completing the trigger, like "" + " -> """)
        if trigger_len > 1 && consecutive_count != trigger_len - 1 {
            return false;
        }

        // For single-char symmetric pairs when there's NO same char ahead:
        // If we're past the multi-char trigger length, don't auto-pair.
        // This handles the case where user types " after """""" - just insert plain quote.
        if trigger_len == 1
            && next_char != Some(last_typed)
            && max_symmetric_trigger_len > 0
            && consecutive_count >= max_symmetric_trigger_len
        {
            return false;
        }

        true
    });

    if candidates.is_empty() {
        return None;
    }

    // Build sliding window of recent chars to support multi-char triggers
    let start = cursor_char.saturating_sub(set.max_trigger_len().saturating_sub(1));
    let slice = doc.slice(start..cursor_char);
    let recent: String = slice.chars().chain(std::iter::once(last_typed)).collect();

    candidates
        .into_iter()
        .filter(|pair| recent.ends_with(&pair.trigger))
        .max_by_key(|pair| pair.trigger.len())
}

/// When completing a multi-char trigger like `{%` or `<!--`, check if a prefix
/// was already auto-paired with a closer that now needs replacement.
///
/// This handles multi-char triggers that have a prefix:
/// - For `{%` trigger with `{` prefix pair: replaces `}` with `%}`
/// - For `<!--` trigger with `<` prefix pair: replaces `>` with `-->`
fn find_prefix_close_to_replace(
    doc: &Rope,
    cursor: usize,
    matched_pair: &BracketPair,
    set: &BracketSet,
) -> Option<usize> {
    let trigger_len = matched_pair.trigger_len();
    if trigger_len <= 1 {
        return None;
    }

    // Find the longest prefix pair whose trigger is a proper prefix of the matched trigger.
    // For example, for trigger "<!--", we should find "<" → ">" as a prefix pair.
    // For trigger "{%", we should find "{" → "}" as a prefix pair.
    let prefix_pair = set
        .pairs()
        .iter()
        .filter(|p| {
            let pt = &p.trigger;
            pt.len() < matched_pair.trigger.len() && matched_pair.trigger.starts_with(pt)
        })
        .max_by_key(|p| p.trigger_len())?;

    // Check if the prefix pair's closer is at the cursor position
    let close_first_char = prefix_pair.close.chars().next()?;
    let char_at_cursor = doc.get_char(cursor)?;

    if char_at_cursor == close_first_char {
        Some(cursor)
    } else {
        None
    }
}

/// Check if a pair should be active in the given context.
pub fn context_allows_pair(context: BracketContext, pair: &BracketPair) -> bool {
    pair.allowed_contexts.intersects(context.to_mask())
}

/// Detect if a close sequence matches at the cursor position.
/// Returns the longest matching close sequence to disambiguate overlapping pairs.
pub fn detect_close_at<'a>(
    doc: &Rope,
    cursor_char: usize,
    last_typed: char,
    set: &'a BracketSet,
) -> Option<&'a BracketPair> {
    let candidates: Vec<_> = set.candidates_for_close(last_typed).collect();

    if candidates.is_empty() {
        return None;
    }

    let mut best_match: Option<&'a BracketPair> = None;
    let mut best_len: usize = 0;

    for pair in candidates {
        let close_len = pair.close_len();
        if cursor_char + close_len > doc.len_chars() {
            continue;
        }

        if doc.slice(cursor_char..cursor_char + close_len) == pair.close {
            // Prefer the longest matching close sequence
            if close_len > best_len {
                best_match = Some(pair);
                best_len = close_len;
            }
        }
    }

    best_match
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePairResult {
    pub delete_before: usize,
    pub delete_after: usize,
}

/// Detect if the cursor is positioned between a matched open/close pair for deletion.
pub fn detect_pair_for_deletion(
    doc: &Rope,
    cursor: usize,
    set: &BracketSet,
) -> Option<DeletePairResult> {
    if cursor == 0 {
        return None;
    }

    let doc_len = doc.len_chars();
    let mut best_match: Option<DeletePairResult> = None;

    for pair in set.pairs() {
        let open_len = pair.open_len();
        let close_len = pair.close_len();

        if cursor < open_len {
            continue;
        }

        if cursor + close_len > doc_len {
            continue;
        }

        let open_start = cursor - open_len;

        if doc.slice(open_start..cursor) == pair.open
            && doc.slice(cursor..cursor + close_len) == pair.close
        {
            // Prefer longer matches for nested pairs like `{%` inside `{`
            if best_match.as_ref().map_or(true, |m| {
                open_len + close_len > m.delete_before + m.delete_after
            }) {
                best_match = Some(DeletePairResult {
                    delete_before: open_len,
                    delete_after: close_len,
                });
            }
        }
    }

    best_match
}

/// State passed to the auto-pairs hook for context-aware pairing.
#[derive(Debug, Clone)]
pub struct AutoPairState<'a> {
    pub doc: &'a Rope,
    pub selection: &'a Selection,
    pub pairs: &'a BracketSet,
    pub contexts: Option<&'a [BracketContext]>,
    pub syntax: Option<&'a crate::syntax::Syntax>,
    pub lang_data: Option<&'a crate::syntax::LanguageData>,
    pub loader: Option<&'a crate::syntax::Loader>,
}

impl<'a> AutoPairState<'a> {
    /// Create a new AutoPairState without context information.
    pub fn new(doc: &'a Rope, selection: &'a Selection, pairs: &'a BracketSet) -> Self {
        Self {
            doc,
            selection,
            pairs,
            contexts: None,
            syntax: None,
            lang_data: None,
            loader: None,
        }
    }

    /// Create a new AutoPairState with context information.
    pub fn with_contexts(
        doc: &'a Rope,
        selection: &'a Selection,
        pairs: &'a BracketSet,
        contexts: &'a [BracketContext],
    ) -> Self {
        Self {
            doc,
            selection,
            pairs,
            contexts: Some(contexts),
            syntax: None,
            lang_data: None,
            loader: None,
        }
    }

    /// Create a new AutoPairState with full syntax information for language-specific behavior.
    pub fn with_syntax(
        doc: &'a Rope,
        selection: &'a Selection,
        pairs: &'a BracketSet,
        contexts: &'a [BracketContext],
        syntax: &'a crate::syntax::Syntax,
        lang_data: &'a crate::syntax::LanguageData,
        loader: &'a crate::syntax::Loader,
    ) -> Self {
        Self {
            doc,
            selection,
            pairs,
            contexts: Some(contexts),
            syntax: Some(syntax),
            lang_data: Some(lang_data),
            loader: Some(loader),
        }
    }

    /// Get the context for a specific range index, defaulting to Code if not available.
    fn context_for_range(&self, range_idx: usize) -> BracketContext {
        self.contexts
            .and_then(|ctx| ctx.get(range_idx).copied())
            .unwrap_or(BracketContext::Code)
    }

}

/// Core auto-pairs hook implementation.
///
/// When `use_context` is true, applies context-aware features:
/// - Escape-by-backslash detection for quotes
/// - Context filtering (code vs string vs comment)
fn hook_core(state: &AutoPairState<'_>, ch: char, use_context: bool) -> Option<Transaction> {
    let mut end_ranges = SmallVec::with_capacity(state.selection.len());
    let mut offs = 0;
    let mut made_changes = false;

    let transaction = Transaction::change_by_selection(state.doc, state.selection, |start_range| {
        let cursor = start_range.cursor(state.doc.slice(..));

        let context = if use_context {
            let range_idx = state
                .selection
                .ranges()
                .iter()
                .position(|r| r == start_range)
                .unwrap_or(0);
            state.context_for_range(range_idx)
        } else {
            BracketContext::Code
        };

        // Escape check: if quote is preceded by odd number of backslashes,
        // just insert the quote without any pairing/skipping (it's an escape sequence like \")
        if use_context
            && (ch == '"' || ch == '\'')
            && matches!(context, BracketContext::Code | BracketContext::String)
            && is_escaped_by_backslash(state.doc, cursor)
        {
            let mut t = Tendril::new();
            t.push(ch);
            let next_range = get_next_range(state.doc, start_range, offs, 1);
            end_ranges.push(next_range);
            offs += 1;
            made_changes = true;
            return (cursor, cursor, Some(t));
        }

        if let Some(pair) = detect_trigger_at(state.doc, cursor, ch, state.pairs) {
            log::trace!(
                "autopairs: detected trigger '{}' at cursor {}, next_char={:?}",
                pair.trigger,
                cursor,
                state.doc.get_char(cursor)
            );

            if use_context && !context_allows_pair(context, pair) {
                let mut t = Tendril::new();
                t.push(ch);
                let next_range = get_next_range(state.doc, start_range, offs, 1);
                end_ranges.push(next_range);
                offs += 1;
                made_changes = true;
                return (cursor, cursor, Some(t));
            }

            let next_char = state.doc.get_char(cursor);

            // For symmetric pairs, check if we should skip over existing close.
            // We skip when the full close sequence is ahead - this indicates an
            // auto-inserted closer that we should step over rather than insert into.
            if pair.same() && next_char == Some(ch) {
                let close_len = pair.close_len();
                let full_close_ahead = cursor + close_len <= state.doc.len_chars()
                    && state.doc.slice(cursor..cursor + close_len) == pair.close;

                if full_close_ahead {
                    let next_range = get_next_range(state.doc, start_range, offs, 0);
                    end_ranges.push(next_range);
                    made_changes = true;
                    return (cursor, cursor, None);
                }
            }

            if pair.should_close(state.doc, start_range) {
                // Check for symmetric multi-char pair upgrade (e.g., "" + " → """|""")
                // When completing a multi-char symmetric trigger like """, we need to
                // replace the prefix (the already-typed quotes) with the full open+close.
                if pair.same() {
                    let trigger_len = pair.trigger_len();
                    if trigger_len > 1 {
                        let prefix_len = trigger_len - 1;
                        if cursor >= prefix_len {
                            let prefix_start = cursor - prefix_len;
                            let prefix = state.doc.slice(prefix_start..cursor);

                            if prefix.chars().all(|c| c == ch) {
                                log::trace!(
                                    "autopairs: UPGRADE path - prefix '{}' at {}..{}, inserting '{}{}'",
                                    prefix,
                                    prefix_start,
                                    cursor,
                                    pair.open,
                                    pair.close
                                );
                                // UPGRADE: "" + " → """|"""
                                // We delete the prefix and insert full open+close.
                                // Cursor should end up right after the open delimiter.
                                let mut pair_str = Tendril::new();
                                pair_str.push_str(&pair.open);
                                pair_str.push_str(&pair.close);

                                let delete_start = prefix_start;
                                let delete_end = cursor;

                                let open_len = pair.open_len();
                                let close_len = pair.close_len();
                                let chars_removed = delete_end - delete_start;
                                let net_change =
                                    (open_len + close_len) as isize - chars_removed as isize;

                                // Position cursor after the open delimiter
                                // The new cursor position is: prefix_start + open_len + offs
                                let new_cursor_pos = prefix_start + offs + open_len;
                                let next_range = Range::new(new_cursor_pos, new_cursor_pos + 1);
                                end_ranges.push(next_range);

                                if net_change >= 0 {
                                    offs += net_change as usize;
                                }
                                made_changes = true;

                                return (delete_start, delete_end, Some(pair_str));
                            }
                        }
                    }
                }

                let prefix_close_to_remove =
                    find_prefix_close_to_replace(state.doc, cursor, pair, state.pairs);

                let mut pair_str = Tendril::new();
                pair_str.push(ch);
                pair_str.push_str(&pair.close);

                let len_inserted = pair_str.chars().count();

                let (delete_start, delete_end) =
                    if let Some(close_char_pos) = prefix_close_to_remove {
                        (cursor, close_char_pos + 1)
                    } else {
                        (cursor, cursor)
                    };

                let chars_removed = delete_end - delete_start;
                let net_inserted = len_inserted.saturating_sub(chars_removed);

                // Cursor should end up after the typed char, before the close sequence.
                // So cursor moves by 1, but offs accumulates net_inserted for multi-cursor.
                let next_range = get_next_range(state.doc, start_range, offs, 1);
                end_ranges.push(next_range);
                offs += net_inserted;
                made_changes = true;
                return (delete_start, delete_end, Some(pair_str));
            } else {
                let mut t = Tendril::new();
                t.push(ch);
                let next_range = get_next_range(state.doc, start_range, offs, 1);
                end_ranges.push(next_range);
                offs += 1;
                made_changes = true;
                return (cursor, cursor, Some(t));
            }
        }

        if let Some(pair) = detect_close_at(state.doc, cursor, ch, state.pairs) {
            if !pair.same() {
                let next_range = get_next_range(state.doc, start_range, offs, 0);
                end_ranges.push(next_range);
                made_changes = true;
                return (cursor, cursor, None);
            }
        }

        let next_range = get_next_range(state.doc, start_range, offs, 0);
        end_ranges.push(next_range);
        (cursor, cursor, None)
    });

    if made_changes {
        Some(
            transaction.with_selection(Selection::new(end_ranges, state.selection.primary_index())),
        )
    } else {
        None
    }
}

/// Hook for multi-character auto-pairs with context awareness.
#[must_use]
pub fn hook_with_context(state: &AutoPairState<'_>, ch: char) -> Option<Transaction> {
    log::trace!(
        "autopairs hook_with_context selection: {:#?}",
        state.selection
    );
    hook_core(state, ch, true)
}

/// Hook for multi-character auto-pairs with automatic context detection from syntax tree.
#[must_use]
pub fn hook_with_syntax(
    doc: &Rope,
    selection: &Selection,
    ch: char,
    pairs: &BracketSet,
    syntax: Option<&crate::syntax::Syntax>,
    lang_data: &crate::syntax::LanguageData,
    loader: &crate::syntax::Loader,
) -> Option<Transaction> {
    log::trace!("autopairs hook_with_syntax selection: {:#?}", selection);

    let contexts: SmallVec<[BracketContext; 4]> = selection
        .ranges()
        .iter()
        .map(|range| {
            let cursor = range.cursor(doc.slice(..));
            syntax
                .map(|syn| lang_data.bracket_context_at(syn.tree(), doc.slice(..), cursor, loader))
                .unwrap_or(BracketContext::Code)
        })
        .collect();

    let state = match syntax {
        Some(syn) => {
            AutoPairState::with_syntax(doc, selection, pairs, &contexts, syn, lang_data, loader)
        }
        None => AutoPairState::with_contexts(doc, selection, pairs, &contexts),
    };
    hook_with_context(&state, ch)
}

/// Hook for multi-character auto-pairs without context awareness.
#[must_use]
pub fn hook_multi(
    doc: &Rope,
    selection: &Selection,
    ch: char,
    pairs: &BracketSet,
) -> Option<Transaction> {
    log::trace!("autopairs hook_multi selection: {:#?}", selection);
    let state = AutoPairState::new(doc, selection, pairs);
    hook_core(&state, ch, false)
}

fn prev_char(doc: &Rope, pos: usize) -> Option<char> {
    if pos == 0 {
        return None;
    }

    doc.get_char(pos - 1)
}

/// Check if a quote at the given position is escaped by backslashes.
/// Returns true if there's an odd number of consecutive backslashes before the position.
/// This is used to detect escape sequences like `\"` or `\'` in strings.
pub fn is_escaped_by_backslash(doc: &Rope, cursor: usize) -> bool {
    if cursor == 0 {
        return false;
    }
    let count = doc
        .chars_at(cursor)
        .reversed()
        .take_while(|&c| c == '\\')
        .count();
    count % 2 == 1
}

/// Get the character bounds (start, end) of the line containing the given position.
/// Returns (line_start_char, line_end_char) where line_end_char is exclusive.
pub fn line_bounds(doc: &Rope, pos: usize) -> (usize, usize) {
    let line_idx = doc.char_to_line(pos);
    let line_start = doc.line_to_char(line_idx);
    let line_end = if line_idx + 1 < doc.len_lines() {
        doc.line_to_char(line_idx + 1)
    } else {
        doc.len_chars()
    };
    (line_start, line_end)
}

/// Check if there's an unclosed string literal on the current line.
/// This implements IntelliJ's hasNonClosedLiteral algorithm:
/// 1. Determine if cursor is inside a literal by counting unescaped quotes from line start
/// 2. If inside, scan forward to end-of-line for a closing quote
/// 3. Return true if no closing quote is found (literal is unclosed)
///
/// This is used to decide whether to auto-insert a closing quote.
pub fn has_non_closed_literal_on_line(doc: &Rope, cursor: usize, quote_ch: char) -> bool {
    let line_idx = doc.char_to_line(cursor);
    let line_start = doc.line_to_char(line_idx);
    let line = doc.line(line_idx);
    let cursor_in_line = cursor - line_start;

    // 1) Determine if cursor is inside a literal by counting unescaped quotes
    let inside = line
        .chars()
        .take(cursor_in_line)
        .enumerate()
        .filter(|&(i, c)| c == quote_ch && !is_escaped_by_backslash(doc, line_start + i))
        .count()
        % 2
        == 1;

    if !inside {
        return false;
    }

    // 2) Scan forwards for a closing quote
    let has_closing = line
        .chars()
        .enumerate()
        .skip(cursor_in_line)
        .any(|(i, c)| c == quote_ch && !is_escaped_by_backslash(doc, line_start + i));

    !has_closing
}

/// Get the indentation (leading whitespace) of the line containing the given position.
pub fn get_line_indent(doc: &Rope, pos: usize) -> String {
    let line_idx = doc.char_to_line(pos);
    doc.line(line_idx)
        .chars()
        .take_while(|&c| c == ' ' || c == '\t')
        .collect()
}

/// calculate what the resulting range should be for an auto pair insertion
fn get_next_range(doc: &Rope, start_range: &Range, offset: usize, len_inserted: usize) -> Range {
    // When the character under the cursor changes due to complete pair
    // insertion, we must look backward a grapheme and then add the length
    // of the insertion to put the resulting cursor in the right place, e.g.
    //
    // foo[\r\n] - anchor: 3, head: 5
    // foo([)]\r\n - anchor: 4, head: 5
    //
    // foo[\r\n] - anchor: 3, head: 5
    // foo'[\r\n] - anchor: 4, head: 6
    //
    // foo([)]\r\n - anchor: 4, head: 5
    // foo()[\r\n] - anchor: 5, head: 7
    //
    // [foo]\r\n - anchor: 0, head: 3
    // [foo(])\r\n - anchor: 0, head: 5

    // inserting at the very end of the document after the last newline
    if start_range.head == doc.len_chars() && start_range.anchor == doc.len_chars() {
        return Range::new(
            start_range.anchor + offset + 1,
            start_range.head + offset + 1,
        );
    }

    let doc_slice = doc.slice(..);
    let single_grapheme = start_range.is_single_grapheme(doc_slice);

    // just skip over graphemes
    if len_inserted == 0 {
        let end_anchor = if single_grapheme {
            graphemes::next_grapheme_boundary(doc_slice, start_range.anchor) + offset

        // even for backward inserts with multiple grapheme selections,
        // we want the anchor to stay where it is so that the relative
        // selection does not change, e.g.:
        //
        // foo([) wor]d -> insert ) -> foo()[ wor]d
        } else {
            start_range.anchor + offset
        };

        return Range::new(
            end_anchor,
            graphemes::next_grapheme_boundary(doc_slice, start_range.head) + offset,
        );
    }

    // trivial case: only inserted a single-char opener, just move the selection
    if len_inserted == 1 {
        let end_anchor = if single_grapheme || start_range.direction() == Direction::Backward {
            start_range.anchor + offset + 1
        } else {
            start_range.anchor + offset
        };

        return Range::new(end_anchor, start_range.head + offset + 1);
    }

    // If the head = 0, then we must be in insert mode with a backward
    // cursor, which implies the head will just move
    let end_head = if start_range.head == 0 || start_range.direction() == Direction::Backward {
        start_range.head + offset + 1
    } else {
        // We must have a forward cursor, which means we must move to the
        // other end of the grapheme to get to where the new characters
        // are inserted, then move the head to where it should be
        let prev_bound = graphemes::prev_grapheme_boundary(doc_slice, start_range.head);
        log::trace!(
            "prev_bound: {}, offset: {}, len_inserted: {}",
            prev_bound,
            offset,
            len_inserted
        );
        prev_bound + offset + len_inserted
    };

    let end_anchor = match (start_range.len(), start_range.direction()) {
        // if we have a zero width cursor, it shifts to the same number
        (0, _) => end_head,

        // If we are inserting for a regular one-width cursor, the anchor
        // moves with the head. This is the fast path for ASCII.
        (1, Direction::Forward) => end_head - 1,
        (1, Direction::Backward) => end_head + 1,

        (_, Direction::Forward) => {
            if single_grapheme {
                graphemes::prev_grapheme_boundary(doc.slice(..), start_range.head) + 1

            // if we are appending, the anchor stays where it is; only offset
            // for multiple range insertions
            } else {
                start_range.anchor + offset
            }
        }

        (_, Direction::Backward) => {
            if single_grapheme {
                // if we're backward, then the head is at the first char
                // of the typed char, so we need to add the length of
                // the closing char
                graphemes::prev_grapheme_boundary(doc.slice(..), start_range.anchor)
                    + len_inserted
                    + offset
            } else {
                // when we are inserting in front of a selection, we need to move
                // the anchor over by however many characters were inserted overall
                start_range.anchor + offset + len_inserted
            }
        }
    };

    Range::new(end_anchor, end_head)
}

/// Registry of auto-pairs loaded from auto-pairs.toml.
///
/// This provides a central store of per-language bracket configurations that
/// can be looked up by language name. Languages without explicit configuration
/// fall back to the `default` entry.
#[derive(Debug, Clone, Default)]
pub struct AutoPairsRegistry {
    languages: HashMap<String, BracketSet>,
    default: BracketSet,
}

impl AutoPairsRegistry {
    /// Create a new empty registry with default pairs.
    pub fn new() -> Self {
        Self {
            languages: HashMap::new(),
            default: BracketSet::from_default_pairs(),
        }
    }

    /// Load registry from a parsed TOML value (from auto-pairs.toml).
    ///
    /// The TOML structure is expected to be:
    /// ```toml
    /// [default]
    /// pairs = [{ open = "(", close = ")" }, ...]
    ///
    /// [rust]
    /// pairs = [{ open = "(", close = ")" }, ...]
    /// ```
    pub fn from_toml(value: &toml::Value) -> Result<Self, AutoPairsRegistryError> {
        let table = value
            .as_table()
            .ok_or_else(|| AutoPairsRegistryError::new("expected table"))?;

        let mut languages = HashMap::new();
        let mut default = BracketSet::from_default_pairs();

        for (key, val) in table {
            let pairs = Self::parse_pairs(key, val)?;
            let bracket_set = BracketSet::new(pairs);

            if key == "default" {
                default = bracket_set;
            } else {
                languages.insert(key.clone(), bracket_set);
            }
        }

        Ok(Self { languages, default })
    }

    fn parse_pairs(
        language: &str,
        val: &toml::Value,
    ) -> Result<Vec<BracketPair>, AutoPairsRegistryError> {
        let pairs_val = val.get("pairs").ok_or_else(|| {
            AutoPairsRegistryError::new("missing 'pairs' key").with_language(language)
        })?;

        let pairs_arr = pairs_val.as_array().ok_or_else(|| {
            AutoPairsRegistryError::new("'pairs' must be array").with_language(language)
        })?;

        let mut pairs = Vec::with_capacity(pairs_arr.len());

        for (idx, pair_val) in pairs_arr.iter().enumerate() {
            let open = pair_val.get("open").and_then(|v| v.as_str()).ok_or_else(|| {
                AutoPairsRegistryError::new("pair missing 'open'")
                    .with_language(language)
                    .with_pair_index(idx)
            })?;

            let close = pair_val.get("close").and_then(|v| v.as_str()).ok_or_else(|| {
                AutoPairsRegistryError::new("pair missing 'close'")
                    .with_language(language)
                    .with_pair_index(idx)
            })?;

            let mut bracket_pair = BracketPair::new(open, close);

            if let Some(kind_str) = pair_val.get("kind").and_then(|v| v.as_str()) {
                bracket_pair = bracket_pair.with_kind(match kind_str {
                    "bracket" => BracketKind::Bracket,
                    "quote" => BracketKind::Quote,
                    "delimiter" => BracketKind::Delimiter,
                    "custom" => BracketKind::Custom,
                    _ => BracketKind::Bracket,
                });
            } else {
                let kind = if open == close {
                    BracketKind::Quote
                } else if open.len() > 1 || close.len() > 1 {
                    BracketKind::Delimiter
                } else {
                    BracketKind::Bracket
                };
                bracket_pair = bracket_pair.with_kind(kind);
            }

            if let Some(trigger) = pair_val.get("trigger").and_then(|v| v.as_str()) {
                bracket_pair = bracket_pair.with_trigger(trigger);
            }

            if let Some(surround) = pair_val.get("surround").and_then(|v| v.as_bool()) {
                bracket_pair = bracket_pair.with_surround(surround);
            }

            if let Some(contexts) = pair_val.get("allowed-contexts").and_then(|v| v.as_array()) {
                let mut mask = ContextMask::empty();
                for ctx in contexts {
                    if let Some(ctx_str) = ctx.as_str() {
                        match ctx_str {
                            "code" => mask |= ContextMask::CODE,
                            "string" => mask |= ContextMask::STRING,
                            "comment" => mask |= ContextMask::COMMENT,
                            "regex" => mask |= ContextMask::REGEX,
                            "all" => mask |= ContextMask::ALL,
                            _ => {}
                        }
                    }
                }
                if !mask.is_empty() {
                    bracket_pair = bracket_pair.with_contexts(mask);
                }
            }

            pairs.push(bracket_pair);
        }

        Ok(pairs)
    }

    /// Get the BracketSet for a language, falling back to default if not found.
    pub fn get(&self, language_name: &str) -> &BracketSet {
        self.languages.get(language_name).unwrap_or(&self.default)
    }

    /// Check if a specific language has configuration.
    pub fn has_language(&self, language_name: &str) -> bool {
        self.languages.contains_key(language_name)
    }

    /// Get the default BracketSet.
    pub fn default_set(&self) -> &BracketSet {
        &self.default
    }

    /// Get all configured language names.
    pub fn language_names(&self) -> impl Iterator<Item = &str> {
        self.languages.keys().map(|s| s.as_str())
    }
}

/// Error type for AutoPairsRegistry parsing.
#[derive(Debug, Clone)]
pub struct AutoPairsRegistryError {
    pub message: &'static str,
    pub language: Option<String>,
    pub pair_index: Option<usize>,
}

impl AutoPairsRegistryError {
    fn new(message: &'static str) -> Self {
        Self {
            message,
            language: None,
            pair_index: None,
        }
    }

    fn with_language(mut self, language: &str) -> Self {
        self.language = Some(language.to_string());
        self
    }

    fn with_pair_index(mut self, index: usize) -> Self {
        self.pair_index = Some(index);
        self
    }
}

impl std::fmt::Display for AutoPairsRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid auto-pairs.toml")?;
        if let Some(lang) = &self.language {
            write!(f, " in [{}]", lang)?;
        }
        if let Some(idx) = self.pair_index {
            write!(f, " at pairs[{}]", idx)?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for AutoPairsRegistryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_pair_creation() {
        let pair = BracketPair::new("(", ")");
        assert_eq!(pair.open, "(");
        assert_eq!(pair.close, ")");
        assert_eq!(pair.trigger, "(");
        assert!(!pair.same());
    }

    #[test]
    fn test_bracket_pair_same() {
        let pair = BracketPair::new("\"", "\"");
        assert!(pair.same());

        let pair = BracketPair::new("(", ")");
        assert!(!pair.same());
    }

    #[test]
    fn test_bracket_set_candidates() {
        let pairs = vec![
            BracketPair::new("(", ")"),
            BracketPair::new("{", "}"),
            BracketPair::new("{{", "}}"),
        ];
        let set = BracketSet::new(pairs);

        let candidates: Vec<_> = set.candidates_for_trigger('{').collect();
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn test_bracket_set_max_trigger_len() {
        let pairs = vec![
            BracketPair::new("(", ")"),
            BracketPair::new("```", "```"),
            BracketPair::new("<!--", "-->"),
        ];
        let set = BracketSet::new(pairs);

        assert_eq!(set.max_trigger_len(), 4);
    }

    #[test]
    fn test_context_mask() {
        let mask = ContextMask::CODE | ContextMask::STRING;
        assert!(mask.contains(ContextMask::CODE));
        assert!(mask.contains(ContextMask::STRING));
        assert!(!mask.contains(ContextMask::COMMENT));
    }

    #[test]
    fn test_bracket_kind() {
        let bracket = BracketPair::from(('(', ')'));
        assert_eq!(bracket.kind, BracketKind::Bracket);

        let quote = BracketPair::from(('"', '"'));
        assert_eq!(quote.kind, BracketKind::Quote);

        let delimiter = BracketPair::from(("<!--", "-->"));
        assert_eq!(delimiter.kind, BracketKind::Delimiter);
    }

    #[test]
    fn test_bracket_context_to_mask() {
        assert_eq!(BracketContext::Code.to_mask(), ContextMask::CODE);
        assert_eq!(BracketContext::String.to_mask(), ContextMask::STRING);
        assert_eq!(BracketContext::Comment.to_mask(), ContextMask::COMMENT);
        assert_eq!(BracketContext::Regex.to_mask(), ContextMask::REGEX);
        assert_eq!(BracketContext::Unknown.to_mask(), ContextMask::CODE);
    }

    #[test]
    fn test_bracket_context_allows_pair() {
        let bracket = BracketPair::new("(", ")").with_contexts(ContextMask::CODE);
        let quote =
            BracketPair::new("\"", "\"").with_contexts(ContextMask::CODE | ContextMask::STRING);

        // Bracket only allowed in code
        assert!(bracket
            .allowed_contexts
            .intersects(BracketContext::Code.to_mask()));
        assert!(!bracket
            .allowed_contexts
            .intersects(BracketContext::String.to_mask()));
        assert!(!bracket
            .allowed_contexts
            .intersects(BracketContext::Comment.to_mask()));

        // Quote allowed in code and string
        assert!(quote
            .allowed_contexts
            .intersects(BracketContext::Code.to_mask()));
        assert!(quote
            .allowed_contexts
            .intersects(BracketContext::String.to_mask()));
        assert!(!quote
            .allowed_contexts
            .intersects(BracketContext::Comment.to_mask()));
    }

    #[test]
    fn test_detect_trigger_single_char() {
        let doc = Rope::from("test");
        let pairs = vec![BracketPair::new("(", ")")];
        let set = BracketSet::new(pairs);

        let result = detect_trigger_at(&doc, 4, '(', &set);
        assert!(result.is_some());
        assert_eq!(result.unwrap().open, "(");
    }

    #[test]
    fn test_detect_trigger_multi_char() {
        let doc = Rope::from("test{");
        let pairs = vec![BracketPair::new("{", "}"), BracketPair::new("{%", "%}")];
        let set = BracketSet::new(pairs);

        // After typing single brace
        let result = detect_trigger_at(&doc, 5, '{', &set);
        assert!(result.is_some());
        // Should match single brace since that's what's in the doc
        assert_eq!(result.unwrap().open, "{");

        // Now test with Jinja delimiter
        let doc = Rope::from("test{");
        let result = detect_trigger_at(&doc, 5, '%', &set);
        assert!(result.is_some());
        // Should match Jinja delimiter (longest match)
        assert_eq!(result.unwrap().open, "{%");
    }

    #[test]
    fn test_bracket_set_from_default() {
        let set = BracketSet::from_default_pairs();
        assert_eq!(set.len(), DEFAULT_PAIRS.len());
    }

    #[test]
    fn test_multi_char_pair_builder() {
        let pair = BracketPair::new("{%", "%}")
            .with_kind(BracketKind::Delimiter)
            .with_contexts(ContextMask::CODE)
            .with_surround(false);

        assert_eq!(pair.open, "{%");
        assert_eq!(pair.close, "%}");
        assert_eq!(pair.kind, BracketKind::Delimiter);
        assert_eq!(pair.allowed_contexts, ContextMask::CODE);
        assert!(!pair.surround);
    }

    #[test]
    fn test_hook_multi_single_char_insert() {
        // The hook is called BEFORE the character is inserted.
        // It inserts BOTH the typed char AND the closing char.
        let doc = Rope::from("test\n");
        let selection = Selection::single(4, 5); // cursor at end of "test", before \n
        let set = BracketSet::from_default_pairs();

        let result = hook_multi(&doc, &selection, '(', &set);
        assert!(result.is_some());

        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // hook_multi inserts "(" + ")" at cursor position
        assert_eq!(new_doc.to_string(), "test()\n");
    }

    #[test]
    fn test_hook_multi_jinja_delimiter() {
        // Document has "{" and we're typing "%"
        // The trigger is "{%" so after typing %, we should get "%" + "%}"
        let doc = Rope::from("test{\n");
        let selection = Selection::single(5, 6); // cursor after { (on the \n)

        let pairs = vec![BracketPair::new("{", "}"), BracketPair::new("{%", "%}")];
        let set = BracketSet::new(pairs);

        let result = hook_multi(&doc, &selection, '%', &set);
        assert!(result.is_some());

        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // The hook inserts "%" + "%}" at cursor position 5
        // So: "test{" + "%" + "%}" + "\n" = "test{%%}\n"
        assert_eq!(new_doc.to_string(), "test{%%}\n");
    }

    #[test]
    fn test_hook_multi_skip_over_close() {
        // Test skipping over existing close bracket
        let doc = Rope::from("test()\n");
        let selection = Selection::single(5, 6); // cursor between ( and )

        let set = BracketSet::from_default_pairs();

        let result = hook_multi(&doc, &selection, ')', &set);
        assert!(result.is_some());

        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // Document should be unchanged (just cursor movement)
        assert_eq!(new_doc.to_string(), "test()\n");
    }

    #[test]
    fn test_hook_multi_symmetric_quote_not_after_alpha() {
        // Quotes shouldn't auto-pair after alphanumeric - just insert the quote
        let doc = Rope::from("test\n");
        let selection = Selection::single(4, 5); // cursor right after "test"

        let set = BracketSet::from_default_pairs();

        // After an alphanumeric, quotes should NOT auto-pair, just insert single quote
        let result = hook_multi(&doc, &selection, '"', &set);
        assert!(result.is_some());

        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // Only the typed quote is inserted, not a pair
        assert_eq!(new_doc.to_string(), "test\"\n");
    }

    #[test]
    fn test_hook_multi_symmetric_quote_after_space() {
        // Quotes should auto-pair after space
        let doc = Rope::from("test \n");
        let selection = Selection::single(5, 6); // cursor after space

        let set = BracketSet::from_default_pairs();

        let result = hook_multi(&doc, &selection, '"', &set);
        assert!(result.is_some());

        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // Should have inserted " + "
        assert_eq!(new_doc.to_string(), "test \"\"\n");
    }

    #[test]
    fn test_hook_multi_no_match() {
        let doc = Rope::from("test\n");
        let selection = Selection::single(4, 5);

        let set = BracketSet::from_default_pairs();

        // 'x' is not a trigger for any pair
        let result = hook_multi(&doc, &selection, 'x', &set);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_trigger_considers_position() {
        // When cursor is at position 5 and we type %,
        // we need to look at chars before position 5
        let doc = Rope::from("test{");
        let set = BracketSet::new(vec![
            BracketPair::new("{", "}"),
            BracketPair::new("{%", "%}"),
        ]);

        // Cursor is at 5, we're typing '%'
        // Characters before cursor are "test{"
        // The trigger "{%" should match "{" + "%" (last typed)
        let result = detect_trigger_at(&doc, 5, '%', &set);
        assert!(result.is_some());
        assert_eq!(result.unwrap().open, "{%");
    }

    #[test]
    fn test_hook_multi_replaces_prefix_close() {
        // Scenario: User has both { → } and {% → %} pairs configured.
        // They type "{" which auto-pairs to "{|}" (cursor at |).
        // Then they type "%" - we should replace "}" with "%}" to get "{%|%}"
        let doc = Rope::from("test{}\n");
        let selection = Selection::single(5, 6); // cursor between { and }

        let pairs = vec![BracketPair::new("{", "}"), BracketPair::new("{%", "%}")];
        let set = BracketSet::new(pairs);

        let result = hook_multi(&doc, &selection, '%', &set);
        assert!(result.is_some());

        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // "{}" with cursor at 5 (between { and }) → type "%" → "{%%}"
        assert_eq!(new_doc.to_string(), "test{%%}\n");
    }

    #[test]
    fn test_hook_multi_replaces_prefix_close_html_comment() {
        // Scenario: User has both < → > and <!-- → --> pairs configured.
        // They type "<" which auto-pairs to "<|>" (cursor at |).
        // Then they type "!--" - we should replace ">" with "-->" to get "<!--|-->"

        // Start with document that has "<>" already (from previous auto-pair)
        // and simulate document states as if the user typed "!--"

        // Document after typing "<!-" manually (no triggers matched for ! or first -)
        // "test<!->\n" with positions: t(0) e(1) s(2) t(3) <(4) !(5) -(6) >(7) \n(8)
        let doc = Rope::from("test<!->\n");
        let selection = Selection::single(7, 8); // cursor at position 7 (the ">")

        let pairs = vec![BracketPair::new("<", ">"), BracketPair::new("<!--", "-->")];
        let set = BracketSet::new(pairs);

        // Type "-" after "<!-": this completes "<!--" trigger
        let result = hook_multi(&doc, &selection, '-', &set);
        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // Should replace ">" with "-->" to get "<!---->"
        // Result: "test<!---->\n" with positions: t(0) e(1) s(2) t(3) <(4) !(5) -(6) -(7) -(8) -(9) >(10) \n(11)
        assert_eq!(new_doc.to_string(), "test<!---->\n");

        // Verify cursor position: should be at position 8 (between "<!--" and "-->")
        let new_selection = transaction.selection().unwrap();
        let cursor = new_selection.primary().cursor(new_doc.slice(..));
        assert_eq!(cursor, 8, "cursor should be between <!-- and -->");
    }

    #[test]
    fn test_detect_pair_for_deletion_single_char() {
        let doc = Rope::from("test{}\n");
        let set = BracketSet::from_default_pairs();

        // Cursor at position 5 (between { and })
        let result = detect_pair_for_deletion(&doc, 5, &set);
        assert!(result.is_some());
        let del = result.unwrap();
        assert_eq!(del.delete_before, 1);
        assert_eq!(del.delete_after, 1);
    }

    #[test]
    fn test_detect_pair_for_deletion_multi_char() {
        // String: "test{%%}\n"
        // Positions: t(0) e(1) s(2) t(3) {(4) %(5) %(6) }(7) \n(8)
        // Cursor at position 6 means we're between "{%" and "%}"
        let doc = Rope::from("test{%%}\n");
        let pairs = vec![BracketPair::new("{", "}"), BracketPair::new("{%", "%}")];
        let set = BracketSet::new(pairs);

        // Cursor at position 6 (between {% and %})
        let result = detect_pair_for_deletion(&doc, 6, &set);
        assert!(result.is_some());
        let del = result.unwrap();
        assert_eq!(del.delete_before, 2);
        assert_eq!(del.delete_after, 2);
    }

    #[test]
    fn test_detect_pair_for_deletion_prefers_longer_match() {
        // When both { and {% could match, prefer the longer one
        let doc = Rope::from("{%%}\n");
        let pairs = vec![BracketPair::new("{", "}"), BracketPair::new("{%", "%}")];
        let set = BracketSet::new(pairs);

        // Cursor at position 2 (between {% and %})
        let result = detect_pair_for_deletion(&doc, 2, &set);
        assert!(result.is_some());
        let del = result.unwrap();
        assert_eq!(del.delete_before, 2);
        assert_eq!(del.delete_after, 2);
    }

    #[test]
    fn test_detect_pair_for_deletion_no_match() {
        let doc = Rope::from("test\n");
        let set = BracketSet::from_default_pairs();

        // Cursor at position 2, no brackets around
        let result = detect_pair_for_deletion(&doc, 2, &set);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_pair_for_deletion_at_start() {
        let doc = Rope::from("{}\n");
        let set = BracketSet::from_default_pairs();

        // Cursor at position 0, can't delete before
        let result = detect_pair_for_deletion(&doc, 0, &set);
        assert!(result.is_none());
    }

    #[test]
    fn test_context_allows_pair_code() {
        let pair = BracketPair::new("(", ")").with_contexts(ContextMask::CODE);

        // CODE context should allow CODE-only pair
        assert!(context_allows_pair(BracketContext::Code, &pair));
        // STRING context should NOT allow CODE-only pair
        assert!(!context_allows_pair(BracketContext::String, &pair));
        // COMMENT context should NOT allow CODE-only pair
        assert!(!context_allows_pair(BracketContext::Comment, &pair));
        // Unknown falls back to CODE
        assert!(context_allows_pair(BracketContext::Unknown, &pair));
    }

    #[test]
    fn test_context_allows_pair_multi_context() {
        let pair =
            BracketPair::new("\"", "\"").with_contexts(ContextMask::CODE | ContextMask::STRING);

        assert!(context_allows_pair(BracketContext::Code, &pair));
        assert!(context_allows_pair(BracketContext::String, &pair));
        assert!(!context_allows_pair(BracketContext::Comment, &pair));
    }

    #[test]
    fn test_context_allows_pair_all_contexts() {
        let pair = BracketPair::new("(", ")").with_contexts(ContextMask::ALL);

        assert!(context_allows_pair(BracketContext::Code, &pair));
        assert!(context_allows_pair(BracketContext::String, &pair));
        assert!(context_allows_pair(BracketContext::Comment, &pair));
        assert!(context_allows_pair(BracketContext::Regex, &pair));
    }

    #[test]
    fn test_auto_pair_state_creation() {
        let doc = Rope::from("test\n");
        let selection = Selection::single(4, 5);
        let pairs = BracketSet::from_default_pairs();

        let state = AutoPairState::new(&doc, &selection, &pairs);
        assert!(state.contexts.is_none());

        let contexts = vec![BracketContext::Code];
        let state_with_ctx = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        assert!(state_with_ctx.contexts.is_some());
    }

    #[test]
    fn test_hook_with_context_in_code() {
        let doc = Rope::from("test\n");
        let selection = Selection::single(4, 5);
        let pairs = BracketSet::from_default_pairs();
        let contexts = vec![BracketContext::Code];

        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '(');

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));
        assert_eq!(new_doc.to_string(), "test()\n");
    }

    #[test]
    fn test_hook_with_context_blocked_in_string() {
        let doc = Rope::from("test\n");
        let selection = Selection::single(4, 5);
        // Bracket only allowed in CODE context
        let pairs = BracketSet::new(vec![
            BracketPair::new("(", ")").with_contexts(ContextMask::CODE)
        ]);
        // But we're in a STRING context
        let contexts = vec![BracketContext::String];

        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '(');

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));
        // Should only insert the typed char, NOT the pair
        assert_eq!(new_doc.to_string(), "test(\n");
    }

    #[test]
    fn test_hook_with_context_allowed_in_string() {
        // Use a space before cursor so quotes will pair (not after alphanumeric)
        let doc = Rope::from("test \n");
        let selection = Selection::single(5, 6);
        // Quote allowed in CODE and STRING contexts
        let pairs = BracketSet::new(vec![BracketPair::new("'", "'")
            .with_kind(BracketKind::Quote)
            .with_contexts(ContextMask::CODE | ContextMask::STRING)]);
        // We're in a STRING context
        let contexts = vec![BracketContext::String];

        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '\'');

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));
        // Should insert the pair since quotes are allowed in strings
        assert_eq!(new_doc.to_string(), "test ''\n");
    }

    #[test]
    fn test_hook_with_context_no_context_defaults_to_code() {
        let doc = Rope::from("test\n");
        let selection = Selection::single(4, 5);
        let pairs = BracketSet::new(vec![
            BracketPair::new("(", ")").with_contexts(ContextMask::CODE)
        ]);

        // No contexts provided - should default to CODE
        let state = AutoPairState::new(&doc, &selection, &pairs);
        let result = hook_with_context(&state, '(');

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));
        assert_eq!(new_doc.to_string(), "test()\n");
    }

    #[test]
    fn test_hook_with_context_multi_cursor_different_contexts() {
        // Original: "code \"str\" code\n"
        // Positions: c(0) o(1) d(2) e(3) " "(4) \"(5) s(6) t(7) r(8) \"(9) ...
        let doc = Rope::from("code \"str\" code\n");
        // Two cursors: one in code (pos 4 = space), one in string (pos 7 = 't')
        let selection = Selection::new(smallvec::smallvec![Range::new(4, 5), Range::new(7, 8)], 0);
        let pairs = BracketSet::new(vec![
            BracketPair::new("(", ")").with_contexts(ContextMask::CODE)
        ]);
        // First cursor in CODE, second in STRING
        let contexts = vec![BracketContext::Code, BracketContext::String];

        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '(');

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));
        // First cursor (pos 4) gets pair "()" in CODE context
        // Second cursor (pos 7) gets just "(" in STRING context (pair not allowed)
        // After first insert at pos 4: "code() \"str\" code\n" (inserted 2 chars)
        // After second insert at pos 7+2=9: "code() \"s(tr\" code\n" (inserted 1 char)
        assert_eq!(new_doc.to_string(), "code() \"s(tr\" code\n");
    }

    #[test]
    fn test_surround_pairs_iterator() {
        let pairs = vec![
            BracketPair::new("(", ")").with_surround(true),
            BracketPair::new("{%", "%}").with_surround(false),
            BracketPair::new("[", "]").with_surround(true),
        ];
        let set = BracketSet::new(pairs);

        let surround: Vec<_> = set.surround_pairs().collect();
        assert_eq!(surround.len(), 2);
        assert_eq!(surround[0].open, "(");
        assert_eq!(surround[1].open, "[");
    }

    #[test]
    fn test_get_pair_by_char_open() {
        let set = BracketSet::from_default_pairs();

        // Look up by open char
        let result = set.get_pair_by_char('(');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, "(");
        assert_eq!(close, ")");
    }

    #[test]
    fn test_get_pair_by_char_close() {
        let set = BracketSet::from_default_pairs();

        // Look up by close char
        let result = set.get_pair_by_char(')');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, "(");
        assert_eq!(close, ")");
    }

    #[test]
    fn test_get_pair_by_char_symmetric() {
        let set = BracketSet::from_default_pairs();

        // Symmetric pairs (quotes)
        let result = set.get_pair_by_char('"');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, "\"");
        assert_eq!(close, "\"");
    }

    #[test]
    fn test_get_pair_by_char_not_found() {
        let set = BracketSet::from_default_pairs();

        // Character not in any pair
        let result = set.get_pair_by_char('x');
        assert!(result.is_none());
    }

    #[test]
    fn test_get_pair_by_char_only_surround_pairs() {
        let pairs = vec![
            BracketPair::new("(", ")").with_surround(true),
            BracketPair::new("{%", "%}").with_surround(false),
        ];
        let set = BracketSet::new(pairs);

        // ( is a surround pair
        let result = set.get_pair_by_char('(');
        assert!(result.is_some());

        // { starts the non-surround pair, should not be found
        let result = set.get_pair_by_char('{');
        assert!(result.is_none());
    }

    #[test]
    fn test_get_pair_by_char_multi_char() {
        let pairs = vec![
            BracketPair::new("{", "}").with_surround(true),
            BracketPair::new("{%", "%}").with_surround(true),
        ];
        let set = BracketSet::new(pairs);

        // { should match single-char pair
        let result = set.get_pair_by_char('{');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, "{");
        assert_eq!(close, "}");
    }

    #[test]
    fn test_get_surround_strings_fallback() {
        let set = BracketSet::from_default_pairs();

        // Known pair
        let (open, close) = set.get_surround_strings('(');
        assert_eq!(open, "(");
        assert_eq!(close, ")");

        // Unknown char - falls back to same char
        let (open, close) = set.get_surround_strings('x');
        assert_eq!(open, "x");
        assert_eq!(close, "x");
    }

    // =========================================================================
    // Tests for IntelliJ-style quote handling
    // =========================================================================

    #[test]
    fn test_is_escaped_by_backslash_no_backslash() {
        let doc = Rope::from("hello\"world");
        // Position 5 is after 'hello', no backslashes before
        assert!(!is_escaped_by_backslash(&doc, 5));
    }

    #[test]
    fn test_is_escaped_by_backslash_single() {
        let doc = Rope::from("hello\\\"world");
        // Position 6 is after the backslash, should be escaped
        assert!(is_escaped_by_backslash(&doc, 6));
    }

    #[test]
    fn test_is_escaped_by_backslash_double() {
        let doc = Rope::from("hello\\\\\"world");
        // Position 7 is after two backslashes, should NOT be escaped (backslashes cancel)
        assert!(!is_escaped_by_backslash(&doc, 7));
    }

    #[test]
    fn test_is_escaped_by_backslash_triple() {
        let doc = Rope::from("hello\\\\\\\"world");
        // Position 8 is after three backslashes, should be escaped (odd count)
        assert!(is_escaped_by_backslash(&doc, 8));
    }

    #[test]
    fn test_is_escaped_by_backslash_at_start() {
        let doc = Rope::from("\"hello");
        // Position 0, nothing before
        assert!(!is_escaped_by_backslash(&doc, 0));
    }

    #[test]
    fn test_line_bounds_single_line() {
        let doc = Rope::from("hello world");
        let (start, end) = line_bounds(&doc, 5);
        assert_eq!(start, 0);
        assert_eq!(end, 11); // length of "hello world"
    }

    #[test]
    fn test_line_bounds_multi_line() {
        let doc = Rope::from("line1\nline2\nline3");
        // Position in "line2"
        let (start, end) = line_bounds(&doc, 8);
        assert_eq!(start, 6); // "line1\n" = 6 chars
        assert_eq!(end, 12); // "line1\nline2\n" = 12 chars
    }

    #[test]
    fn test_line_bounds_last_line() {
        let doc = Rope::from("line1\nline2");
        // Position in "line2"
        let (start, end) = line_bounds(&doc, 8);
        assert_eq!(start, 6);
        assert_eq!(end, 11); // end of document
    }

    #[test]
    fn test_has_non_closed_literal_closed_string() {
        // String with both opening and closing quote on same line
        let doc = Rope::from("\"hello world\"\n");
        // Cursor is inside the string, after 'hello'
        assert!(!has_non_closed_literal_on_line(&doc, 6, '"'));
    }

    #[test]
    fn test_has_non_closed_literal_unclosed_string() {
        // String with only opening quote
        let doc = Rope::from("\"hello world\n");
        // Cursor is inside the string
        assert!(has_non_closed_literal_on_line(&doc, 6, '"'));
    }

    #[test]
    fn test_has_non_closed_literal_at_start() {
        // At start of line, not inside a string
        let doc = Rope::from("hello\n");
        assert!(!has_non_closed_literal_on_line(&doc, 0, '"'));
    }

    #[test]
    fn test_has_non_closed_literal_escaped_quote() {
        // String with escaped quote followed by real close
        let doc = Rope::from("\"hello\\\"world\"\n");
        // The \" at position 7 is escaped, so the literal should be closed
        assert!(!has_non_closed_literal_on_line(&doc, 3, '"'));
    }

    #[test]
    fn test_has_non_closed_literal_escaped_quote_unclosed() {
        // String with escaped quote but no real close
        let doc = Rope::from("\"hello\\\"world\n");
        // The \" at position 7 is escaped, so the literal is NOT closed
        assert!(has_non_closed_literal_on_line(&doc, 3, '"'));
    }

    #[test]
    fn test_get_line_indent_no_indent() {
        let doc = Rope::from("hello world");
        let indent = get_line_indent(&doc, 5);
        assert_eq!(indent, "");
    }

    #[test]
    fn test_get_line_indent_spaces() {
        let doc = Rope::from("    hello world");
        let indent = get_line_indent(&doc, 8);
        assert_eq!(indent, "    ");
    }

    #[test]
    fn test_get_line_indent_tabs() {
        let doc = Rope::from("\t\thello world");
        let indent = get_line_indent(&doc, 5);
        assert_eq!(indent, "\t\t");
    }

    #[test]
    fn test_get_line_indent_mixed() {
        let doc = Rope::from("  \t hello world");
        let indent = get_line_indent(&doc, 8);
        assert_eq!(indent, "  \t ");
    }

    #[test]
    fn test_escape_check_in_hook_with_context() {
        // When typing a quote after a backslash, it should NOT auto-pair
        let doc = Rope::from("test\\\n");
        let selection = Selection::single(5, 6); // cursor after backslash
        let set = BracketSet::from_default_pairs();
        let contexts = vec![BracketContext::String];

        let state = AutoPairState::with_contexts(&doc, &selection, &set, &contexts);
        let result = hook_with_context(&state, '"');

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // Should insert only a single quote, not a pair
        assert_eq!(new_doc.to_string(), "test\\\"\n");
    }

    #[test]
    fn test_no_escape_check_double_backslash() {
        // When typing a quote after two backslashes, it SHOULD auto-pair
        // because the backslashes cancel out
        let doc = Rope::from("test\\\\\n");
        let selection = Selection::single(6, 7); // cursor after two backslashes
        let set = BracketSet::from_default_pairs();
        let contexts = vec![BracketContext::Code];

        let state = AutoPairState::with_contexts(&doc, &selection, &set, &contexts);
        let result = hook_with_context(&state, '"');

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));

        // Should insert a quote pair since it's not escaped
        assert_eq!(new_doc.to_string(), "test\\\\\"\"\n");
    }

    // =========================================================================
    // Tests for overlapping " and """ pairs
    // =========================================================================

    fn create_overlapping_quote_pairs() -> BracketSet {
        BracketSet::new(vec![
            BracketPair::new("\"", "\"")
                .with_kind(BracketKind::Quote)
                .with_contexts(ContextMask::CODE | ContextMask::STRING),
            BracketPair::new("\"\"\"", "\"\"\"")
                .with_kind(BracketKind::Quote)
                .with_contexts(ContextMask::CODE | ContextMask::STRING),
        ])
    }

    #[test]
    fn test_overlapping_quotes_step1_first_quote() {
        // Step 1: | -> "|" (insert single quote pair)
        let doc = Rope::from("\n");
        let selection = Selection::single(0, 1);
        let pairs = create_overlapping_quote_pairs();

        let state = AutoPairState::new(&doc, &selection, &pairs);
        let result = hook_multi(&doc, &selection, '"', &pairs);

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        assert!(transaction.apply(&mut new_doc));
        assert_eq!(new_doc.to_string(), "\"\"\n");
    }

    #[test]
    fn test_overlapping_quotes_step2_second_quote() {
        // Step 2: "|" -> ""| (skip over closing quote)
        let doc = Rope::from("\"\"\n");
        let selection = Selection::single(1, 2); // cursor between quotes
        let pairs = create_overlapping_quote_pairs();

        let result = hook_multi(&doc, &selection, '"', &pairs);

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        let new_sel = transaction.selection().unwrap();
        assert!(transaction.apply(&mut new_doc));
        // Document should remain the same (we just skipped)
        assert_eq!(new_doc.to_string(), "\"\"\n");
        // Cursor should now be after the closing quote
        assert_eq!(new_sel.primary().cursor(new_doc.slice(..)), 2);
    }

    #[test]
    fn test_overlapping_quotes_step3_third_quote() {
        // Step 3: ""| -> """|""" (upgrade to triple quote)
        let doc = Rope::from("\"\"\n");
        let selection = Selection::single(2, 3); // cursor after ""
        let pairs = create_overlapping_quote_pairs();

        let result = hook_multi(&doc, &selection, '"', &pairs);

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        let new_sel = transaction.selection().unwrap();
        assert!(transaction.apply(&mut new_doc));
        // Document should now have 6 quotes
        assert_eq!(new_doc.to_string(), "\"\"\"\"\"\"\n");
        // Cursor should be after the opening triple quote (position 3)
        assert_eq!(new_sel.primary().cursor(new_doc.slice(..)), 3);
    }

    #[test]
    fn test_overlapping_quotes_step4_fourth_quote() {
        // Step 4: """|""" -> """"|"" (skip one closing quote)
        let doc = Rope::from("\"\"\"\"\"\"\n");
        let selection = Selection::single(3, 4); // cursor after opening """
        let pairs = create_overlapping_quote_pairs();

        let result = hook_multi(&doc, &selection, '"', &pairs);

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        let new_sel = transaction.selection().unwrap();
        assert!(transaction.apply(&mut new_doc));
        // Document should remain the same (we just skipped)
        assert_eq!(new_doc.to_string(), "\"\"\"\"\"\"\n");
        // Cursor should now be at position 4
        assert_eq!(new_sel.primary().cursor(new_doc.slice(..)), 4);
    }

    #[test]
    fn test_overlapping_quotes_step5_fifth_quote() {
        // Step 5: """"|"" -> """""|" (skip another closing quote)
        let doc = Rope::from("\"\"\"\"\"\"\n");
        let selection = Selection::single(4, 5); // cursor after 4 quotes
        let pairs = create_overlapping_quote_pairs();

        let result = hook_multi(&doc, &selection, '"', &pairs);

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        let new_sel = transaction.selection().unwrap();
        assert!(transaction.apply(&mut new_doc));
        // Document should remain the same (we just skipped)
        assert_eq!(new_doc.to_string(), "\"\"\"\"\"\"\n");
        // Cursor should now be at position 5
        assert_eq!(new_sel.primary().cursor(new_doc.slice(..)), 5);
    }

    #[test]
    fn test_overlapping_quotes_step6_sixth_quote() {
        // Step 6: """""|" -> """"""| (skip final closing quote)
        let doc = Rope::from("\"\"\"\"\"\"\n");
        let selection = Selection::single(5, 6); // cursor after 5 quotes
        let pairs = create_overlapping_quote_pairs();

        let result = hook_multi(&doc, &selection, '"', &pairs);

        assert!(result.is_some());
        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        let new_sel = transaction.selection().unwrap();
        assert!(transaction.apply(&mut new_doc));
        // Document should remain the same (we just skipped)
        assert_eq!(new_doc.to_string(), "\"\"\"\"\"\"\n");
        // Cursor should now be at position 6 (after all quotes)
        assert_eq!(new_sel.primary().cursor(new_doc.slice(..)), 6);
    }

    #[test]
    fn test_overlapping_quotes_full_sequence() {
        // Test the full sequence from empty to """"""
        let pairs = create_overlapping_quote_pairs();
        let mut doc = Rope::from("\n");
        let mut cursor_pos = 0;

        // Step 1: | -> "|"
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let result = hook_multi(&doc, &selection, '"', &pairs).unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\n");
        assert_eq!(cursor_pos, 1);

        // Step 2: "|" -> ""|
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let result = hook_multi(&doc, &selection, '"', &pairs).unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\n");
        assert_eq!(cursor_pos, 2);

        // Step 3: ""| -> """|"""
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let result = hook_multi(&doc, &selection, '"', &pairs).unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 3);

        // Step 4: """|""" -> """"|""
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let result = hook_multi(&doc, &selection, '"', &pairs).unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 4);

        // Step 5: """"|"" -> """""|"
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let result = hook_multi(&doc, &selection, '"', &pairs).unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 5);

        // Step 6: """""|" -> """"""|
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let result = hook_multi(&doc, &selection, '"', &pairs).unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 6);
    }

    #[test]
    fn test_detect_trigger_filters_multi_char_symmetric_when_same_char_ahead() {
        // When cursor is before a quote, multi-char symmetric pair """ should be filtered
        let doc = Rope::from("\"\"\"\n");
        let pairs = create_overlapping_quote_pairs();

        // Cursor at position 3 (right before a quote at position 3? No, at newline)
        // Let's test at position 0 where next char is "
        let result = detect_trigger_at(&doc, 0, '"', &pairs);
        assert!(result.is_some());
        // Should pick single quote pair since next char is "
        assert_eq!(result.unwrap().trigger, "\"");
    }

    #[test]
    fn test_detect_trigger_picks_triple_quote_when_no_quote_ahead() {
        // When cursor is NOT before a quote, triple quote should be selected
        let doc = Rope::from("\"\"\n");
        let pairs = create_overlapping_quote_pairs();

        // After typing "" and about to type third ", cursor at 2, next char is \n
        let result = detect_trigger_at(&doc, 2, '"', &pairs);
        assert!(result.is_some());
        // Should pick triple quote pair since next char is not "
        assert_eq!(result.unwrap().trigger, "\"\"\"");
    }

    #[test]
    fn test_detect_trigger_step3_exactly() {
        // Exact state at step 3: doc is "" (just two quotes), cursor at 2
        let doc = Rope::from("\"\"");
        let pairs = create_overlapping_quote_pairs();

        // At position 2, there's no character (end of doc)
        assert_eq!(doc.len_chars(), 2);
        assert_eq!(doc.get_char(2), None);

        let result = detect_trigger_at(&doc, 2, '"', &pairs);
        assert!(result.is_some(), "Should detect a trigger");
        assert_eq!(
            result.unwrap().trigger,
            "\"\"\"",
            "Should pick triple quote when no quote ahead"
        );
    }

    #[test]
    fn test_overlapping_quotes_step7_seventh_quote() {
        // Step 7: """"""| -> """""""| (just insert a plain quote)
        // After 6 quotes, no pair should match because we have more
        // consecutive quotes than the trigger length. The hook returns None,
        // which tells the editor to fall back to plain character insertion.
        let doc = Rope::from("\"\"\"\"\"\"\n");
        let selection = Selection::single(6, 7); // cursor after 6 quotes
        let pairs = create_overlapping_quote_pairs();

        let result = hook_multi(&doc, &selection, '"', &pairs);

        // Should return None - no auto-pair action, let editor insert plain char
        assert!(
            result.is_none(),
            "After complete triple-quote pair, no auto-pairing should occur"
        );
    }

    #[test]
    fn test_step3_full_hook_simulation() {
        // Simulate exactly what happens at step 3
        let pairs = create_overlapping_quote_pairs();

        // After step 2, doc is "" and cursor is at position 2
        let doc = Rope::from("\"\"");
        let selection = Selection::single(2, 2); // Point cursor at end

        let result = hook_multi(&doc, &selection, '"', &pairs);
        assert!(result.is_some(), "Hook should return a transaction");

        let transaction = result.unwrap();
        let mut new_doc = doc.clone();
        let new_sel = transaction.selection().unwrap();
        assert!(transaction.apply(&mut new_doc));

        // Should produce 6 quotes with cursor at position 3
        assert_eq!(
            new_doc.to_string(),
            "\"\"\"\"\"\"",
            "Should upgrade to triple quotes"
        );
        assert_eq!(new_sel.primary().cursor(new_doc.slice(..)), 3);
    }

    #[test]
    fn test_overlapping_quotes_with_context_full_sequence() {
        // Test the full sequence using hook_with_context (what the real editor uses)
        let pairs = create_overlapping_quote_pairs();
        let mut doc = Rope::from("\n");
        let mut cursor_pos = 0;

        // Step 1: | -> "|"
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let contexts = vec![BracketContext::Code];
        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '"').unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\n");
        assert_eq!(cursor_pos, 1);

        // Step 2: "|" -> ""|
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let contexts = vec![BracketContext::String]; // Now we're inside a string
        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '"').unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\n");
        assert_eq!(cursor_pos, 2);

        // Step 3: ""| -> """|"""
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let contexts = vec![BracketContext::Code]; // Back in code context
        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '"').unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 3);

        // Step 4: """|""" -> """"|""
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let contexts = vec![BracketContext::String]; // Inside triple-quoted string
        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '"').unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 4);

        // Step 5: """"|"" -> """""|"
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let contexts = vec![BracketContext::String];
        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '"').unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 5);

        // Step 6: """""|" -> """"""|
        let selection = Selection::single(cursor_pos, cursor_pos + 1);
        let contexts = vec![BracketContext::String];
        let state = AutoPairState::with_contexts(&doc, &selection, &pairs, &contexts);
        let result = hook_with_context(&state, '"').unwrap();
        let new_sel = result.selection().unwrap();
        result.apply(&mut doc);
        cursor_pos = new_sel.primary().cursor(doc.slice(..));
        assert_eq!(doc.to_string(), "\"\"\"\"\"\"\n");
        assert_eq!(cursor_pos, 6);
    }

}
