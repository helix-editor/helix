//! Support for [EditorConfig](https://EditorConfig.org) configuration loading.
//!
//! EditorConfig is an editor-agnostic format for specifying configuration in an INI-like, human
//! friendly syntax in `.editorconfig` files (which are intended to be checked into VCS). This
//! module provides functions to search for all `.editorconfig` files that apply to a given path
//! and returns an `EditorConfig` type containing any specified configuration options.
//!
//! At time of writing, this module follows the [spec](https://spec.editorconfig.org/) at
//! version 0.17.2.

use std::{
    collections::HashMap,
    fs,
    num::{NonZeroU16, NonZeroU8},
    path::Path,
    str::FromStr,
};

use encoding_rs::Encoding;
use globset::{GlobBuilder, GlobMatcher};

use crate::{
    indent::{IndentStyle, MAX_INDENT},
    LineEnding,
};

/// Configuration declared for a path in `.editorconfig` files.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EditorConfig {
    pub indent_style: Option<IndentStyle>,
    pub tab_width: Option<NonZeroU8>,
    pub line_ending: Option<LineEnding>,
    pub encoding: Option<&'static Encoding>,
    // pub spelling_language: Option<SpellingLanguage>,
    pub trim_trailing_whitespace: Option<bool>,
    pub insert_final_newline: Option<bool>,
    pub max_line_length: Option<NonZeroU16>,
}

impl EditorConfig {
    /// Finds any configuration in `.editorconfig` files which applies to the given path.
    ///
    /// If no configuration applies then `EditorConfig::default()` is returned.
    pub fn find(path: &Path) -> Self {
        let mut configs = Vec::new();
        // <https://spec.editorconfig.org/#file-processing>
        for ancestor in path.ancestors() {
            let editor_config_file = ancestor.join(".editorconfig");
            let Ok(contents) = fs::read_to_string(&editor_config_file) else {
                continue;
            };
            let ini = match contents.parse::<Ini>() {
                Ok(ini) => ini,
                Err(err) => {
                    log::warn!("Ignoring EditorConfig file at '{editor_config_file:?}' because a glob failed to compile: {err}");
                    continue;
                }
            };
            let is_root = ini.pairs.get("root").map(AsRef::as_ref) == Some("true");
            configs.push((ini, ancestor));
            // > The search shall stop if an EditorConfig file is found with the `root` key set to
            // > `true` in the preamble or when reaching the root filesystem directory.
            if is_root {
                break;
            }
        }

        let mut pairs = Pairs::new();
        // Reverse the configuration stack so that the `.editorconfig` files closest to `path`
        // are applied last and overwrite settings in files closer to the search ceiling.
        //
        // > If multiple EditorConfig files have matching sections, the pairs from the closer
        // > EditorConfig file are read last, so pairs in closer files take precedence.
        for (config, dir) in configs.into_iter().rev() {
            let relative_path = path.strip_prefix(dir).expect("dir is an ancestor of path");

            for section in config.sections {
                if section.glob.is_match(relative_path) {
                    log::info!(
                        "applying EditorConfig from section '{}' in file {:?}",
                        section.glob.glob(),
                        dir.join(".editorconfig")
                    );
                    pairs.extend(section.pairs);
                }
            }
        }

        Self::from_pairs(pairs)
    }

    fn from_pairs(pairs: Pairs) -> Self {
        enum IndentSize {
            Tab,
            Spaces(NonZeroU8),
        }

        // <https://spec.editorconfig.org/#supported-pairs>
        let indent_size = pairs.get("indent_size").and_then(|value| {
            if value.as_ref() == "tab" {
                Some(IndentSize::Tab)
            } else if let Ok(spaces) = value.parse::<NonZeroU8>() {
                Some(IndentSize::Spaces(spaces))
            } else {
                None
            }
        });
        let tab_width = pairs
            .get("tab_width")
            .and_then(|value| value.parse::<NonZeroU8>().ok())
            .or(match indent_size {
                Some(IndentSize::Spaces(spaces)) => Some(spaces),
                _ => None,
            });
        let indent_style = pairs
            .get("indent_style")
            .and_then(|value| match value.as_ref() {
                "tab" => Some(IndentStyle::Tabs),
                "space" => {
                    let spaces = match indent_size {
                        Some(IndentSize::Spaces(spaces)) => spaces.get(),
                        Some(IndentSize::Tab) => tab_width.map(|n| n.get()).unwrap_or(4),
                        None => 4,
                    };
                    Some(IndentStyle::Spaces(spaces.clamp(1, MAX_INDENT)))
                }
                _ => None,
            });
        let line_ending = pairs
            .get("end_of_line")
            .and_then(|value| match value.as_ref() {
                "lf" => Some(LineEnding::LF),
                "crlf" => Some(LineEnding::Crlf),
                #[cfg(feature = "unicode-lines")]
                "cr" => Some(LineEnding::CR),
                _ => None,
            });
        let encoding = pairs.get("charset").and_then(|value| match value.as_ref() {
            "latin1" => Some(encoding_rs::WINDOWS_1252),
            "utf-8" => Some(encoding_rs::UTF_8),
            // `utf-8-bom` is intentionally ignored.
            // > `utf-8-bom` is discouraged.
            "utf-16le" => Some(encoding_rs::UTF_16LE),
            "utf-16be" => Some(encoding_rs::UTF_16BE),
            _ => None,
        });
        let trim_trailing_whitespace =
            pairs
                .get("trim_trailing_whitespace")
                .and_then(|value| match value.as_ref() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                });
        let insert_final_newline = pairs
            .get("insert_final_newline")
            .and_then(|value| match value.as_ref() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            });
        // This option is not in the spec but is supported by some editors.
        // <https://github.com/editorconfig/editorconfig/wiki/EditorConfig-Properties#max_line_length>
        let max_line_length = pairs
            .get("max_line_length")
            .and_then(|value| value.parse::<NonZeroU16>().ok());

        Self {
            indent_style,
            tab_width,
            line_ending,
            encoding,
            trim_trailing_whitespace,
            insert_final_newline,
            max_line_length,
        }
    }
}

type Pairs = HashMap<Box<str>, Box<str>>;

#[derive(Debug)]
struct Section {
    glob: GlobMatcher,
    pairs: Pairs,
}

#[derive(Debug, Default)]
struct Ini {
    pairs: Pairs,
    sections: Vec<Section>,
}

impl FromStr for Ini {
    type Err = globset::Error;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        // <https://spec.editorconfig.org/#file-format>
        let mut ini = Ini::default();
        // > EditorConfig files are in an INI-like file format. To read an EditorConfig file, take
        // > one line at a time, from beginning to end. For each line:
        for full_line in source.lines() {
            // > 1. Remove all leading and trailing whitespace.
            let line = full_line.trim();
            // > 2. Process the remaining text as specified for its type below.
            // > The types of lines are:
            // > * Blank: contains nothing. Blank lines are ignored.
            if line.is_empty() {
                continue;
            }
            // > * Comment: starts with a ';' or '#'. Comment lines are ignored.
            if line.starts_with([';', '#']) {
                continue;
            }
            if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                // > * Section Header: starts with a `[` and ends with a `]`. These lines define
                // >   globs...

                // <https://spec.editorconfig.org/#glob-expressions>
                // We need to modify the glob string slightly since EditorConfig's glob flavor
                // doesn't match `globset`'s exactly. `globset` only allows '**' at the beginning
                // or end of a glob or between two '/'s. (This replacement is not very fancy but
                // should cover most practical cases.)
                let mut glob_str = section.replace("**.", "**/*.");
                if !is_glob_relative(section) {
                    glob_str.insert_str(0, "**/");
                }
                let glob = GlobBuilder::new(&glob_str)
                    .literal_separator(true)
                    .backslash_escape(true)
                    .build()?;
                ini.sections.push(Section {
                    glob: glob.compile_matcher(),
                    pairs: Pairs::new(),
                });
            } else if let Some((key, value)) = line.split_once('=') {
                // > * Key-Value Pair (or Pair): contains a key and a value, separated by an `=`.
                // >     * Key: The part before the first `=` on the line.
                // >     * Value: The part, if any, after the first `=` on the line.
                // >     * Keys and values are trimmed of leading and trailing whitespace, but
                // >       include any whitespace that is between non-whitespace characters.
                // >     * If a value is not provided, then the value is an empty string.
                let key = key.trim().to_lowercase().into_boxed_str();
                let value = value.trim().to_lowercase().into_boxed_str();
                if let Some(section) = ini.sections.last_mut() {
                    section.pairs.insert(key, value);
                } else {
                    ini.pairs.insert(key, value);
                }
            }
        }
        Ok(ini)
    }
}

/// Determines whether a glob is relative to the directory of the config file.
fn is_glob_relative(source: &str) -> bool {
    // > If the glob contains a path separator (a `/` not inside square brackets), then the
    // > glob is relative to the directory level of the particular `.editorconfig` file itself.
    let mut idx = 0;
    while let Some(open) = source[idx..].find('[').map(|open| idx + open) {
        if source[..open].contains('/') {
            return true;
        }
        idx = source[open..]
            .find(']')
            .map_or(source.len(), |close| idx + close);
    }
    source[idx..].contains('/')
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_glob_relative_test() {
        assert!(is_glob_relative("subdir/*.c"));
        assert!(!is_glob_relative("*.txt"));
        assert!(!is_glob_relative("[a/b].c"));
    }

    fn editor_config(path: impl AsRef<Path>, source: &str) -> EditorConfig {
        let path = path.as_ref();
        let ini = source.parse::<Ini>().unwrap();
        let pairs = ini
            .sections
            .into_iter()
            .filter(|section| section.glob.is_match(path))
            .fold(Pairs::new(), |mut acc, section| {
                acc.extend(section.pairs);
                acc
            });
        EditorConfig::from_pairs(pairs)
    }

    #[test]
    fn parse_test() {
        let source = r#"
        [*]
        indent_style = space

        [Makefile]
        indent_style = tab

        [docs/**.txt]
        insert_final_newline = true
        "#;

        assert_eq!(
            editor_config("a.txt", source),
            EditorConfig {
                indent_style: Some(IndentStyle::Spaces(4)),
                ..Default::default()
            }
        );
        assert_eq!(
            editor_config("pkg/Makefile", source),
            EditorConfig {
                indent_style: Some(IndentStyle::Tabs),
                ..Default::default()
            }
        );
        assert_eq!(
            editor_config("docs/config/editor.txt", source),
            EditorConfig {
                indent_style: Some(IndentStyle::Spaces(4)),
                insert_final_newline: Some(true),
                ..Default::default()
            }
        );
    }
}
