use std::path::Path;

#[cfg(feature = "git")]
pub use git::Git;
#[cfg(not(feature = "git"))]
pub use Dummy as Git;

#[cfg(feature = "git")]
mod git;

mod diff;

pub use diff::{DiffHandle, Hunk};

pub struct VersionControlData {
    pub diff_base: Vec<u8>,
    pub head_name: String,
}

pub trait DiffProvider {
    /// Returns the data that a diff should be computed against
    /// if this provider is used.
    /// The data is returned as raw byte without any decoding or encoding performed
    /// to ensure all file encodings are handled correctly.
    fn get_version_control_data(&self, file: &Path) -> Option<VersionControlData>;
}

#[doc(hidden)]
pub struct Dummy;
impl DiffProvider for Dummy {
    fn get_version_control_data(&self, _file: &Path) -> Option<VersionControlData> {
        None
    }
}

pub struct DiffProviderRegistry {
    pub last_known_head_name: Option<String>,
    providers: Vec<Box<dyn DiffProvider>>,
}

impl DiffProviderRegistry {
    pub fn load_version_control_data(&mut self, file: &Path) -> Option<VersionControlData> {
        let data = self
            .providers
            .iter()
            .find_map(|provider| provider.get_version_control_data(file));

        self.last_known_head_name = data.as_ref().map(|data| data.head_name.clone());
        data
    }
}

impl Default for DiffProviderRegistry {
    fn default() -> Self {
        // currently only git is supported
        // TODO make this configurable when more providers are added
        let git: Box<dyn DiffProvider> = Box::new(Git);
        let providers = vec![git];

        DiffProviderRegistry {
            providers,
            last_known_head_name: None,
        }
    }
}
