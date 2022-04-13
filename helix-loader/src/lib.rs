pub mod grammar;

use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

static PATHS: once_cell::sync::OnceCell<Paths> = once_cell::sync::OnceCell::new();

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String")]
pub enum Path {
    LanguageFile,
    LogFile,
    GrammarDir,
    ThemeDir,
    #[serde(skip)] // no point in tutor file being configurable
    TutorFile,
    QueryDir,
}

impl TryFrom<String> for Path {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "language-file" => Ok(Path::LanguageFile),
            "log-file" => Ok(Path::LogFile),
            "grammar-dir" => Ok(Path::GrammarDir),
            "theme-dir" => Ok(Path::ThemeDir),
            "query-dir" => Ok(Path::QueryDir),
            _ => bail!("Invalid path key '{}'", s),
        }
    }
}

/// Ensure that Paths always contain all required paths
/// by setting whatever isn't specified in config to default values
impl<'de> Deserialize<'de> for Paths {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut loaded = HashMap::<Path, PathBuf>::deserialize(deserializer)?;
        let mut defaults = Self::default();
        for (k, v) in defaults.0.drain() {
            if loaded.get(&k).is_none() {
                loaded.insert(k, v);
            }
        }
        Ok(Self(loaded))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paths(HashMap<Path, PathBuf>);

impl Default for Paths {
    fn default() -> Self {
        let grammars = runtime_dir().join("grammars");
        let log = cache_dir().join("helix.log");
        let languages = project_dirs().config_dir().join("languages.toml");
        let themes = runtime_dir().join("themes");
        let queries = runtime_dir().join("queries");

        Self(HashMap::from([
            (Path::GrammarDir, grammars),
            (Path::LanguageFile, languages),
            (Path::LogFile, log),
            (Path::ThemeDir, themes),
            (Path::QueryDir, queries),
        ]))
    }
}

impl Paths {
    pub fn get(&self, path_type: &Path) -> &std::path::Path {
        // unwrap: all types always present, either from config or default()
        self.0.get(path_type).unwrap()
    }
}

/// Get paths loaded from config, complemented by defaults
fn get_path(path_type: &Path) -> &std::path::Path {
    PATHS
        .get()
        .expect("must init paths from config first")
        .get(path_type)
}

/// Set static PATHS to valid values
/// Should be called before getting any path except config_dir
pub fn init_paths(mut paths: Paths) -> Result<()> {
    for (k, v) in &mut paths.0 {
        let dir = match k {
            Path::LanguageFile | Path::LogFile | Path::TutorFile => v.parent().unwrap(),
            Path::GrammarDir | Path::ThemeDir | Path::QueryDir => v,
        };
        create_dir(dir)?;
        // grammar building can't handle relative paths
        // todo: replace canonicalize with std::path::absolute
        // then simply call `*v = std::path::absolute(k)`;
        // tracking issue: https://github.com/rust-lang/rust/issues/92750
        if k == &Path::LanguageFile || k == &Path::LogFile || k == &Path::TutorFile {
            *v = std::fs::canonicalize(dir)?.join(v.file_name().unwrap());
        } else {
            *v = std::fs::canonicalize(dir)?;
        }
    }
    PATHS
        .set(paths)
        .expect("trying to set paths multiple times");
    Ok(())
}

fn create_dir(dir: &std::path::Path) -> Result<()> {
    if dir.exists() && !dir.is_dir() {
        bail!("{} exists but is not a directory!", dir.display())
    } else if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}

fn project_dirs() -> directories::ProjectDirs {
    directories::ProjectDirs::from("com", "helix-editor", "helix")
        .expect("Unable to continue. User has no home.")
}

pub fn runtime_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HELIX_RUNTIME") {
        return dir.into();
    }

    const RT_DIR: &str = "runtime";
    let conf_dir = config_dir().join(RT_DIR);
    if conf_dir.exists() {
        return conf_dir;
    }

    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // this is the directory of the crate being run by cargo, we need the workspace path so we take the parent
        return PathBuf::from(dir).parent().unwrap().join(RT_DIR);
    }

    // fallback to location of the executable being run
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|path| path.to_path_buf().join(RT_DIR)))
        .unwrap()
}

fn config_dir() -> PathBuf {
    project_dirs().config_dir().to_path_buf()
}

fn cache_dir() -> PathBuf {
    project_dirs().cache_dir().to_path_buf()
}

pub fn config_file() -> PathBuf {
    if let Ok(dir) = std::env::var("HELIX_CONFIG") {
        return PathBuf::from(dir);
    }
    config_dir().join("config.toml")
}

pub fn grammar_dir() -> &'static std::path::Path {
    get_path(&Path::GrammarDir)
}

pub fn lang_config_file() -> &'static std::path::Path {
    get_path(&Path::LanguageFile)
}

pub fn log_file() -> &'static std::path::Path {
    get_path(&Path::LogFile)
}

pub fn theme_dir() -> &'static std::path::Path {
    get_path(&Path::ThemeDir)
}

pub fn tutor_file() -> PathBuf {
    config_dir().join("tutor.txt")
}

pub fn query_dir() -> &'static std::path::Path {
    get_path(&Path::QueryDir)
}

/// Default bultin-in languages.toml.
pub fn default_lang_config() -> toml::Value {
    toml::from_slice(include_bytes!("../../languages.toml"))
        .expect("Could not parse bultin-in languages.toml to valid toml")
}

/// User configured languages.toml file, merged with the default config.
pub fn user_lang_config() -> Result<toml::Value, toml::de::Error> {
    let def_lang_conf = default_lang_config();
    let data = std::fs::read(lang_config_file());
    let user_lang_conf = match data {
        Ok(raw) => {
            let value = toml::from_slice(&raw)?;
            merge_toml_values(def_lang_conf, value)
        }
        Err(_) => def_lang_conf,
    };

    Ok(user_lang_conf)
}

// right overrides left
pub fn merge_toml_values(left: toml::Value, right: toml::Value) -> toml::Value {
    use toml::Value;

    fn get_name(v: &Value) -> Option<&str> {
        v.get("name").and_then(Value::as_str)
    }

    match (left, right) {
        (Value::Array(mut left_items), Value::Array(right_items)) => {
            left_items.reserve(right_items.len());
            for rvalue in right_items {
                let lvalue = get_name(&rvalue)
                    .and_then(|rname| left_items.iter().position(|v| get_name(v) == Some(rname)))
                    .map(|lpos| left_items.remove(lpos));
                let mvalue = match lvalue {
                    Some(lvalue) => merge_toml_values(lvalue, rvalue),
                    None => rvalue,
                };
                left_items.push(mvalue);
            }
            Value::Array(left_items)
        }
        (Value::Table(mut left_map), Value::Table(right_map)) => {
            for (rname, rvalue) in right_map {
                match left_map.remove(&rname) {
                    Some(lvalue) => {
                        let merged_value = merge_toml_values(lvalue, rvalue);
                        left_map.insert(rname, merged_value);
                    }
                    None => {
                        left_map.insert(rname, rvalue);
                    }
                }
            }
            Value::Table(left_map)
        }
        // Catch everything else we didn't handle, and use the right value
        (_, value) => value,
    }
}

#[cfg(test)]
mod merge_toml_tests {
    use super::merge_toml_values;

    #[test]
    fn language_tomls() {
        use toml::Value;

        const USER: &str = "
        [[language]]
        name = \"nix\"
        test = \"bbb\"
        indent = { tab-width = 4, unit = \"    \", test = \"aaa\" }
        ";

        let base: Value = toml::from_slice(include_bytes!("../../languages.toml"))
            .expect("Couldn't parse built-in languages config");
        let user: Value = toml::from_str(USER).unwrap();

        let merged = merge_toml_values(base, user);
        let languages = merged.get("language").unwrap().as_array().unwrap();
        let nix = languages
            .iter()
            .find(|v| v.get("name").unwrap().as_str().unwrap() == "nix")
            .unwrap();
        let nix_indent = nix.get("indent").unwrap();

        // We changed tab-width and unit in indent so check them if they are the new values
        assert_eq!(
            nix_indent.get("tab-width").unwrap().as_integer().unwrap(),
            4
        );
        assert_eq!(nix_indent.get("unit").unwrap().as_str().unwrap(), "    ");
        // We added a new keys, so check them
        assert_eq!(nix.get("test").unwrap().as_str().unwrap(), "bbb");
        assert_eq!(nix_indent.get("test").unwrap().as_str().unwrap(), "aaa");
        // We didn't change comment-token so it should be same
        assert_eq!(nix.get("comment-token").unwrap().as_str().unwrap(), "#");
    }
}


// multiple test mods due to work around shared static
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "trying to set paths multiple times")]
    fn calling_init_paths_repeatedly_errors() {
        assert!(init_paths(Paths::default()).is_ok());
        let _ = init_paths(Paths::default());
    }

    #[test]
    fn config_is_always_available() {
        config_file();
    }
}
