use helix_stdx::faccess::write_with_perms;
use std::{collections::HashSet, fs, path::PathBuf};

use crate::{data_dir, workspace_exclude_file, workspace_trust_file};

pub struct WorkspaceTrust {
    trusted: HashSet<PathBuf>,
    excluded: Option<HashSet<PathBuf>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        let trusted: HashSet<_> = match fs::read_to_string(workspace_trust_file()) {
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
            let untrusted: HashSet<_> = match fs::read_to_string(workspace_exclude_file()) {
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
        // TO-DO: apply mask for group and others, while setting owner
        if let Err(e) = write_with_perms(workspace_trust_file(), trust_text, 0o0640) {
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
            // TO-DO: apply mask for group and others, while setting owner
            if let Err(e) = write_with_perms(workspace_exclude_file(), trust_text, 0o0640) {
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
    }

    /// Remove trusted mark from current workspace
    pub fn untrust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.remove(&workspace);
        self.write_trust_to_file();
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
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub enum TrustUntrustStatus {
    DenyAlways,
    #[default]
    DenyOnce,
    AllowAlways,
}

pub fn quick_query_workspace(insecure: bool) -> TrustStatus {
    if insecure {
        return TrustStatus::Trusted;
    }

    let workspace = crate::find_workspace().0;
    match fs::read_to_string(workspace_trust_file()) {
        Ok(workspace_trust_file) => {
            for line in workspace_trust_file.split('\n') {
                if PathBuf::from(line) == workspace {
                    return TrustStatus::Trusted;
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
        Err(err) => log::error!("workspace file couldn't be read: {err:?}"),
    };
    TrustStatus::Untrusted
}

pub fn quick_query_workspace_with_explicit_untrust(insecure: bool) -> TrustUntrustStatus {
    if insecure {
        return TrustUntrustStatus::AllowAlways;
    }

    let workspace = crate::find_workspace().0;
    match fs::read_to_string(workspace_trust_file()) {
        Ok(workspace_trust_file) => {
            for line in workspace_trust_file.split('\n') {
                if PathBuf::from(line) == workspace {
                    return TrustUntrustStatus::AllowAlways;
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
        Err(err) => log::error!("workspace_trust file couldn't be read: {err:?}"),
    };

    match fs::read_to_string(workspace_exclude_file()) {
        Ok(workspace_untrust_file) => {
            for line in workspace_untrust_file.split('\n') {
                if PathBuf::from(line) == workspace {
                    return TrustUntrustStatus::DenyAlways;
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
        Err(err) => log::error!("workspace_untrust file couldn't be read: {err:?}"),
    };
    TrustUntrustStatus::DenyOnce
}
