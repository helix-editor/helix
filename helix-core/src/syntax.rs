pub mod config;

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt, iter,
    ops::{self, RangeBounds},
    path::Path,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use arc_swap::{ArcSwap, Guard};
use config::{Configuration, FileType, LanguageConfiguration, LanguageServerConfiguration};
use helix_loader::grammar::get_language;
use helix_stdx::rope::RopeSliceExt as _;
use once_cell::sync::OnceCell;
use ropey::RopeSlice;
use tree_house::{
    highlighter,
    query_iter::QueryIter,
    tree_sitter::{Grammar, InactiveQueryCursor, InputEdit, Node, Query, RopeInput, Tree},
    Error, InjectionLanguageMarker, LanguageConfig as SyntaxConfig, Layer,
};

use crate::{indent::IndentQuery, tree_sitter, ChangeSet, Language};

pub use tree_house::{
    highlighter::{Highlight, HighlightEvent},
    Error as HighlighterError, LanguageLoader, TreeCursor, TREE_SITTER_MATCH_LIMIT,
};

#[derive(Debug)]
pub struct LanguageData {
    config: Arc<LanguageConfiguration>,
    syntax: OnceCell<Option<SyntaxConfig>>,
    indent_query: OnceCell<Option<IndentQuery>>,
    textobject_query: OnceCell<Option<TextObjectQuery>>,
}

impl Clone for LanguageData {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            syntax: OnceCell::new(),
            indent_query: OnceCell::new(),
            textobject_query: OnceCell::new(),
        }
    }
}

impl LanguageData {
    fn new(config: LanguageConfiguration) -> Self {
        Self {
            config: Arc::new(config),
            syntax: OnceCell::new(),
            indent_query: OnceCell::new(),
            textobject_query: OnceCell::new(),
        }
    }

    pub fn config(&self) -> &Arc<LanguageConfiguration> {
        &self.config
    }

    /// Loads the grammar and compiles the highlights, injections and locals for the language.
    /// This function should only be used by this module or the xtask crate.
    pub fn compile_syntax_config(
        config: &LanguageConfiguration,
        loader: &Loader,
    ) -> Result<Option<SyntaxConfig>> {
        let name = &config.language_id;
        let parser_name = config.grammar.as_deref().unwrap_or(name);
        let Some(grammar) = get_language(parser_name)? else {
            log::info!("Skipping syntax config for '{name}' because the parser's shared library does not exist");
            return Ok(None);
        };
        let highlight_query_text = read_query(name, "highlights.scm");
        let injection_query_text = read_query(name, "injections.scm");
        let local_query_text = read_query(name, "locals.scm");
        let config = SyntaxConfig::new(
            grammar,
            &highlight_query_text,
            &injection_query_text,
            &local_query_text,
        )
        .with_context(|| format!("Failed to compile highlights for '{name}'"))?;

        reconfigure_highlights(&config, &loader.scopes());

        Ok(Some(config))
    }

    fn syntax_config(&self, loader: &Loader) -> Option<&SyntaxConfig> {
        self.syntax
            .get_or_init(|| {
                Self::compile_syntax_config(&self.config, loader)
                    .map_err(|err| {
                        log::error!("{err:#}");
                    })
                    .ok()
                    .flatten()
            })
            .as_ref()
    }

    /// Compiles the indents.scm query for a language.
    /// This function should only be used by this module or the xtask crate.
    pub fn compile_indent_query(
        grammar: Grammar,
        config: &LanguageConfiguration,
    ) -> Result<Option<IndentQuery>> {
        let name = &config.language_id;
        let text = read_query(name, "indents.scm");
        if text.is_empty() {
            return Ok(None);
        }
        let indent_query = IndentQuery::new(grammar, &text)
            .with_context(|| format!("Failed to compile indents.scm query for '{name}'"))?;
        Ok(Some(indent_query))
    }

    fn indent_query(&self, loader: &Loader) -> Option<&IndentQuery> {
        self.indent_query
            .get_or_init(|| {
                let grammar = self.syntax_config(loader)?.grammar;
                Self::compile_indent_query(grammar, &self.config)
                    .map_err(|err| {
                        log::error!("{err}");
                    })
                    .ok()
                    .flatten()
            })
            .as_ref()
    }

    /// Compiles the textobjects.scm query for a language.
    /// This function should only be used by this module or the xtask crate.
    pub fn compile_textobject_query(
        grammar: Grammar,
        config: &LanguageConfiguration,
    ) -> Result<Option<TextObjectQuery>> {
        let name = &config.language_id;
        let text = read_query(name, "textobjects.scm");
        if text.is_empty() {
            return Ok(None);
        }
        let query = Query::new(grammar, &text, |_, _| Ok(()))
            .with_context(|| format!("Failed to compile textobjects.scm queries for '{name}'"))?;
        Ok(Some(TextObjectQuery::new(query)))
    }

    fn textobject_query(&self, loader: &Loader) -> Option<&TextObjectQuery> {
        self.textobject_query
            .get_or_init(|| {
                let grammar = self.syntax_config(loader)?.grammar;
                Self::compile_textobject_query(grammar, &self.config)
                    .map_err(|err| {
                        log::error!("{err}");
                    })
                    .ok()
                    .flatten()
            })
            .as_ref()
    }

    fn reconfigure(&self, scopes: &[String]) {
        if let Some(Some(config)) = self.syntax.get() {
            reconfigure_highlights(config, scopes);
        }
    }
}

fn reconfigure_highlights(config: &SyntaxConfig, recognized_names: &[String]) {
    config.configure(move |capture_name| {
        let capture_parts: Vec<_> = capture_name.split('.').collect();

        let mut best_index = None;
        let mut best_match_len = 0;
        for (i, recognized_name) in recognized_names.iter().enumerate() {
            let mut len = 0;
            let mut matches = true;
            for (i, part) in recognized_name.split('.').enumerate() {
                match capture_parts.get(i) {
                    Some(capture_part) if *capture_part == part => len += 1,
                    _ => {
                        matches = false;
                        break;
                    }
                }
            }
            if matches && len > best_match_len {
                best_index = Some(i);
                best_match_len = len;
            }
        }
        best_index.map(|idx| Highlight::new(idx as u32))
    });
}

pub fn read_query(lang: &str, query_filename: &str) -> String {
    tree_house::read_query(lang, |language| {
        helix_loader::grammar::load_runtime_file(language, query_filename).unwrap_or_default()
    })
}

#[derive(Debug, Default, Clone)]
pub struct Loader {
    languages: Vec<LanguageData>,
    languages_by_extension: HashMap<String, Language>,
    languages_by_shebang: HashMap<String, Language>,
    languages_glob_matcher: FileTypeGlobMatcher,
    language_server_configs: HashMap<String, LanguageServerConfiguration>,
    scopes: Arc<ArcSwap<Vec<String>>>,
}

pub type LoaderError = globset::Error;

impl Loader {
    pub fn new(config: Configuration) -> Result<Self, LoaderError> {
        let mut languages = Vec::with_capacity(config.language.len());
        let mut languages_by_extension = HashMap::new();
        let mut languages_by_shebang = HashMap::new();
        let mut file_type_globs = Vec::new();

        for mut config in config.language {
            let language = Language(languages.len() as u32);
            config.language = Some(language);

            for file_type in &config.file_types {
                match file_type {
                    FileType::Extension(extension) => {
                        languages_by_extension.insert(extension.clone(), language);
                    }
                    FileType::Glob(glob) => {
                        file_type_globs.push(FileTypeGlob::new(glob.to_owned(), language));
                    }
                };
            }
            for shebang in &config.shebangs {
                languages_by_shebang.insert(shebang.clone(), language);
            }

            languages.push(LanguageData::new(config));
        }

        Ok(Self {
            languages,
            languages_by_extension,
            languages_by_shebang,
            languages_glob_matcher: FileTypeGlobMatcher::new(file_type_globs)?,
            language_server_configs: config.language_server,
            scopes: Arc::new(ArcSwap::from_pointee(Vec::new())),
        })
    }

    pub fn languages(&self) -> impl ExactSizeIterator<Item = (Language, &LanguageData)> {
        self.languages
            .iter()
            .enumerate()
            .map(|(idx, data)| (Language(idx as u32), data))
    }

    pub fn language_configs(&self) -> impl ExactSizeIterator<Item = &LanguageConfiguration> {
        self.languages.iter().map(|language| &*language.config)
    }

    pub fn language(&self, lang: Language) -> &LanguageData {
        &self.languages[lang.idx()]
    }

    pub fn language_for_name(&self, name: impl PartialEq<String>) -> Option<Language> {
        self.languages.iter().enumerate().find_map(|(idx, config)| {
            (name == config.config.language_id).then_some(Language(idx as u32))
        })
    }

    pub fn language_for_scope(&self, scope: &str) -> Option<Language> {
        self.languages.iter().enumerate().find_map(|(idx, config)| {
            (scope == config.config.scope).then_some(Language(idx as u32))
        })
    }

    pub fn language_for_match(&self, text: RopeSlice) -> Option<Language> {
        // PERF: If the name matches up with the id, then this saves the need to do expensive regex.
        let shortcircuit = self.language_for_name(text);
        if shortcircuit.is_some() {
            return shortcircuit;
        }

        // If the name did not match up with a known id, then match on injection regex.

        let mut best_match_length = 0;
        let mut best_match_position = None;
        for (idx, data) in self.languages.iter().enumerate() {
            if let Some(injection_regex) = &data.config.injection_regex {
                if let Some(mat) = injection_regex.find(text.regex_input()) {
                    let length = mat.end() - mat.start();
                    if length > best_match_length {
                        best_match_position = Some(idx);
                        best_match_length = length;
                    }
                }
            }
        }

        best_match_position.map(|i| Language(i as u32))
    }

    pub fn language_for_filename(&self, path: &Path) -> Option<Language> {
        // Find all the language configurations that match this file name
        // or a suffix of the file name.

        // TODO: content_regex handling conflict resolution
        self.languages_glob_matcher
            .language_for_path(path)
            .or_else(|| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .and_then(|extension| self.languages_by_extension.get(extension).copied())
            })
    }

    pub fn language_for_shebang(&self, text: RopeSlice) -> Option<Language> {
        // NOTE: this is slightly different than the one for injection markers in tree-house. It
        // is anchored at the beginning.
        use helix_stdx::rope::Regex;
        use once_cell::sync::Lazy;
        const SHEBANG: &str = r"^#!\s*(?:\S*[/\\](?:env\s+(?:\-\S+\s+)*)?)?([^\s\.\d]+)";
        static SHEBANG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(SHEBANG).unwrap());

        let marker = SHEBANG_REGEX
            .captures_iter(regex_cursor::Input::new(text))
            .map(|cap| text.byte_slice(cap.get_group(1).unwrap().range()))
            .next()?;
        self.language_for_shebang_marker(marker)
    }

    pub fn language_configs_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut Arc<LanguageConfiguration>> {
        self.languages
            .iter_mut()
            .map(|language| &mut language.config)
    }

    pub fn language_server_configs_mut(
        &mut self,
    ) -> &mut HashMap<String, LanguageServerConfiguration> {
        &mut self.language_server_configs
    }

    fn language_for_shebang_marker(&self, marker: RopeSlice) -> Option<Language> {
        let shebang: Cow<str> = marker.into();
        self.languages_by_shebang.get(shebang.as_ref()).copied()
    }

    pub fn indent_query(&self, lang: Language) -> Option<&IndentQuery> {
        self.language(lang).indent_query(self)
    }

    pub fn textobject_query(&self, lang: Language) -> Option<&TextObjectQuery> {
        self.language(lang).textobject_query(self)
    }

    pub fn language_server_configs(&self) -> &HashMap<String, LanguageServerConfiguration> {
        &self.language_server_configs
    }

    pub fn scopes(&self) -> Guard<Arc<Vec<String>>> {
        self.scopes.load()
    }

    pub fn set_scopes(&self, scopes: Vec<String>) {
        self.scopes.store(Arc::new(scopes));

        // Reconfigure existing grammars
        for data in &self.languages {
            data.reconfigure(&self.scopes());
        }
    }
}

impl LanguageLoader for Loader {
    fn language_for_marker(&self, marker: InjectionLanguageMarker) -> Option<Language> {
        match marker {
            InjectionLanguageMarker::Name(name) => self.language_for_name(name),
            InjectionLanguageMarker::Match(text) => self.language_for_match(text),
            InjectionLanguageMarker::Filename(text) => {
                let path: Cow<str> = text.into();
                self.language_for_filename(Path::new(path.as_ref()))
            }
            InjectionLanguageMarker::Shebang(text) => self.language_for_shebang_marker(text),
        }
    }

    fn get_config(&self, lang: Language) -> Option<&SyntaxConfig> {
        self.languages[lang.idx()].syntax_config(self)
    }
}

#[derive(Debug, Clone)]
struct FileTypeGlob {
    glob: globset::Glob,
    language: Language,
}

impl FileTypeGlob {
    pub fn new(glob: globset::Glob, language: Language) -> Self {
        Self { glob, language }
    }
}

#[derive(Debug, Clone)]
struct FileTypeGlobMatcher {
    matcher: globset::GlobSet,
    file_types: Vec<FileTypeGlob>,
}

impl Default for FileTypeGlobMatcher {
    fn default() -> Self {
        Self {
            matcher: globset::GlobSet::empty(),
            file_types: Default::default(),
        }
    }
}

impl FileTypeGlobMatcher {
    fn new(file_types: Vec<FileTypeGlob>) -> Result<Self, globset::Error> {
        let mut builder = globset::GlobSetBuilder::new();
        for file_type in &file_types {
            builder.add(file_type.glob.clone());
        }

        Ok(Self {
            matcher: builder.build()?,
            file_types,
        })
    }

    fn language_for_path(&self, path: &Path) -> Option<Language> {
        self.matcher
            .matches(path)
            .iter()
            .filter_map(|idx| self.file_types.get(*idx))
            .max_by_key(|file_type| file_type.glob.glob().len())
            .map(|file_type| file_type.language)
    }
}

#[derive(Debug)]
pub struct Syntax {
    inner: tree_house::Syntax,
}

const PARSE_TIMEOUT: Duration = Duration::from_millis(500); // half a second is pretty generous

impl Syntax {
    pub fn new(source: RopeSlice, language: Language, loader: &Loader) -> Result<Self, Error> {
        let inner = tree_house::Syntax::new(source, language, PARSE_TIMEOUT, loader)?;
        Ok(Self { inner })
    }

    pub fn update(
        &mut self,
        old_source: RopeSlice,
        source: RopeSlice,
        changeset: &ChangeSet,
        loader: &Loader,
    ) -> Result<(), Error> {
        let edits = generate_edits(old_source, changeset);
        if edits.is_empty() {
            Ok(())
        } else {
            self.inner.update(source, PARSE_TIMEOUT, &edits, loader)
        }
    }

    pub fn layer(&self, layer: Layer) -> &tree_house::LayerData {
        self.inner.layer(layer)
    }

    pub fn root_layer(&self) -> Layer {
        self.inner.root()
    }

    pub fn layer_for_byte_range(&self, start: u32, end: u32) -> Layer {
        self.inner.layer_for_byte_range(start, end)
    }

    pub fn root_language(&self) -> Language {
        self.layer(self.root_layer()).language
    }

    pub fn tree(&self) -> &Tree {
        self.inner.tree()
    }

    pub fn tree_for_byte_range(&self, start: u32, end: u32) -> &Tree {
        self.inner.tree_for_byte_range(start, end)
    }

    pub fn named_descendant_for_byte_range(&self, start: u32, end: u32) -> Option<Node> {
        self.inner.named_descendant_for_byte_range(start, end)
    }

    pub fn descendant_for_byte_range(&self, start: u32, end: u32) -> Option<Node> {
        self.inner.descendant_for_byte_range(start, end)
    }

    pub fn walk(&self) -> TreeCursor {
        self.inner.walk()
    }

    pub fn highlighter<'a>(
        &'a self,
        source: RopeSlice<'a>,
        loader: &'a Loader,
        range: impl RangeBounds<u32>,
    ) -> Highlighter<'a> {
        Highlighter::new(&self.inner, source, loader, range)
    }

    pub fn query_iter<'a, QueryLoader, LayerState, Range>(
        &'a self,
        source: RopeSlice<'a>,
        loader: QueryLoader,
        range: Range,
    ) -> QueryIter<'a, 'a, QueryLoader, LayerState>
    where
        QueryLoader: FnMut(Language) -> Option<&'a Query> + 'a,
        LayerState: Default,
        Range: RangeBounds<u32>,
    {
        QueryIter::new(&self.inner, source, loader, range)
    }
}

pub type Highlighter<'a> = highlighter::Highlighter<'a, 'a, Loader>;

fn generate_edits(old_text: RopeSlice, changeset: &ChangeSet) -> Vec<InputEdit> {
    use crate::Operation::*;
    use tree_sitter::Point;

    let mut old_pos = 0;

    let mut edits = Vec::new();

    if changeset.changes.is_empty() {
        return edits;
    }

    let mut iter = changeset.changes.iter().peekable();

    // TODO; this is a lot easier with Change instead of Operation.
    while let Some(change) = iter.next() {
        let len = match change {
            Delete(i) | Retain(i) => *i,
            Insert(_) => 0,
        };
        let mut old_end = old_pos + len;

        match change {
            Retain(_) => {}
            Delete(_) => {
                let start_byte = old_text.char_to_byte(old_pos) as u32;
                let old_end_byte = old_text.char_to_byte(old_end) as u32;

                // deletion
                edits.push(InputEdit {
                    start_byte,               // old_pos to byte
                    old_end_byte,             // old_end to byte
                    new_end_byte: start_byte, // old_pos to byte
                    start_point: Point::ZERO,
                    old_end_point: Point::ZERO,
                    new_end_point: Point::ZERO,
                });
            }
            Insert(s) => {
                let start_byte = old_text.char_to_byte(old_pos) as u32;

                // a subsequent delete means a replace, consume it
                if let Some(Delete(len)) = iter.peek() {
                    old_end = old_pos + len;
                    let old_end_byte = old_text.char_to_byte(old_end) as u32;

                    iter.next();

                    // replacement
                    edits.push(InputEdit {
                        start_byte,                                // old_pos to byte
                        old_end_byte,                              // old_end to byte
                        new_end_byte: start_byte + s.len() as u32, // old_pos to byte + s.len()
                        start_point: Point::ZERO,
                        old_end_point: Point::ZERO,
                        new_end_point: Point::ZERO,
                    });
                } else {
                    // insert
                    edits.push(InputEdit {
                        start_byte,                                // old_pos to byte
                        old_end_byte: start_byte,                  // same
                        new_end_byte: start_byte + s.len() as u32, // old_pos + s.len()
                        start_point: Point::ZERO,
                        old_end_point: Point::ZERO,
                        new_end_point: Point::ZERO,
                    });
                }
            }
        }
        old_pos = old_end;
    }
    edits
}

/// A set of "overlay" highlights and ranges they apply to.
///
/// As overlays, the styles for the given `Highlight`s are merged on top of the syntax highlights.
#[derive(Debug)]
pub enum OverlayHighlights {
    /// All highlights use a single `Highlight`.
    ///
    /// Note that, currently, all ranges are assumed to be non-overlapping. This could change in
    /// the future though.
    Homogeneous {
        highlight: Highlight,
        ranges: Vec<ops::Range<usize>>,
    },
    /// A collection of different highlights for given ranges.
    ///
    /// Note that the ranges **must be non-overlapping**.
    Heterogenous {
        highlights: Vec<(Highlight, ops::Range<usize>)>,
    },
}

impl OverlayHighlights {
    pub fn single(highlight: Highlight, range: ops::Range<usize>) -> Self {
        Self::Homogeneous {
            highlight,
            ranges: vec![range],
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Homogeneous { ranges, .. } => ranges.is_empty(),
            Self::Heterogenous { highlights } => highlights.is_empty(),
        }
    }
}

#[derive(Debug)]
struct Overlay {
    highlights: OverlayHighlights,
    /// The position of the highlighter into the Vec of ranges of the overlays.
    ///
    /// Used by the `OverlayHighlighter`.
    idx: usize,
    /// The currently active highlight (and the ending character index) for this overlay.
    ///
    /// Used by the `OverlayHighlighter`.
    active_highlight: Option<(Highlight, usize)>,
}

impl Overlay {
    fn new(highlights: OverlayHighlights) -> Option<Self> {
        (!highlights.is_empty()).then_some(Self {
            highlights,
            idx: 0,
            active_highlight: None,
        })
    }

    fn current(&self) -> Option<(Highlight, ops::Range<usize>)> {
        match &self.highlights {
            OverlayHighlights::Homogeneous { highlight, ranges } => ranges
                .get(self.idx)
                .map(|range| (*highlight, range.clone())),
            OverlayHighlights::Heterogenous { highlights } => highlights.get(self.idx).cloned(),
        }
    }

    fn start(&self) -> Option<usize> {
        match &self.highlights {
            OverlayHighlights::Homogeneous { ranges, .. } => {
                ranges.get(self.idx).map(|range| range.start)
            }
            OverlayHighlights::Heterogenous { highlights } => highlights
                .get(self.idx)
                .map(|(_highlight, range)| range.start),
        }
    }
}

/// A collection of highlights to apply when rendering which merge on top of syntax highlights.
#[derive(Debug)]
pub struct OverlayHighlighter {
    overlays: Vec<Overlay>,
    next_highlight_start: usize,
    next_highlight_end: usize,
}

impl OverlayHighlighter {
    pub fn new(overlays: impl IntoIterator<Item = OverlayHighlights>) -> Self {
        let overlays: Vec<_> = overlays.into_iter().filter_map(Overlay::new).collect();
        let next_highlight_start = overlays
            .iter()
            .filter_map(|overlay| overlay.start())
            .min()
            .unwrap_or(usize::MAX);

        Self {
            overlays,
            next_highlight_start,
            next_highlight_end: usize::MAX,
        }
    }

    /// The current position in the overlay highlights.
    ///
    /// This method is meant to be used when treating this type as a cursor over the overlay
    /// highlights.
    ///
    /// `usize::MAX` is returned when there are no more overlay highlights.
    pub fn next_event_offset(&self) -> usize {
        self.next_highlight_start.min(self.next_highlight_end)
    }

    pub fn advance(&mut self) -> (HighlightEvent, impl Iterator<Item = Highlight> + '_) {
        let mut refresh = false;
        let prev_stack_size = self
            .overlays
            .iter()
            .filter(|overlay| overlay.active_highlight.is_some())
            .count();
        let pos = self.next_event_offset();

        if self.next_highlight_end == pos {
            for overlay in self.overlays.iter_mut() {
                if overlay
                    .active_highlight
                    .is_some_and(|(_highlight, end)| end == pos)
                {
                    overlay.active_highlight.take();
                }
            }

            refresh = true;
        }

        while self.next_highlight_start == pos {
            let mut activated_idx = usize::MAX;
            for (idx, overlay) in self.overlays.iter_mut().enumerate() {
                let Some((highlight, range)) = overlay.current() else {
                    continue;
                };
                if range.start != self.next_highlight_start {
                    continue;
                }

                // If this overlay has a highlight at this start index, set its active highlight
                // and increment the cursor position within the overlay.
                overlay.active_highlight = Some((highlight, range.end));
                overlay.idx += 1;

                activated_idx = activated_idx.min(idx);
            }

            // If `self.next_highlight_start == pos` that means that some overlay was ready to
            // emit a highlight, so `activated_idx` must have been set to an existing index.
            assert!(
                (0..self.overlays.len()).contains(&activated_idx),
                "expected an overlay to highlight (at pos {pos}, there are {} overlays)",
                self.overlays.len()
            );

            // If any overlays are active after the (lowest) one which was just activated, the
            // highlights need to be refreshed.
            refresh |= self.overlays[activated_idx..]
                .iter()
                .any(|overlay| overlay.active_highlight.is_some());

            self.next_highlight_start = self
                .overlays
                .iter()
                .filter_map(|overlay| overlay.start())
                .min()
                .unwrap_or(usize::MAX);
        }

        self.next_highlight_end = self
            .overlays
            .iter()
            .filter_map(|overlay| Some(overlay.active_highlight?.1))
            .min()
            .unwrap_or(usize::MAX);

        let (event, start) = if refresh {
            (HighlightEvent::Refresh, 0)
        } else {
            (HighlightEvent::Push, prev_stack_size)
        };

        (
            event,
            self.overlays
                .iter()
                .flat_map(|overlay| overlay.active_highlight)
                .map(|(highlight, _end)| highlight)
                .skip(start),
        )
    }
}

#[derive(Debug)]
pub enum CapturedNode<'a> {
    Single(Node<'a>),
    /// Guaranteed to be not empty
    Grouped(Vec<Node<'a>>),
}

impl CapturedNode<'_> {
    pub fn start_byte(&self) -> usize {
        match self {
            Self::Single(n) => n.start_byte() as usize,
            Self::Grouped(ns) => ns[0].start_byte() as usize,
        }
    }

    pub fn end_byte(&self) -> usize {
        match self {
            Self::Single(n) => n.end_byte() as usize,
            Self::Grouped(ns) => ns.last().unwrap().end_byte() as usize,
        }
    }

    pub fn byte_range(&self) -> ops::Range<usize> {
        self.start_byte()..self.end_byte()
    }
}

#[derive(Debug)]
pub struct TextObjectQuery {
    query: Query,
}

impl TextObjectQuery {
    pub fn new(query: Query) -> Self {
        Self { query }
    }

    /// Run the query on the given node and return sub nodes which match given
    /// capture ("function.inside", "class.around", etc).
    ///
    /// Captures may contain multiple nodes by using quantifiers (+, *, etc),
    /// and support for this is partial and could use improvement.
    ///
    /// ```query
    /// (comment)+ @capture
    ///
    /// ; OR
    /// (
    ///   (comment)*
    ///   .
    ///   (function)
    /// ) @capture
    /// ```
    pub fn capture_nodes<'a>(
        &'a self,
        capture_name: &str,
        node: &Node<'a>,
        slice: RopeSlice<'a>,
    ) -> Option<impl Iterator<Item = CapturedNode<'a>>> {
        self.capture_nodes_any(&[capture_name], node, slice)
    }

    /// Find the first capture that exists out of all given `capture_names`
    /// and return sub nodes that match this capture.
    pub fn capture_nodes_any<'a>(
        &'a self,
        capture_names: &[&str],
        node: &Node<'a>,
        slice: RopeSlice<'a>,
    ) -> Option<impl Iterator<Item = CapturedNode<'a>>> {
        let capture = capture_names
            .iter()
            .find_map(|cap| self.query.get_capture(cap))?;

        let mut cursor = InactiveQueryCursor::new(0..u32::MAX, TREE_SITTER_MATCH_LIMIT)
            .execute_query(&self.query, node, RopeInput::new(slice));
        let capture_node = iter::from_fn(move || {
            let (mat, _) = cursor.next_matched_node()?;
            Some(mat.nodes_for_capture(capture).cloned().collect())
        })
        .filter_map(move |nodes: Vec<_>| {
            if nodes.len() > 1 {
                Some(CapturedNode::Grouped(nodes))
            } else {
                nodes.into_iter().map(CapturedNode::Single).next()
            }
        });
        Some(capture_node)
    }
}

pub fn pretty_print_tree<W: fmt::Write>(fmt: &mut W, node: Node) -> fmt::Result {
    if node.child_count() == 0 {
        if node_is_visible(&node) {
            write!(fmt, "({})", node.kind())
        } else {
            write!(fmt, "\"{}\"", format_anonymous_node_kind(node.kind()))
        }
    } else {
        pretty_print_tree_impl(fmt, &mut node.walk(), 0)
    }
}

fn node_is_visible(node: &Node) -> bool {
    node.is_missing() || (node.is_named() && node.grammar().node_kind_is_visible(node.kind_id()))
}

fn format_anonymous_node_kind(kind: &str) -> Cow<str> {
    if kind.contains('"') {
        Cow::Owned(kind.replace('"', "\\\""))
    } else {
        Cow::Borrowed(kind)
    }
}

fn pretty_print_tree_impl<W: fmt::Write>(
    fmt: &mut W,
    cursor: &mut tree_sitter::TreeCursor,
    depth: usize,
) -> fmt::Result {
    let node = cursor.node();
    let visible = node_is_visible(&node);

    if visible {
        let indentation_columns = depth * 2;
        write!(fmt, "{:indentation_columns$}", "")?;

        if let Some(field_name) = cursor.field_name() {
            write!(fmt, "{}: ", field_name)?;
        }

        write!(fmt, "({}", node.kind())?;
    } else {
        write!(fmt, " \"{}\"", format_anonymous_node_kind(node.kind()))?;
    }

    // Handle children.
    if cursor.goto_first_child() {
        loop {
            if node_is_visible(&cursor.node()) {
                fmt.write_char('\n')?;
            }

            pretty_print_tree_impl(fmt, cursor, depth + 1)?;

            if !cursor.goto_next_sibling() {
                break;
            }
        }

        let moved = cursor.goto_parent();
        // The parent of the first child must exist, and must be `node`.
        debug_assert!(moved);
        debug_assert!(cursor.node() == node);
    }

    if visible {
        fmt.write_char(')')?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use once_cell::sync::Lazy;

    use super::*;
    use crate::{Rope, Transaction};

    static LOADER: Lazy<Loader> = Lazy::new(crate::config::default_lang_loader);

    #[test]
    fn test_textobject_queries() {
        let query_str = r#"
        (line_comment)+ @quantified_nodes
        ((line_comment)+) @quantified_nodes_grouped
        ((line_comment) (line_comment)) @multiple_nodes_grouped
        "#;
        let source = Rope::from_str(
            r#"
/// a comment on
/// multiple lines
        "#,
        );

        let language = LOADER.language_for_name("rust").unwrap();
        let grammar = LOADER.get_config(language).unwrap().grammar;
        let query = Query::new(grammar, query_str, |_, _| Ok(())).unwrap();
        let textobject = TextObjectQuery::new(query);
        let syntax = Syntax::new(source.slice(..), language, &LOADER).unwrap();

        let root = syntax.tree().root_node();
        let test = |capture, range| {
            let matches: Vec<_> = textobject
                .capture_nodes(capture, &root, source.slice(..))
                .unwrap()
                .collect();

            assert_eq!(
                matches[0].byte_range(),
                range,
                "@{} expected {:?}",
                capture,
                range
            )
        };

        test("quantified_nodes", 1..37);
        // NOTE: Enable after implementing proper node group capturing
        // test("quantified_nodes_grouped", 1..37);
        // test("multiple_nodes_grouped", 1..37);
    }

    #[test]
    fn test_input_edits() {
        use tree_sitter::{InputEdit, Point};

        let doc = Rope::from("hello world!\ntest 123");
        let transaction = Transaction::change(
            &doc,
            vec![(6, 11, Some("test".into())), (12, 17, None)].into_iter(),
        );
        let edits = generate_edits(doc.slice(..), transaction.changes());
        // transaction.apply(&mut state);

        assert_eq!(
            edits,
            &[
                InputEdit {
                    start_byte: 6,
                    old_end_byte: 11,
                    new_end_byte: 10,
                    start_point: Point::ZERO,
                    old_end_point: Point::ZERO,
                    new_end_point: Point::ZERO
                },
                InputEdit {
                    start_byte: 12,
                    old_end_byte: 17,
                    new_end_byte: 12,
                    start_point: Point::ZERO,
                    old_end_point: Point::ZERO,
                    new_end_point: Point::ZERO
                }
            ]
        );

        // Testing with the official example from tree-sitter
        let mut doc = Rope::from("fn test() {}");
        let transaction =
            Transaction::change(&doc, vec![(8, 8, Some("a: u32".into()))].into_iter());
        let edits = generate_edits(doc.slice(..), transaction.changes());
        transaction.apply(&mut doc);

        assert_eq!(doc, "fn test(a: u32) {}");
        assert_eq!(
            edits,
            &[InputEdit {
                start_byte: 8,
                old_end_byte: 8,
                new_end_byte: 14,
                start_point: Point::ZERO,
                old_end_point: Point::ZERO,
                new_end_point: Point::ZERO
            }]
        );
    }

    #[track_caller]
    fn assert_pretty_print(
        language_name: &str,
        source: &str,
        expected: &str,
        start: usize,
        end: usize,
    ) {
        let source = Rope::from_str(source);
        let language = LOADER.language_for_name(language_name).unwrap();
        let syntax = Syntax::new(source.slice(..), language, &LOADER).unwrap();

        let root = syntax
            .tree()
            .root_node()
            .descendant_for_byte_range(start as u32, end as u32)
            .unwrap();

        let mut output = String::new();
        pretty_print_tree(&mut output, root).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn test_pretty_print() {
        let source = r#"// Hello"#;
        assert_pretty_print("rust", source, "(line_comment \"//\")", 0, source.len());

        // A large tree should be indented with fields:
        let source = r#"fn main() {
            println!("Hello, World!");
        }"#;
        assert_pretty_print(
            "rust",
            source,
            concat!(
                "(function_item \"fn\"\n",
                "  name: (identifier)\n",
                "  parameters: (parameters \"(\" \")\")\n",
                "  body: (block \"{\"\n",
                "    (expression_statement\n",
                "      (macro_invocation\n",
                "        macro: (identifier) \"!\"\n",
                "        (token_tree \"(\"\n",
                "          (string_literal \"\\\"\"\n",
                "            (string_content) \"\\\"\") \")\")) \";\") \"}\"))",
            ),
            0,
            source.len(),
        );

        // Selecting a token should print just that token:
        let source = r#"fn main() {}"#;
        assert_pretty_print("rust", source, r#""fn""#, 0, 1);

        // Error nodes are printed as errors:
        let source = r#"}{"#;
        assert_pretty_print("rust", source, "(ERROR \"}\" \"{\")", 0, source.len());

        // Fields broken under unnamed nodes are determined correctly.
        // In the following source, `object` belongs to the `singleton_method`
        // rule but `name` and `body` belong to an unnamed helper `_method_rest`.
        // This can cause a bug with a pretty-printing implementation that
        // uses `Node::field_name_for_child` to determine field names but is
        // fixed when using `tree_sitter::TreeCursor::field_name`.
        let source = "def self.method_name
          true
        end";
        assert_pretty_print(
            "ruby",
            source,
            concat!(
                "(singleton_method \"def\"\n",
                "  object: (self) \".\"\n",
                "  name: (identifier)\n",
                "  body: (body_statement\n",
                "    (true)) \"end\")"
            ),
            0,
            source.len(),
        );
    }
}
