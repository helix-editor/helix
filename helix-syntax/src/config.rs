use std::path::Path;
use std::sync::Arc;

use arc_swap::ArcSwap;
use helix_stdx::rope::{self, RopeSliceExt};
use once_cell::sync::Lazy;
use regex::Regex;
use ropey::RopeSlice;
use tree_sitter::{Language as Grammar, Node, Query, QueryError, QueryMatch};

use crate::highlighter::Highlight;
use crate::{byte_range_to_str, IncludedChildren, InjectionLanguageMarker, SHEBANG};

/// Contains the data needed to highlight code written in a particular language.
///
/// This struct is immutable and can be shared between threads.
#[derive(Debug)]
pub struct HighlightConfiguration {
    pub language: Grammar,
    pub query: Query,
    pub(crate) injections_query: Query,
    pub(crate) combined_injections_patterns: Vec<usize>,
    pub(crate) highlights_pattern_index: usize,
    pub(crate) highlight_indices: ArcSwap<Vec<Option<Highlight>>>,
    pub(crate) non_local_variable_patterns: Vec<bool>,
    pub(crate) injection_content_capture_index: Option<u32>,
    pub(crate) injection_language_capture_index: Option<u32>,
    pub(crate) injection_filename_capture_index: Option<u32>,
    pub(crate) injection_shebang_capture_index: Option<u32>,
    pub(crate) local_scope_capture_index: Option<u32>,
    pub(crate) local_def_capture_index: Option<u32>,
    pub(crate) local_def_value_capture_index: Option<u32>,
    pub(crate) local_ref_capture_index: Option<u32>,
}

impl HighlightConfiguration {
    /// Creates a `HighlightConfiguration` for a given `Grammar` and set of highlighting
    /// queries.
    ///
    /// # Parameters
    ///
    /// * `language`  - The Tree-sitter `Grammar` that should be used for parsing.
    /// * `highlights_query` - A string containing tree patterns for syntax highlighting. This
    ///   should be non-empty, otherwise no syntax highlights will be added.
    /// * `injections_query` -  A string containing tree patterns for injecting other languages
    ///   into the document. This can be empty if no injections are desired.
    /// * `locals_query` - A string containing tree patterns for tracking local variable
    ///   definitions and references. This can be empty if local variable tracking is not needed.
    ///
    /// Returns a `HighlightConfiguration` that can then be used with the `highlight` method.
    pub fn new(
        language: Grammar,
        highlights_query: &str,
        injection_query: &str,
        locals_query: &str,
    ) -> Result<Self, QueryError> {
        // Concatenate the query strings, keeping track of the start offset of each section.
        let mut query_source = String::new();
        query_source.push_str(locals_query);
        let highlights_query_offset = query_source.len();
        query_source.push_str(highlights_query);

        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let query = Query::new(&language, &query_source)?;
        let mut highlights_pattern_index = 0;
        for i in 0..(query.pattern_count()) {
            let pattern_offset = query.start_byte_for_pattern(i);
            if pattern_offset < highlights_query_offset {
                highlights_pattern_index += 1;
            }
        }

        let injections_query = Query::new(&language, injection_query)?;
        let combined_injections_patterns = (0..injections_query.pattern_count())
            .filter(|&i| {
                injections_query
                    .property_settings(i)
                    .iter()
                    .any(|s| &*s.key == "injection.combined")
            })
            .collect();

        // Find all of the highlighting patterns that are disabled for nodes that
        // have been identified as local variables.
        let non_local_variable_patterns = (0..query.pattern_count())
            .map(|i| {
                query
                    .property_predicates(i)
                    .iter()
                    .any(|(prop, positive)| !*positive && prop.key.as_ref() == "local")
            })
            .collect();

        // Store the numeric ids for all of the special captures.
        let mut injection_content_capture_index = None;
        let mut injection_language_capture_index = None;
        let mut injection_filename_capture_index = None;
        let mut injection_shebang_capture_index = None;
        let mut local_def_capture_index = None;
        let mut local_def_value_capture_index = None;
        let mut local_ref_capture_index = None;
        let mut local_scope_capture_index = None;
        for (i, name) in query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match *name {
                "local.definition" => local_def_capture_index = i,
                "local.definition-value" => local_def_value_capture_index = i,
                "local.reference" => local_ref_capture_index = i,
                "local.scope" => local_scope_capture_index = i,
                _ => {}
            }
        }

        for (i, name) in injections_query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match *name {
                "injection.content" => injection_content_capture_index = i,
                "injection.language" => injection_language_capture_index = i,
                "injection.filename" => injection_filename_capture_index = i,
                "injection.shebang" => injection_shebang_capture_index = i,
                _ => {}
            }
        }

        let highlight_indices = ArcSwap::from_pointee(vec![None; query.capture_names().len()]);
        Ok(Self {
            language,
            query,
            injections_query,
            combined_injections_patterns,
            highlights_pattern_index,
            highlight_indices,
            non_local_variable_patterns,
            injection_content_capture_index,
            injection_language_capture_index,
            injection_filename_capture_index,
            injection_shebang_capture_index,
            local_scope_capture_index,
            local_def_capture_index,
            local_def_value_capture_index,
            local_ref_capture_index,
        })
    }

    /// Get a slice containing all of the highlight names used in the configuration.
    pub fn names(&self) -> &[&str] {
        self.query.capture_names()
    }

    /// Set the list of recognized highlight names.
    ///
    /// Tree-sitter syntax-highlighting queries specify highlights in the form of dot-separated
    /// highlight names like `punctuation.bracket` and `function.method.builtin`. Consumers of
    /// these queries can choose to recognize highlights with different levels of specificity.
    /// For example, the string `function.builtin` will match against `function.builtin.constructor`
    /// but will not match `function.method.builtin` and `function.method`.
    ///
    /// When highlighting, results are returned as `Highlight` values, which contain the index
    /// of the matched highlight this list of highlight names.
    pub fn configure(&self, recognized_names: &[String]) {
        let mut capture_parts = Vec::new();
        let indices: Vec<_> = self
            .query
            .capture_names()
            .iter()
            .map(move |capture_name| {
                capture_parts.clear();
                capture_parts.extend(capture_name.split('.'));

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
                best_index.map(Highlight)
            })
            .collect();

        self.highlight_indices.store(Arc::new(indices));
    }

    fn injection_pair<'a>(
        &self,
        query_match: &QueryMatch<'a, 'a>,
        source: RopeSlice<'a>,
    ) -> (Option<InjectionLanguageMarker<'a>>, Option<Node<'a>>) {
        let mut injection_capture = None;
        let mut content_node = None;

        for capture in query_match.captures {
            let index = Some(capture.index);
            if index == self.injection_language_capture_index {
                let name = byte_range_to_str(capture.node.byte_range(), source);
                injection_capture = Some(InjectionLanguageMarker::Name(name));
            } else if index == self.injection_filename_capture_index {
                let name = byte_range_to_str(capture.node.byte_range(), source);
                let path = Path::new(name.as_ref()).to_path_buf();
                injection_capture = Some(InjectionLanguageMarker::Filename(path.into()));
            } else if index == self.injection_shebang_capture_index {
                let node_slice = source.byte_slice(capture.node.byte_range());

                // some languages allow space and newlines before the actual string content
                // so a shebang could be on either the first or second line
                let lines = if let Ok(end) = node_slice.try_line_to_byte(2) {
                    node_slice.byte_slice(..end)
                } else {
                    node_slice
                };

                static SHEBANG_REGEX: Lazy<rope::Regex> =
                    Lazy::new(|| rope::Regex::new(SHEBANG).unwrap());

                injection_capture = SHEBANG_REGEX
                    .captures_iter(lines.regex_input())
                    .map(|cap| {
                        let cap = lines.byte_slice(cap.get_group(1).unwrap().range());
                        InjectionLanguageMarker::Shebang(cap.into())
                    })
                    .next()
            } else if index == self.injection_content_capture_index {
                content_node = Some(capture.node);
            }
        }
        (injection_capture, content_node)
    }

    pub(super) fn injection_for_match<'a>(
        &self,
        query: &'a Query,
        query_match: &QueryMatch<'a, 'a>,
        source: RopeSlice<'a>,
    ) -> (
        Option<InjectionLanguageMarker<'a>>,
        Option<Node<'a>>,
        IncludedChildren,
    ) {
        let (mut injection_capture, content_node) = self.injection_pair(query_match, source);

        let mut included_children = IncludedChildren::default();
        for prop in query.property_settings(query_match.pattern_index) {
            match prop.key.as_ref() {
                // In addition to specifying the language name via the text of a
                // captured node, it can also be hard-coded via a `#set!` predicate
                // that sets the injection.language key.
                "injection.language" if injection_capture.is_none() => {
                    injection_capture = prop
                        .value
                        .as_ref()
                        .map(|s| InjectionLanguageMarker::Name(s.as_ref().into()));
                }

                // By default, injections do not include the *children* of an
                // `injection.content` node - only the ranges that belong to the
                // node itself. This can be changed using a `#set!` predicate that
                // sets the `injection.include-children` key.
                "injection.include-children" => included_children = IncludedChildren::All,

                // Some queries might only exclude named children but include unnamed
                // children in their `injection.content` node. This can be enabled using
                // a `#set!` predicate that sets the `injection.include-unnamed-children` key.
                "injection.include-unnamed-children" => {
                    included_children = IncludedChildren::Unnamed
                }
                _ => {}
            }
        }

        (injection_capture, content_node, included_children)
    }
    pub fn load_query(
        &self,
        language: &str,
        filename: &str,
        read_query_text: impl FnMut(&str, &str) -> String,
    ) -> Result<Option<Query>, QueryError> {
        let query_text = read_query(language, filename, read_query_text);
        if query_text.is_empty() {
            return Ok(None);
        }
        Query::new(&self.language, &query_text).map(Some)
    }
}

/// reads a query by invoking `read_query_text`, handeles any `inherits` directives
pub fn read_query(
    language: &str,
    filename: &str,
    mut read_query_text: impl FnMut(&str, &str) -> String,
) -> String {
    fn read_query_impl(
        language: &str,
        filename: &str,
        read_query_text: &mut impl FnMut(&str, &str) -> String,
    ) -> String {
        static INHERITS_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r";+\s*inherits\s*:?\s*([a-z_,()-]+)\s*").unwrap());

        let query = read_query_text(language, filename);

        // replaces all "; inherits <language>(,<language>)*" with the queries of the given language(s)
        INHERITS_REGEX
            .replace_all(&query, |captures: &regex::Captures| {
                captures[1]
                    .split(',')
                    .map(|language| {
                        format!(
                            "\n{}\n",
                            read_query_impl(language, filename, &mut *read_query_text)
                        )
                    })
                    .collect::<String>()
            })
            .to_string()
    }
    read_query_impl(language, filename, &mut read_query_text)
}
