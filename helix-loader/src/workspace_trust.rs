use crate::{data_dir, workspace_exclude_file, workspace_trust_file};
use globset::GlobBuilder;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

static WORKSPACE_TRUST_CACHE: Lazy<Mutex<HashMap<std::path::PathBuf, TrustUntrustStatus>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct WorkspaceTrust {
    trusted: HashSet<PathBuf>,
    excluded: Option<HashSet<PathBuf>>,
}

#[derive(Clone, Copy)]
pub enum TrustStatus {
    Untrusted,
    Trusted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    pub paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config { paths: vec![] }
    }
}

impl WorkspaceTrust {
    /// Loads `WorkspaceTrust`.
    ///
    /// Should be used only when there is a need to change trust status
    /// of a particular workspace.
    ///
    /// For querying trust status of a workspace use `quick_query_workspace()` or
    /// `quick_query_workspace_with_explicit_untrust()`
    pub fn load(with_exclusion: bool) -> Self {
        let mut trusted = HashSet::new();

        match fs::read_to_string(workspace_trust_file()) {
            Ok(workspace_trust_file) => {
                for line in workspace_trust_file.split('\n') {
                    if !line.is_empty() {
                        let path = PathBuf::from(line);
                        trusted.insert(path);
                    }
                }
            }
            Err(e) => log::error!("workspace file couldn't be read: {:?}", e),
        };

        let excluded = if with_exclusion {
            let mut untrusted = HashSet::new();

            match fs::read_to_string(workspace_exclude_file()) {
                Ok(workspace_untrust_file) => {
                    for line in workspace_untrust_file.split('\n') {
                        if !line.is_empty() {
                            let path = PathBuf::from(line);
                            untrusted.insert(path);
                        }
                    }
                }
                Err(e) => log::error!("workspace file couldn't be read: {:?}", e),
            };

            Some(untrusted)
        } else {
            None
        };
        WorkspaceTrust { trusted, excluded }
    }

    fn write_trust_to_file(&self) {
        let mut trust_text = String::new();
        for workspace in self.trusted.iter() {
            if let Some(path_str) = workspace.to_str() {
                trust_text += &format!("{path_str}\n");
            }
        }
        // let chains aren't supported in current MSRV
        if let Ok(false) = fs::exists(data_dir()) {
            if let Err(e) = fs::create_dir_all(data_dir()) {
                log::error!("Couldn't create helix's data directory: {:?}", e);
            };
        }
        if let Err(e) = fs::write(workspace_trust_file(), trust_text) {
            log::error!("Error during write of workspace_trust file: {:?}", e);
        }
    }

    fn write_exclusion_to_file(&self) {
        if let Some(untrusted) = &self.excluded {
            let mut trust_text = String::new();
            for workspace in untrusted.iter() {
                if let Some(path_str) = workspace.to_str() {
                    trust_text += &format!("{path_str}\n");
                }
            }
            // let chains aren't supported in current MSRV
            if let Ok(false) = fs::exists(data_dir()) {
                if let Err(e) = fs::create_dir_all(data_dir()) {
                    log::error!("Couldn't create helix's data directory: {:?}", e);
                };
            }
            if let Err(e) = fs::write(workspace_exclude_file(), trust_text) {
                log::error!("Error during write of workspace_trust file: {:?}", e);
            }
        } else {
            log::error!("Called write_untrust_to_file() when self.untrusted is None");
        }
    }

    /// Mark current workspace trusted
    pub fn trust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.insert(workspace.clone());
        self.write_trust_to_file();
        let mut cache = WORKSPACE_TRUST_CACHE.lock();
        cache.insert(workspace, TrustUntrustStatus::AllowAlways);
    }

    /// Remove trusted mark from current workspace
    pub fn untrust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        if self.trusted.remove(&workspace) {
            // only update the file if there was a change
            self.write_trust_to_file();
        }
        let mut cache = WORKSPACE_TRUST_CACHE.lock();
        cache.insert(workspace, TrustUntrustStatus::DenyOnce);
    }

    /// Mark current workspace excluded.
    ///
    /// Should be called only if `WorkspaceTrust` was created with `WorkspaceTrust::load(true)`
    pub fn exclude_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.remove(&workspace);
        if let Some(excluded) = &mut self.excluded {
            excluded.insert(workspace);
            self.write_exclusion_to_file();
        } else {
            log::error!("Called untrust_workspace_permanent() when self.untrusted is None");
        }
        let workspace = crate::find_workspace().0;
        let mut cache = WORKSPACE_TRUST_CACHE.lock();
        cache.insert(workspace, TrustUntrustStatus::DenyAlways);
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum TrustUntrustStatus {
    DenyAlways,
    #[default]
    DenyOnce,
    AllowAlways,
}

fn is_path_in_file(needle: &Path, haystack_path: &Path) -> bool {
    let file = match fs::File::open(haystack_path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            log::debug!("workspace trust file {haystack_path:?} does not exist");
            return false;
        }
        Err(err) => {
            log::error!("workspace trust file {haystack_path:?} couldn't be read: {err:?}");
            return false;
        }
    };

    for (lineno, line) in BufReader::new(file).lines().enumerate() {
        match line {
            Ok(line) => {
                if PathBuf::from(line) == needle {
                    return true;
                }
            }
            Err(err) => {
                log::error!(
                    "workspace trust file {haystack_path:?}:{} couldn't be read: {err:?}",
                    lineno + 1
                )
            }
        }
    }

    false
}

pub fn quick_query_workspace(config: &Config) -> TrustStatus {
    match quick_query_workspace_with_explicit_untrust(config) {
        Some(TrustUntrustStatus::AllowAlways) => TrustStatus::Trusted,
        _ => TrustStatus::Untrusted,
    }
}

fn is_path_matching_glob(path: &Path, glob: &str, home: &Path) -> bool {
    let (path, glob) = if let Some(glob_remain) = glob.strip_prefix("~/") {
        match path.strip_prefix(home) {
            Ok(path_remain) => (path_remain, glob_remain),
            Err(_) => return false,
        }
    } else {
        (path, glob)
    };

    match GlobBuilder::new(glob).literal_separator(true).build() {
        Ok(glob) => glob.compile_matcher().is_match(path),
        _ => false,
    }
}

#[test]
fn is_path_matching_glob_test() {
    let test_cases = vec![
        (
            PathBuf::from("/home/user/repo"),
            "/home/user/repo",
            PathBuf::from("/home/user"),
            true,
        ),
        (
            PathBuf::from("/home/user/repo"),
            "~/repo",
            PathBuf::from("/home/user"),
            true,
        ),
        (
            PathBuf::from("/home/user2/repo"),
            "~/repo",
            PathBuf::from("/home/user"),
            false,
        ),
    ];

    for (path, glob, home, result) in test_cases {
        assert_eq!(
            is_path_matching_glob(&path, glob, &home),
            result,
            "is_path_matching_glob({path:?}, \"{glob}\", {home:?}) != {result}"
        );
    }
}

fn trust_from_globs(
    workspace: &Path,
    home: &Path,
    globs: &Vec<String>,
) -> Option<TrustUntrustStatus> {
    let mut result: Option<TrustUntrustStatus> = None;
    for glob in globs {
        if let Some(glob) = glob.strip_prefix("!") {
            if is_path_matching_glob(workspace, glob, home) {
                result = Some(TrustUntrustStatus::DenyAlways);
            }
        } else {
            if is_path_matching_glob(workspace, &glob, home) {
                result = Some(TrustUntrustStatus::AllowAlways);
            }
        }
    }

    result
}

#[test]
fn trust_from_globs_test() {
    let home = PathBuf::from("/home/user");

    let globs: Vec<String> = vec!["!**".to_string(), "~/repos/helix".to_string()];
    let cases = vec![
        ("/foo/bar", Some(TrustUntrustStatus::DenyAlways)),
        (
            "/home/user/repos/helix",
            Some(TrustUntrustStatus::AllowAlways),
        ),
        (
            "/home/user/repos/helix/branch_a",
            Some(TrustUntrustStatus::DenyAlways),
        ),
    ];

    for (workspace, result) in cases {
        assert_eq!(
            trust_from_globs(&PathBuf::from(workspace), &home, &globs),
            result,
            "trust_from_globs({workspace:?}, {home:?}, globs) != {result:?}"
        );
    }

    // This matches the examples given in the documentation, see
    // book/src/workspace-trust.md
    let globs: Vec<String> = vec![
        "~/repos/helix".to_string(),
        "~/repos/foo/*".to_string(),
        "~/repos/bar/**".to_string(),
        "!~/repos/bar/untrusted".to_string(),
    ];
    let cases = vec![
        ("/home/user/foobar", None),
        (
            "/home/user/repos/helix",
            Some(TrustUntrustStatus::AllowAlways),
        ),
        ("/home/other/repos/helix", None),
        ("/home/user/repos/helix/branch_a", None),
        ("/home/user/repos/foo", None),
        (
            "/home/user/repos/foo/branch_a",
            Some(TrustUntrustStatus::AllowAlways),
        ),
        ("/home/user/repos/foo/remote_a/branch_a", None),
        (
            "/home/user/repos/bar/branch_a",
            Some(TrustUntrustStatus::AllowAlways),
        ),
        (
            "/home/user/repos/bar/remote_a/branch_a",
            Some(TrustUntrustStatus::AllowAlways),
        ),
        (
            "/home/user/repos/bar/untrusted",
            Some(TrustUntrustStatus::DenyAlways),
        ),
    ];

    for (workspace, result) in cases {
        assert_eq!(
            trust_from_globs(&PathBuf::from(workspace), &home, &globs),
            result,
            "trust_from_globs({workspace:?}, {home:?}, globs) != {result:?}"
        );
    }
}

pub fn quick_query_workspace_with_explicit_untrust(config: &Config) -> Option<TrustUntrustStatus> {
    let workspace = crate::find_workspace().0;
    let mut cache = WORKSPACE_TRUST_CACHE.lock();
    if let Some(trust) = cache.get(&workspace) {
        return Some(*trust);
    }

    // trust_from_config is cheap for `trust_config.paths.len() == 0`. But
    // bailing out with DenyAlways when no `$HOME` directory is available is
    // better only done if there actualy is a trust configuration that does
    //  understand `~` as shorthand for `$HOME`.
    if config.paths.len() > 0 {
        let Some(home_dir) = env::home_dir() else {
            log::error!("Unable to get HOME directory needed to process trust configuration. Denying trust as fallback.");
            return Some(TrustUntrustStatus::DenyAlways);
        };
        if let Some(trust) = trust_from_globs(&workspace, &home_dir, &config.paths) {
            cache.insert(workspace, trust);
            return Some(trust);
        }
    }

    if is_path_in_file(&workspace, &workspace_trust_file()) {
        cache.insert(workspace, TrustUntrustStatus::AllowAlways);
        return Some(TrustUntrustStatus::AllowAlways);
    }

    if is_path_in_file(&workspace, &workspace_exclude_file()) {
        cache.insert(workspace, TrustUntrustStatus::DenyAlways);
        return Some(TrustUntrustStatus::DenyAlways);
    }

    None
}

pub fn clear_trust_cache() {
    WORKSPACE_TRUST_CACHE.lock().clear();
}
