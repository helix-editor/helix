use helix_stdx::faccess::write_sensitive_file;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::{
    data_dir, workspace_config_file, workspace_exclude_file, workspace_lang_config_file,
    workspace_trust_file,
};

#[derive(Clone)]
pub struct WorkspaceTrust {
    cache: Arc<Mutex<HashMap<PathBuf, TrustStatus>>>,
    trust_level: ImplicitTrustLevel,
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
    pub fn new(trust_level: ImplicitTrustLevel) -> Self {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        Self { cache, trust_level }
    }

    pub fn new_bogus() -> Self {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        Self {
            cache,
            trust_level: ImplicitTrustLevel::All,
        }
    }

    /// Mark current workspace trusted
    pub fn trust_workspace(&self) {
        let workspace = crate::find_workspace().0;
        let mut mutable = WorkspaceTrustMutable::load(false);
        mutable.trust_workspace(&workspace);
        let mut cache = self.cache.lock();
        cache.insert(workspace, TrustStatus::Trusted);
    }

    /// Remove trusted mark from current workspace
    pub fn untrust_workspace(&self) {
        let workspace = crate::find_workspace().0;
        let mut mutable = WorkspaceTrustMutable::load(true);
        mutable.untrust_workspace(&workspace);
        let mut cache = self.cache.lock();
        cache.remove(&workspace);
    }

    /// Mark current workspace excluded.
    pub fn exclude_workspace(&self) {
        let workspace = crate::find_workspace().0;
        let mut mutable = WorkspaceTrustMutable::load(true);
        mutable.exclude_workspace(&workspace);
        let mut cache = self.cache.lock();
        cache.insert(workspace, TrustStatus::Untrusted);
    }

    pub fn cache_non_trust_in_current_workspace(&self) {
        let mut cache = self.cache.lock();
        cache.insert(crate::find_workspace().0, TrustStatus::Untrusted);
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
            return Some(*trust);
        }

        if is_path_in_file(&workspace, &workspace_trust_file()) {
            cache.insert(workspace, TrustStatus::Trusted);
            return Some(TrustStatus::Trusted);
        }

        if is_path_in_file(&workspace, &workspace_exclude_file()) {
            cache.insert(workspace, TrustStatus::Untrusted);
            return Some(TrustStatus::Untrusted);
        }

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
