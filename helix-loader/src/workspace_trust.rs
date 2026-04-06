use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use parking_lot::Mutex;

pub static WORKSPACE_TRUST_CACHE: Lazy<Mutex<HashMap<std::path::PathBuf, TrustUntrustStatus>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

use once_cell::sync::Lazy;

use crate::{data_dir, workspace_exclude_file, workspace_trust_file};

pub struct WorkspaceTrust {
    trusted: HashSet<PathBuf>,
    excluded: Option<HashSet<PathBuf>>,
}

#[derive(Clone, Copy)]
pub enum TrustStatus {
    Untrusted,
    Trusted,
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
        self.trusted.insert(workspace);
        self.write_trust_to_file();
        let workspace = crate::find_workspace().0;
        let mut cache = WORKSPACE_TRUST_CACHE.lock();
        cache.insert(workspace, TrustUntrustStatus::AllowAlways);
    }

    /// Remove trusted mark from current workspace
    pub fn untrust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.remove(&workspace);
        self.write_trust_to_file();
        let workspace = crate::find_workspace().0;
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

#[derive(Default, Clone, Copy, Debug)]
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

pub fn quick_query_workspace(insecure: bool) -> TrustStatus {
    match quick_query_workspace_with_explicit_untrust(insecure) {
        Some(TrustUntrustStatus::AllowAlways) => TrustStatus::Trusted,
        _ => TrustStatus::Untrusted,
    }
}

pub fn quick_query_workspace_with_explicit_untrust(insecure: bool) -> Option<TrustUntrustStatus> {
    if insecure {
        return Some(TrustUntrustStatus::AllowAlways);
    }

    let workspace = crate::find_workspace().0;
    let mut cache = WORKSPACE_TRUST_CACHE.lock();
    if let Some(trust) = cache.get(&workspace) {
        return Some(*trust);
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
