use crate::{auto_pairs::AutoPairs, diagnostic::Severity, Language};

use globset::GlobSet;
use helix_stdx::rope;
use serde::{ser::SerializeSeq as _, Deserialize, Serialize};

use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
    num::NonZeroU8,
    path::PathBuf,
    str::FromStr,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Configuration {
    pub language: Vec<LanguageConfiguration>,
    #[serde(default)]
    pub language_server: HashMap<String, LanguageServerConfiguration>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LanguageConfiguration {
    #[serde(skip)]
    pub(super) language: Option<Language>,

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

    /// If set, overrides `editor.path-completion`.
    pub path_completion: Option<bool>,
    /// If set, overrides `editor.word-completion`.
    pub word_completion: Option<WordCompletion>,

    #[serde(default)]
    pub diagnostic_severity: Severity,

    pub grammar: Option<String>, // tree-sitter grammar name, defaults to language_id

    // content_regex
    #[serde(default, skip_serializing, deserialize_with = "deserialize_regex")]
    pub injection_regex: Option<rope::Regex>,
    // first_line_regex
    //
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_lang_features",
        deserialize_with = "deserialize_lang_features"
    )]
    pub language_servers: Vec<LanguageServerFeatures>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent: Option<IndentationConfiguration>,

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

impl LanguageConfiguration {
    pub fn language(&self) -> Language {
        // This value must be set by `super::Loader::new`.
        self.language.unwrap()
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
    DocumentColors,
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
            DocumentColors => "document-colors",
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct WordCompletion {
    pub enable: Option<bool>,
    pub trigger_length: Option<NonZeroU8>,
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Option<rope::Regex>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|buf| rope::Regex::new(&buf).map_err(serde::de::Error::custom))
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
