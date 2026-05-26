//! Workspace trust.
//!
//! Helix can load workspace-local configuration (`.helix/`) and launch language servers, both of
//! which can execute arbitrary code. By default these are gated behind explicit user trust granted
//! per-workspace.
//!
//! Trust is granted with `:workspace-trust` (or the popup) and revoked with `:workspace-untrust`.
//! A grant snapshots a hash of every file under `.helix/`. If those files change later the
//! workspace becomes [`TrustStatus::Stale`] and local config is no longer loaded until the user
//! re-runs `:workspace-trust`. Language servers continue to launch under stale trust because the
//! binaries are configured globally and were not part of the changed surface.
//!
//! ## Storage
//!
//! Each trust entry is a small file at `data_dir()/workspace_trust/<sha256(workspace_path)>`. The
//! filename is the SHA-256 of the workspace's absolute path; the contents are a small
//! `key = value` block:
//!
//! ```text
//! path = /home/user/proj1
//! hash = sha256:abc123...
//! excluded = false
//! ```
//!
//! `hash` is omitted for excluded entries. The "one file per workspace" shape is safe under
//! multiple concurrent helix instances writing trust for *different* workspaces (different
//! filenames). Two instances racing to trust the same workspace converge to identical content.
//!
//! ## Trusted globs (discouraged)
//!
//! `[editor.workspace-trust] trusted = [...]` is an escape hatch for users who keep many repos in a
//! predictable layout (`~/src/github.com/me/*`) and don't want to grant trust one workspace at a
//! time. A workspace whose path matches one of these globs is implicitly trusted for everything.
//!
//! This is deliberately weaker than an explicit `:workspace-trust` grant and is discouraged: it
//! bypasses the `.helix/` hash pin (changes to local config are never re-checked) and it trusts any
//! repository that happens to land under a matching directory, including ones cloned there later. An
//! explicit exclude still wins over a matching glob.

use hashbrown::HashMap;
use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};

use crate::{data_dir, find_workspace, find_workspace_in};

/// Checks whether a specific capability is trusted
#[derive(Debug, Clone, Copy)]
pub enum TrustQuery {
    /// Query language server permissions
    Lsp,
    /// Query debug adapter permissions
    Dap,
    /// Query whether `.helix/` config can be loaded
    LocalConfig,
    /// Query whether git integration can trust the .git/config
    Git,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustStatus {
    /// Workspace is trusted for the queried capability.
    Trusted,
    /// No trust decision has been made (and no implicit trust applies).
    Untrusted,
    /// Workspace was previously trusted, but the `.helix/` tree has changed since the grant — the
    /// user should re-trust before local config is re-loaded. LSP launches may still proceed under
    /// stale trust because they use the (unchanged) globally-configured binaries.
    Stale,
    /// Workspace is on the exclude list. Never prompts again.
    Excluded,
}

impl TrustStatus {
    pub fn is_trusted(&self) -> bool {
        matches!(self, Self::Trusted)
    }

    pub fn is_stale(&self) -> bool {
        matches!(self, Self::Stale)
    }

    pub fn is_excluded(&self) -> bool {
        matches!(self, Self::Excluded)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImplicitTrustLevel {
    /// Prompt for every workspace.
    None,
    /// Helix-launched server processes (LSP and DAP) start implicitly. Workspace-local config and
    /// git `Trust::Full` still require explicit trust.
    ///
    /// Default: language servers are how most people use the editor and the binaries are global
    /// (PATH-resolved, user-installed), so auto-launching them in a fresh workspace matches
    /// expectations.
    #[default]
    Servers,
    /// Everything is trusted unless the workspace is explicitly excluded.
    Insecure,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub level: ImplicitTrustLevel,
    /// Whether to surface the trust modal on `DocumentDidOpen` for untrusted workspaces.
    pub prompt: bool,
    /// Workspaces whose path matches one of these globs are implicitly trusted.
    pub trusted_globs: GlobSet,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            level: ImplicitTrustLevel::default(),
            prompt: true,
            trusted_globs: GlobSet::empty(),
        }
    }
}

/// Compile workspace-trust glob patterns into a matcher. `~` and environment variables are expanded
/// in each pattern; invalid patterns are logged and skipped rather than failing the whole config
/// load. Returns an empty set (matches nothing) when `patterns` is empty or all entries are invalid.
pub fn build_trusted_globs(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let expanded = helix_stdx::path::expand(pattern);
        match Glob::new(&expanded.to_string_lossy()) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(err) => log::error!("ignoring invalid workspace-trust glob {pattern:?}: {err}"),
        }
    }
    builder.build().unwrap_or_else(|err| {
        log::error!("failed to compile workspace-trust globs: {err}");
        GlobSet::empty()
    })
}

/// Runtime workspace-trust state. Cheap to clone (shared `Arc`).
#[derive(Clone)]
pub struct WorkspaceTrust {
    inner: Arc<Mutex<HashMap<PathBuf, CacheEntry>>>,
    config: Config,
}

#[derive(Clone, Copy)]
struct CacheEntry {
    status: TrustStatus,
    /// Whether `.helix/config.toml` or `.helix/languages.toml` exists in the workspace at the time
    /// of the first uncached query. Snapshotted to keep [`WorkspaceTrust::workspace_restricted`]
    /// off the syscall path on repeat calls (statusline indicator runs per render).
    has_local_config: bool,
}

impl WorkspaceTrust {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// A trust state that grants every capability. Use for non-interactive contexts (CLI grammar
    /// build, `hx --health`) where prompting isn't meaningful.
    pub fn fully_trusted() -> Self {
        Self::new(Config {
            level: ImplicitTrustLevel::Insecure,
            ..Config::default()
        })
    }

    pub fn implicit_level(&self) -> ImplicitTrustLevel {
        self.config.level
    }

    /// Whether the trust modal should be surfaced on `DocumentDidOpen`. When false the user is
    /// only informed via the statusline indicator and acts explicitly via `:workspace-trust`.
    pub fn prompts_enabled(&self) -> bool {
        self.config.prompt
    }

    /// Replace the configuration in-place. Clears the trust cache so the next query re-reads from
    /// disk. This catches external mutations to `.helix/` that happened while helix was running
    /// (the editor's in-memory cache would otherwise keep returning a stale `Trusted` even though
    /// `compute_workspace_hash` would now produce a different digest). Used by `:config-reload`.
    ///
    /// Session-only decisions made via [`Self::deny_once`] are discarded as part of the cache
    /// clear; the trust popup's `prompted` set (scoped to the hook closure) is what suppresses
    /// re-prompting across the reload, not this cache.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
        self.inner.lock().clear();
    }

    /// Raw on-disk trust status for `workspace`, ignoring implicit-trust-level shortcuts and the
    /// `demote_for_query` mapping. Use this when you need to distinguish *Stale* (was trusted,
    /// `.helix/` changed) from *Untrusted* (never trusted)
    pub fn status(&self, workspace: &Path) -> TrustStatus {
        self.entry(workspace).status
    }

    /// Cache entry for `workspace`, loaded from disk on first call.
    fn entry(&self, workspace: &Path) -> CacheEntry {
        if let Some(entry) = self.inner.lock().get(workspace).copied() {
            return entry;
        }
        let status = load_status(workspace);
        let has_local_config = has_local_config(workspace);
        let entry = CacheEntry {
            status,
            has_local_config,
        };
        self.inner.lock().insert(workspace.to_path_buf(), entry);
        entry
    }

    /// Query trust for `workspace` from the perspective of a specific subsystem.
    pub fn query(&self, workspace: &Path, query: TrustQuery) -> TrustStatus {
        let raw = self.status(workspace);

        // Explicit excludes always win.
        if raw == TrustStatus::Excluded {
            return TrustStatus::Excluded;
        }

        // Implicit-trust shortcuts only apply once we've ruled out excludes.
        if self.config.level == ImplicitTrustLevel::Insecure {
            return TrustStatus::Trusted;
        }
        // A config-listed glob grants full implicit trust to the matching workspace.
        if self.is_glob_trusted(workspace) {
            return TrustStatus::Trusted;
        }
        if self.config.level == ImplicitTrustLevel::Servers
            && matches!(query, TrustQuery::Lsp | TrustQuery::Dap)
        {
            return TrustStatus::Trusted;
        }

        demote_for_query(raw, query)
    }

    /// Query trust for the workspace containing `file`.
    pub fn query_for_file(&self, file: &Path, query: TrustQuery) -> TrustStatus {
        let workspace = file
            .parent()
            .map(|dir| find_workspace_in(dir).0)
            .unwrap_or_else(|| find_workspace().0);
        self.query(&workspace, query)
    }

    /// Query trust for the current working directory's workspace.
    pub fn query_current(&self, query: TrustQuery) -> TrustStatus {
        let workspace = find_workspace().0;
        self.query(&workspace, query)
    }

    /// Whether `workspace` matches one of the configured trust globs. Empty glob set never matches.
    fn is_glob_trusted(&self, workspace: &Path) -> bool {
        !self.config.trusted_globs.is_empty() && self.config.trusted_globs.is_match(workspace)
    }

    /// Workspace-wide "is this workspace in restricted mode and would running `trust`
    /// change anything visible at the workspace level?" check.
    ///
    /// Reads the entire cache entry through `Self::entry` in a single lock acquisition (so the
    /// `Untrusted` branch's `has_local_config` snapshot is consistent with the status read). Cheap
    /// on the hot render path after the first query.
    pub fn workspace_restricted(&self, workspace: &Path) -> bool {
        // If the workspace is fully trusted there's no restrictions.
        if self.config.level == ImplicitTrustLevel::Insecure || self.is_glob_trusted(workspace) {
            return false;
        }
        let entry = self.entry(workspace);
        match entry.status {
            TrustStatus::Stale => true,
            TrustStatus::Trusted | TrustStatus::Excluded => false,
            TrustStatus::Untrusted => entry.has_local_config,
        }
    }

    /// Per-document version of [`Self::workspace_restricted`].
    pub fn restricted_for_doc(&self, workspace: &Path, servers_to_load: bool) -> bool {
        if self.workspace_restricted(workspace) {
            return true;
        }
        if !servers_to_load {
            return false;
        }
        if self.status(workspace) != TrustStatus::Untrusted {
            return false;
        }
        !self.query(workspace, TrustQuery::Lsp).is_trusted()
            || !self.query(workspace, TrustQuery::Dap).is_trusted()
    }

    /// Mark `workspace` trusted. Snapshots the current `.helix/` hash.
    pub fn trust(&self, workspace: &Path) {
        let hash = compute_workspace_hash(workspace);
        let has_local_config = has_local_config(workspace);
        write_entry(
            workspace,
            &DiskEntry {
                hash,
                excluded: false,
            },
        );
        self.inner.lock().insert(
            workspace.to_path_buf(),
            CacheEntry {
                status: TrustStatus::Trusted,
                has_local_config,
            },
        );
    }

    /// Revoke any persisted trust grant or exclusion for `workspace`.
    pub fn untrust(&self, workspace: &Path) {
        remove_entry(workspace);
        self.inner.lock().remove(workspace);
    }

    /// Mark `workspace` excluded — never prompts again.
    pub fn exclude(&self, workspace: &Path) {
        let has_local_config = has_local_config(workspace);
        write_entry(
            workspace,
            &DiskEntry {
                hash: None,
                excluded: true,
            },
        );
        self.inner.lock().insert(
            workspace.to_path_buf(),
            CacheEntry {
                status: TrustStatus::Excluded,
                has_local_config,
            },
        );
    }

    /// Cache an untrusted decision for the current session.
    pub fn deny_once(&self, workspace: &Path) {
        let has_local_config = has_local_config(workspace);
        self.inner.lock().insert(
            workspace.to_path_buf(),
            CacheEntry {
                status: TrustStatus::Untrusted,
                has_local_config,
            },
        );
    }
}

fn has_local_config(workspace: &Path) -> bool {
    workspace.join(".helix").join("config.toml").exists()
        || workspace.join(".helix").join("languages.toml").exists()
}

fn demote_for_query(status: TrustStatus, query: TrustQuery) -> TrustStatus {
    // Stale workspaces have their `.helix/` config changed since trust was granted. LSP launches
    // still rely on globally-configured binaries that weren't part of the changed surface, so they
    // remain Trusted; other queries demote to Untrusted until the user re-trusts.
    match (status, query) {
        (TrustStatus::Stale, TrustQuery::Lsp) => TrustStatus::Trusted,
        (TrustStatus::Stale, _) => TrustStatus::Untrusted,
        _ => status,
    }
}

fn load_status(workspace: &Path) -> TrustStatus {
    match read_entry(workspace) {
        Some(entry) if entry.excluded => TrustStatus::Excluded,
        Some(entry) => {
            let current = compute_workspace_hash(workspace);
            if entry.hash == current {
                TrustStatus::Trusted
            } else {
                TrustStatus::Stale
            }
        }
        None => TrustStatus::Untrusted,
    }
}

// ---------- on-disk format ----------
//
// `data_dir()/workspace_trust/<sha256(canonical_workspace_path)>` is a small key-value file. The
// filename derives from the path, so concurrent helix instances writing trust for *different*
// workspaces never touch the same file. The original path is stored inside as a sanity check and
// for `cat workspace_trust/*` debugging.

struct DiskEntry {
    hash: Option<String>,
    excluded: bool,
}

fn workspace_trust_dir() -> PathBuf {
    data_dir().join("workspace_trust")
}

fn entry_path(workspace: &Path) -> PathBuf {
    workspace_trust_dir().join(path_filename(workspace))
}

fn path_filename(workspace: &Path) -> String {
    let mut hasher = Sha256::new();
    // `Path` is OsStr; encode lossy-but-deterministically. Two workspaces whose normalized paths
    // differ only in non-UTF-8 bytes will collide here, but find_workspace() returns paths derived
    // from CWD walking and that's already lossy on the FS side — accepting parity.
    hasher.update(workspace.as_os_str().to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    format!("{digest:x}")
}

fn read_entry(workspace: &Path) -> Option<DiskEntry> {
    let path = entry_path(workspace);
    let contents = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            log::error!("workspace trust file {path:?} unreadable: {err:?}");
            return None;
        }
    };

    let mut stored_path: Option<String> = None;
    let mut hash: Option<String> = None;
    let mut excluded = false;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim();
        match k {
            "path" => stored_path = Some(v.to_string()),
            "hash" => hash = Some(v.to_string()),
            "excluded" => excluded = v == "true",
            _ => {}
        }
    }

    // Sanity check that we didn't hit a path collision: the path inside the file should match the
    // workspace we looked up.
    if let Some(stored) = stored_path {
        if Path::new(&stored) != workspace {
            log::error!(
                "workspace trust file {path:?} contains path {stored:?}, expected {workspace:?}"
            );
            return None;
        }
    }

    Some(DiskEntry { hash, excluded })
}

fn write_entry(workspace: &Path, entry: &DiskEntry) {
    let dir = workspace_trust_dir();
    if let Err(err) = fs::create_dir_all(&dir) {
        log::error!("Couldn't create workspace trust dir {dir:?}: {err:?}");
        return;
    }
    let path = entry_path(workspace);

    let mut contents = String::new();
    let _ = writeln!(contents, "path = {}", workspace.display());
    if let Some(hash) = &entry.hash {
        let _ = writeln!(contents, "hash = {hash}");
    }
    let _ = writeln!(contents, "excluded = {}", entry.excluded);

    if let Err(err) = fs::write(&path, contents) {
        log::error!("Error writing workspace trust file {path:?}: {err:?}");
    }
}

fn remove_entry(workspace: &Path) {
    let path = entry_path(workspace);
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => log::error!("Error removing workspace trust file {path:?}: {err:?}"),
    }
}

// ---------- hashing ----------

/// SHA-256 of all files under `.helix/`, used to detect changes to local config after trust was
/// granted. Returns `None` if `.helix/` is absent or has no files, so a workspace with no local
/// config can still be trusted.
pub fn compute_workspace_hash(workspace: &Path) -> Option<String> {
    let helix_dir = workspace.join(".helix");
    if !helix_dir.is_dir() {
        return None;
    }

    let mut files: Vec<PathBuf> = Vec::new();
    walk(&helix_dir, &mut files);
    if files.is_empty() {
        return None;
    }
    files.sort();

    let mut hasher = Sha256::new();
    for file in &files {
        let rel = file.strip_prefix(&helix_dir).unwrap_or(file);
        hash_field(&mut hasher, rel.to_string_lossy().as_bytes());
        match fs::read(file) {
            Ok(bytes) => hash_field(&mut hasher, &bytes),
            Err(err) => {
                log::warn!("workspace hash: treating unreadable file {file:?} as empty: {err:?}");
                hash_field(&mut hasher, &[]);
            }
        }
    }
    let digest = hasher.finalize();
    Some(format!("sha256:{digest:x}"))
}

/// Length-prefix a field before feeding it to the hasher.
fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(direct) = entry.file_type() else {
            continue;
        };
        if direct.is_dir() {
            // Real directories only — don't follow symlinked directories to avoid filesystem
            // cycles via path-loop symlinks.
            walk(&path, out);
        } else if direct.is_file() {
            out.push(path);
        } else if direct.is_symlink() {
            // Symlinks to files (extremely common: dotfiles managers symlink `.helix/config.toml`
            // to an external location). Follow once via `fs::metadata` which traverses links, then
            // include the file in the hash so mutations to the *target* are still detected.
            if let Ok(target) = fs::metadata(&path) {
                if target.is_file() {
                    out.push(path);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn hash_changes_when_helix_dir_changes() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        let empty = compute_workspace_hash(workspace);
        assert_eq!(empty, None, ".helix/ absent should hash to None");

        write_file(&workspace.join(".helix").join("config.toml"), "a = 1");
        let h1 = compute_workspace_hash(workspace).expect("has files");
        assert!(h1.starts_with("sha256:"));

        // Same content → same hash
        let h1b = compute_workspace_hash(workspace).expect("has files");
        assert_eq!(h1, h1b);

        // Different content → different hash
        write_file(&workspace.join(".helix").join("config.toml"), "a = 2");
        let h2 = compute_workspace_hash(workspace).expect("has files");
        assert_ne!(h1, h2);

        // Added file → different hash
        write_file(&workspace.join(".helix").join("languages.toml"), "");
        let h3 = compute_workspace_hash(workspace).expect("has files");
        assert_ne!(h2, h3);
    }

    #[test]
    fn sha256_known_vector() {
        // Cross-check sha2 crate against a well-known SHA-256 of "abc".
        let mut h = Sha256::new();
        h.update(b"abc");
        assert_eq!(
            format!("{:x}", h.finalize()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn workspace_restricted_hides_when_no_local_config() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        let trust = WorkspaceTrust::new(Config::default());
        // Untrusted + no .helix/ → indicator should hide.
        assert!(!trust.workspace_restricted(workspace));

        // Untrusted + .helix/config.toml exists → indicator should show.
        write_file(&workspace.join(".helix").join("config.toml"), "a = 1");
        // Bust the cache so the new file is picked up.
        trust.untrust(workspace);
        assert!(trust.workspace_restricted(workspace));

        // After explicit trust, indicator hides.
        trust.trust(workspace);
        assert!(!trust.workspace_restricted(workspace));

        // After mutation (and revoking the cached entry to simulate fresh load), workspace becomes
        // Stale → indicator should show again.
        write_file(&workspace.join(".helix").join("config.toml"), "a = 2");
        trust.inner.lock().remove(workspace);
        assert!(trust.workspace_restricted(workspace));
    }

    #[test]
    fn set_config_invalidates_cache_so_stale_is_detected() {
        // Regression: prior to this fix the editor's WorkspaceTrust cache held a `Trusted` entry
        // after `:workspace-trust` was run. If the user (or another process) modified `.helix/`
        // while helix was running, subsequent queries kept returning Trusted even though
        // `Config::load_default`'s transient WorkspaceTrust would correctly see Stale. Reloading
        // config now clears the cache so the editor sees the new state on the next query.
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();
        write_file(&workspace.join(".helix").join("config.toml"), "a = 1");

        let mut trust = WorkspaceTrust::new(Config::default());
        trust.trust(workspace);
        assert_eq!(trust.status(workspace), TrustStatus::Trusted);

        // External mutation while helix is running.
        write_file(&workspace.join(".helix").join("config.toml"), "a = 2");

        // Without a refresh, the cache still says Trusted.
        assert_eq!(trust.status(workspace), TrustStatus::Trusted);

        // Simulate `:config-reload`.
        trust.set_config(Config::default());

        // Fresh disk read now detects the hash mismatch.
        assert_eq!(trust.status(workspace), TrustStatus::Stale);
    }

    #[cfg(unix)]
    #[test]
    fn hash_includes_symlinked_config_file() {
        // Regression: `.helix/config.toml` is commonly a symlink to an external location (dotfiles
        // managers). The old `walk` only matched `is_dir() | is_file()`, so symlinks fell through
        // and were excluded from the hash entirely — trust was granted with an effectively empty
        // hash and config mutations went undetected.
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();
        let external = dir.path().join("external_config.toml");
        write_file(&external, "a = 1");

        fs::create_dir_all(workspace.join(".helix")).unwrap();
        symlink(&external, workspace.join(".helix").join("config.toml")).unwrap();

        let h1 = compute_workspace_hash(workspace).expect("symlink should be hashed");

        // Mutating the symlink *target* must change the hash.
        write_file(&external, "a = 2");
        let h2 = compute_workspace_hash(workspace).expect("symlink should be hashed");
        assert_ne!(
            h1, h2,
            "symlinked config file mutations must affect the hash"
        );
    }

    #[test]
    fn hash_distinguishes_file_split() {
        // Regression: without length-prefixing, two files
        //   foo.toml: "a"
        //   bar.toml: "b"
        // would feed the same byte stream to the hasher as a single file
        //   foo.toml: "a\0bar.toml\0b"
        // because the \0 separators were indistinguishable from content.
        let dir1 = tempfile::tempdir().unwrap();
        let split = dir1.path();
        write_file(&split.join(".helix").join("foo.toml"), "a");
        write_file(&split.join(".helix").join("bar.toml"), "b");

        let dir2 = tempfile::tempdir().unwrap();
        let merged = dir2.path();
        write_file(&merged.join(".helix").join("foo.toml"), "a\0bar.toml\0b");

        let h1 = compute_workspace_hash(split).expect("split has files");
        let h2 = compute_workspace_hash(merged).expect("merged has files");
        assert_ne!(
            h1, h2,
            "length-prefixing must prevent file-vs-content ambiguity"
        );
    }

    #[test]
    fn level_insecure_does_not_bypass_excluded() {
        // Regression: under `level = "insecure"` an exclude entry on disk must still produce
        // `TrustStatus::Excluded` from `query()`. The old shortcut read only the in-memory cache,
        // so a cold cache silently returned `Trusted`.
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        // Seed an excluded entry via a level=None trust state.
        let bootstrap = WorkspaceTrust::new(Config::default());
        bootstrap.exclude(workspace);

        // Fresh state (cold cache) under level=Insecure.
        let trust = WorkspaceTrust::new(Config {
            level: ImplicitTrustLevel::Insecure,
            ..Config::default()
        });
        assert_eq!(
            trust.query(workspace, TrustQuery::Lsp),
            TrustStatus::Excluded,
            "level=insecure must honor disk-persisted excludes even on a cold cache"
        );
        assert_eq!(
            trust.query(workspace, TrustQuery::LocalConfig),
            TrustStatus::Excluded
        );
    }

    #[test]
    fn trusted_glob_grants_full_trust_but_exclude_wins() {
        let dir = tempfile::tempdir().unwrap();
        let trusted = dir.path().join("trusted_proj");
        fs::create_dir_all(&trusted).unwrap();

        let pattern = format!("{}/*", dir.path().display());
        let trust = WorkspaceTrust::new(Config {
            level: ImplicitTrustLevel::None,
            trusted_globs: build_trusted_globs(&[pattern]),
            ..Config::default()
        });

        // Matching workspace is fully trusted for every capability, even local config (which
        // `level = "servers"` would still gate).
        assert_eq!(
            trust.query(&trusted, TrustQuery::LocalConfig),
            TrustStatus::Trusted
        );
        assert_eq!(trust.query(&trusted, TrustQuery::Git), TrustStatus::Trusted);
        assert!(!trust.workspace_restricted(&trusted));

        // A non-matching path (no `dir/*` segment match: it has a deeper component) still trusts
        // since `other` is directly under `dir`. Use a sibling outside `dir` to confirm no match.
        let outside = tempfile::tempdir().unwrap();
        assert_eq!(
            trust.query(outside.path(), TrustQuery::LocalConfig),
            TrustStatus::Untrusted
        );

        // Explicit excludes beat a matching glob.
        trust.exclude(&trusted);
        assert_eq!(
            trust.query(&trusted, TrustQuery::LocalConfig),
            TrustStatus::Excluded
        );
        assert_eq!(
            trust.query(&trusted, TrustQuery::Lsp),
            TrustStatus::Excluded
        );
    }

    #[test]
    fn workspace_restricted_detects_stale() {
        // Regression: a previously-trusted workspace whose .helix/ has since been modified must be
        // reported as restricted (so the indicator and stale-hint can fire). The old code piped
        // Stale through `demote_for_query` and matched Untrusted, which made the `Stale` arm of
        // `workspace_restricted` unreachable.
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();
        write_file(&workspace.join(".helix").join("config.toml"), "a = 1");

        let trust = WorkspaceTrust::new(Config::default());
        trust.trust(workspace);
        assert!(!trust.workspace_restricted(workspace));
        assert_eq!(trust.status(workspace), TrustStatus::Trusted);

        // Mutate `.helix/` and bust the in-memory cache to force a re-read.
        write_file(&workspace.join(".helix").join("config.toml"), "a = 2");
        trust.inner.lock().remove(workspace);

        assert_eq!(trust.status(workspace), TrustStatus::Stale);
        assert!(
            trust.workspace_restricted(workspace),
            "Stale workspace must light up the restricted indicator"
        );
    }

    #[test]
    fn path_filename_is_stable_and_path_specific() {
        let a = path_filename(Path::new("/home/u/proj1"));
        let b = path_filename(Path::new("/home/u/proj1"));
        let c = path_filename(Path::new("/home/u/proj2"));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64); // hex sha256
    }
}
