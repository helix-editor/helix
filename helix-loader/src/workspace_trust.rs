use helix_stdx::faccess::write_sensitive_file;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use parking_lot::Mutex;

pub static WORKSPACE_TRUST_CACHE: Lazy<Mutex<HashMap<PathBuf, TrustStatus>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub static WORKSPACE_IMPLICIT_TRUST_LEVEL: Lazy<Mutex<ImplicitTrustLevel>> =
    Lazy::new(|| Mutex::new(ImplicitTrustLevel::None));

use crate::{
    data_dir, workspace_config_file, workspace_exclude_file, workspace_lang_config_file,
    workspace_trust_file,
};

pub struct WorkspaceTrust {
    trusted: HashSet<PathBuf>,
    excluded: Option<HashSet<PathBuf>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustStatus {
    Untrusted,
    Trusted,
}

/// Level at which trust is applied implicitly:
///
/// `None`: don't trust anything implicitly;
/// `Lsp`: trust LSP server implicitly;
/// `All`: trust everything implicitly.
#[derive(Default)]
pub enum ImplicitTrustLevel {
    #[default]
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
    /// Loads `WorkspaceTrust`.
    ///
    /// Should be used only when there is a need to change trust status
    /// of a particular workspace.
    ///
    /// For querying trust status of a workspace use `quick_query_workspace()` or
    /// `quick_query_workspace_with_explicit_untrust()`
    pub fn load(with_exclusion: bool) -> Self {
        let trusted = match fs::read_to_string(workspace_trust_file()) {
            Ok(workspace_trust_file) => workspace_trust_file
                .split('\n')
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect(),
            Err(e) => {
                log::error!("workspace file couldn't be read: {:?}", e);
                HashSet::new()
            }
        };

        let excluded = if with_exclusion {
            let untrusted = match fs::read_to_string(workspace_exclude_file()) {
                Ok(workspace_untrust_file) => workspace_untrust_file
                    .split('\n')
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from)
                    .collect(),
                Err(e) => {
                    log::error!("workspace file couldn't be read: {:?}", e);
                    HashSet::new()
                }
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
    pub fn trust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.insert(workspace.clone());
        self.write_trust_to_file();
        let mut cache = WORKSPACE_TRUST_CACHE.lock();
        cache.insert(workspace, TrustStatus::Trusted);
    }

    /// Remove trusted mark from current workspace
    pub fn untrust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        if self.trusted.remove(&workspace) {
            // only update the file if there was a change
            self.write_trust_to_file();
        }
        let mut cache = WORKSPACE_TRUST_CACHE.lock();
        cache.insert(workspace, TrustStatus::Untrusted);
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
        cache.insert(workspace, TrustStatus::Untrusted);
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

/// Sets the level of implicit untrust
///
/// Should be called when the level is known,
/// otherwise implicit untrust defaults to `None`
pub fn set_implicit_trust_level(trust_level: ImplicitTrustLevel) {
    let mut global_trust_level = WORKSPACE_IMPLICIT_TRUST_LEVEL.lock();
    *global_trust_level = trust_level;
}

pub fn cache_non_trust_in_current_workspace() {
    let mut cache = WORKSPACE_TRUST_CACHE.lock();
    cache.insert(crate::find_workspace().0, TrustStatus::Untrusted);
}

pub fn quick_query_workspace(trust_type: TrustType) -> TrustStatus {
    match quick_query_workspace_with_explicit_untrust(trust_type) {
        Some(status) => status,
        _ => TrustStatus::Untrusted,
    }
}

/// `None` indicates that trust level is unknown;
///
/// `value` in `Some(value)` is known trust status.
pub fn quick_query_workspace_with_explicit_untrust(trust_type: TrustType) -> Option<TrustStatus> {
    let trust_level = WORKSPACE_IMPLICIT_TRUST_LEVEL.lock();

    if let ImplicitTrustLevel::All = *trust_level {
        return Some(TrustStatus::Trusted);
    }

    // no let-chains in our rust edition
    if let TrustType::Lsp = trust_type {
        if let ImplicitTrustLevel::Lsp = *trust_level {
            return Some(TrustStatus::Trusted);
        };
    }

    // no let-chains in our rust edition
    if let TrustType::Select {
        language_servers_to_load,
    } = trust_type
    {
        if let ImplicitTrustLevel::Lsp = *trust_level {
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

    let workspace = crate::find_workspace().0;
    let mut cache = WORKSPACE_TRUST_CACHE.lock();
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

pub fn clear_trust_cache() {
    let mut cache = WORKSPACE_TRUST_CACHE.lock();
    cache.clear();
}
