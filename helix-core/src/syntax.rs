use crate::{
    chars::char_is_line_ending,
    regex::Regex,
    transaction::{ChangeSet, Operation},
    Rope, RopeSlice, Tendril,
};

pub use helix_syntax::get_language;

use arc_swap::ArcSwap;

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    path::Path,
    sync::Arc,
};

use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Option<Regex>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|buf| Regex::new(&buf).map_err(serde::de::Error::custom))
        .transpose()
}

fn deserialize_lsp_config<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<toml::Value>::deserialize(deserializer)?
        .map(|toml| toml.try_into().map_err(serde::de::Error::custom))
        .transpose()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub language: Vec<LanguageConfiguration>,
}

// largely based on tree-sitter/cli/src/loader.rs
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LanguageConfiguration {
    #[serde(rename = "name")]
    pub language_id: String,
    pub scope: String,           // source.rust
    pub file_types: Vec<String>, // filename ends_with? <Gemfile, rb, etc>
    pub roots: Vec<String>,      // these indicate project roots <.git, Cargo.toml>
    pub comment_token: Option<String>,

    #[serde(default, skip_serializing, deserialize_with = "deserialize_lsp_config")]
    pub config: Option<serde_json::Value>,

    #[serde(default)]
    pub auto_format: bool,

    // content_regex
    #[serde(default, skip_serializing, deserialize_with = "deserialize_regex")]
    pub injection_regex: Option<Regex>,
    // first_line_regex
    //
    #[serde(skip)]
    pub(crate) highlight_config: OnceCell<Option<Arc<HighlightConfiguration>>>,
    // tags_config OnceCell<> https://github.com/tree-sitter/tree-sitter/pull/583
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_server: Option<LanguageServerConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent: Option<IndentationConfiguration>,

    #[serde(skip)]
    pub(crate) indent_query: OnceCell<Option<IndentQuery>>,
    #[serde(skip)]
    pub(crate) textobject_query: OnceCell<Option<TextObjectQuery>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LanguageServerConfiguration {
    pub command: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IndentationConfiguration {
    pub tab_width: usize,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IndentQuery {
    #[serde(default)]
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    pub indent: HashSet<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashSet::is_empty")]
    pub outdent: HashSet<String>,
}

#[derive(Debug)]
pub struct TextObjectQuery {
    pub query: Query,
}

impl TextObjectQuery {
    /// Run the query on the given node and return sub nodes which match given
    /// capture ("function.inside", "class.around", etc).
    pub fn capture_nodes<'a>(
        &'a self,
        capture_name: &str,
        node: Node<'a>,
        slice: RopeSlice<'a>,
        cursor: &'a mut QueryCursor,
    ) -> Option<impl Iterator<Item = Node<'a>>> {
        let capture_idx = self.query.capture_index_for_name(capture_name)?;
        let captures = cursor.captures(&self.query, node, RopeProvider(slice));

        captures
            .filter_map(move |(mat, idx)| {
                (mat.captures[idx].index == capture_idx).then(|| mat.captures[idx].node)
            })
            .into()
    }
}

fn load_runtime_file(language: &str, filename: &str) -> Result<String, std::io::Error> {
    let path = crate::RUNTIME_DIR
        .join("queries")
        .join(language)
        .join(filename);
    std::fs::read_to_string(&path)
}

fn read_query(language: &str, filename: &str) -> String {
    static INHERITS_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r";+\s*inherits\s*:?\s*([a-z_,()]+)\s*").unwrap());

    let query = load_runtime_file(language, filename).unwrap_or_default();

    // TODO: the collect() is not ideal
    let inherits = INHERITS_REGEX
        .captures_iter(&query)
        .flat_map(|captures| {
            captures[1]
                .split(',')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    if inherits.is_empty() {
        return query;
    }

    let mut queries = inherits
        .iter()
        .map(|language| read_query(language, filename))
        .collect::<Vec<_>>();

    queries.push(query);

    queries.concat()
}

impl LanguageConfiguration {
    fn initialize_highlight(&self, scopes: &[String]) -> Option<Arc<HighlightConfiguration>> {
        let language = self.language_id.to_ascii_lowercase();

        let highlights_query = read_query(&language, "highlights.scm");
        // always highlight syntax errors
        // highlights_query += "\n(ERROR) @error";

        let injections_query = read_query(&language, "injections.scm");
        let locals_query = read_query(&language, "locals.scm");

        if highlights_query.is_empty() {
            None
        } else {
            let language = get_language(&crate::RUNTIME_DIR, &self.language_id)
                .map_err(|e| log::info!("{}", e))
                .ok()?;
            let config = HighlightConfiguration::new(
                language,
                &highlights_query,
                &injections_query,
                &locals_query,
            );

            let config = match config {
                Ok(config) => config,
                Err(err) => panic!("{}", err),
            }; // TODO: avoid panic
            config.configure(scopes);
            Some(Arc::new(config))
        }
    }

    pub fn reconfigure(&self, scopes: &[String]) {
        if let Some(Some(config)) = self.highlight_config.get() {
            config.configure(scopes);
        }
    }

    pub fn highlight_config(&self, scopes: &[String]) -> Option<Arc<HighlightConfiguration>> {
        self.highlight_config
            .get_or_init(|| self.initialize_highlight(scopes))
            .clone()
    }

    pub fn is_highlight_initialized(&self) -> bool {
        self.highlight_config.get().is_some()
    }

    pub fn indent_query(&self) -> Option<&IndentQuery> {
        self.indent_query
            .get_or_init(|| {
                let language = self.language_id.to_ascii_lowercase();

                let toml = load_runtime_file(&language, "indents.toml").ok()?;
                toml::from_slice(toml.as_bytes()).ok()
            })
            .as_ref()
    }

    pub fn textobject_query(&self) -> Option<&TextObjectQuery> {
        self.textobject_query
            .get_or_init(|| -> Option<TextObjectQuery> {
                let lang_name = self.language_id.to_ascii_lowercase();
                let query_text = read_query(&lang_name, "textobjects.scm");
                let lang = self.highlight_config.get()?.as_ref()?.language;
                let query = Query::new(lang, &query_text).ok()?;
                Some(TextObjectQuery { query })
            })
            .as_ref()
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }
}

#[derive(Debug)]
pub struct Loader {
    // highlight_names ?
    language_configs: Vec<Arc<LanguageConfiguration>>,
    language_config_ids_by_file_type: HashMap<String, usize>, // Vec<usize>
}

impl Loader {
    pub fn new(config: Configuration) -> Self {
        let mut loader = Self {
            language_configs: Vec::new(),
            language_config_ids_by_file_type: HashMap::new(),
        };

        for config in config.language {
            // get the next id
            let language_id = loader.language_configs.len();

            for file_type in &config.file_types {
                // entry().or_insert(Vec::new).push(language_id);
                loader
                    .language_config_ids_by_file_type
                    .insert(file_type.clone(), language_id);
            }

            loader.language_configs.push(Arc::new(config));
        }

        loader
    }

    pub fn language_config_for_file_name(&self, path: &Path) -> Option<Arc<LanguageConfiguration>> {
        // Find all the language configurations that match this file name
        // or a suffix of the file name.
        let configuration_id = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|file_name| self.language_config_ids_by_file_type.get(file_name))
            .or_else(|| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .and_then(|extension| self.language_config_ids_by_file_type.get(extension))
            });

        configuration_id.and_then(|&id| self.language_configs.get(id).cloned())

        // TODO: content_regex handling conflict resolution
    }

    pub fn language_config_for_scope(&self, scope: &str) -> Option<Arc<LanguageConfiguration>> {
        self.language_configs
            .iter()
            .find(|config| config.scope == scope)
            .cloned()
    }

    pub fn language_configuration_for_injection_string(
        &self,
        string: &str,
    ) -> Option<Arc<LanguageConfiguration>> {
        let mut best_match_length = 0;
        let mut best_match_position = None;
        for (i, configuration) in self.language_configs.iter().enumerate() {
            if let Some(injection_regex) = &configuration.injection_regex {
                if let Some(mat) = injection_regex.find(string) {
                    let length = mat.end() - mat.start();
                    if length > best_match_length {
                        best_match_position = Some(i);
                        best_match_length = length;
                    }
                }
            }
        }

        if let Some(i) = best_match_position {
            let configuration = &self.language_configs[i];
            return Some(configuration.clone());
        }
        None
    }
    pub fn language_configs_iter(&self) -> impl Iterator<Item = &Arc<LanguageConfiguration>> {
        self.language_configs.iter()
    }
}

pub struct TsParser {
    parser: tree_sitter::Parser,
    cursors: Vec<QueryCursor>,
}

impl fmt::Debug for TsParser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TsParser").finish()
    }
}

// could also just use a pool, or a single instance?
thread_local! {
    pub static PARSER: RefCell<TsParser> = RefCell::new(TsParser {
        parser: Parser::new(),
        cursors: Vec::new(),
    })
}

#[derive(Debug)]
pub struct Syntax {
    config: Arc<HighlightConfiguration>,

    root_layer: LanguageLayer,
}

fn byte_range_to_str(range: std::ops::Range<usize>, source: RopeSlice) -> Cow<str> {
    let start_char = source.byte_to_char(range.start);
    let end_char = source.byte_to_char(range.end);
    Cow::from(source.slice(start_char..end_char))
}

impl Syntax {
    // buffer, grammar, config, grammars, sync_timeout?
    pub fn new(
        /*language: Lang,*/ source: &Rope,
        config: Arc<HighlightConfiguration>,
    ) -> Self {
        let root_layer = LanguageLayer { tree: None };

        // track markers of injections
        // track scope_descriptor: a Vec of scopes for item in tree

        let mut syntax = Self {
            // grammar,
            config,
            root_layer,
        };

        // update root layer
        PARSER.with(|ts_parser| {
            // TODO: handle the returned `Result` properly.
            let _ = syntax.root_layer.parse(
                &mut ts_parser.borrow_mut(),
                &syntax.config,
                source,
                0,
                vec![Range {
                    start_byte: 0,
                    end_byte: usize::MAX,
                    start_point: Point::new(0, 0),
                    end_point: Point::new(usize::MAX, usize::MAX),
                }],
            );
        });
        syntax
    }

    pub fn update(
        &mut self,
        old_source: &Rope,
        source: &Rope,
        changeset: &ChangeSet,
    ) -> Result<(), Error> {
        PARSER.with(|ts_parser| {
            self.root_layer.update(
                &mut ts_parser.borrow_mut(),
                &self.config,
                old_source,
                source,
                changeset,
            )
        })

        // TODO: deal with injections and update them too
    }

    // fn buffer_changed -> call layer.update(range, new_text) on root layer and then all marker layers

    // call this on transaction.apply() -> buffer_changed(changes)
    //
    // fn parse(language, old_tree, ranges)
    //
    pub fn tree(&self) -> &Tree {
        self.root_layer.tree()
    }
    //
    // <!--update_for_injection(grammar)-->

    // Highlighting

    /// Iterate over the highlighted regions for a given slice of source code.
    pub fn highlight_iter<'a>(
        &'a self,
        source: RopeSlice<'a>,
        range: Option<std::ops::Range<usize>>,
        cancellation_flag: Option<&'a AtomicUsize>,
        injection_callback: impl FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
    ) -> impl Iterator<Item = Result<HighlightEvent, Error>> + 'a {
        // The `captures` iterator borrows the `Tree` and the `QueryCursor`, which
        // prevents them from being moved. But both of these values are really just
        // pointers, so it's actually ok to move them.

        // reuse a cursor from the pool if possible
        let mut cursor = PARSER.with(|ts_parser| {
            let highlighter = &mut ts_parser.borrow_mut();
            highlighter.cursors.pop().unwrap_or_else(QueryCursor::new)
        });
        let tree_ref = self.tree();
        let cursor_ref = unsafe { mem::transmute::<_, &'static mut QueryCursor>(&mut cursor) };
        let query_ref = &self.config.query;
        let config_ref = self.config.as_ref();

        // if reusing cursors & no range this resets to whole range
        cursor_ref.set_byte_range(range.clone().unwrap_or(0..usize::MAX));

        let captures = cursor_ref
            .captures(query_ref, tree_ref.root_node(), RopeProvider(source))
            .peekable();

        // manually craft the root layer based on the existing tree
        let layer = HighlightIterLayer {
            highlight_end_stack: Vec::new(),
            scope_stack: vec![LocalScope {
                inherits: false,
                range: 0..usize::MAX,
                local_defs: Vec::new(),
            }],
            cursor,
            depth: 0,
            _tree: None,
            captures,
            config: config_ref,
            ranges: vec![Range {
                start_byte: 0,
                end_byte: usize::MAX,
                start_point: Point::new(0, 0),
                end_point: Point::new(usize::MAX, usize::MAX),
            }],
        };

        let mut result = HighlightIter {
            source,
            byte_offset: range.map_or(0, |r| r.start), // TODO: simplify
            injection_callback,
            cancellation_flag,
            iter_count: 0,
            layers: vec![layer],
            next_event: None,
            last_highlight_range: None,
        };
        result.sort_layers();
        result
    }
    // on_tokenize
    // on_change_highlighting

    // Commenting
    // comment_strings_for_pos
    // is_commented

    // Indentation
    // suggested_indent_for_line_at_buffer_row
    // suggested_indent_for_buffer_row
    // indent_level_for_line

    // TODO: Folding

    // Syntax APIs
    // get_syntax_node_containing_range ->
    // ...
    // get_syntax_node_at_pos
    // buffer_range_for_scope_at_pos
}

#[derive(Debug)]
pub struct LanguageLayer {
    // mode
    // grammar
    // depth
    pub(crate) tree: Option<Tree>,
}

impl LanguageLayer {
    // pub fn new() -> Self {
    //     Self { tree: None }
    // }

    pub fn tree(&self) -> &Tree {
        // TODO: no unwrap
        self.tree.as_ref().unwrap()
    }

    fn parse(
        &mut self,
        ts_parser: &mut TsParser,
        config: &HighlightConfiguration,
        source: &Rope,
        _depth: usize,
        ranges: Vec<Range>,
    ) -> Result<(), Error> {
        if ts_parser.parser.set_included_ranges(&ranges).is_ok() {
            ts_parser
                .parser
                .set_language(config.language)
                .map_err(|_| Error::InvalidLanguage)?;

            // unsafe { syntax.parser.set_cancellation_flag(cancellation_flag) };
            let tree = ts_parser
                .parser
                .parse_with(
                    &mut |byte, _| {
                        if byte <= source.len_bytes() {
                            let (chunk, start_byte, _, _) = source.chunk_at_byte(byte);
                            chunk[byte - start_byte..].as_bytes()
                        } else {
                            // out of range
                            &[]
                        }
                    },
                    self.tree.as_ref(),
                )
                .ok_or(Error::Cancelled)?;

            self.tree = Some(tree)
        }
        Ok(())
    }

    pub(crate) fn generate_edits(
        old_text: RopeSlice,
        changeset: &ChangeSet,
    ) -> Vec<tree_sitter::InputEdit> {
        use Operation::*;
        let mut old_pos = 0;

        let mut edits = Vec::new();

        let mut iter = changeset.changes.iter().peekable();

        // TODO; this is a lot easier with Change instead of Operation.

        fn point_at_pos(text: RopeSlice, pos: usize) -> (usize, Point) {
            let byte = text.char_to_byte(pos); // <- attempted to index past end
            let line = text.char_to_line(pos);
            let line_start_byte = text.line_to_byte(line);
            let col = byte - line_start_byte;

            (byte, Point::new(line, col))
        }

        fn traverse(point: Point, text: &Tendril) -> Point {
            let Point {
                mut row,
                mut column,
            } = point;

            // TODO: there should be a better way here.
            let mut chars = text.chars().peekable();
            while let Some(ch) = chars.next() {
                if char_is_line_ending(ch) && !(ch == '\r' && chars.peek() == Some(&'\n')) {
                    row += 1;
                    column = 0;
                } else {
                    column += 1;
                }
            }
            Point { row, column }
        }

        while let Some(change) = iter.next() {
            let len = match change {
                Delete(i) | Retain(i) => *i,
                Insert(_) => 0,
            };
            let mut old_end = old_pos + len;

            match change {
                Retain(_) => {}
                Delete(_) => {
                    let (start_byte, start_position) = point_at_pos(old_text, old_pos);
                    let (old_end_byte, old_end_position) = point_at_pos(old_text, old_end);

                    // TODO: Position also needs to be byte based...
                    // let byte = char_to_byte(old_pos)
                    // let line = char_to_line(old_pos)
                    // let line_start_byte = line_to_byte()
                    // Position::new(line, line_start_byte - byte)

                    // deletion
                    edits.push(tree_sitter::InputEdit {
                        start_byte,                       // old_pos to byte
                        old_end_byte,                     // old_end to byte
                        new_end_byte: start_byte,         // old_pos to byte
                        start_position,                   // old pos to coords
                        old_end_position,                 // old_end to coords
                        new_end_position: start_position, // old pos to coords
                    });
                }
                Insert(s) => {
                    let (start_byte, start_position) = point_at_pos(old_text, old_pos);

                    // a subsequent delete means a replace, consume it
                    if let Some(Delete(len)) = iter.peek() {
                        old_end = old_pos + len;
                        let (old_end_byte, old_end_position) = point_at_pos(old_text, old_end);

                        iter.next();

                        // replacement
                        edits.push(tree_sitter::InputEdit {
                            start_byte,                                    // old_pos to byte
                            old_end_byte,                                  // old_end to byte
                            new_end_byte: start_byte + s.len(), // old_pos to byte + s.len()
                            start_position,                     // old pos to coords
                            old_end_position,                   // old_end to coords
                            new_end_position: traverse(start_position, s), // old pos + chars, newlines matter too (iter over)
                        });
                    } else {
                        // insert
                        edits.push(tree_sitter::InputEdit {
                            start_byte,                                    // old_pos to byte
                            old_end_byte: start_byte,                      // same
                            new_end_byte: start_byte + s.len(),            // old_pos + s.len()
                            start_position,                                // old pos to coords
                            old_end_position: start_position,              // same
                            new_end_position: traverse(start_position, s), // old pos + chars, newlines matter too (iter over)
                        });
                    }
                }
            }
            old_pos = old_end;
        }
        edits
    }

    fn update(
        &mut self,
        ts_parser: &mut TsParser,
        config: &HighlightConfiguration,
        old_source: &Rope,
        source: &Rope,
        changeset: &ChangeSet,
    ) -> Result<(), Error> {
        if changeset.is_empty() {
            return Ok(());
        }

        let edits = Self::generate_edits(old_source.slice(..), changeset);

        // Notify the tree about all the changes
        for edit in edits.iter().rev() {
            // apply the edits in reverse. If we applied them in order then edit 1 would disrupt
            // the positioning of edit 2
            self.tree.as_mut().unwrap().edit(edit);
        }

        self.parse(
            ts_parser,
            config,
            source,
            0,
            // TODO: what to do about this range on update
            vec![Range {
                start_byte: 0,
                end_byte: usize::MAX,
                start_point: Point::new(0, 0),
                end_point: Point::new(usize::MAX, usize::MAX),
            }],
        )
    }

    // fn highlight_iter() -> same as Mode but for this layer. Mode composits these
    // fn buffer_changed
    // fn update(range)
    // fn update_injections()
}

// -- refactored from tree-sitter-highlight to be able to retain state
// TODO: add seek() to iter

// problem: any time a layer is updated it must update it's injections on the parent (potentially
// removing some from use)
// can't modify to vec and exist in it at the same time since that would violate borrows
// maybe we can do with an arena
// maybe just caching on the top layer and nevermind the injections for now?
//
// Grammar {
//  layers: Vec<Box<Layer>> to prevent memory moves when vec is modified
// }
// injections tracked by marker:
// if marker areas match it's fine and update
// if not found add new layer
// if length 0 then area got removed, clean up the layer
//
// layer update:
// if range.len = 0 then remove the layer
// for change in changes { tree.edit(change) }
// tree = parser.parse(.., tree, ..)
// calculate affected range and update injections
// injection update:
// look for existing injections
// if present, range = (first injection start, last injection end)
//
// For now cheat and just throw out non-root layers if they exist. This should still improve
// parsing in majority of cases.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::{iter, mem, ops, str, usize};
use tree_sitter::{
    Language as Grammar, Node, Parser, Point, Query, QueryCaptures, QueryCursor, QueryError,
    QueryMatch, Range, TextProvider, Tree,
};

const CANCELLATION_CHECK_INTERVAL: usize = 100;

/// Indicates which highlight should be applied to a region of source code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Highlight(pub usize);

/// Represents the reason why syntax highlighting failed.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    InvalidLanguage,
    Unknown,
}

/// Represents a single step in rendering a syntax-highlighted document.
#[derive(Copy, Clone, Debug)]
pub enum HighlightEvent {
    Source { start: usize, end: usize },
    HighlightStart(Highlight),
    HighlightEnd,
}

/// Contains the data neeeded to higlight code written in a particular language.
///
/// This struct is immutable and can be shared between threads.
#[derive(Debug)]
pub struct HighlightConfiguration {
    pub language: Grammar,
    pub query: Query,
    combined_injections_query: Option<Query>,
    locals_pattern_index: usize,
    highlights_pattern_index: usize,
    highlight_indices: ArcSwap<Vec<Option<Highlight>>>,
    non_local_variable_patterns: Vec<bool>,
    injection_content_capture_index: Option<u32>,
    injection_language_capture_index: Option<u32>,
    local_scope_capture_index: Option<u32>,
    local_def_capture_index: Option<u32>,
    local_def_value_capture_index: Option<u32>,
    local_ref_capture_index: Option<u32>,
}

#[derive(Debug)]
struct LocalDef<'a> {
    name: Cow<'a, str>,
    value_range: ops::Range<usize>,
    highlight: Option<Highlight>,
}

#[derive(Debug)]
struct LocalScope<'a> {
    inherits: bool,
    range: ops::Range<usize>,
    local_defs: Vec<LocalDef<'a>>,
}

#[derive(Debug)]
struct HighlightIter<'a, F>
where
    F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
{
    source: RopeSlice<'a>,
    byte_offset: usize,
    injection_callback: F,
    cancellation_flag: Option<&'a AtomicUsize>,
    layers: Vec<HighlightIterLayer<'a>>,
    iter_count: usize,
    next_event: Option<HighlightEvent>,
    last_highlight_range: Option<(usize, usize, usize)>,
}

// Adapter to convert rope chunks to bytes
struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}
impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}

struct RopeProvider<'a>(RopeSlice<'a>);
impl<'a> TextProvider<'a> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let start_char = self.0.byte_to_char(node.start_byte());
        let end_char = self.0.byte_to_char(node.end_byte());
        let fragment = self.0.slice(start_char..end_char);
        ChunksBytes {
            chunks: fragment.chunks(),
        }
    }
}

struct HighlightIterLayer<'a> {
    _tree: Option<Tree>,
    cursor: QueryCursor,
    captures: iter::Peekable<QueryCaptures<'a, 'a, RopeProvider<'a>>>,
    config: &'a HighlightConfiguration,
    highlight_end_stack: Vec<usize>,
    scope_stack: Vec<LocalScope<'a>>,
    ranges: Vec<Range>,
    depth: usize,
}

impl<'a> fmt::Debug for HighlightIterLayer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HighlightIterLayer").finish()
    }
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
        query_source.push_str(injection_query);
        let locals_query_offset = query_source.len();
        query_source.push_str(locals_query);
        let highlights_query_offset = query_source.len();
        query_source.push_str(highlights_query);

        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let mut query = Query::new(language, &query_source)?;
        let mut locals_pattern_index = 0;
        let mut highlights_pattern_index = 0;
        for i in 0..(query.pattern_count()) {
            let pattern_offset = query.start_byte_for_pattern(i);
            if pattern_offset < highlights_query_offset {
                if pattern_offset < highlights_query_offset {
                    highlights_pattern_index += 1;
                }
                if pattern_offset < locals_query_offset {
                    locals_pattern_index += 1;
                }
            }
        }

        // Construct a separate query just for dealing with the 'combined injections'.
        // Disable the combined injection patterns in the main query.
        let mut combined_injections_query = Query::new(language, injection_query)?;
        let mut has_combined_queries = false;
        for pattern_index in 0..locals_pattern_index {
            let settings = query.property_settings(pattern_index);
            if settings.iter().any(|s| &*s.key == "injection.combined") {
                has_combined_queries = true;
                query.disable_pattern(pattern_index);
            } else {
                combined_injections_query.disable_pattern(pattern_index);
            }
        }
        let combined_injections_query = if has_combined_queries {
            Some(combined_injections_query)
        } else {
            None
        };

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
        let mut local_def_capture_index = None;
        let mut local_def_value_capture_index = None;
        let mut local_ref_capture_index = None;
        let mut local_scope_capture_index = None;
        for (i, name) in query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match name.as_str() {
                "injection.content" => injection_content_capture_index = i,
                "injection.language" => injection_language_capture_index = i,
                "local.definition" => local_def_capture_index = i,
                "local.definition-value" => local_def_value_capture_index = i,
                "local.reference" => local_ref_capture_index = i,
                "local.scope" => local_scope_capture_index = i,
                _ => {}
            }
        }

        let highlight_indices = ArcSwap::from_pointee(vec![None; query.capture_names().len()]);
        Ok(Self {
            language,
            query,
            combined_injections_query,
            locals_pattern_index,
            highlights_pattern_index,
            highlight_indices,
            non_local_variable_patterns,
            injection_content_capture_index,
            injection_language_capture_index,
            local_scope_capture_index,
            local_def_capture_index,
            local_def_value_capture_index,
            local_ref_capture_index,
        })
    }

    /// Get a slice containing all of the highlight names used in the configuration.
    pub fn names(&self) -> &[String] {
        self.query.capture_names()
    }

    /// Set the list of recognized highlight names.
    ///
    /// Tree-sitter syntax-highlighting queries specify highlights in the form of dot-separated
    /// highlight names like `punctuation.bracket` and `function.method.builtin`. Consumers of
    /// these queries can choose to recognize highlights with different levels of specificity.
    /// For example, the string `function.builtin` will match against `function.method.builtin`
    /// and `function.builtin.constructor`, but will not match `function.method`.
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
                    let recognized_name = recognized_name;
                    let mut len = 0;
                    let mut matches = true;
                    for part in recognized_name.split('.') {
                        len += 1;
                        if !capture_parts.contains(&part) {
                            matches = false;
                            break;
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
}

impl<'a> HighlightIterLayer<'a> {
    /// Create a new 'layer' of highlighting for this document.
    ///
    /// In the even that the new layer contains "combined injections" (injections where multiple
    /// disjoint ranges are parsed as one syntax tree), these will be eagerly processed and
    /// added to the returned vector.
    fn new<F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a>(
        source: RopeSlice<'a>,
        cancellation_flag: Option<&'a AtomicUsize>,
        injection_callback: &mut F,
        mut config: &'a HighlightConfiguration,
        mut depth: usize,
        mut ranges: Vec<Range>,
    ) -> Result<Vec<Self>, Error> {
        let mut result = Vec::with_capacity(1);
        let mut queue = Vec::new();
        loop {
            // --> Tree parsing part

            PARSER.with(|ts_parser| {
                let highlighter = &mut ts_parser.borrow_mut();

                if highlighter.parser.set_included_ranges(&ranges).is_ok() {
                    highlighter
                        .parser
                        .set_language(config.language)
                        .map_err(|_| Error::InvalidLanguage)?;

                    unsafe { highlighter.parser.set_cancellation_flag(cancellation_flag) };
                    let tree = highlighter
                        .parser
                        .parse_with(
                            &mut |byte, _| {
                                if byte <= source.len_bytes() {
                                    let (chunk, start_byte, _, _) = source.chunk_at_byte(byte);
                                    chunk[byte - start_byte..].as_bytes()
                                } else {
                                    // out of range
                                    &[]
                                }
                            },
                            None,
                        )
                        .ok_or(Error::Cancelled)?;
                    unsafe { highlighter.parser.set_cancellation_flag(None) };
                    let mut cursor = highlighter.cursors.pop().unwrap_or_else(QueryCursor::new);

                    // Process combined injections.
                    if let Some(combined_injections_query) = &config.combined_injections_query {
                        let mut injections_by_pattern_index = vec![
                            (None, Vec::new(), false);
                            combined_injections_query
                                .pattern_count()
                        ];
                        let matches = cursor.matches(
                            combined_injections_query,
                            tree.root_node(),
                            RopeProvider(source),
                        );
                        for mat in matches {
                            let entry = &mut injections_by_pattern_index[mat.pattern_index];
                            let (language_name, content_node, include_children) =
                                injection_for_match(
                                    config,
                                    combined_injections_query,
                                    &mat,
                                    source,
                                );
                            if language_name.is_some() {
                                entry.0 = language_name;
                            }
                            if let Some(content_node) = content_node {
                                entry.1.push(content_node);
                            }
                            entry.2 = include_children;
                        }
                        for (lang_name, content_nodes, includes_children) in
                            injections_by_pattern_index
                        {
                            if let (Some(lang_name), false) = (lang_name, content_nodes.is_empty())
                            {
                                if let Some(next_config) = (injection_callback)(&lang_name) {
                                    let ranges = Self::intersect_ranges(
                                        &ranges,
                                        &content_nodes,
                                        includes_children,
                                    );
                                    if !ranges.is_empty() {
                                        queue.push((next_config, depth + 1, ranges));
                                    }
                                }
                            }
                        }
                    }

                    // --> Highlighting query part

                    // The `captures` iterator borrows the `Tree` and the `QueryCursor`, which
                    // prevents them from being moved. But both of these values are really just
                    // pointers, so it's actually ok to move them.
                    let tree_ref = unsafe { mem::transmute::<_, &'static Tree>(&tree) };
                    let cursor_ref =
                        unsafe { mem::transmute::<_, &'static mut QueryCursor>(&mut cursor) };
                    let captures = cursor_ref
                        .captures(&config.query, tree_ref.root_node(), RopeProvider(source))
                        .peekable();

                    result.push(HighlightIterLayer {
                        highlight_end_stack: Vec::new(),
                        scope_stack: vec![LocalScope {
                            inherits: false,
                            range: 0..usize::MAX,
                            local_defs: Vec::new(),
                        }],
                        cursor,
                        depth,
                        _tree: Some(tree),
                        captures,
                        config,
                        ranges,
                    });
                }

                Ok(()) // so we can use the try operator
            })?;

            if queue.is_empty() {
                break;
            }

            let (next_config, next_depth, next_ranges) = queue.remove(0);
            config = next_config;
            depth = next_depth;
            ranges = next_ranges;
        }

        Ok(result)
    }

    // Compute the ranges that should be included when parsing an injection.
    // This takes into account three things:
    // * `parent_ranges` - The ranges must all fall within the *current* layer's ranges.
    // * `nodes` - Every injection takes place within a set of nodes. The injection ranges
    //   are the ranges of those nodes.
    // * `includes_children` - For some injections, the content nodes' children should be
    //   excluded from the nested document, so that only the content nodes' *own* content
    //   is reparsed. For other injections, the content nodes' entire ranges should be
    //   reparsed, including the ranges of their children.
    fn intersect_ranges(
        parent_ranges: &[Range],
        nodes: &[Node],
        includes_children: bool,
    ) -> Vec<Range> {
        let mut cursor = nodes[0].walk();
        let mut result = Vec::new();
        let mut parent_range_iter = parent_ranges.iter();
        let mut parent_range = parent_range_iter
            .next()
            .expect("Layers should only be constructed with non-empty ranges vectors");
        for node in nodes.iter() {
            let mut preceding_range = Range {
                start_byte: 0,
                start_point: Point::new(0, 0),
                end_byte: node.start_byte(),
                end_point: node.start_position(),
            };
            let following_range = Range {
                start_byte: node.end_byte(),
                start_point: node.end_position(),
                end_byte: usize::MAX,
                end_point: Point::new(usize::MAX, usize::MAX),
            };

            for excluded_range in node
                .children(&mut cursor)
                .filter_map(|child| {
                    if includes_children {
                        None
                    } else {
                        Some(child.range())
                    }
                })
                .chain([following_range].iter().cloned())
            {
                let mut range = Range {
                    start_byte: preceding_range.end_byte,
                    start_point: preceding_range.end_point,
                    end_byte: excluded_range.start_byte,
                    end_point: excluded_range.start_point,
                };
                preceding_range = excluded_range;

                if range.end_byte < parent_range.start_byte {
                    continue;
                }

                while parent_range.start_byte <= range.end_byte {
                    if parent_range.end_byte > range.start_byte {
                        if range.start_byte < parent_range.start_byte {
                            range.start_byte = parent_range.start_byte;
                            range.start_point = parent_range.start_point;
                        }

                        if parent_range.end_byte < range.end_byte {
                            if range.start_byte < parent_range.end_byte {
                                result.push(Range {
                                    start_byte: range.start_byte,
                                    start_point: range.start_point,
                                    end_byte: parent_range.end_byte,
                                    end_point: parent_range.end_point,
                                });
                            }
                            range.start_byte = parent_range.end_byte;
                            range.start_point = parent_range.end_point;
                        } else {
                            if range.start_byte < range.end_byte {
                                result.push(range);
                            }
                            break;
                        }
                    }

                    if let Some(next_range) = parent_range_iter.next() {
                        parent_range = next_range;
                    } else {
                        return result;
                    }
                }
            }
        }
        result
    }

    // First, sort scope boundaries by their byte offset in the document. At a
    // given position, emit scope endings before scope beginnings. Finally, emit
    // scope boundaries from deeper layers first.
    fn sort_key(&mut self) -> Option<(usize, bool, isize)> {
        let depth = -(self.depth as isize);
        let next_start = self
            .captures
            .peek()
            .map(|(m, i)| m.captures[*i].node.start_byte());
        let next_end = self.highlight_end_stack.last().cloned();
        match (next_start, next_end) {
            (Some(start), Some(end)) => {
                if start < end {
                    Some((start, true, depth))
                } else {
                    Some((end, false, depth))
                }
            }
            (Some(i), None) => Some((i, true, depth)),
            (None, Some(j)) => Some((j, false, depth)),
            _ => None,
        }
    }
}

impl<'a, F> HighlightIter<'a, F>
where
    F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
{
    fn emit_event(
        &mut self,
        offset: usize,
        event: Option<HighlightEvent>,
    ) -> Option<Result<HighlightEvent, Error>> {
        let result;
        if self.byte_offset < offset {
            result = Some(Ok(HighlightEvent::Source {
                start: self.byte_offset,
                end: offset,
            }));
            self.byte_offset = offset;
            self.next_event = event;
        } else {
            result = event.map(Ok);
        }
        self.sort_layers();
        result
    }

    fn sort_layers(&mut self) {
        while !self.layers.is_empty() {
            if let Some(sort_key) = self.layers[0].sort_key() {
                let mut i = 0;
                while i + 1 < self.layers.len() {
                    if let Some(next_offset) = self.layers[i + 1].sort_key() {
                        if next_offset < sort_key {
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }
                if i > 0 {
                    self.layers[0..(i + 1)].rotate_left(1);
                }
                break;
            } else {
                let layer = self.layers.remove(0);
                PARSER.with(|ts_parser| {
                    let highlighter = &mut ts_parser.borrow_mut();
                    highlighter.cursors.push(layer.cursor);
                });
            }
        }
    }

    fn insert_layer(&mut self, mut layer: HighlightIterLayer<'a>) {
        if let Some(sort_key) = layer.sort_key() {
            let mut i = 1;
            while i < self.layers.len() {
                if let Some(sort_key_i) = self.layers[i].sort_key() {
                    if sort_key_i > sort_key {
                        self.layers.insert(i, layer);
                        return;
                    }
                    i += 1;
                } else {
                    self.layers.remove(i);
                }
            }
            self.layers.push(layer);
        }
    }
}

impl<'a, F> Iterator for HighlightIter<'a, F>
where
    F: FnMut(&str) -> Option<&'a HighlightConfiguration> + 'a,
{
    type Item = Result<HighlightEvent, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        'main: loop {
            // If we've already determined the next highlight boundary, just return it.
            if let Some(e) = self.next_event.take() {
                return Some(Ok(e));
            }

            // Periodically check for cancellation, returning `Cancelled` error if the
            // cancellation flag was flipped.
            if let Some(cancellation_flag) = self.cancellation_flag {
                self.iter_count += 1;
                if self.iter_count >= CANCELLATION_CHECK_INTERVAL {
                    self.iter_count = 0;
                    if cancellation_flag.load(Ordering::Relaxed) != 0 {
                        return Some(Err(Error::Cancelled));
                    }
                }
            }

            // If none of the layers have any more highlight boundaries, terminate.
            if self.layers.is_empty() {
                let len = self.source.len_bytes();
                return if self.byte_offset < len {
                    let result = Some(Ok(HighlightEvent::Source {
                        start: self.byte_offset,
                        end: len,
                    }));
                    self.byte_offset = len;
                    result
                } else {
                    None
                };
            }

            // Get the next capture from whichever layer has the earliest highlight boundary.
            let range;
            let layer = &mut self.layers[0];
            if let Some((next_match, capture_index)) = layer.captures.peek() {
                let next_capture = next_match.captures[*capture_index];
                range = next_capture.node.byte_range();

                // If any previous highlight ends before this node starts, then before
                // processing this capture, emit the source code up until the end of the
                // previous highlight, and an end event for that highlight.
                if let Some(end_byte) = layer.highlight_end_stack.last().cloned() {
                    if end_byte <= range.start {
                        layer.highlight_end_stack.pop();
                        return self.emit_event(end_byte, Some(HighlightEvent::HighlightEnd));
                    }
                }
            }
            // If there are no more captures, then emit any remaining highlight end events.
            // And if there are none of those, then just advance to the end of the document.
            else if let Some(end_byte) = layer.highlight_end_stack.last().cloned() {
                layer.highlight_end_stack.pop();
                return self.emit_event(end_byte, Some(HighlightEvent::HighlightEnd));
            } else {
                // return self.emit_event(self.source.len(), None);
                return None;
            };

            let (mut match_, capture_index) = layer.captures.next().unwrap();
            let mut capture = match_.captures[capture_index];

            // If this capture represents an injection, then process the injection.
            if match_.pattern_index < layer.config.locals_pattern_index {
                let (language_name, content_node, include_children) =
                    injection_for_match(layer.config, &layer.config.query, &match_, self.source);

                // Explicitly remove this match so that none of its other captures will remain
                // in the stream of captures.
                match_.remove();

                // If a language is found with the given name, then add a new language layer
                // to the highlighted document.
                if let (Some(language_name), Some(content_node)) = (language_name, content_node) {
                    if let Some(config) = (self.injection_callback)(&language_name) {
                        let ranges = HighlightIterLayer::intersect_ranges(
                            &self.layers[0].ranges,
                            &[content_node],
                            include_children,
                        );
                        if !ranges.is_empty() {
                            match HighlightIterLayer::new(
                                self.source,
                                self.cancellation_flag,
                                &mut self.injection_callback,
                                config,
                                self.layers[0].depth + 1,
                                ranges,
                            ) {
                                Ok(layers) => {
                                    for layer in layers {
                                        self.insert_layer(layer);
                                    }
                                }
                                Err(e) => return Some(Err(e)),
                            }
                        }
                    }
                }

                self.sort_layers();
                continue 'main;
            }

            // Remove from the local scope stack any local scopes that have already ended.
            while range.start > layer.scope_stack.last().unwrap().range.end {
                layer.scope_stack.pop();
            }

            // If this capture is for tracking local variables, then process the
            // local variable info.
            let mut reference_highlight = None;
            let mut definition_highlight = None;
            while match_.pattern_index < layer.config.highlights_pattern_index {
                // If the node represents a local scope, push a new local scope onto
                // the scope stack.
                if Some(capture.index) == layer.config.local_scope_capture_index {
                    definition_highlight = None;
                    let mut scope = LocalScope {
                        inherits: true,
                        range: range.clone(),
                        local_defs: Vec::new(),
                    };
                    for prop in layer.config.query.property_settings(match_.pattern_index) {
                        if let "local.scope-inherits" = prop.key.as_ref() {
                            scope.inherits =
                                prop.value.as_ref().map_or(true, |r| r.as_ref() == "true");
                        }
                    }
                    layer.scope_stack.push(scope);
                }
                // If the node represents a definition, add a new definition to the
                // local scope at the top of the scope stack.
                else if Some(capture.index) == layer.config.local_def_capture_index {
                    reference_highlight = None;
                    let scope = layer.scope_stack.last_mut().unwrap();

                    let mut value_range = 0..0;
                    for capture in match_.captures {
                        if Some(capture.index) == layer.config.local_def_value_capture_index {
                            value_range = capture.node.byte_range();
                        }
                    }

                    let name = byte_range_to_str(range.clone(), self.source);
                    scope.local_defs.push(LocalDef {
                        name,
                        value_range,
                        highlight: None,
                    });
                    definition_highlight = scope.local_defs.last_mut().map(|s| &mut s.highlight);
                }
                // If the node represents a reference, then try to find the corresponding
                // definition in the scope stack.
                else if Some(capture.index) == layer.config.local_ref_capture_index
                    && definition_highlight.is_none()
                {
                    definition_highlight = None;
                    let name = byte_range_to_str(range.clone(), self.source);
                    for scope in layer.scope_stack.iter().rev() {
                        if let Some(highlight) = scope.local_defs.iter().rev().find_map(|def| {
                            if def.name == name && range.start >= def.value_range.end {
                                Some(def.highlight)
                            } else {
                                None
                            }
                        }) {
                            reference_highlight = highlight;
                            break;
                        }
                        if !scope.inherits {
                            break;
                        }
                    }
                }

                // Continue processing any additional matches for the same node.
                if let Some((next_match, next_capture_index)) = layer.captures.peek() {
                    let next_capture = next_match.captures[*next_capture_index];
                    if next_capture.node == capture.node {
                        capture = next_capture;
                        match_ = layer.captures.next().unwrap().0;
                        continue;
                    }
                }

                self.sort_layers();
                continue 'main;
            }

            // Otherwise, this capture must represent a highlight.
            // If this exact range has already been highlighted by an earlier pattern, or by
            // a different layer, then skip over this one.
            if let Some((last_start, last_end, last_depth)) = self.last_highlight_range {
                if range.start == last_start && range.end == last_end && layer.depth < last_depth {
                    self.sort_layers();
                    continue 'main;
                }
            }

            // If the current node was found to be a local variable, then skip over any
            // highlighting patterns that are disabled for local variables.
            if definition_highlight.is_some() || reference_highlight.is_some() {
                while layer.config.non_local_variable_patterns[match_.pattern_index] {
                    if let Some((next_match, next_capture_index)) = layer.captures.peek() {
                        let next_capture = next_match.captures[*next_capture_index];
                        if next_capture.node == capture.node {
                            capture = next_capture;
                            match_ = layer.captures.next().unwrap().0;
                            continue;
                        }
                    }

                    self.sort_layers();
                    continue 'main;
                }
            }

            // Once a highlighting pattern is found for the current node, skip over
            // any later highlighting patterns that also match this node. Captures
            // for a given node are ordered by pattern index, so these subsequent
            // captures are guaranteed to be for highlighting, not injections or
            // local variables.
            while let Some((next_match, next_capture_index)) = layer.captures.peek() {
                let next_capture = next_match.captures[*next_capture_index];
                if next_capture.node == capture.node {
                    layer.captures.next();
                } else {
                    break;
                }
            }

            let current_highlight = layer.config.highlight_indices.load()[capture.index as usize];

            // If this node represents a local definition, then store the current
            // highlight value on the local scope entry representing this node.
            if let Some(definition_highlight) = definition_highlight {
                *definition_highlight = current_highlight;
            }

            // Emit a scope start event and push the node's end position to the stack.
            if let Some(highlight) = reference_highlight.or(current_highlight) {
                self.last_highlight_range = Some((range.start, range.end, layer.depth));
                layer.highlight_end_stack.push(range.end);
                return self
                    .emit_event(range.start, Some(HighlightEvent::HighlightStart(highlight)));
            }

            self.sort_layers();
        }
    }
}

fn injection_for_match<'a>(
    config: &HighlightConfiguration,
    query: &'a Query,
    query_match: &QueryMatch<'a, 'a>,
    source: RopeSlice<'a>,
) -> (Option<Cow<'a, str>>, Option<Node<'a>>, bool) {
    let content_capture_index = config.injection_content_capture_index;
    let language_capture_index = config.injection_language_capture_index;

    let mut language_name = None;
    let mut content_node = None;
    for capture in query_match.captures {
        let index = Some(capture.index);
        if index == language_capture_index {
            let name = byte_range_to_str(capture.node.byte_range(), source);
            language_name = Some(name);
        } else if index == content_capture_index {
            content_node = Some(capture.node);
        }
    }

    let mut include_children = false;
    for prop in query.property_settings(query_match.pattern_index) {
        match prop.key.as_ref() {
            // In addition to specifying the language name via the text of a
            // captured node, it can also be hard-coded via a `#set!` predicate
            // that sets the injection.language key.
            "injection.language" => {
                if language_name.is_none() {
                    language_name = prop.value.as_ref().map(|s| s.as_ref().into())
                }
            }

            // By default, injections do not include the *children* of an
            // `injection.content` node - only the ranges that belong to the
            // node itself. This can be changed using a `#set!` predicate that
            // sets the `injection.include-children` key.
            "injection.include-children" => include_children = true,
            _ => {}
        }
    }

    (language_name, content_node, include_children)
}

// fn shrink_and_clear<T>(vec: &mut Vec<T>, capacity: usize) {
//     if vec.len() > capacity {
//         vec.truncate(capacity);
//         vec.shrink_to_fit();
//     }
//     vec.clear();
// }

pub struct Merge<I> {
    iter: I,
    spans: Box<dyn Iterator<Item = (usize, std::ops::Range<usize>)>>,

    next_event: Option<HighlightEvent>,
    next_span: Option<(usize, std::ops::Range<usize>)>,

    queue: Vec<HighlightEvent>,
}

/// Merge a list of spans into the highlight event stream.
pub fn merge<I: Iterator<Item = HighlightEvent>>(
    iter: I,
    spans: Vec<(usize, std::ops::Range<usize>)>,
) -> Merge<I> {
    let spans = Box::new(spans.into_iter());
    let mut merge = Merge {
        iter,
        spans,
        next_event: None,
        next_span: None,
        queue: Vec::new(),
    };
    merge.next_event = merge.iter.next();
    merge.next_span = merge.spans.next();
    merge
}

impl<I: Iterator<Item = HighlightEvent>> Iterator for Merge<I> {
    type Item = HighlightEvent;
    fn next(&mut self) -> Option<Self::Item> {
        use HighlightEvent::*;
        if let Some(event) = self.queue.pop() {
            return Some(event);
        }

        loop {
            match (self.next_event, &self.next_span) {
                // this happens when range is partially or fully offscreen
                (Some(Source { start, .. }), Some((span, range))) if start > range.start => {
                    if start > range.end {
                        self.next_span = self.spans.next();
                    } else {
                        self.next_span = Some((*span, start..range.end));
                    };
                }
                _ => break,
            }
        }

        match (self.next_event, &self.next_span) {
            (Some(HighlightStart(i)), _) => {
                self.next_event = self.iter.next();
                Some(HighlightStart(i))
            }
            (Some(HighlightEnd), _) => {
                self.next_event = self.iter.next();
                Some(HighlightEnd)
            }
            (Some(Source { start, end }), Some((_, range))) if start < range.start => {
                let intersect = range.start.min(end);
                let event = Source {
                    start,
                    end: intersect,
                };

                if end == intersect {
                    // the event is complete
                    self.next_event = self.iter.next();
                } else {
                    // subslice the event
                    self.next_event = Some(Source {
                        start: intersect,
                        end,
                    });
                };

                Some(event)
            }
            (Some(Source { start, end }), Some((span, range))) if start == range.start => {
                let intersect = range.end.min(end);
                let event = HighlightStart(Highlight(*span));

                // enqueue in reverse order
                self.queue.push(HighlightEnd);
                self.queue.push(Source {
                    start,
                    end: intersect,
                });

                if end == intersect {
                    // the event is complete
                    self.next_event = self.iter.next();
                } else {
                    // subslice the event
                    self.next_event = Some(Source {
                        start: intersect,
                        end,
                    });
                };

                if intersect == range.end {
                    self.next_span = self.spans.next();
                } else {
                    self.next_span = Some((*span, intersect..range.end));
                }

                Some(event)
            }
            (Some(event), None) => {
                self.next_event = self.iter.next();
                Some(event)
            }
            // Can happen if cursor at EOF and/or diagnostic reaches past the end.
            // We need to actually emit events for the cursor-at-EOF situation,
            // even though the range is past the end of the text.  This needs to be
            // handled appropriately by the drawing code by not assuming that
            // all `Source` events point to valid indices in the rope.
            (None, Some((span, range))) => {
                let event = HighlightStart(Highlight(*span));
                self.queue.push(HighlightEnd);
                self.queue.push(Source {
                    start: range.start,
                    end: range.end,
                });
                self.next_span = self.spans.next();
                Some(event)
            }
            (None, None) => None,
            e => unreachable!("{:?}", e),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Rope, Transaction};

    #[test]
    fn test_parser() {
        let highlight_names: Vec<String> = [
            "attribute",
            "constant",
            "function.builtin",
            "function",
            "keyword",
            "operator",
            "property",
            "punctuation",
            "punctuation.bracket",
            "punctuation.delimiter",
            "string",
            "string.special",
            "tag",
            "type",
            "type.builtin",
            "variable",
            "variable.builtin",
            "variable.parameter",
        ]
        .iter()
        .cloned()
        .map(String::from)
        .collect();

        let language = get_language(&crate::RUNTIME_DIR, "Rust").unwrap();
        let config = HighlightConfiguration::new(
            language,
            &std::fs::read_to_string(
                "../helix-syntax/languages/tree-sitter-rust/queries/highlights.scm",
            )
            .unwrap(),
            &std::fs::read_to_string(
                "../helix-syntax/languages/tree-sitter-rust/queries/injections.scm",
            )
            .unwrap(),
            "", // locals.scm
        )
        .unwrap();
        config.configure(&highlight_names);

        let source = Rope::from_str(
            "
            struct Stuff {}
            fn main() {}
        ",
        );
        let syntax = Syntax::new(&source, Arc::new(config));
        let tree = syntax.tree();
        let root = tree.root_node();
        assert_eq!(root.kind(), "source_file");

        assert_eq!(
            root.to_sexp(),
            concat!(
                "(source_file ",
                "(struct_item name: (type_identifier) body: (field_declaration_list)) ",
                "(function_item name: (identifier) parameters: (parameters) body: (block)))"
            )
        );

        let struct_node = root.child(0).unwrap();
        assert_eq!(struct_node.kind(), "struct_item");
    }

    #[test]
    fn test_input_edits() {
        use tree_sitter::InputEdit;

        let doc = Rope::from("hello world!\ntest 123");
        let transaction = Transaction::change(
            &doc,
            vec![(6, 11, Some("test".into())), (12, 17, None)].into_iter(),
        );
        let edits = LanguageLayer::generate_edits(doc.slice(..), transaction.changes());
        // transaction.apply(&mut state);

        assert_eq!(
            edits,
            &[
                InputEdit {
                    start_byte: 6,
                    old_end_byte: 11,
                    new_end_byte: 10,
                    start_position: Point { row: 0, column: 6 },
                    old_end_position: Point { row: 0, column: 11 },
                    new_end_position: Point { row: 0, column: 10 }
                },
                InputEdit {
                    start_byte: 12,
                    old_end_byte: 17,
                    new_end_byte: 12,
                    start_position: Point { row: 0, column: 12 },
                    old_end_position: Point { row: 1, column: 4 },
                    new_end_position: Point { row: 0, column: 12 }
                }
            ]
        );

        // Testing with the official example from tree-sitter
        let mut doc = Rope::from("fn test() {}");
        let transaction =
            Transaction::change(&doc, vec![(8, 8, Some("a: u32".into()))].into_iter());
        let edits = LanguageLayer::generate_edits(doc.slice(..), transaction.changes());
        transaction.apply(&mut doc);

        assert_eq!(doc, "fn test(a: u32) {}");
        assert_eq!(
            edits,
            &[InputEdit {
                start_byte: 8,
                old_end_byte: 8,
                new_end_byte: 14,
                start_position: Point { row: 0, column: 8 },
                old_end_position: Point { row: 0, column: 8 },
                new_end_position: Point { row: 0, column: 14 }
            }]
        );
    }

    #[test]
    fn test_load_runtime_file() {
        // Test to make sure we can load some data from the runtime directory.
        let contents = load_runtime_file("rust", "indents.toml").unwrap();
        assert!(!contents.is_empty());

        let results = load_runtime_file("rust", "does-not-exist");
        assert!(results.is_err());
    }
}
