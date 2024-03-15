use std::borrow::Cow;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use arc_swap::{ArcSwap, Guard};
use globset::GlobSet;
pub use helix_syntax::highlighter::{Highlight, HighlightEvent};
pub use helix_syntax::{
    merge, pretty_print_tree, HighlightConfiguration, InjectionLanguageMarker, RopeProvider,
    TextObjectQuery, TreeCursor,
};
pub use helix_syntax::{with_cursor, Syntax};
use once_cell::sync::{Lazy, OnceCell};
use regex::Regex;
use ropey::RopeSlice;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Point, Query};

use crate::auto_pairs::AutoPairs;
use crate::chars::char_is_line_ending;
use crate::diagnostic::Severity;
use crate::{ChangeSet, Operation, Tendril};

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

fn deserialize_tab_width<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    usize::deserialize(deserializer).and_then(|n| {
        if n > 0 && n <= 16 {
            Ok(n)
        } else {
            Err(serde::de::Error::custom(
                "tab width must be a value from 1 to 16 inclusive",
            ))
        }
    })
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
#[serde(rename_all = "kebab-case")]
pub struct Configuration {
    pub language: Vec<LanguageConfiguration>,
    #[serde(default)]
    pub language_server: HashMap<String, LanguageServerConfiguration>,
}

// largely based on tree-sitter/cli/src/loader.rs
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LanguageConfiguration {
    #[serde(rename = "name")]
    pub language_id: String, // c-sharp, rust, tsx
    #[serde(rename = "language-id")]
    // see the table under https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocumentItem
    pub language_server_language_id: Option<String>, // csharp, rust, typescriptreact, for the language-server
    pub scope: String,             // source.rust
    pub file_types: Vec<FileType>, // filename extension or ends_with? <Gemfile, rb, etc>
    #[serde(default)]
    pub shebangs: Vec<String>, // interpreter(s) associated with language
    #[serde(default)]
    pub roots: Vec<String>, // these indicate project roots <.git, Cargo.toml>
    #[serde(
        default,
        skip_serializing,
        deserialize_with = "from_comment_tokens",
        alias = "comment-token"
    )]
    pub comment_tokens: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing,
        deserialize_with = "from_block_comment_tokens"
    )]
    pub block_comment_tokens: Option<Vec<BlockCommentToken>>,
    pub text_width: Option<usize>,
    pub soft_wrap: Option<SoftWrap>,

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
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_lang_features",
        deserialize_with = "deserialize_lang_features"
    )]
    pub language_servers: Vec<LanguageServerFeatures>,
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
    #[serde(default)]
    pub persistent_diagnostic_sources: Vec<String>,
}

fn read_query(language: &str, filename: &str) -> String {
    helix_syntax::read_query(language, filename, |lang, filename| {
        load_runtime_file(lang, filename).unwrap_or_default()
    })
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

    pub fn get_highlight_config(&self) -> Option<Arc<HighlightConfiguration>> {
        self.highlight_config.get().cloned().flatten()
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
        let lang = &self.highlight_config.get()?.as_ref()?.language;
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    /// The extension of the file, either the `Path::extension` or the full
    /// filename if the file does not have an extension.
    Extension(String),
    /// A Unix-style path glob. This is compared to the file's absolute path, so
    /// it can be used to detect files based on their directories. If the glob
    /// is not an absolute path and does not already start with a glob pattern,
    /// a glob pattern will be prepended to it.
    Glob(globset::Glob),
}

impl Serialize for FileType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            FileType::Extension(extension) => serializer.serialize_str(extension),
            FileType::Glob(glob) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("glob", glob.glob())?;
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
                    Some((key, mut glob)) if key == "glob" => {
                        // If the glob isn't an absolute path or already starts
                        // with a glob pattern, add a leading glob so we
                        // properly match relative paths.
                        if !glob.starts_with('/') && !glob.starts_with("*/") {
                            glob.insert_str(0, "*/");
                        }

                        globset::Glob::new(glob.as_str())
                            .map(FileType::Glob)
                            .map_err(|err| {
                                serde::de::Error::custom(format!("invalid `glob` pattern: {}", err))
                            })
                    }
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

fn from_comment_tokens<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum CommentTokens {
        Multiple(Vec<String>),
        Single(String),
    }
    Ok(
        Option::<CommentTokens>::deserialize(deserializer)?.map(|tokens| match tokens {
            CommentTokens::Single(val) => vec![val],
            CommentTokens::Multiple(vals) => vals,
        }),
    )
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockCommentToken {
    pub start: String,
    pub end: String,
}

impl Default for BlockCommentToken {
    fn default() -> Self {
        BlockCommentToken {
            start: "/*".to_string(),
            end: "*/".to_string(),
        }
    }
}

fn from_block_comment_tokens<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<BlockCommentToken>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BlockCommentTokens {
        Multiple(Vec<BlockCommentToken>),
        Single(BlockCommentToken),
    }
    Ok(
        Option::<BlockCommentTokens>::deserialize(deserializer)?.map(|tokens| match tokens {
            BlockCommentTokens::Single(val) => vec![val],
            BlockCommentTokens::Multiple(vals) => vals,
        }),
    )
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum LanguageServerFeature {
    Format,
    GotoDeclaration,
    GotoDefinition,
    GotoTypeDefinition,
    GotoReference,
    GotoImplementation,
    // Goto, use bitflags, combining previous Goto members?
    SignatureHelp,
    Hover,
    DocumentHighlight,
    Completion,
    CodeAction,
    WorkspaceCommand,
    DocumentSymbols,
    WorkspaceSymbols,
    // Symbols, use bitflags, see above?
    Diagnostics,
    RenameSymbol,
    InlayHints,
}

impl Display for LanguageServerFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LanguageServerFeature::*;
        let feature = match self {
            Format => "format",
            GotoDeclaration => "goto-declaration",
            GotoDefinition => "goto-definition",
            GotoTypeDefinition => "goto-type-definition",
            GotoReference => "goto-reference",
            GotoImplementation => "goto-implementation",
            SignatureHelp => "signature-help",
            Hover => "hover",
            DocumentHighlight => "document-highlight",
            Completion => "completion",
            CodeAction => "code-action",
            WorkspaceCommand => "workspace-command",
            DocumentSymbols => "document-symbols",
            WorkspaceSymbols => "workspace-symbols",
            Diagnostics => "diagnostics",
            RenameSymbol => "rename-symbol",
            InlayHints => "inlay-hints",
        };
        write!(f, "{feature}",)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "kebab-case", deny_unknown_fields)]
enum LanguageServerFeatureConfiguration {
    #[serde(rename_all = "kebab-case")]
    Features {
        #[serde(default, skip_serializing_if = "HashSet::is_empty")]
        only_features: HashSet<LanguageServerFeature>,
        #[serde(default, skip_serializing_if = "HashSet::is_empty")]
        except_features: HashSet<LanguageServerFeature>,
        name: String,
    },
    Simple(String),
}

#[derive(Debug, Default)]
pub struct LanguageServerFeatures {
    pub name: String,
    pub only: HashSet<LanguageServerFeature>,
    pub excluded: HashSet<LanguageServerFeature>,
}

impl LanguageServerFeatures {
    pub fn has_feature(&self, feature: LanguageServerFeature) -> bool {
        (self.only.is_empty() || self.only.contains(&feature)) && !self.excluded.contains(&feature)
    }
}

fn deserialize_lang_features<'de, D>(
    deserializer: D,
) -> Result<Vec<LanguageServerFeatures>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Vec<LanguageServerFeatureConfiguration> = Deserialize::deserialize(deserializer)?;
    let res = raw
        .into_iter()
        .map(|config| match config {
            LanguageServerFeatureConfiguration::Simple(name) => LanguageServerFeatures {
                name,
                ..Default::default()
            },
            LanguageServerFeatureConfiguration::Features {
                only_features,
                except_features,
                name,
            } => LanguageServerFeatures {
                name,
                only: only_features,
                excluded: except_features,
            },
        })
        .collect();
    Ok(res)
}
fn serialize_lang_features<S>(
    map: &Vec<LanguageServerFeatures>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut serializer = serializer.serialize_seq(Some(map.len()))?;
    for features in map {
        let features = if features.only.is_empty() && features.excluded.is_empty() {
            LanguageServerFeatureConfiguration::Simple(features.name.to_owned())
        } else {
            LanguageServerFeatureConfiguration::Features {
                only_features: features.only.clone(),
                except_features: features.excluded.clone(),
                name: features.name.to_owned(),
            }
        };
        serializer.serialize_element(&features)?;
    }
    serializer.end()
}

fn deserialize_required_root_patterns<'de, D>(deserializer: D) -> Result<Option<GlobSet>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let patterns = Vec::<String>::deserialize(deserializer)?;
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in patterns {
        let glob = globset::Glob::new(&pattern).map_err(serde::de::Error::custom)?;
        builder.add(glob);
    }
    builder.build().map(Some).map_err(serde::de::Error::custom)
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
    #[serde(default, skip_serializing, deserialize_with = "deserialize_lsp_config")]
    pub config: Option<serde_json::Value>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(
        default,
        skip_serializing,
        deserialize_with = "deserialize_required_root_patterns"
    )]
    pub required_root_patterns: Option<GlobSet>,
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
    #[serde(default)]
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
    #[serde(deserialize_with = "deserialize_tab_width")]
    pub tab_width: usize,
    pub unit: String,
}

/// How the indentation for a newly inserted line should be determined.
/// If the selected heuristic is not available (e.g. because the current
/// language has no tree-sitter indent queries), a simpler one will be used.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndentationHeuristic {
    /// Just copy the indentation of the line that the cursor is currently on.
    Simple,
    /// Use tree-sitter indent queries to compute the expected absolute indentation level of the new line.
    TreeSitter,
    /// Use tree-sitter indent queries to compute the expected difference in indentation between the new line
    /// and the line before. Add this to the actual indentation level of the line before.
    #[default]
    Hybrid,
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

#[derive(Debug)]
struct FileTypeGlob {
    glob: globset::Glob,
    language_id: usize,
}

impl FileTypeGlob {
    fn new(glob: globset::Glob, language_id: usize) -> Self {
        Self { glob, language_id }
    }
}

#[derive(Debug)]
struct FileTypeGlobMatcher {
    matcher: globset::GlobSet,
    file_types: Vec<FileTypeGlob>,
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

    fn language_id_for_path(&self, path: &Path) -> Option<&usize> {
        self.matcher
            .matches(path)
            .iter()
            .filter_map(|idx| self.file_types.get(*idx))
            .max_by_key(|file_type| file_type.glob.glob().len())
            .map(|file_type| &file_type.language_id)
    }
}

// Expose loader as Lazy<> global since it's always static?

#[derive(Debug)]
pub struct Loader {
    // highlight_names ?
    language_configs: Vec<Arc<LanguageConfiguration>>,
    language_config_ids_by_extension: HashMap<String, usize>, // Vec<usize>
    language_config_ids_glob_matcher: FileTypeGlobMatcher,
    language_config_ids_by_shebang: HashMap<String, usize>,

    language_server_configs: HashMap<String, LanguageServerConfiguration>,

    scopes: ArcSwap<Vec<String>>,
}

pub type LoaderError = globset::Error;

impl Loader {
    pub fn new(config: Configuration) -> Result<Self, LoaderError> {
        let mut language_configs = Vec::new();
        let mut language_config_ids_by_extension = HashMap::new();
        let mut language_config_ids_by_shebang = HashMap::new();
        let mut file_type_globs = Vec::new();

        for config in config.language {
            // get the next id
            let language_id = language_configs.len();

            for file_type in &config.file_types {
                // entry().or_insert(Vec::new).push(language_id);
                match file_type {
                    FileType::Extension(extension) => {
                        language_config_ids_by_extension.insert(extension.clone(), language_id);
                    }
                    FileType::Glob(glob) => {
                        file_type_globs.push(FileTypeGlob::new(glob.to_owned(), language_id));
                    }
                };
            }
            for shebang in &config.shebangs {
                language_config_ids_by_shebang.insert(shebang.clone(), language_id);
            }

            language_configs.push(Arc::new(config));
        }

        Ok(Self {
            language_configs,
            language_config_ids_by_extension,
            language_config_ids_glob_matcher: FileTypeGlobMatcher::new(file_type_globs)?,
            language_config_ids_by_shebang,
            language_server_configs: config.language_server,
            scopes: ArcSwap::from_pointee(Vec::new()),
        })
    }

    pub fn language_config_for_file_name(&self, path: &Path) -> Option<Arc<LanguageConfiguration>> {
        // Find all the language configurations that match this file name
        // or a suffix of the file name.
        let configuration_id = self
            .language_config_ids_glob_matcher
            .language_id_for_path(path)
            .or_else(|| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .and_then(|extension| self.language_config_ids_by_extension.get(extension))
            });

        configuration_id.and_then(|&id| self.language_configs.get(id).cloned())

        // TODO: content_regex handling conflict resolution
    }

    pub fn language_config_for_shebang(
        &self,
        source: RopeSlice,
    ) -> Option<Arc<LanguageConfiguration>> {
        let line = Cow::from(source.line(0));
        // TODO: resue detection from helix-syntax
        const SHEBANG: &str = r"#!\s*(?:\S*[/\\](?:env\s+(?:\-\S+\s+)*)?)?([^\s\.\d]+)";
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

    pub fn language_server_configs(&self) -> &HashMap<String, LanguageServerConfiguration> {
        &self.language_server_configs
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

pub fn generate_edits(old_text: RopeSlice, changeset: &ChangeSet) -> Vec<tree_sitter::InputEdit> {
    use Operation::*;
    let mut old_pos = 0;

    let mut edits = Vec::new();

    if changeset.changes.is_empty() {
        return edits;
    }

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

#[cfg(test)]
mod test {
    use tree_sitter::QueryCursor;

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

        let language = get_language("rust").unwrap();

        let query = Query::new(&language, query_str).unwrap();
        let textobject = TextObjectQuery { query };
        let mut cursor = QueryCursor::new();

        let config = HighlightConfiguration::new(language, "", "", "").unwrap();
        let syntax = Syntax::new(source.slice(..), Arc::new(config), |_| None).unwrap();

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

        test("quantified_nodes", 1..37);
        // NOTE: Enable after implementing proper node group capturing
        // test("quantified_nodes_grouped", 1..37);
        // test("multiple_nodes_grouped", 1..37);
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
        let syntax = Syntax::new(source.slice(..), Arc::new(config), |_| None).unwrap();
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
        let edits = generate_edits(doc.slice(..), transaction.changes());
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
        let edits = generate_edits(doc.slice(..), transaction.changes());
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

        let language = get_language(language_name).unwrap();

        let config = HighlightConfiguration::new(language, "", "", "").unwrap();
        let syntax = Syntax::new(source.slice(..), Arc::new(config), |_| None).unwrap();

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
        let source = r#"// Hello"#;
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
                "          (string_literal\n",
                "            (string_content)))))))",
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
        // fixed when using `tree_sitter::TreeCursor::field_name`.
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
