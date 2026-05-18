use crate::{
    data_dir, workspace_config_file, workspace_exclude_file, workspace_lang_config_file,
    workspace_trust_file,
};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use helix_stdx::faccess::write_sensitive_file;
use parking_lot::Mutex;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub struct WorkspaceTrust {
    cache: Arc<Mutex<HashMap<PathBuf, Option<TrustStatus>>>>,
    trust_level: ImplicitTrustLevel,
    trust_globs: GlobSet,
    exclude_globs: GlobSet,
}

pub struct Config {
    pub trust_level: ImplicitTrustLevel,
    pub globs: Vec<String>,
}

struct WorkspaceTrustMutable {
    trusted: HashSet<PathBuf>,
    excluded: Option<HashSet<PathBuf>>,
}

impl WorkspaceTrustMutable {
    /// Loads `WorkspaceTrust`.
    ///
    /// Should be used only when there is a need to change trust status
    /// of a particular workspace.
    ///
    /// For querying trust status of a workspace use `quick_query_workspace()` or
    /// `quick_query_workspace_with_explicit_untrust()`
    fn load(with_exclusion: bool) -> Self {
        let load = |path| match fs::read_to_string(path) {
            Ok(string) => string
                .split('\n')
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect(),
            Err(e) => {
                log::error!("Workspace trust file couldn't be read: {:?}", e);
                HashSet::new()
            }
        };

        let trusted = load(workspace_trust_file());
        let excluded = if with_exclusion {
            Some(load(workspace_exclude_file()))
        } else {
            None
        };
        WorkspaceTrustMutable { trusted, excluded }
    }

    fn write_trust_to_file(&self) {
        let mut trust_text = String::new();
        for workspace in self.trusted.iter() {
            if let Some(path_str) = workspace.to_str() {
                if path_str.contains('\n') {
                    log::error!("Unsupported path (contains \\n): {:?}", path_str);
                    continue;
                }
                trust_text += path_str;
                trust_text += "\n";
            }
        }
        // let chains aren't supported in current MSRV
        if let Ok(false) = fs::exists(data_dir()) {
            if let Err(e) = fs::create_dir_all(data_dir()) {
                log::error!("Couldn't create helix's data directory: {:?}", e);
            };
        }
        if let Err(e) = write_sensitive_file(workspace_trust_file(), trust_text) {
            log::error!("Error during write of workspace_trust file: {:?}", e);
        }
    }

    fn write_exclusion_to_file(&self) {
        if let Some(untrusted) = &self.excluded {
            let mut trust_text = String::new();
            for workspace in untrusted.iter() {
                if let Some(path_str) = workspace.to_str() {
                    if path_str.contains('\n') {
                        log::error!("Unsupported path (contains \\n): {:?}", path_str);
                        continue;
                    }
                    trust_text += path_str;
                    trust_text += "\n";
                }
            }
            // let chains aren't supported in current MSRV
            if let Ok(false) = fs::exists(data_dir()) {
                if let Err(e) = fs::create_dir_all(data_dir()) {
                    log::error!("Couldn't create helix's data directory: {:?}", e);
                };
            }
            if let Err(e) = write_sensitive_file(workspace_exclude_file(), trust_text) {
                log::error!("Error during write of workspace_trust file: {:?}", e);
            }
        } else {
            log::error!("Called write_untrust_to_file() when self.untrusted is None");
        }
    }

    /// Mark current workspace trusted
    fn trust_workspace(&mut self, workspace: &Path) {
        self.trusted.insert(workspace.to_path_buf());
        self.write_trust_to_file();
    }

    /// Remove trusted mark from current workspace
    fn untrust_workspace(&mut self, workspace: &Path) {
        if self.trusted.remove(workspace) {
            self.write_trust_to_file();
        }

        if let Some(excluded) = self.excluded.as_mut() {
            if excluded.remove(workspace) {
                self.write_exclusion_to_file();
            }
        }
    }
    /// Mark current workspace excluded.
    ///
    /// Should be called only if `WorkspaceTrustMutable` was created with `WorkspaceTrustMutable::load(true)`
    fn exclude_workspace(&mut self, workspace: &Path) {
        self.trusted.remove(workspace);
        if let Some(excluded) = &mut self.excluded {
            excluded.insert(workspace.to_path_buf());
            self.write_trust_to_file();
            self.write_exclusion_to_file();
        } else {
            log::error!("Called exclude_workspace() when self.untrusted is None");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustStatus {
    Untrusted,
    Trusted,
}

impl TrustStatus {
    pub fn is_trusted(&self) -> bool {
        matches!(self, Self::Trusted)
    }

    pub fn is_untrusted(&self) -> bool {
        matches!(self, Self::Untrusted)
    }
}

/// Level at which trust is applied implicitly:
///
/// `None`: don't trust anything implicitly;
/// `Lsp`: trust LSP server implicitly;
/// `All`: trust everything implicitly.
#[derive(Clone)]
pub enum ImplicitTrustLevel {
    None,
    Lsp,
    All,
}

/// Declares what kind of subsystem is querying trust
///
/// Anything that doesn't touch LSP or Select pop-up menu should choose `Other`
pub enum TrustType {
    Lsp,
    Select { language_servers_to_load: bool },
    Other,
}

impl WorkspaceTrust {
    pub fn new(config: Config) -> Self {
        let expand_home = |path: &str| {
            if let Some(home) = crate::home_dir().to_str() {
                #[cfg(not(windows))]
                let expanded = format!("{home}/");
                #[cfg(windows)]
                let expanded = format!("{home}\\");
                path.replace("~/", &expanded)
            } else {
                path.to_string()
            }
        };

        let cache = Arc::new(Mutex::new(HashMap::new()));

        let mut trust_builder = GlobSetBuilder::new();
        let mut exclude_builder = GlobSetBuilder::new();
        for string in config.globs {
            if let Some(Ok(glob)) = string
                .strip_prefix('!')
                .map(expand_home)
                .as_deref()
                .map(|v| GlobBuilder::new(v).literal_separator(true).build())
            {
                exclude_builder.add(glob);
            } else if let Ok(glob) = GlobBuilder::new(&expand_home(&string))
                .literal_separator(true)
                .build()
            {
                trust_builder.add(glob);
            }
        }

        let trust_globs = trust_builder.build().unwrap_or_default();
        let exclude_globs = exclude_builder.build().unwrap_or_default();

        Self {
            cache,
            trust_level: config.trust_level,
            trust_globs,
            exclude_globs,
        }
    }

    pub fn new_bogus() -> Self {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        Self {
            cache,
            trust_level: ImplicitTrustLevel::All,
            trust_globs: Default::default(),
            exclude_globs: Default::default(),
        }
    }

    /// Mark current workspace trusted
    pub fn trust_workspace(&self) {
        let workspace = crate::find_workspace().0;
        let mut mutable = WorkspaceTrustMutable::load(false);
        mutable.trust_workspace(&workspace);
        let mut cache = self.cache.lock();
        cache.insert(workspace, Some(TrustStatus::Trusted));
    }

    /// Remove trusted mark from current workspace
    pub fn untrust_workspace(&self) {
        let workspace = crate::find_workspace().0;
        let mut mutable = WorkspaceTrustMutable::load(true);
        mutable.untrust_workspace(&workspace);
        let mut cache = self.cache.lock();
        cache.insert(workspace, None);
    }

    /// Mark current workspace excluded.
    pub fn exclude_workspace(&self) {
        let workspace = crate::find_workspace().0;
        let mut mutable = WorkspaceTrustMutable::load(true);
        mutable.exclude_workspace(&workspace);
        let mut cache = self.cache.lock();
        cache.insert(workspace, Some(TrustStatus::Untrusted));
    }

    pub fn cache_non_trust_in_current_workspace(&self) {
        let mut cache = self.cache.lock();
        cache.insert(crate::find_workspace().0, Some(TrustStatus::Untrusted));
    }

    pub fn cache_trust_in_current_workspace(&self) {
        let mut cache = self.cache.lock();
        cache.insert(crate::find_workspace().0, Some(TrustStatus::Trusted));
    }

    /// Query trust in current workspace.
    pub fn query_status(&self, trust_type: TrustType) -> TrustStatus {
        self.query_status_with_explicit_untrust(trust_type)
            .unwrap_or(TrustStatus::Untrusted)
    }

    /// `None` indicates that trust level is unknown;
    ///
    /// `value` in `Some(value)` is known trust status.
    pub fn query_status_with_explicit_untrust(&self, trust_type: TrustType) -> Option<TrustStatus> {
        if let ImplicitTrustLevel::All = self.trust_level {
            return Some(TrustStatus::Trusted);
        }

        // no let-chains in our rust edition
        if let TrustType::Lsp = trust_type {
            if let ImplicitTrustLevel::Lsp = self.trust_level {
                return Some(TrustStatus::Trusted);
            };
        }

        // no let-chains in our rust edition
        if let TrustType::Select {
            language_servers_to_load,
        } = trust_type
        {
            if let ImplicitTrustLevel::Lsp = self.trust_level {
                if !(workspace_config_file().exists() || workspace_lang_config_file().exists()) {
                    return Some(TrustStatus::Trusted);
                }
            };
            if !language_servers_to_load
                && !workspace_config_file().exists()
                && !workspace_lang_config_file().exists()
            {
                return Some(TrustStatus::Trusted);
            }
        }

        let mut cache = self.cache.lock();
        let workspace = crate::find_workspace().0;
        if let Some(trust) = cache.get(&workspace) {
            return *trust;
        }

        if is_path_in_file(&workspace, &workspace_trust_file())
            || self.trust_globs.is_match(&workspace)
        {
            cache.insert(workspace, Some(TrustStatus::Trusted));
            return Some(TrustStatus::Trusted);
        }

        if is_path_in_file(&workspace, &workspace_exclude_file())
            || self.exclude_globs.is_match(&workspace)
        {
            cache.insert(workspace, Some(TrustStatus::Untrusted));
            return Some(TrustStatus::Untrusted);
        }

        cache.insert(workspace, None);
        None
    }
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
                if Path::new(&line) == needle {
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

#[cfg(test)]
mod test {
    use crate::{
        home_dir,
        workspace_trust::{Config, TrustStatus, WorkspaceTrust},
    };
    use helix_stdx::env::set_current_working_dir;

    #[test]
    fn workspace_trust_home_expansion() {
        // I sure hope that this will never backfire and we will never
        // get bugreports saying that people get some garbage directories
        // in their $HOME after running `cargo test --workspace`
        let temp_dir = tempfile::tempdir_in(home_dir()).unwrap();
        let temp_path = temp_dir.path();

        let globs = vec!["~/**".to_string()];
        let wst_globbed = WorkspaceTrust::new(Config {
            trust_level: super::ImplicitTrustLevel::None,
            globs,
        });

        let wst_empty = WorkspaceTrust::new(Config {
            trust_level: super::ImplicitTrustLevel::None,
            globs: Vec::new(),
        });

        set_current_working_dir(temp_path).unwrap();

        // I would print something helpful here, but debug-print
        // of globsets is REALLY verbose, that wouldn't help much.
        assert_eq!(
            wst_globbed.query_status(super::TrustType::Other),
            TrustStatus::Trusted,
            "WorkspaceTrust::new() doesn't expand '~/' properly."
        );

        assert_eq!(
            wst_empty.query_status(super::TrustType::Other),
            TrustStatus::Untrusted,
            "This is a sanity check. The test itself is likely wrong."
        );
    }
}
