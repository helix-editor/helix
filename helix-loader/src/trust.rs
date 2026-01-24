//! Workspace trust management for Helix.
//!
//! This module provides functionality to track whether workspaces are trusted,
//! which determines whether potentially dangerous features like LSP servers,
//! shell commands, and workspace-local configuration are enabled.
//!
//! # Configuration
//!
//! Users can configure trust profiles in their config.toml:
//!
//! ```toml
//! [editor.trust]
//! default = "prompt"  # "prompt" | "trust" | "untrust"
//!
//! [editor.trust.trusted]
//! lsp = true
//! dap = true
//! shell-commands = true
//! workspace-config = true
//!
//! [editor.trust.untrusted]
//! lsp = false
//! dap = false
//! shell-commands = false
//! workspace-config = false
//!
//! # Per-workspace overrides
//! [[editor.trust.workspaces]]
//! path = "~/.config/nvim"
//! lsp = true
//! dap = false
//! shell-commands = true
//! workspace-config = false
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Trust status for a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    /// Workspace is explicitly trusted - uses trusted profile.
    Trusted,
    /// Workspace is explicitly untrusted - uses untrusted profile.
    Untrusted,
    /// Trust status is unknown - user should be prompted.
    #[default]
    Unknown,
}

/// Default behavior for unknown workspaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrustDefault {
    /// Prompt the user to decide (default).
    #[default]
    Prompt,
    /// Automatically trust all workspaces.
    Trust,
    /// Automatically untrust all workspaces.
    Untrust,
}

/// A trust profile defining what features are allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TrustProfile {
    /// Whether LSP servers are allowed.
    #[serde(default = "default_true")]
    pub lsp: bool,
    /// Whether DAP (debug adapters) are allowed.
    #[serde(default = "default_true")]
    pub dap: bool,
    /// Whether shell commands are allowed.
    #[serde(default = "default_true")]
    pub shell_commands: bool,
    /// Whether workspace-local config (.helix/) is loaded.
    #[serde(default = "default_true")]
    pub workspace_config: bool,
}

fn default_true() -> bool {
    true
}

impl Default for TrustProfile {
    fn default() -> Self {
        Self {
            lsp: true,
            dap: true,
            shell_commands: true,
            workspace_config: true,
        }
    }
}

impl TrustProfile {
    /// Create a fully trusted profile (all features enabled).
    pub fn trusted() -> Self {
        Self::default()
    }

    /// Create a fully untrusted profile (all dangerous features disabled).
    pub fn untrusted() -> Self {
        Self {
            lsp: false,
            dap: false,
            shell_commands: false,
            workspace_config: false,
        }
    }
}

/// Per-workspace trust configuration override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WorkspaceTrustOverride {
    /// The workspace path (supports ~ expansion).
    pub path: String,
    /// Whether LSP servers are allowed.
    #[serde(default)]
    pub lsp: Option<bool>,
    /// Whether DAP (debug adapters) are allowed.
    #[serde(default)]
    pub dap: Option<bool>,
    /// Whether shell commands are allowed.
    #[serde(default)]
    pub shell_commands: Option<bool>,
    /// Whether workspace-local config (.helix/) is loaded.
    #[serde(default)]
    pub workspace_config: Option<bool>,
}

impl WorkspaceTrustOverride {
    /// Convert this override into a full TrustProfile, using defaults for unset values.
    pub fn to_profile(&self, defaults: &TrustProfile) -> TrustProfile {
        TrustProfile {
            lsp: self.lsp.unwrap_or(defaults.lsp),
            dap: self.dap.unwrap_or(defaults.dap),
            shell_commands: self.shell_commands.unwrap_or(defaults.shell_commands),
            workspace_config: self.workspace_config.unwrap_or(defaults.workspace_config),
        }
    }

    /// Get the expanded path.
    pub fn expanded_path(&self) -> PathBuf {
        helix_stdx::path::expand_tilde(Path::new(&self.path)).to_path_buf()
    }
}

/// Full trust configuration from editor config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TrustConfig {
    /// Default behavior for unknown workspaces.
    #[serde(default)]
    pub default: TrustDefault,
    /// Profile for trusted workspaces.
    #[serde(default = "TrustProfile::trusted")]
    pub trusted: TrustProfile,
    /// Profile for untrusted workspaces.
    #[serde(default = "TrustProfile::untrusted")]
    pub untrusted: TrustProfile,
    /// Per-workspace overrides.
    #[serde(default)]
    pub workspaces: Vec<WorkspaceTrustOverride>,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            default: TrustDefault::default(),
            trusted: TrustProfile::trusted(),
            untrusted: TrustProfile::untrusted(),
            workspaces: Vec::new(),
        }
    }
}

impl TrustConfig {
    /// Find a workspace-specific override for the given path.
    ///
    /// Resolves symlinks in both the workspace path and override paths to ensure
    /// consistent matching regardless of how the path was accessed.
    ///
    /// For best performance, pass an already-canonicalized workspace path.
    pub fn find_workspace_override(&self, workspace: &Path) -> Option<&WorkspaceTrustOverride> {
        // Always try to canonicalize the workspace path to resolve symlinks,
        // regardless of whether it is absolute or relative.
        let expanded = helix_stdx::path::canonicalize(workspace);
        let canonical = std::fs::canonicalize(&expanded).unwrap_or(expanded);

        self.workspaces.iter().find(|w| {
            // Resolve symlinks for the override path
            let override_expanded = helix_stdx::path::canonicalize(w.expanded_path());
            let override_path = std::fs::canonicalize(&override_expanded).unwrap_or(override_expanded);
            canonical == override_path || canonical.starts_with(&override_path)
        })
    }

    /// Resolve the trust profile for a workspace.
    ///
    /// Resolution order:
    /// 1. Check for workspace-specific override in config
    /// 2. Use trusted/untrusted profile based on trust level
    pub fn resolve_profile(&self, workspace: &Path, level: TrustLevel) -> TrustProfile {
        // Check for workspace-specific override first
        if let Some(override_config) = self.find_workspace_override(workspace) {
            let base = match level {
                TrustLevel::Trusted => &self.trusted,
                TrustLevel::Untrusted | TrustLevel::Unknown => &self.untrusted,
            };
            return override_config.to_profile(base);
        }

        // Fall back to trust level profiles
        match level {
            TrustLevel::Trusted => self.trusted.clone(),
            TrustLevel::Untrusted | TrustLevel::Unknown => self.untrusted.clone(),
        }
    }
}

/// A single trust entry for a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEntry {
    /// The trust level for this workspace.
    pub level: TrustLevel,
    /// Unix timestamp when the trust decision was made.
    pub decided_at: u64,
}

/// Persisted trust decisions for all workspaces.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceTrustStore {
    /// Schema version for future migrations.
    #[serde(default)]
    pub version: u32,
    /// Map from canonical workspace path to trust entry.
    #[serde(default)]
    pub workspaces: HashMap<PathBuf, TrustEntry>,
}

/// Runtime trust state for the current workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceTrust {
    /// The workspace path.
    pub workspace_path: PathBuf,
    /// Current trust level.
    pub trust_level: TrustLevel,
    /// Whether trust was set via CLI flag (overrides persistence).
    pub cli_override: bool,
    /// The resolved trust profile for this workspace.
    pub profile: TrustProfile,
}

impl WorkspaceTrust {
    /// Create a new WorkspaceTrust with the given parameters.
    pub fn new(
        workspace_path: PathBuf,
        trust_level: TrustLevel,
        cli_override: bool,
        profile: TrustProfile,
    ) -> Self {
        Self {
            workspace_path,
            trust_level,
            cli_override,
            profile,
        }
    }

    /// Returns true if the workspace is trusted.
    pub fn is_trusted(&self) -> bool {
        matches!(self.trust_level, TrustLevel::Trusted)
    }

    /// Returns true if trust status is unknown (user should be prompted).
    pub fn is_pending(&self) -> bool {
        matches!(self.trust_level, TrustLevel::Unknown)
    }

    /// Returns true if LSP is allowed for this workspace.
    pub fn lsp_allowed(&self) -> bool {
        self.profile.lsp
    }

    /// Returns true if DAP is allowed for this workspace.
    pub fn dap_allowed(&self) -> bool {
        self.profile.dap
    }

    /// Returns true if shell commands are allowed for this workspace.
    pub fn shell_allowed(&self) -> bool {
        self.profile.shell_commands
    }

    /// Returns true if workspace config should be loaded.
    pub fn workspace_config_allowed(&self) -> bool {
        self.profile.workspace_config
    }
}

impl Default for WorkspaceTrust {
    fn default() -> Self {
        Self {
            workspace_path: PathBuf::new(),
            trust_level: TrustLevel::Unknown,
            cli_override: false,
            profile: TrustProfile::untrusted(),
        }
    }
}

/// Returns the path to the workspace trust file.
pub fn trust_file() -> PathBuf {
    crate::config_dir().join("workspace-trust.toml")
}

/// Load the trust store from disk.
///
/// Returns a default empty store if the file doesn't exist or can't be parsed.
pub fn load_trust_store() -> WorkspaceTrustStore {
    let path = trust_file();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(store) => store,
                Err(e) => {
                    log::warn!("Failed to parse workspace trust file: {}", e);
                    WorkspaceTrustStore::default()
                }
            },
            Err(e) => {
                log::warn!("Failed to read workspace trust file: {}", e);
                WorkspaceTrustStore::default()
            }
        }
    } else {
        WorkspaceTrustStore::default()
    }
}

/// Save the trust store to disk.
pub fn save_trust_store(store: &WorkspaceTrustStore) -> std::io::Result<()> {
    let path = trust_file();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let contents = toml::to_string_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&path, contents)
}

/// Canonicalize a workspace path for consistent lookups.
///
/// This resolves symlinks to ensure that `/home/user/my-link` and
/// `/home/user/actual-project` are recognized as the same workspace
/// when `my-link` is a symlink to `actual-project`.
///
/// Falls back to path normalization if the path doesn't exist.
fn canonicalize_workspace(workspace: &Path) -> PathBuf {
    // First expand tilde and make absolute
    let expanded = helix_stdx::path::canonicalize(workspace);

    // Try to resolve symlinks with std::fs::canonicalize
    // This requires the path to exist, so fall back to expanded path if it fails
    std::fs::canonicalize(&expanded).unwrap_or(expanded)
}

/// Get the trust level for a workspace.
///
/// Returns `TrustLevel::Unknown` if no trust decision has been recorded.
/// Get the trust level for a workspace.
///
/// Supports nested workspaces - if `~/Workspace/helix/` is not explicitly set,
/// but `~/Workspace/` is trusted, then `~/Workspace/helix/` inherits that trust.
/// More specific paths take precedence over parent paths.
///
/// Returns `TrustLevel::Unknown` if no trust decision has been recorded for
/// this workspace or any of its parents.
pub fn get_workspace_trust(workspace: &Path) -> TrustLevel {
    let store = load_trust_store();
    let canonical = canonicalize_workspace(workspace);

    // First check for exact match
    if let Some(entry) = store.workspaces.get(&canonical) {
        return entry.level;
    }

    // Check parent directories, finding the most specific (longest) match
    let mut best_match: Option<(&PathBuf, &TrustEntry)> = None;

    for (path, entry) in &store.workspaces {
        if canonical.starts_with(path) {
            match best_match {
                None => best_match = Some((path, entry)),
                Some((best_path, _)) if path.as_os_str().len() > best_path.as_os_str().len() => {
                    best_match = Some((path, entry));
                }
                _ => {}
            }
        }
    }

    best_match
        .map(|(_, entry)| entry.level)
        .unwrap_or(TrustLevel::Unknown)
}

/// Set the trust level for a workspace.
///
/// Persists the decision to the trust store file.
pub fn set_workspace_trust(workspace: &Path, level: TrustLevel) -> std::io::Result<()> {
    let mut store = load_trust_store();
    let canonical = canonicalize_workspace(workspace);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    store.workspaces.insert(
        canonical,
        TrustEntry {
            level,
            decided_at: now,
        },
    );

    save_trust_store(&store)
}

/// Remove trust decision for a workspace.
///
/// After removal, the workspace will be treated as `TrustLevel::Unknown`.
pub fn clear_workspace_trust(workspace: &Path) -> std::io::Result<()> {
    let mut store = load_trust_store();
    let canonical = canonicalize_workspace(workspace);

    store.workspaces.remove(&canonical);

    save_trust_store(&store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_level_default() {
        assert_eq!(TrustLevel::default(), TrustLevel::Unknown);
    }

    #[test]
    fn test_workspace_trust_is_trusted() {
        let trust = WorkspaceTrust::new(
            PathBuf::from("/test"),
            TrustLevel::Trusted,
            false,
            TrustProfile::trusted(),
        );
        assert!(trust.is_trusted());
        assert!(!trust.is_pending());
        assert!(trust.lsp_allowed());

        let trust = WorkspaceTrust::new(
            PathBuf::from("/test"),
            TrustLevel::Untrusted,
            false,
            TrustProfile::untrusted(),
        );
        assert!(!trust.is_trusted());
        assert!(!trust.is_pending());
        assert!(!trust.lsp_allowed());

        let trust = WorkspaceTrust::new(
            PathBuf::from("/test"),
            TrustLevel::Unknown,
            false,
            TrustProfile::untrusted(),
        );
        assert!(!trust.is_trusted());
        assert!(trust.is_pending());
    }

    #[test]
    fn test_trust_store_serialization() {
        let mut store = WorkspaceTrustStore::default();
        store.workspaces.insert(
            PathBuf::from("/test/workspace"),
            TrustEntry {
                level: TrustLevel::Trusted,
                decided_at: 1234567890,
            },
        );

        let serialized = toml::to_string_pretty(&store).unwrap();
        let deserialized: WorkspaceTrustStore = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.workspaces.len(), 1);
        let entry = deserialized
            .workspaces
            .get(&PathBuf::from("/test/workspace"))
            .unwrap();
        assert_eq!(entry.level, TrustLevel::Trusted);
        assert_eq!(entry.decided_at, 1234567890);
    }

    #[test]
    fn test_trust_profile_trusted() {
        let profile = TrustProfile::trusted();
        assert!(profile.lsp);
        assert!(profile.dap);
        assert!(profile.shell_commands);
        assert!(profile.workspace_config);
    }

    #[test]
    fn test_trust_profile_untrusted() {
        let profile = TrustProfile::untrusted();
        assert!(!profile.lsp);
        assert!(!profile.dap);
        assert!(!profile.shell_commands);
        assert!(!profile.workspace_config);
    }

    #[test]
    fn test_trust_default_enum() {
        assert_eq!(TrustDefault::default(), TrustDefault::Prompt);

        // Test deserialization via a wrapper struct
        #[derive(Deserialize)]
        struct Wrapper {
            value: TrustDefault,
        }

        let prompt: Wrapper = toml::from_str("value = \"prompt\"").unwrap();
        assert_eq!(prompt.value, TrustDefault::Prompt);

        let trust: Wrapper = toml::from_str("value = \"trust\"").unwrap();
        assert_eq!(trust.value, TrustDefault::Trust);

        let untrust: Wrapper = toml::from_str("value = \"untrust\"").unwrap();
        assert_eq!(untrust.value, TrustDefault::Untrust);
    }

    #[test]
    fn test_trust_config_resolve_profile_trusted() {
        let config = TrustConfig::default();
        let profile = config.resolve_profile(Path::new("/test"), TrustLevel::Trusted);

        assert!(profile.lsp);
        assert!(profile.dap);
        assert!(profile.shell_commands);
        assert!(profile.workspace_config);
    }

    #[test]
    fn test_trust_config_resolve_profile_untrusted() {
        let config = TrustConfig::default();
        let profile = config.resolve_profile(Path::new("/test"), TrustLevel::Untrusted);

        assert!(!profile.lsp);
        assert!(!profile.dap);
        assert!(!profile.shell_commands);
        assert!(!profile.workspace_config);
    }

    #[test]
    fn test_trust_config_resolve_profile_unknown() {
        let config = TrustConfig::default();
        let profile = config.resolve_profile(Path::new("/test"), TrustLevel::Unknown);

        // Unknown should resolve to untrusted profile
        assert!(!profile.lsp);
        assert!(!profile.dap);
        assert!(!profile.shell_commands);
        assert!(!profile.workspace_config);
    }

    #[test]
    fn test_workspace_trust_override_to_profile() {
        let override_config = WorkspaceTrustOverride {
            path: "/test".to_string(),
            lsp: Some(true),
            dap: Some(false),
            shell_commands: None,
            workspace_config: Some(true),
        };

        let defaults = TrustProfile::untrusted();
        let profile = override_config.to_profile(&defaults);

        assert!(profile.lsp); // Overridden to true
        assert!(!profile.dap); // Overridden to false
        assert!(!profile.shell_commands); // Uses default (false)
        assert!(profile.workspace_config); // Overridden to true
    }

    #[test]
    fn test_workspace_trust_all_allowed_methods() {
        let trust = WorkspaceTrust::new(
            PathBuf::from("/test"),
            TrustLevel::Trusted,
            false,
            TrustProfile {
                lsp: true,
                dap: false,
                shell_commands: true,
                workspace_config: false,
            },
        );

        assert!(trust.lsp_allowed());
        assert!(!trust.dap_allowed());
        assert!(trust.shell_allowed());
        assert!(!trust.workspace_config_allowed());
    }

    #[test]
    fn test_workspace_trust_default() {
        let trust = WorkspaceTrust::default();
        assert_eq!(trust.trust_level, TrustLevel::Unknown);
        assert!(!trust.cli_override);
        assert!(!trust.lsp_allowed());
        assert!(!trust.dap_allowed());
        assert!(!trust.shell_allowed());
        assert!(!trust.workspace_config_allowed());
    }

    #[test]
    fn test_trust_profile_serialization() {
        let profile = TrustProfile {
            lsp: true,
            dap: false,
            shell_commands: true,
            workspace_config: false,
        };

        let serialized = toml::to_string(&profile).unwrap();
        let deserialized: TrustProfile = toml::from_str(&serialized).unwrap();

        assert_eq!(profile, deserialized);
    }

    #[test]
    fn test_trust_config_with_workspaces() {
        let toml_str = r#"
            default = "prompt"

            [trusted]
            lsp = true
            dap = true
            shell-commands = true
            workspace-config = true

            [untrusted]
            lsp = false
            dap = false
            shell-commands = false
            workspace-config = false

            [[workspaces]]
            path = "/trusted/project"
            lsp = true
            dap = true
            shell-commands = true
            workspace-config = true
        "#;

        let config: TrustConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default, TrustDefault::Prompt);
        assert_eq!(config.workspaces.len(), 1);
        assert_eq!(config.workspaces[0].path, "/trusted/project");
    }

    #[test]
    fn test_canonicalize_workspace_normalizes_path() {
        // Test that paths with . and .. are normalized
        let path = Path::new("/test/./foo/../bar");
        let canonical = canonicalize_workspace(path);

        // The path should be normalized (though the exact result depends on
        // whether /test/bar exists - if not, we get the normalized version)
        assert!(!canonical.to_string_lossy().contains("./"));
        assert!(!canonical.to_string_lossy().contains(".."));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_resolution() {
        use std::os::unix::fs::symlink;

        // Create a temp directory structure with a symlink
        let temp_dir = std::env::temp_dir().join("helix_trust_test");
        let actual_dir = temp_dir.join("actual");
        let link_dir = temp_dir.join("link");

        // Clean up any previous test artifacts
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Create directories
        std::fs::create_dir_all(&actual_dir).unwrap();

        // Create symlink
        symlink(&actual_dir, &link_dir).unwrap();

        // Both paths should canonicalize to the same path
        let canonical_actual = canonicalize_workspace(&actual_dir);
        let canonical_link = canonicalize_workspace(&link_dir);

        assert_eq!(canonical_actual, canonical_link);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_nested_workspace_trust_resolution() {
        // Test that child workspaces inherit trust from parents
        let mut store = WorkspaceTrustStore::default();

        // Trust the parent workspace
        store.workspaces.insert(
            PathBuf::from("/home/user/Workspace"),
            TrustEntry {
                level: TrustLevel::Trusted,
                decided_at: 1000,
            },
        );

        // Untrust a specific child
        store.workspaces.insert(
            PathBuf::from("/home/user/Workspace/untrusted-project"),
            TrustEntry {
                level: TrustLevel::Untrusted,
                decided_at: 2000,
            },
        );

        // Helper to check trust from store
        let get_trust = |path: &Path| -> TrustLevel {
            let canonical = path.to_path_buf();

            // First check exact match
            if let Some(entry) = store.workspaces.get(&canonical) {
                return entry.level;
            }

            // Check parent directories
            let mut best_match: Option<(&PathBuf, &TrustEntry)> = None;
            for (p, entry) in &store.workspaces {
                if canonical.starts_with(p) {
                    match best_match {
                        None => best_match = Some((p, entry)),
                        Some((best_path, _)) if p.as_os_str().len() > best_path.as_os_str().len() => {
                            best_match = Some((p, entry));
                        }
                        _ => {}
                    }
                }
            }
            best_match.map(|(_, e)| e.level).unwrap_or(TrustLevel::Unknown)
        };

        // Parent is trusted
        assert_eq!(
            get_trust(Path::new("/home/user/Workspace")),
            TrustLevel::Trusted
        );

        // Child inherits trust from parent
        assert_eq!(
            get_trust(Path::new("/home/user/Workspace/helix")),
            TrustLevel::Trusted
        );

        // Deeply nested child also inherits
        assert_eq!(
            get_trust(Path::new("/home/user/Workspace/helix/src/commands")),
            TrustLevel::Trusted
        );

        // Explicitly untrusted child overrides parent
        assert_eq!(
            get_trust(Path::new("/home/user/Workspace/untrusted-project")),
            TrustLevel::Untrusted
        );

        // Children of untrusted inherit untrust
        assert_eq!(
            get_trust(Path::new("/home/user/Workspace/untrusted-project/src")),
            TrustLevel::Untrusted
        );

        // Unrelated path is unknown
        assert_eq!(
            get_trust(Path::new("/home/user/other")),
            TrustLevel::Unknown
        );
    }
}
