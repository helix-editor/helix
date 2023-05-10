use crate::{
    auto_pairs::AutoPairs,
    chars::char_is_line_ending,
    diagnostic::Severity,
    regex::Regex,
    transaction::{ChangeSet, Operation},
    Rope, RopeSlice, Tendril,
};

use ahash::RandomState;
use arc_swap::{ArcSwap, Guard};
use bitflags::bitflags;
use hashbrown::raw::RawTable;
use slotmap::{DefaultKey as LayerId, HopSlotMap};

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt,
    hash::{Hash, Hasher},
    mem::{replace, transmute},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};

use helix_loader::grammar::{get_language, load_runtime_file};

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

pub fn deserialize_auto_pairs<'de, D>(deserializer: D) -> Result<Option<AutoPairs>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<AutoPairConfig>::deserialize(deserializer)?.and_then(AutoPairConfig::into))
}

fn default_timeout() -> u64 {
    20
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub language: Vec<LanguageConfiguration>,
}

impl Default for Configuration {
    fn default() -> Self {
        crate::config::default_syntax_loader()
    }
}

// largely based on tree-sitter/cli/src/loader.rs
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LanguageConfiguration {
    #[serde(rename = "name")]
    pub language_id: String, // c-sharp, rust
    pub scope: String,             // source.rust
    pub file_types: Vec<FileType>, // filename extension or ends_with? <Gemfile, rb, etc>
    #[serde(default)]
    pub shebangs: Vec<String>, // interpreter(s) associated with language
    pub roots: Vec<String>,        // these indicate project roots <.git, Cargo.toml>
    pub comment_token: Option<String>,
    pub text_width: Option<usize>,
    pub soft_wrap: Option<SoftWrap>,

    #[serde(default, skip_serializing, deserialize_with = "deserialize_lsp_config")]
    pub config: Option<serde_json::Value>,

    #[serde(default)]
    pub auto_format: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatter: Option<FormatterConfiguration>,

    #[serde(default)]
    pub diagnostic_severity: Severity,

    pub grammar: Option<String>, // tree-sitter grammar name, defaults to language_id

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
    pub(crate) indent_query: OnceCell<Option<Query>>,
    #[serde(skip)]
    pub(crate) textobject_query: OnceCell<Option<TextObjectQuery>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debugger: Option<DebugAdapterConfig>,

    /// Automatic insertion of pairs to parentheses, brackets,
    /// etc. Defaults to true. Optionally, this can be a list of 2-tuples
    /// to specify a list of characters to pair. This overrides the
    /// global setting.
    #[serde(default, skip_serializing, deserialize_with = "deserialize_auto_pairs")]
    pub auto_pairs: Option<AutoPairs>,

    pub rulers: Option<Vec<u16>>, // if set, override editor's rulers

    /// Hardcoded LSP root directories relative to the workspace root, like `examples` or `tools/fuzz`.
    /// Falling back to the current working directory if none are configured.
    pub workspace_lsp_roots: Option<Vec<PathBuf>>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    /// The extension of the file, either the `Path::extension` or the full
    /// filename if the file does not have an extension.
    Extension(String),
    /// The suffix of a file. This is compared to a given file's absolute
    /// path, so it can be used to detect files based on their directories.
    Suffix(String),
}

impl Serialize for FileType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            FileType::Extension(extension) => serializer.serialize_str(extension),
            FileType::Suffix(suffix) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("suffix", &suffix.replace(std::path::MAIN_SEPARATOR, "/"))?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for FileType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct FileTypeVisitor;

        impl<'de> serde::de::Visitor<'de> for FileTypeVisitor {
            type Value = FileType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("string or table")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(FileType::Extension(value.to_string()))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                match map.next_entry::<String, String>()? {
                    Some((key, suffix)) if key == "suffix" => Ok(FileType::Suffix({
                        // FIXME: use `suffix.replace('/', std::path::MAIN_SEPARATOR_STR)`
                        //        if MSRV is updated to 1.68
                        let mut seperator = [0; 1];
                        suffix.replace('/', std::path::MAIN_SEPARATOR.encode_utf8(&mut seperator))
                    })),
                    Some((key, _value)) => Err(serde::de::Error::custom(format!(
                        "unknown key in `file-types` list: {}",
                        key
                    ))),
                    None => Err(serde::de::Error::custom(
                        "expected a `suffix` key in the `file-types` entry",
                    )),
                }
            }
        }

        deserializer.deserialize_any(FileTypeVisitor)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LanguageServerConfiguration {
    pub command: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environment: HashMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    pub language_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FormatterConfiguration {
    pub command: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AdvancedCompletion {
    pub name: Option<String>,
    pub completion: Option<String>,
    pub default: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", untagged)]
pub enum DebugConfigCompletion {
    Named(String),
    Advanced(AdvancedCompletion),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DebugArgumentValue {
    String(String),
    Array(Vec<String>),
    Boolean(bool),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebugTemplate {
    pub name: String,
    pub request: String,
    pub completion: Vec<DebugConfigCompletion>,
    pub args: HashMap<String, DebugArgumentValue>,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebugAdapterConfig {
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub port_arg: Option<String>,
    pub templates: Vec<DebugTemplate>,
    #[serde(default)]
    pub quirks: DebuggerQuirks,
}

// Different workarounds for adapters' differences
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct DebuggerQuirks {
    #[serde(default)]
    pub absolute_paths: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IndentationConfiguration {
    pub tab_width: usize,
    pub unit: String,
}

/// Configuration for auto pairs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, untagged)]
pub enum AutoPairConfig {
    /// Enables or disables auto pairing. False means disabled. True means to use the default pairs.
    Enable(bool),

    /// The mappings of pairs.
    Pairs(HashMap<char, char>),
}

impl Default for AutoPairConfig {
    fn default() -> Self {
        AutoPairConfig::Enable(true)
    }
}

impl From<&AutoPairConfig> for Option<AutoPairs> {
    fn from(auto_pair_config: &AutoPairConfig) -> Self {
        match auto_pair_config {
            AutoPairConfig::Enable(false) => None,
            AutoPairConfig::Enable(true) => Some(AutoPairs::default()),
            AutoPairConfig::Pairs(pairs) => Some(AutoPairs::new(pairs.iter())),
        }
    }
}

impl From<AutoPairConfig> for Option<AutoPairs> {
    fn from(auto_pairs_config: AutoPairConfig) -> Self {
        (&auto_pairs_config).into()
    }
}

impl FromStr for AutoPairConfig {
    type Err = std::str::ParseBoolError;

    // only do bool parsing for runtime setting
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let enable: bool = s.parse()?;
        Ok(AutoPairConfig::Enable(enable))
    }
}

#[derive(Debug)]
pub struct TextObjectQuery {
    pub query: Query,
}

#[derive(Debug)]
pub enum CapturedNode<'a> {
    Single(Node<'a>),
    /// Guaranteed to be not empty
    Grouped(Vec<Node<'a>>),
}

impl<'a> CapturedNode<'a> {
    pub fn start_byte(&self) -> usize {
        match self {
            Self::Single(n) => n.start_byte(),
            Self::Grouped(ns) => ns[0].start_byte(),
        }
    }

    pub fn end_byte(&self) -> usize {
        match self {
            Self::Single(n) => n.end_byte(),
            Self::Grouped(ns) => ns.last().unwrap().end_byte(),
        }
    }

    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.start_byte()..self.end_byte()
    }
}

/// The maximum number of in-progress matches a TS cursor can consider at once.
/// This is set to a constant in order to avoid performance problems for medium to large files. Set with `set_match_limit`.
/// Using such a limit means that we lose valid captures, so there is fundamentally a tradeoff here.
///
///
/// Old tree sitter versions used a limit of 32 by default until this limit was removed in version `0.19.5` (must now be set manually).
/// However, this causes performance issues for medium to large files.
/// In helix, this problem caused treesitter motions to take multiple seconds to complete in medium-sized rust files (3k loc).
///
///
/// Neovim also encountered this problem and reintroduced this limit after it was removed upstream
/// (see <https://github.com/neovim/neovim/issues/14897> and <https://github.com/neovim/neovim/pull/14915>).
/// The number used here is fundamentally a tradeoff between breaking some obscure edge cases and performance.
///
///
/// Neovim chose 64 for this value somewhat arbitrarily (<https://github.com/neovim/neovim/pull/18397>).
/// 64 is too low for some languages though. In particular, it breaks some highlighting for record fields in Erlang record definitions.
/// This number can be increased if new syntax highlight breakages are found, as long as the performance penalty is not too high.
const TREE_SITTER_MATCH_LIMIT: u32 = 256;

impl TextObjectQuery {
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
        node: Node<'a>,
        slice: RopeSlice<'a>,
        cursor: &'a mut QueryCursor,
    ) -> Option<impl Iterator<Item = CapturedNode<'a>>> {
        self.capture_nodes_any(&[capture_name], node, slice, cursor)
    }

    /// Find the first capture that exists out of all given `capture_names`
    /// and return sub nodes that match this capture.
    pub fn capture_nodes_any<'a>(
        &'a self,
        capture_names: &[&str],
        node: Node<'a>,
        slice: RopeSlice<'a>,
        cursor: &'a mut QueryCursor,
    ) -> Option<impl Iterator<Item = CapturedNode<'a>>> {
        let capture_idx = capture_names
            .iter()
            .find_map(|cap| self.query.capture_index_for_name(cap))?;

        cursor.set_match_limit(TREE_SITTER_MATCH_LIMIT);

        let nodes = cursor
            .captures(&self.query, node, RopeProvider(slice))
            .filter_map(move |(mat, _)| {
                let nodes: Vec<_> = mat
                    .captures
                    .iter()
                    .filter_map(|cap| (cap.index == capture_idx).then_some(cap.node))
                    .collect();

                if nodes.len() > 1 {
                    Some(CapturedNode::Grouped(nodes))
                } else {
                    nodes.into_iter().map(CapturedNode::Single).next()
                }
            });

        Some(nodes)
    }
}

pub fn read_query(language: &str, filename: &str) -> String {
    static INHERITS_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r";+\s*inherits\s*:?\s*([a-z_,()-]+)\s*").unwrap());

    let query = load_runtime_file(language, filename).unwrap_or_default();

    // replaces all "; inherits <language>(,<language>)*" with the queries of the given language(s)
    INHERITS_REGEX
        .replace_all(&query, |captures: &regex::Captures| {
            captures[1]
                .split(',')
                .map(|language| format!("\n{}\n", read_query(language, filename)))
                .collect::<String>()
        })
        .to_string()
}

impl LanguageConfiguration {
    fn initialize_highlight(&self, scopes: &[String]) -> Option<Arc<HighlightConfiguration>> {
        let highlights_query = read_query(&self.language_id, "highlights.scm");
        // always highlight syntax errors
        // highlights_query += "\n(ERROR) @error";

        let injections_query = read_query(&self.language_id, "injections.scm");
        let locals_query = read_query(&self.language_id, "locals.scm");

        if highlights_query.is_empty() {
            None
        } else {
            let language = get_language(self.grammar.as_deref().unwrap_or(&self.language_id))
                .map_err(|err| {
                    log::error!(
                        "Failed to load tree-sitter parser for language {:?}: {}",
                        self.language_id,
                        err
                    )
                })
                .ok()?;
            let config = HighlightConfiguration::new(
                language,
                &highlights_query,
                &injections_query,
                &locals_query,
            )
            .map_err(|err| log::error!("Could not parse queries for language {:?}. Are your grammars out of sync? Try running 'hx --grammar fetch' and 'hx --grammar build'. This query could not be parsed: {:?}", self.language_id, err))
            .ok()?;

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

    pub fn indent_query(&self) -> Option<&Query> {
        self.indent_query
            .get_or_init(|| self.load_query("indents.scm"))
            .as_ref()
    }

    pub fn textobject_query(&self) -> Option<&TextObjectQuery> {
        self.textobject_query
            .get_or_init(|| {
                self.load_query("textobjects.scm")
                    .map(|query| TextObjectQuery { query })
            })
            .as_ref()
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    fn load_query(&self, kind: &str) -> Option<Query> {
        let query_text = read_query(&self.language_id, kind);
        if query_text.is_empty() {
            return None;
        }
        let lang = self.highlight_config.get()?.as_ref()?.language;
        Query::new(lang, &query_text)
            .map_err(|e| {
                log::error!(
                    "Failed to parse {} queries for {}: {}",
                    kind,
                    self.language_id,
                    e
                )
            })
            .ok()
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct SoftWrap {
    /// Soft wrap lines that exceed viewport width. Default to off
    // NOTE: Option on purpose because the struct is shared between language config and global config.
    // By default the option is None so that the language config falls back to the global config unless explicitly set.
    pub enable: Option<bool>,
    /// Maximum space left free at the end of the line.
    /// This space is used to wrap text at word boundaries. If that is not possible within this limit
    /// the word is simply split at the end of the line.
    ///
    /// This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
    ///
    /// Default to 20
    pub max_wrap: Option<u16>,
    /// Maximum number of indentation that can be carried over from the previous line when softwrapping.
    /// If a line is indented further then this limit it is rendered at the start of the viewport instead.
    ///
    /// This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
    ///
    /// Default to 40
    pub max_indent_retain: Option<u16>,
    /// Indicator placed at the beginning of softwrapped lines
    ///
    /// Defaults to ↪
    pub wrap_indicator: Option<String>,
    /// Softwrap at `text_width` instead of viewport width if it is shorter
    pub wrap_at_text_width: Option<bool>,
}

// Expose loader as Lazy<> global since it's always static?

#[derive(Debug)]
pub struct Loader {
    // highlight_names ?
    language_configs: Vec<Arc<LanguageConfiguration>>,
    language_config_ids_by_extension: HashMap<String, usize>, // Vec<usize>
    language_config_ids_by_suffix: HashMap<String, usize>,
    language_config_ids_by_shebang: HashMap<String, usize>,

    scopes: ArcSwap<Vec<String>>,
}

impl Loader {
    pub fn new(config: Configuration) -> Self {
        let mut loader = Self {
            language_configs: Vec::new(),
            language_config_ids_by_extension: HashMap::new(),
            language_config_ids_by_suffix: HashMap::new(),
            language_config_ids_by_shebang: HashMap::new(),
            scopes: ArcSwap::from_pointee(Vec::new()),
        };

        for config in config.language {
            // get the next id
            let language_id = loader.language_configs.len();

            for file_type in &config.file_types {
                // entry().or_insert(Vec::new).push(language_id);
                match file_type {
                    FileType::Extension(extension) => loader
                        .language_config_ids_by_extension
                        .insert(extension.clone(), language_id),
                    FileType::Suffix(suffix) => loader
                        .language_config_ids_by_suffix
                        .insert(suffix.clone(), language_id),
                };
            }
            for shebang in &config.shebangs {
                loader
                    .language_config_ids_by_shebang
                    .insert(shebang.clone(), language_id);
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
            .and_then(|file_name| self.language_config_ids_by_extension.get(file_name))
            .or_else(|| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .and_then(|extension| self.language_config_ids_by_extension.get(extension))
            })
            .or_else(|| {
                self.language_config_ids_by_suffix
                    .iter()
                    .find_map(|(file_type, id)| {
                        if path.to_str()?.ends_with(file_type) {
                            Some(id)
                        } else {
                            None
                        }
                    })
            });

        configuration_id.and_then(|&id| self.language_configs.get(id).cloned())

        // TODO: content_regex handling conflict resolution
    }

    pub fn language_config_for_shebang(&self, source: &Rope) -> Option<Arc<LanguageConfiguration>> {
        let line = Cow::from(source.line(0));
        static SHEBANG_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(&["^", SHEBANG].concat()).unwrap());
        let configuration_id = SHEBANG_REGEX
            .captures(&line)
            .and_then(|cap| self.language_config_ids_by_shebang.get(&cap[1]));

        configuration_id.and_then(|&id| self.language_configs.get(id).cloned())
    }

    pub fn language_config_for_scope(&self, scope: &str) -> Option<Arc<LanguageConfiguration>> {
        self.language_configs
            .iter()
            .find(|config| config.scope == scope)
            .cloned()
    }

    pub fn language_config_for_language_id(&self, id: &str) -> Option<Arc<LanguageConfiguration>> {
        self.language_configs
            .iter()
            .find(|config| config.language_id == id)
            .cloned()
    }

    /// Unlike language_config_for_language_id, which only returns Some for an exact id, this
    /// function will perform a regex match on the given string to find the closest language match.
    pub fn language_config_for_name(&self, name: &str) -> Option<Arc<LanguageConfiguration>> {
        let mut best_match_length = 0;
        let mut best_match_position = None;
        for (i, configuration) in self.language_configs.iter().enumerate() {
            if let Some(injection_regex) = &configuration.injection_regex {
                if let Some(mat) = injection_regex.find(name) {
                    let length = mat.end() - mat.start();
                    if length > best_match_length {
                        best_match_position = Some(i);
                        best_match_length = length;
                    }
                }
            }
        }

        best_match_position.map(|i| self.language_configs[i].clone())
    }

    pub fn language_configuration_for_injection_string(
        &self,
        capture: &InjectionLanguageMarker,
    ) -> Option<Arc<LanguageConfiguration>> {
        match capture {
            InjectionLanguageMarker::Name(string) => self.language_config_for_name(string),
            InjectionLanguageMarker::Filename(file) => self.language_config_for_file_name(file),
            InjectionLanguageMarker::Shebang(shebang) => {
                self.language_config_for_language_id(shebang)
            }
        }
    }

    pub fn language_configs(&self) -> impl Iterator<Item = &Arc<LanguageConfiguration>> {
        self.language_configs.iter()
    }

    pub fn set_scopes(&self, scopes: Vec<String>) {
        self.scopes.store(Arc::new(scopes));

        // Reconfigure existing grammars
        for config in self
            .language_configs
            .iter()
            .filter(|cfg| cfg.is_highlight_initialized())
        {
            config.reconfigure(&self.scopes());
        }
    }

    pub fn scopes(&self) -> Guard<Arc<Vec<String>>> {
        self.scopes.load()
    }
}

pub struct TsParser {
    parser: tree_sitter::Parser,
    pub cursors: Vec<QueryCursor>,
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
    layers: HopSlotMap<LayerId, LanguageLayer>,
    root: LayerId,
    loader: Arc<Loader>,
}

fn byte_range_to_str(range: std::ops::Range<usize>, source: RopeSlice) -> Cow<str> {
    Cow::from(source.byte_slice(range))
}

impl Syntax {
    pub fn new(source: &Rope, config: Arc<HighlightConfiguration>, loader: Arc<Loader>) -> Self {
        let root_layer = LanguageLayer {
            tree: None,
            config,
            depth: 0,
            flags: LayerUpdateFlags::empty(),
            ranges: vec![Range {
                start_byte: 0,
                end_byte: usize::MAX,
                start_point: Point::new(0, 0),
                end_point: Point::new(usize::MAX, usize::MAX),
            }],
        };

        // track scope_descriptor: a Vec of scopes for item in tree

        let mut layers = HopSlotMap::default();
        let root = layers.insert(root_layer);

        let mut syntax = Self {
            root,
            layers,
            loader,
        };

        syntax
            .update(source, source, &ChangeSet::new(source))
            .unwrap();

        syntax
    }

    pub fn update(
        &mut self,
        old_source: &Rope,
        source: &Rope,
        changeset: &ChangeSet,
    ) -> Result<(), Error> {
        let mut queue = VecDeque::new();
        queue.push_back(self.root);

        let scopes = self.loader.scopes.load();
        let injection_callback = |language: &InjectionLanguageMarker| {
            self.loader
                .language_configuration_for_injection_string(language)
                .and_then(|language_config| language_config.highlight_config(&scopes))
        };

        // Convert the changeset into tree sitter edits.
        let edits = generate_edits(old_source, changeset);

        // This table allows inverse indexing of `layers`.
        // That is by hashing a `Layer` you can find
        // the `LayerId` of an existing equivalent `Layer` in `layers`.
        //
        // It is used to determine if a new layer exists for an injection
        // or if an existing layer needs to be updated.
        let mut layers_table = RawTable::with_capacity(self.layers.len());
        let layers_hasher = RandomState::new();
        // Use the edits to update all layers markers
        fn point_add(a: Point, b: Point) -> Point {
            if b.row > 0 {
                Point::new(a.row.saturating_add(b.row), b.column)
            } else {
                Point::new(0, a.column.saturating_add(b.column))
            }
        }
        fn point_sub(a: Point, b: Point) -> Point {
            if a.row > b.row {
                Point::new(a.row.saturating_sub(b.row), a.column)
            } else {
                Point::new(0, a.column.saturating_sub(b.column))
            }
        }

        for (layer_id, layer) in self.layers.iter_mut() {
            // The root layer always covers the whole range (0..usize::MAX)
            if layer.depth == 0 {
                layer.flags = LayerUpdateFlags::MODIFIED;
                continue;
            }

            if !edits.is_empty() {
                for range in &mut layer.ranges {
                    // Roughly based on https://github.com/tree-sitter/tree-sitter/blob/ddeaa0c7f534268b35b4f6cb39b52df082754413/lib/src/subtree.c#L691-L720
                    for edit in edits.iter().rev() {
                        let is_pure_insertion = edit.old_end_byte == edit.start_byte;

                        // if edit is after range, skip
                        if edit.start_byte > range.end_byte {
                            // TODO: || (is_noop && edit.start_byte == range.end_byte)
                            continue;
                        }

                        // if edit is before range, shift entire range by len
                        if edit.old_end_byte < range.start_byte {
                            range.start_byte =
                                edit.new_end_byte + (range.start_byte - edit.old_end_byte);
                            range.start_point = point_add(
                                edit.new_end_position,
                                point_sub(range.start_point, edit.old_end_position),
                            );

                            range.end_byte = edit
                                .new_end_byte
                                .saturating_add(range.end_byte - edit.old_end_byte);
                            range.end_point = point_add(
                                edit.new_end_position,
                                point_sub(range.end_point, edit.old_end_position),
                            );

                            layer.flags |= LayerUpdateFlags::MOVED;
                        }
                        // if the edit starts in the space before and extends into the range
                        else if edit.start_byte < range.start_byte {
                            range.start_byte = edit.new_end_byte;
                            range.start_point = edit.new_end_position;

                            range.end_byte = range
                                .end_byte
                                .saturating_sub(edit.old_end_byte)
                                .saturating_add(edit.new_end_byte);
                            range.end_point = point_add(
                                edit.new_end_position,
                                point_sub(range.end_point, edit.old_end_position),
                            );
                            layer.flags = LayerUpdateFlags::MODIFIED;
                        }
                        // If the edit is an insertion at the start of the tree, shift
                        else if edit.start_byte == range.start_byte && is_pure_insertion {
                            range.start_byte = edit.new_end_byte;
                            range.start_point = edit.new_end_position;
                            layer.flags |= LayerUpdateFlags::MOVED;
                        } else {
                            range.end_byte = range
                                .end_byte
                                .saturating_sub(edit.old_end_byte)
                                .saturating_add(edit.new_end_byte);
                            range.end_point = point_add(
                                edit.new_end_position,
                                point_sub(range.end_point, edit.old_end_position),
                            );
                            layer.flags = LayerUpdateFlags::MODIFIED;
                        }
                    }
                }
            }

            let hash = layers_hasher.hash_one(layer);
            // Safety: insert_no_grow is unsafe because it assumes that the table
            // has enough capacity to hold additional elements.
            // This is always the case as we reserved enough capacity above.
            unsafe { layers_table.insert_no_grow(hash, layer_id) };
        }

        PARSER.with(|ts_parser| {
            let ts_parser = &mut ts_parser.borrow_mut();
            let mut cursor = ts_parser.cursors.pop().unwrap_or_else(QueryCursor::new);
            // TODO: might need to set cursor range
            cursor.set_byte_range(0..usize::MAX);
            cursor.set_match_limit(TREE_SITTER_MATCH_LIMIT);

            let source_slice = source.slice(..);

            while let Some(layer_id) = queue.pop_front() {
                let layer = &mut self.layers[layer_id];

                // Mark the layer as touched
                layer.flags |= LayerUpdateFlags::TOUCHED;

                // If a tree already exists, notify it of changes.
                if let Some(tree) = &mut layer.tree {
                    if layer
                        .flags
                        .intersects(LayerUpdateFlags::MODIFIED | LayerUpdateFlags::MOVED)
                    {
                        for edit in edits.iter().rev() {
                            // Apply the edits in reverse.
                            // If we applied them in order then edit 1 would disrupt the positioning of edit 2.
                            tree.edit(edit);
                        }
                    }

                    if layer.flags.contains(LayerUpdateFlags::MODIFIED) {
                        // Re-parse the tree.
                        layer.parse(&mut ts_parser.parser, source)?;
                    }
                } else {
                    // always parse if this layer has never been parsed before
                    layer.parse(&mut ts_parser.parser, source)?;
                }

                // Switch to an immutable borrow.
                let layer = &self.layers[layer_id];

                // Process injections.
                let matches = cursor.matches(
                    &layer.config.injections_query,
                    layer.tree().root_node(),
                    RopeProvider(source_slice),
                );
                let mut injections = Vec::new();
                for mat in matches {
                    let (injection_capture, content_node, included_children) = layer
                        .config
                        .injection_for_match(&layer.config.injections_query, &mat, source_slice);

                    // Explicitly remove this match so that none of its other captures will remain
                    // in the stream of captures.
                    mat.remove();

                    // If a language is found with the given name, then add a new language layer
                    // to the highlighted document.
                    if let (Some(injection_capture), Some(content_node)) =
                        (injection_capture, content_node)
                    {
                        if let Some(config) = (injection_callback)(&injection_capture) {
                            let ranges =
                                intersect_ranges(&layer.ranges, &[content_node], included_children);

                            if !ranges.is_empty() {
                                injections.push((config, ranges));
                            }
                        }
                    }
                }

                // Process combined injections.
                if let Some(combined_injections_query) = &layer.config.combined_injections_query {
                    let mut injections_by_pattern_index =
                        vec![
                            (None, Vec::new(), IncludedChildren::default());
                            combined_injections_query.pattern_count()
                        ];
                    let matches = cursor.matches(
                        combined_injections_query,
                        layer.tree().root_node(),
                        RopeProvider(source_slice),
                    );
                    for mat in matches {
                        let entry = &mut injections_by_pattern_index[mat.pattern_index];
                        let (injection_capture, content_node, included_children) = layer
                            .config
                            .injection_for_match(combined_injections_query, &mat, source_slice);
                        if injection_capture.is_some() {
                            entry.0 = injection_capture;
                        }
                        if let Some(content_node) = content_node {
                            entry.1.push(content_node);
                        }
                        entry.2 = included_children;
                    }
                    for (lang_name, content_nodes, included_children) in injections_by_pattern_index
                    {
                        if let (Some(lang_name), false) = (lang_name, content_nodes.is_empty()) {
                            if let Some(config) = (injection_callback)(&lang_name) {
                                let ranges = intersect_ranges(
                                    &layer.ranges,
                                    &content_nodes,
                                    included_children,
                                );
                                if !ranges.is_empty() {
                                    injections.push((config, ranges));
                                }
                            }
                        }
                    }
                }

                let depth = layer.depth + 1;
                // TODO: can't inline this since matches borrows self.layers
                for (config, ranges) in injections {
                    let new_layer = LanguageLayer {
                        tree: None,
                        config,
                        depth,
                        ranges,
                        flags: LayerUpdateFlags::empty(),
                    };

                    // Find an identical existing layer
                    let layer = layers_table
                        .get(layers_hasher.hash_one(&new_layer), |&it| {
                            self.layers[it] == new_layer
                        })
                        .copied();

                    // ...or insert a new one.
                    let layer_id = layer.unwrap_or_else(|| self.layers.insert(new_layer));

                    queue.push_back(layer_id);
                }

                // TODO: pre-process local scopes at this time, rather than highlight?
                // would solve problems with locals not working across boundaries
            }

            // Return the cursor back in the pool.
            ts_parser.cursors.push(cursor);

            // Reset all `LayerUpdateFlags` and remove all untouched layers
            self.layers.retain(|_, layer| {
                replace(&mut layer.flags, LayerUpdateFlags::empty())
                    .contains(LayerUpdateFlags::TOUCHED)
            });

            Ok(())
        })
    }

    pub fn tree(&self) -> &Tree {
        self.layers[self.root].tree()
    }

    /// Iterate over the highlighted regions for a given slice of source code.
    pub fn highlight_iter<'a>(
        &'a self,
        source: RopeSlice<'a>,
        range: Option<std::ops::Range<usize>>,
        cancellation_flag: Option<&'a AtomicUsize>,
    ) -> impl Iterator<Item = Result<HighlightEvent, Error>> + 'a {
        let mut layers = self
            .layers
            .iter()
            .filter_map(|(_, layer)| {
                // TODO: if range doesn't overlap layer range, skip it

                // Reuse a cursor from the pool if available.
                let mut cursor = PARSER.with(|ts_parser| {
                    let highlighter = &mut ts_parser.borrow_mut();
                    highlighter.cursors.pop().unwrap_or_else(QueryCursor::new)
                });

                // The `captures` iterator borrows the `Tree` and the `QueryCursor`, which
                // prevents them from being moved. But both of these values are really just
                // pointers, so it's actually ok to move them.
                let cursor_ref =
                    unsafe { mem::transmute::<_, &'static mut QueryCursor>(&mut cursor) };

                // if reusing cursors & no range this resets to whole range
                cursor_ref.set_byte_range(range.clone().unwrap_or(0..usize::MAX));
                cursor_ref.set_match_limit(TREE_SITTER_MATCH_LIMIT);

                let mut captures = cursor_ref
                    .captures(
                        &layer.config.query,
                        layer.tree().root_node(),
                        RopeProvider(source),
                    )
                    .peekable();

                // If there's no captures, skip the layer
                captures.peek()?;

                Some(HighlightIterLayer {
                    highlight_end_stack: Vec::new(),
                    scope_stack: vec![LocalScope {
                        inherits: false,
                        range: 0..usize::MAX,
                        local_defs: Vec::new(),
                    }],
                    cursor,
                    _tree: None,
                    captures: RefCell::new(captures),
                    config: layer.config.as_ref(), // TODO: just reuse `layer`
                    depth: layer.depth,            // TODO: just reuse `layer`
                })
            })
            .collect::<Vec<_>>();

        layers.sort_unstable_by_key(|layer| layer.sort_key());

        let mut result = HighlightIter {
            source,
            byte_offset: range.map_or(0, |r| r.start),
            cancellation_flag,
            iter_count: 0,
            layers,
            next_event: None,
            last_highlight_range: None,
        };
        result.sort_layers();
        result
    }

    // Commenting
    // comment_strings_for_pos
    // is_commented

    // Indentation
    // suggested_indent_for_line_at_buffer_row
    // suggested_indent_for_buffer_row
    // indent_level_for_line

    // TODO: Folding
}

bitflags! {
    /// Flags that track the status of a layer
    /// in the `Sytaxn::update` function
    #[derive(Debug)]
    struct LayerUpdateFlags : u32{
        const MODIFIED = 0b001;
        const MOVED = 0b010;
        const TOUCHED = 0b100;
    }
}

#[derive(Debug)]
pub struct LanguageLayer {
    // mode
    // grammar
    pub config: Arc<HighlightConfiguration>,
    pub(crate) tree: Option<Tree>,
    pub ranges: Vec<Range>,
    pub depth: u32,
    flags: LayerUpdateFlags,
}

/// This PartialEq implementation only checks if that
/// two layers are theoretically identical (meaning they highlight the same text range with the same language).
/// It does not check whether the layers have the same internal treesitter
/// state.
impl PartialEq for LanguageLayer {
    fn eq(&self, other: &Self) -> bool {
        self.depth == other.depth
            && self.config.language == other.config.language
            && self.ranges == other.ranges
    }
}

/// Hash implementation belongs to PartialEq implementation above.
/// See its documentation for details.
impl Hash for LanguageLayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.depth.hash(state);
        // The transmute is necessary here because tree_sitter::Language does not derive Hash at the moment.
        // However it does use #[repr] transparent so the transmute here is safe
        // as `Language` (which `Grammar` is an alias for) is just a newtype wrapper around a (thin) pointer.
        // This is also compatible with the PartialEq implementation of language
        // as that is just a pointer comparison.
        let language: *const () = unsafe { transmute(self.config.language) };
        language.hash(state);
        self.ranges.hash(state);
    }
}

impl LanguageLayer {
    pub fn tree(&self) -> &Tree {
        // TODO: no unwrap
        self.tree.as_ref().unwrap()
    }

    fn parse(&mut self, parser: &mut Parser, source: &Rope) -> Result<(), Error> {
        parser
            .set_included_ranges(&self.ranges)
            .map_err(|_| Error::InvalidRanges)?;

        parser
            .set_language(self.config.language)
            .map_err(|_| Error::InvalidLanguage)?;

        // unsafe { syntax.parser.set_cancellation_flag(cancellation_flag) };
        let tree = parser
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
        // unsafe { ts_parser.parser.set_cancellation_flag(None) };
        self.tree = Some(tree);
        Ok(())
    }
}

pub(crate) fn generate_edits(
    old_text: &Rope,
    changeset: &ChangeSet,
) -> Vec<tree_sitter::InputEdit> {
    use Operation::*;
    let mut old_pos = 0;

    let mut edits = Vec::new();

    if changeset.changes.is_empty() {
        return edits;
    }

    let mut iter = changeset.changes.iter().peekable();

    // TODO; this is a lot easier with Change instead of Operation.

    fn point_at_pos(text: &Rope, pos: usize) -> (usize, Point) {
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
                        new_end_byte: start_byte + s.len(),            // old_pos to byte + s.len()
                        start_position,                                // old pos to coords
                        old_end_position,                              // old_end to coords
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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::{iter, mem, ops, str, usize};
use tree_sitter::{
    Language as Grammar, Node, Parser, Point, Query, QueryCaptures, QueryCursor, QueryError,
    QueryMatch, Range, TextProvider, Tree, TreeCursor,
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
    InvalidRanges,
    Unknown,
}

/// Represents a single step in rendering a syntax-highlighted document.
#[derive(Copy, Clone, Debug)]
pub enum HighlightEvent {
    Source { start: usize, end: usize },
    HighlightStart(Highlight),
    HighlightEnd,
}

/// Contains the data needed to highlight code written in a particular language.
///
/// This struct is immutable and can be shared between threads.
#[derive(Debug)]
pub struct HighlightConfiguration {
    pub language: Grammar,
    pub query: Query,
    injections_query: Query,
    combined_injections_query: Option<Query>,
    highlights_pattern_index: usize,
    highlight_indices: ArcSwap<Vec<Option<Highlight>>>,
    non_local_variable_patterns: Vec<bool>,
    injection_content_capture_index: Option<u32>,
    injection_language_capture_index: Option<u32>,
    injection_filename_capture_index: Option<u32>,
    injection_shebang_capture_index: Option<u32>,
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
struct HighlightIter<'a> {
    source: RopeSlice<'a>,
    byte_offset: usize,
    cancellation_flag: Option<&'a AtomicUsize>,
    layers: Vec<HighlightIterLayer<'a>>,
    iter_count: usize,
    next_event: Option<HighlightEvent>,
    last_highlight_range: Option<(usize, usize, u32)>,
}

// Adapter to convert rope chunks to bytes
pub struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}
impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}

pub struct RopeProvider<'a>(pub RopeSlice<'a>);
impl<'a> TextProvider<'a> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let fragment = self.0.byte_slice(node.start_byte()..node.end_byte());
        ChunksBytes {
            chunks: fragment.chunks(),
        }
    }
}

struct HighlightIterLayer<'a> {
    _tree: Option<Tree>,
    cursor: QueryCursor,
    captures: RefCell<iter::Peekable<QueryCaptures<'a, 'a, RopeProvider<'a>>>>,
    config: &'a HighlightConfiguration,
    highlight_end_stack: Vec<usize>,
    scope_stack: Vec<LocalScope<'a>>,
    depth: u32,
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
        query_source.push_str(locals_query);
        let highlights_query_offset = query_source.len();
        query_source.push_str(highlights_query);

        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let query = Query::new(language, &query_source)?;
        let mut highlights_pattern_index = 0;
        for i in 0..(query.pattern_count()) {
            let pattern_offset = query.start_byte_for_pattern(i);
            if pattern_offset < highlights_query_offset {
                highlights_pattern_index += 1;
            }
        }

        let mut injections_query = Query::new(language, injection_query)?;

        // Construct a separate query just for dealing with the 'combined injections'.
        // Disable the combined injection patterns in the main query.
        let mut combined_injections_query = Query::new(language, injection_query)?;
        let mut has_combined_queries = false;
        for pattern_index in 0..injections_query.pattern_count() {
            let settings = injections_query.property_settings(pattern_index);
            if settings.iter().any(|s| &*s.key == "injection.combined") {
                has_combined_queries = true;
                injections_query.disable_pattern(pattern_index);
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
        let mut injection_filename_capture_index = None;
        let mut injection_shebang_capture_index = None;
        let mut local_def_capture_index = None;
        let mut local_def_value_capture_index = None;
        let mut local_ref_capture_index = None;
        let mut local_scope_capture_index = None;
        for (i, name) in query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match name.as_str() {
                "local.definition" => local_def_capture_index = i,
                "local.definition-value" => local_def_value_capture_index = i,
                "local.reference" => local_ref_capture_index = i,
                "local.scope" => local_scope_capture_index = i,
                _ => {}
            }
        }

        for (i, name) in injections_query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match name.as_str() {
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
            combined_injections_query,
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
    pub fn names(&self) -> &[String] {
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
                    let recognized_name = recognized_name;
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

                static SHEBANG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(SHEBANG).unwrap());

                injection_capture = SHEBANG_REGEX
                    .captures(&Cow::from(lines))
                    .map(|cap| InjectionLanguageMarker::Shebang(cap[1].to_owned()))
            } else if index == self.injection_content_capture_index {
                content_node = Some(capture.node);
            }
        }
        (injection_capture, content_node)
    }

    fn injection_for_match<'a>(
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
}

impl<'a> HighlightIterLayer<'a> {
    // First, sort scope boundaries by their byte offset in the document. At a
    // given position, emit scope endings before scope beginnings. Finally, emit
    // scope boundaries from deeper layers first.
    fn sort_key(&self) -> Option<(usize, bool, isize)> {
        let depth = -(self.depth as isize);
        let next_start = self
            .captures
            .borrow_mut()
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
    included_children: IncludedChildren,
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
            .filter_map(|child| match included_children {
                IncludedChildren::None => Some(child.range()),
                IncludedChildren::All => None,
                IncludedChildren::Unnamed => {
                    if child.is_named() {
                        Some(child.range())
                    } else {
                        None
                    }
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

impl<'a> HighlightIter<'a> {
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
                    } else {
                        let layer = self.layers.remove(i + 1);
                        PARSER.with(|ts_parser| {
                            let highlighter = &mut ts_parser.borrow_mut();
                            highlighter.cursors.push(layer.cursor);
                        });
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
}

impl<'a> Iterator for HighlightIter<'a> {
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
            let captures = layer.captures.get_mut();
            if let Some((next_match, capture_index)) = captures.peek() {
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
                return self.emit_event(self.source.len_bytes(), None);
            };

            let (mut match_, capture_index) = captures.next().unwrap();
            let mut capture = match_.captures[capture_index];

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
                if let Some((next_match, next_capture_index)) = captures.peek() {
                    let next_capture = next_match.captures[*next_capture_index];
                    if next_capture.node == capture.node {
                        capture = next_capture;
                        match_ = captures.next().unwrap().0;
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
                    if let Some((next_match, next_capture_index)) = captures.peek() {
                        let next_capture = next_match.captures[*next_capture_index];
                        if next_capture.node == capture.node {
                            capture = next_capture;
                            match_ = captures.next().unwrap().0;
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
            while let Some((next_match, next_capture_index)) = captures.peek() {
                let next_capture = next_match.captures[*next_capture_index];
                if next_capture.node == capture.node {
                    captures.next();
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

#[derive(Debug, Clone)]
pub enum InjectionLanguageMarker<'a> {
    Name(Cow<'a, str>),
    Filename(Cow<'a, Path>),
    Shebang(String),
}

const SHEBANG: &str = r"#!\s*(?:\S*[/\\](?:env\s+(?:\-\S+\s+)*)?)?([^\s\.\d]+)";

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

fn node_is_visible(node: &Node) -> bool {
    node.is_missing() || (node.is_named() && node.language().node_kind_is_visible(node.kind_id()))
}

pub fn pretty_print_tree<W: fmt::Write>(fmt: &mut W, node: Node) -> fmt::Result {
    if node.child_count() == 0 {
        if node_is_visible(&node) {
            write!(fmt, "({})", node.kind())
        } else {
            write!(fmt, "\"{}\"", node.kind())
        }
    } else {
        pretty_print_tree_impl(fmt, &mut node.walk(), 0)
    }
}

fn pretty_print_tree_impl<W: fmt::Write>(
    fmt: &mut W,
    cursor: &mut TreeCursor,
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
    use super::*;
    use crate::{Rope, Transaction};

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

        let loader = Loader::new(Configuration { language: vec![] });
        let language = get_language("rust").unwrap();

        let query = Query::new(language, query_str).unwrap();
        let textobject = TextObjectQuery { query };
        let mut cursor = QueryCursor::new();

        let config = HighlightConfiguration::new(language, "", "", "").unwrap();
        let syntax = Syntax::new(&source, Arc::new(config), Arc::new(loader));

        let root = syntax.tree().root_node();
        let mut test = |capture, range| {
            let matches: Vec<_> = textobject
                .capture_nodes(capture, root, source.slice(..), &mut cursor)
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

        test("quantified_nodes", 1..36);
        // NOTE: Enable after implementing proper node group capturing
        // test("quantified_nodes_grouped", 1..36);
        // test("multiple_nodes_grouped", 1..36);
    }

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

        let loader = Loader::new(Configuration { language: vec![] });

        let language = get_language("rust").unwrap();
        let config = HighlightConfiguration::new(
            language,
            &std::fs::read_to_string("../runtime/grammars/sources/rust/queries/highlights.scm")
                .unwrap(),
            &std::fs::read_to_string("../runtime/grammars/sources/rust/queries/injections.scm")
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
        let syntax = Syntax::new(&source, Arc::new(config), Arc::new(loader));
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
        let edits = generate_edits(&doc, transaction.changes());
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
        let edits = generate_edits(&doc, transaction.changes());
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

    #[track_caller]
    fn assert_pretty_print(
        language_name: &str,
        source: &str,
        expected: &str,
        start: usize,
        end: usize,
    ) {
        let source = Rope::from_str(source);

        let loader = Loader::new(Configuration { language: vec![] });
        let language = get_language(language_name).unwrap();

        let config = HighlightConfiguration::new(language, "", "", "").unwrap();
        let syntax = Syntax::new(&source, Arc::new(config), Arc::new(loader));

        let root = syntax
            .tree()
            .root_node()
            .descendant_for_byte_range(start, end)
            .unwrap();

        let mut output = String::new();
        pretty_print_tree(&mut output, root).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn test_pretty_print() {
        let source = r#"/// Hello"#;
        assert_pretty_print("rust", source, "(line_comment)", 0, source.len());

        // A large tree should be indented with fields:
        let source = r#"fn main() {
            println!("Hello, World!");
        }"#;
        assert_pretty_print(
            "rust",
            source,
            concat!(
                "(function_item\n",
                "  name: (identifier)\n",
                "  parameters: (parameters)\n",
                "  body: (block\n",
                "    (expression_statement\n",
                "      (macro_invocation\n",
                "        macro: (identifier)\n",
                "        (token_tree\n",
                "          (string_literal))))))",
            ),
            0,
            source.len(),
        );

        // Selecting a token should print just that token:
        let source = r#"fn main() {}"#;
        assert_pretty_print("rust", source, r#""fn""#, 0, 1);

        // Error nodes are printed as errors:
        let source = r#"}{"#;
        assert_pretty_print("rust", source, "(ERROR)", 0, source.len());

        // Fields broken under unnamed nodes are determined correctly.
        // In the following source, `object` belongs to the `singleton_method`
        // rule but `name` and `body` belong to an unnamed helper `_method_rest`.
        // This can cause a bug with a pretty-printing implementation that
        // uses `Node::field_name_for_child` to determine field names but is
        // fixed when using `TreeCursor::field_name`.
        let source = "def self.method_name
          true
        end";
        assert_pretty_print(
            "ruby",
            source,
            concat!(
                "(singleton_method\n",
                "  object: (self)\n",
                "  name: (identifier)\n",
                "  body: (body_statement\n",
                "    (true)))"
            ),
            0,
            source.len(),
        );
    }

    #[test]
    fn test_load_runtime_file() {
        // Test to make sure we can load some data from the runtime directory.
        let contents = load_runtime_file("rust", "indents.scm").unwrap();
        assert!(!contents.is_empty());

        let results = load_runtime_file("rust", "does-not-exist");
        assert!(results.is_err());
    }
}
