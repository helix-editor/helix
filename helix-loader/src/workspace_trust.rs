use std::{collections::HashSet, fs, path::PathBuf};

use crate::{data_dir, workspace_trust_file, workspace_untrust_file};

pub struct WorkspaceTrust {
    trusted: HashSet<PathBuf>,
    untrusted: Option<HashSet<PathBuf>>,
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
    pub fn load(explicit_untrust: bool) -> Self {
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

        let untrusted = if explicit_untrust {
            let mut untrusted = HashSet::new();

            match fs::read_to_string(workspace_untrust_file()) {
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
        WorkspaceTrust { trusted, untrusted }
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

    fn write_untrust_to_file(&self) {
        if let Some(untrusted) = &self.untrusted {
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
            if let Err(e) = fs::write(workspace_untrust_file(), trust_text) {
                log::error!("Error during write of workspace_trust file: {:?}", e);
            }
        } else {
            log::error!("Called write_untrust_to_file() when self.untrusted is None");
        }
    }

    /// Trust this workspace
    pub fn trust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.insert(workspace);
        self.write_trust_to_file();
    }

    /// Don't trust this workspace from now onward
    pub fn untrust_workspace(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.remove(&workspace);
        self.write_trust_to_file();
    }

    /// Don't trust this workspace from now onward and mark it explicitly untrusted.
    ///
    /// Should be called only if `WorkspaceTrust` was created with `WorkspaceTrust::load(true)`
    pub fn untrust_workspace_permanent(&mut self) {
        let workspace = crate::find_workspace().0;
        self.trusted.remove(&workspace);
        if let Some(untrusted) = &mut self.untrusted {
            untrusted.insert(workspace);
            self.write_untrust_to_file();
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

    match fs::read_to_string(workspace_untrust_file()) {
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
