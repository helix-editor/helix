use std::{collections::HashSet, fs, path::PathBuf};

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

pub fn quick_query_workspace() -> TrustStatus {
    let workspace = crate::find_workspace().0;
    match fs::read_to_string(workspace_trust_file()) {
        Ok(workspace_trust_file) => {
            for line in workspace_trust_file.split('\n') {
                if PathBuf::from(line) == workspace {
                    return TrustStatus::Trusted;
                }
            }
        }
        Err(e) => log::error!("workspace file couldn't be read: {:?}", e),
    };
    TrustStatus::Untrusted
}

pub fn quick_query_workspace_with_explicit_untrust() -> TrustUntrustStatus {
    let workspace = crate::find_workspace().0;
    match fs::read_to_string(workspace_trust_file()) {
        Ok(workspace_trust_file) => {
            for line in workspace_trust_file.split('\n') {
                if PathBuf::from(line) == workspace {
                    return TrustUntrustStatus::AllowAlways;
                }
            }
        }
        Err(e) => log::error!("workspace_trust file couldn't be read: {:?}", e),
    };

    match fs::read_to_string(workspace_exclude_file()) {
        Ok(workspace_untrust_file) => {
            for line in workspace_untrust_file.split('\n') {
                if PathBuf::from(line) == workspace {
                    return TrustUntrustStatus::DenyAlways;
                }
            }
        }
        Err(e) => log::error!("workspace_untrust file couldn't be read: {:?}", e),
    };
    TrustUntrustStatus::DenyOnce
}
