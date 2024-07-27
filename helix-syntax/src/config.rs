use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use crate::tree_sitter::query::{Capture, Pattern, QueryStr, UserPredicate};
use crate::tree_sitter::{query, Grammar, Query, QueryMatch, SyntaxTreeNode};
use arc_swap::ArcSwap;
use helix_stdx::rope::{self, RopeSliceExt};
use once_cell::sync::Lazy;
use regex::Regex;
use ropey::RopeSlice;

use crate::byte_range_to_str;
use crate::highlighter::Highlight;

/// Contains the data needed to highlight code written in a particular language.
///
/// This struct is immutable and can be shared between threads.
#[derive(Debug)]
pub struct HighlightConfiguration {
    pub grammar: Grammar,
    pub query: Query,
    pub(crate) injections_query: Query,
    pub(crate) combined_injections_patterns: Vec<Pattern>,
    first_highlights_pattern: Pattern,
    pub(crate) highlight_indices: ArcSwap<Vec<Highlight>>,
    pub(crate) non_local_variable_patterns: Vec<bool>,
    pub(crate) injection_content_capture: Option<Capture>,
    pub(crate) injection_language_capture: Option<Capture>,
    pub(crate) injection_filename_capture: Option<Capture>,
    pub(crate) injection_shebang_capture: Option<Capture>,
    pub(crate) local_scope_capture: Option<Capture>,
    pub(crate) local_def_capture: Option<Capture>,
    pub(crate) local_def_value_capture: Option<Capture>,
    pub(crate) local_ref_capture: Option<Capture>,
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
        grammar: Grammar,
        path: impl AsRef<Path>,
        highlights_query: &str,
        injection_query: &str,
        locals_query: &str,
    ) -> Result<Self, query::ParseError> {
        // Concatenate the query strings, keeping track of the start offset of each section.
        let mut query_source = String::new();
        query_source.push_str(locals_query);
        let highlights_query_offset = query_source.len();
        query_source.push_str(highlights_query);

        let mut non_local_variable_patterns = Vec::with_capacity(32);
        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let query = Query::new(grammar, &query_source, path, |pattern, predicate| {
            match predicate {
                UserPredicate::IsPropertySet {
                    negate: true,
                    key: "local",
                    val: None,
                } => {
                    if non_local_variable_patterns.len() < pattern.idx() {
                        non_local_variable_patterns.resize(pattern.idx(), false)
                    }
                    non_local_variable_patterns[pattern.idx()] = true;
                }
                predicate => {
                    return Err(format!("unsupported predicate {predicate}").into());
                }
            }
            Ok(())
        })?;

        let mut combined_injections_patterns = Vec::new();
        let injections_query = Query::new(grammar, injection_query, path, |pattern, predicate| {
            match predicate {
                UserPredicate::SetProperty {
                    key: "injection.combined",
                    val: None,
                } => combined_injections_patterns.push(pattern),
                predicate => {
                    return Err(format!("unsupported predicate {predicate}").into());
                }
            }
            Ok(())
        })?;

        let first_highlights_pattern = query
            .patterns()
            .find(|pattern| query.start_byte_for_pattern(*pattern) >= highlights_query_offset)
            .unwrap_or(Pattern::SENTINEL);

        let injection_content_capture = query.get_capture("injection.content");
        let injection_language_capture = query.get_capture("injection.language");
        let injection_filename_capture = query.get_capture("injection.filename");
        let injection_shebang_capture = query.get_capture("injection.shebang");
        let local_def_capture = query.get_capture("local.definition");
        let local_def_value_capture = query.get_capture("local.definition-value");
        let local_ref_capture = query.get_capture("local.reference");
        let local_scope_capture = query.get_capture("local.scope");

        let highlight_indices =
            ArcSwap::from_pointee(vec![Highlight::NONE; query.num_captures() as usize]);
        Ok(Self {
            grammar,
            query,
            injections_query,
            combined_injections_patterns,
            first_highlights_pattern,
            highlight_indices,
            non_local_variable_patterns,
            injection_content_capture,
            injection_language_capture,
            injection_filename_capture,
            injection_shebang_capture,
            local_scope_capture,
            local_def_capture,
            local_def_value_capture,
            local_ref_capture,
        })
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
            .captures()
            .map(move |(_, capture_name)| {
                capture_parts.clear();
                capture_parts.extend(capture_name.split('.'));

                let mut best_index = u32::MAX;
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
                        best_index = i as u32;
                        best_match_len = len;
                    }
                }
                Highlight(best_index)
            })
            .collect();

        self.highlight_indices.store(Arc::new(indices));
    }

    fn injection_pair<'a>(
        &self,
        query_match: &QueryMatch<'a, 'a>,
        source: RopeSlice<'a>,
    ) -> (
        Option<InjectionLanguageMarker<'a>>,
        Option<SyntaxTreeNode<'a>>,
    ) {
        let mut injection_capture = None;
        let mut content_node = None;

        for matched_node in query_match.matched_nodes() {
            let capture = Some(matched_node.capture);
            if capture == self.injection_language_capture {
                let name = byte_range_to_str(matched_node.syntax_node.byte_range(), source);
                injection_capture = Some(InjectionLanguageMarker::Name(name));
            } else if capture == self.injection_filename_capture {
                let name = byte_range_to_str(matched_node.syntax_node.byte_range(), source);
                let path = Path::new(name.as_ref()).to_path_buf();
                injection_capture = Some(InjectionLanguageMarker::Filename(path.into()));
            } else if capture == self.injection_shebang_capture {
                let node_slice = source.byte_slice(matched_node.syntax_node.byte_range());

                // some languages allow space and newlines before the actual string content
                // so a shebang could be on either the first or second line
                let lines = if let Ok(end) = node_slice.try_line_to_byte(2) {
                    node_slice.byte_slice(..end)
                } else {
                    node_slice
                };

                injection_capture = SHEBANG_REGEX
                    .captures_iter(lines.regex_input())
                    .map(|cap| {
                        let cap = lines.byte_slice(cap.get_group(1).unwrap().range());
                        InjectionLanguageMarker::Shebang(cap.into())
                    })
                    .next()
            } else if capture == self.injection_content_capture {
                content_node = Some(matched_node.syntax_node.clone());
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
        Option<SyntaxTreeNode<'a>>,
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

    // pub fn load_query(
    //     &self,
    //     language: &str,
    //     filename: &str,
    //     read_query_text: impl FnMut(&str, &str) -> String,
    // ) -> Result<Option<Query>, QueryError> {
    //     let query_text = read_query(language, filename, read_query_text);
    //     if query_text.is_empty() {
    //         return Ok(None);
    //     }

    //     Query::new(&self.grammar, &query_text, ).map(Some)
    // }
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

const SHEBANG: &str = r"#!\s*(?:\S*[/\\](?:env\s+(?:\-\S+\s+)*)?)?([^\s\.\d]+)";
static SHEBANG_REGEX: Lazy<rope::Regex> = Lazy::new(|| rope::Regex::new(SHEBANG).unwrap());

struct InjectionSettings {
    include_children: IncludedChildren,
    language: Option<QueryStr>,
}

#[derive(Debug, Clone)]
pub enum InjectionLanguageMarker<'a> {
    Name(Cow<'a, str>),
    Filename(Cow<'a, Path>),
    Shebang(String),
}

#[derive(Clone)]
enum IncludedChildren {
    None,
    All,
    Unnamed,
}

impl Default for IncludedChildren {
    fn default() -> Self {
        Self::None
    }
}
