//! Configuration for helix's VCS handling, to provide diffs and current VCS state.

use serde::{Deserialize, Serialize};

/// Main configuration struct, to be embedded in the larger editor configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Vcs {
    /// Providers for diff and state.
    ///
    /// The default is either the empty vec or just git (if the relevant feature is active).
    #[serde(default = "default_providers")]
    pub providers: Vec<Provider>,
}

impl Default for Vcs {
    fn default() -> Self {
        Self {
            providers: default_providers(),
        }
    }
}

/// Supported providers
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    #[cfg(feature = "git")]
    Git,
}

fn default_providers() -> Vec<Provider> {
    vec![
        #[cfg(feature = "git")]
        Provider::Git,
    ]
}
