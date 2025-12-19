use std::borrow::Borrow;
use std::mem::replace;
use std::path::{Path, PathBuf};
use std::slice;
use std::sync::Arc;
use std::time::SystemTime;

// Re-export filesentry types (available on all platforms)
pub use filesentry::{CanonicalPathBuf, Event, EventType, Events, Filter, ShutdownOnDrop};

use helix_event::{dispatch, events};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::{Deserialize, Serialize};

events! {
    FileSystemDidChange {
        fs_events: Events
    }
}

/// Create an Events collection from an iterator of paths.
/// All paths will have EventType::Modified.
pub fn events_from_paths(paths: impl IntoIterator<Item = PathBuf>) -> Events {
    use filesentry::CanonicalPathBuf;
    let events: Vec<Event> = paths
        .into_iter()
        .filter_map(|path| {
            let canonical = path.canonicalize().ok()?;
            Some(Event {
                path: CanonicalPathBuf::assert_canonicalized(&canonical),
                ty: EventType::Modified,
            })
        })
        .collect();
    Events::from(events)
}

/// Config for file watching
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    /// Enable file watching enable by default
    pub enable: bool,
    pub watch_vcs: bool,
    /// Only enable the file watcher inside helix workspaces (VCS repos and directories with .helix
    /// directory) this prevents watching large directories like $HOME by default
    ///
    /// Defaults to `true`
    pub require_workspace: bool,
    /// Enables ignoring hidden files.
    /// Whether to hide hidden files in file picker and global search results. Defaults to true.
    pub hidden: bool,
    /// Enables reading `.ignore` files.
    /// Whether to hide files listed in .ignore in file picker and global search results. Defaults to true.
    pub ignore: bool,
    /// Enables reading `.gitignore` files.
    /// Whether to hide files listed in .gitignore in file picker and global search results. Defaults to true.
    pub git_ignore: bool,
    /// Enables reading global .gitignore, whose path is specified in git's config: `core.excludefile` option.
    /// Whether to hide files listed in global .gitignore in file picker and global search results. Defaults to true.
    pub git_global: bool,
    // /// Enables reading `.git/info/exclude` files.
    // /// Whether to hide files listed in .git/info/exclude in file picker and global search results. Defaults to true.
    // pub git_exclude: bool,
    /// Maximum Depth to recurse for filewatching
    pub max_depth: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enable: true,
            watch_vcs: true,
            require_workspace: true,
            hidden: true,
            ignore: true,
            git_ignore: true,
            git_global: true,
            max_depth: Some(10),
        }
    }
}

pub struct Watcher {
    watcher: Option<(filesentry::Watcher, ShutdownOnDrop)>,
    filter: Arc<WatchFilter>,
    roots: Vec<(PathBuf, usize)>,
    config: Config,
    /// Extra paths that need polling (e.g., VCS HEAD files outside workspace)
    /// Stored with their last known mtime for change detection
    extra_watched_paths: Vec<(PathBuf, Option<SystemTime>)>,
}

impl Watcher {
    pub fn new(config: &Config) -> Watcher {
        let mut watcher = Watcher {
            watcher: None,
            filter: Arc::new(WatchFilter {
                filesentry_ignores: Gitignore::empty(),
                ignore_files: Vec::new(),
                global_ignores: Vec::new(),
                hidden: true,
                watch_vcs: true,
            }),
            roots: Vec::new(),
            config: config.clone(),
            extra_watched_paths: Vec::new(),
        };
        watcher.reload(config);
        watcher
    }

    pub fn reload(&mut self, config: &Config) {
        let old_config = replace(&mut self.config, config.clone());
        let (workspace, no_workspace) = helix_loader::find_workspace();

        if !config.enable || config.require_workspace && no_workspace {
            self.watcher = None;
            return;
        }
        self.filter = Arc::new(WatchFilter::new(
            config,
            &workspace,
            self.roots.iter().map(|(it, _)| &**it),
        ));
        let watcher = match &mut self.watcher {
            Some((watcher, _)) => {
                // TODO: more fine grained detection of when recrawl is nedded
                watcher.set_filter(self.filter.clone(), old_config != self.config);
                watcher
            }
            None => match filesentry::Watcher::new() {
                Ok(watcher) => {
                    watcher.set_filter(self.filter.clone(), false);
                    watcher.add_handler(move |events| {
                        dispatch(FileSystemDidChange { fs_events: events });
                        true
                    });
                    let shutdown_guard = watcher.shutdown_guard();
                    &mut self.watcher.insert((watcher, shutdown_guard)).0
                }
                Err(err) => {
                    log::info!("file-watcher not available: {err}");
                    return;
                }
            },
        };
        if let Err(err) = watcher.add_root(&workspace, true, |_| ()) {
            log::error!("failed to start file-watcher: {err}");
        }
        for (root, _) in &self.roots {
            if let Err(err) = watcher.add_root(root, true, |_| ()) {
                log::error!("failed to start file-watcher: {err}");
            }
        }
        watcher.start();
    }

    pub fn remove_root(&mut self, root: PathBuf) {
        let i = self.roots.partition_point(|(it, _)| it < &root);
        if self.roots.get(i).is_none_or(|(it, _)| it != &root) {
            log::error!("tried to remove root {root:?} from watch list that does not exist!");
            return;
        }
        if self.roots[i].1 <= 1 {
            self.roots.remove(i);
        } else {
            self.roots[i].1 -= 1;
        }
    }

    /// Returns true if the file watcher is active.
    pub fn is_active(&self) -> bool {
        self.watcher.is_some()
    }

    /// Check if a given path is being actively watched.
    /// Returns true if the path is under a watched root and not filtered out.
    pub fn is_watching(&self, path: &Path) -> bool {
        if self.watcher.is_none() {
            return false;
        }
        let path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };
        let (workspace, _) = helix_loader::find_workspace();
        // Check if under workspace and not filtered
        if path.starts_with(&workspace) && !self.filter.ignore_path_rec(&path, Some(false)) {
            return true;
        }
        // Check if under any explicitly added root and not filtered
        for (root, _) in &self.roots {
            if path.starts_with(root) && !self.filter.ignore_path_rec(&path, Some(false)) {
                return true;
            }
        }
        false
    }

    /// Poll extra watched paths for changes and return paths that have changed.
    /// Updates internal mtime tracking for the changed paths.
    pub fn poll_extra_paths(&mut self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        for (path, last_mtime) in &mut self.extra_watched_paths {
            let current_mtime = path.metadata().ok().and_then(|m| m.modified().ok());
            if current_mtime != *last_mtime {
                changed.push(path.clone());
                *last_mtime = current_mtime;
            }
        }
        changed
    }

    /// Returns true if there are extra paths that need polling.
    pub fn has_extra_watched_paths(&self) -> bool {
        !self.extra_watched_paths.is_empty()
    }

    /// Set extra paths to watch via polling.
    /// These are paths outside the main watched workspace that need change detection.
    /// Only paths outside the workspace are added (paths inside are already watched).
    pub fn set_extra_watched_paths(&mut self, paths: Vec<PathBuf>) {
        let (workspace, _) = helix_loader::find_workspace();
        self.extra_watched_paths = paths
            .into_iter()
            .filter(|path| !path.starts_with(&workspace))
            .map(|path| {
                let mtime = path.metadata().ok().and_then(|m| m.modified().ok());
                (path, mtime)
            })
            .collect();
        if !self.extra_watched_paths.is_empty() {
            log::info!(
                "added {} extra paths for polling: {:?}",
                self.extra_watched_paths.len(),
                self.extra_watched_paths
                    .iter()
                    .map(|(p, _)| p)
                    .collect::<Vec<_>>()
            );
        }
    }

    pub fn add_root(&mut self, root: &Path) {
        let root = match root.canonicalize() {
            Ok(root) => root,
            Err(err) => {
                log::error!("failed to watch {root:?}: {err}");
                return;
            }
        };
        let i = self.roots.partition_point(|(it, _)| it < &root);
        if let Some((_, refcnt)) = self.roots.get_mut(i).filter(|(path, _)| path == &root) {
            *refcnt += 1;
            return;
        }
        if self.roots[..i]
            .iter()
            .rev()
            .find(|(it, _)| it.parent().is_none_or(|it| root.starts_with(it)))
            .is_some_and(|(it, _)| root.starts_with(it))
            && !self.filter.ignore_path_rec(&root, Some(true))
        {
            return;
        }
        let (workspace, _) = helix_loader::find_workspace();
        if root.starts_with(&workspace) {
            return;
        }
        self.roots.insert(i, (root.clone(), 1));
        self.filter = Arc::new(WatchFilter::new(
            &self.config,
            &workspace,
            self.roots.iter().map(|(it, _)| &**it),
        ));
        if let Some((watcher, _)) = &self.watcher {
            watcher.set_filter(self.filter.clone(), false);
            if let Err(err) = watcher.add_root(&root, true, |_| ()) {
                log::error!("failed to watch {root:?}: {err}");
            }
        }
    }
}

fn build_ignore(paths: impl IntoIterator<Item = PathBuf> + Clone, dir: &Path) -> Option<Gitignore> {
    let mut builder = GitignoreBuilder::new(dir);
    for path in paths.clone() {
        if let Some(err) = builder.add(&path) {
            if !err.is_io() {
                log::error!("failed to read ignorefile at {path:?}: {err}");
            }
        }
    }
    match builder.build() {
        Ok(ignore) => (!ignore.is_empty()).then_some(ignore),
        Err(err) => {
            if !err.is_io() {
                log::error!(
                    "failed to read ignorefile at {:?}: {err}",
                    paths.into_iter().collect::<Vec<_>>()
                );
            }
            None
        }
    }
}

struct IgnoreFiles {
    root: PathBuf,
    ignores: Vec<Arc<Gitignore>>,
}

impl IgnoreFiles {
    fn new(
        workspace_ignore: Option<Arc<Gitignore>>,
        config: &Config,
        root: &Path,
        globals: &[Arc<Gitignore>],
    ) -> Self {
        let mut ignores = Vec::with_capacity(8);
        // .helix/ignore
        if let Some(workspace_ignore) = workspace_ignore {
            ignores.push(workspace_ignore);
        }
        for ancestor in root.ancestors() {
            let ignore = if config.ignore {
                if config.git_ignore {
                    // the second path takes priority
                    build_ignore(
                        [ancestor.join(".gitignore"), ancestor.join(".ignore")],
                        ancestor,
                    )
                } else {
                    build_ignore([ancestor.join(".ignore")], ancestor)
                }
            } else if config.git_ignore {
                build_ignore([ancestor.join(".gitignore")], ancestor)
            } else {
                None
            };
            if let Some(ignore) = ignore {
                ignores.push(Arc::new(ignore));
            }
        }
        ignores.extend(globals.iter().cloned());
        Self {
            root: root.into(),
            ignores,
        }
    }

    fn shared_ignores(
        workspace: &Path,
        config: &Config,
    ) -> (Vec<Arc<Gitignore>>, Option<Arc<Gitignore>>) {
        let mut ignores = Vec::new();
        let workspace_ignore = build_ignore(
            [
                helix_loader::config_dir().join("ignore"),
                workspace.join(".helix/ignore"),
            ],
            workspace,
        )
        .map(Arc::new);
        if config.git_global {
            let (gitignore_global, err) = Gitignore::global();
            if let Some(err) = err {
                if !err.is_io() {
                    log::error!("failed to read global global ignorefile: {err}");
                }
            }
            if !gitignore_global.is_empty() {
                ignores.push(Arc::new(gitignore_global));
            }
        }
        // if config.git_exclude {
        // TODO git_exclude implementation, this isn't quite trivial unfortunaetly
        // due to detached workspace etc.
        // }
        // TODO: git exclude
        (ignores, workspace_ignore)
    }

    fn filesentry_ignores(workspace: &Path) -> Gitignore {
        // the second path takes priority
        build_ignore(
            [
                helix_loader::config_dir().join("filesentryignore"),
                workspace.join(".helix/filesentryignore"),
            ],
            workspace,
        )
        .unwrap_or(Gitignore::empty())
    }

    fn is_ignored(
        ignores: &[impl Borrow<Gitignore>],
        path: &Path,
        is_dir: Option<bool>,
    ) -> Option<bool> {
        match is_dir {
            Some(is_dir) => {
                for ignore in ignores {
                    match ignore.borrow().matched(path, is_dir) {
                        ignore::Match::None => continue,
                        ignore::Match::Ignore(_) => return Some(true),
                        ignore::Match::Whitelist(_) => return Some(false),
                    }
                }
            }
            None => {
                // if we don't know wether this is a directory (on windows)
                // then we are conservative and allow the dirs
                for ignore in ignores {
                    match ignore.borrow().matched(path, true) {
                        ignore::Match::None => continue,
                        ignore::Match::Ignore(glob) => {
                            if glob.is_only_dir() {
                                match ignore.borrow().matched(path, false) {
                                    ignore::Match::None => continue,
                                    ignore::Match::Ignore(_) => return Some(true),
                                    ignore::Match::Whitelist(_) => return Some(false),
                                }
                            } else {
                                return Some(true);
                            }
                        }
                        ignore::Match::Whitelist(_) => return Some(false),
                    }
                }
            }
        }
        None
    }
}

/// a filter to ignore hiddeng/ingored files. The point of this
/// is to avoid overwhelming the watcher with watching a ton of
/// files/directories (like the cargo target directory, node_modules or
/// VCS files) so ignoring a file is a performance optimization.
///
/// By default we ignore ignored
struct WatchFilter {
    filesentry_ignores: Gitignore,
    ignore_files: Vec<IgnoreFiles>,
    global_ignores: Vec<Arc<Gitignore>>,
    hidden: bool,
    watch_vcs: bool,
}

impl WatchFilter {
    fn new<'a>(
        config: &Config,
        workspace: &'a Path,
        roots: impl Iterator<Item = &'a Path> + Clone,
    ) -> WatchFilter {
        let filesentry_ignores = IgnoreFiles::filesentry_ignores(workspace);
        let (global_ignores, workspace_ignore) = IgnoreFiles::shared_ignores(workspace, config);
        let ignore_files = roots
            .chain([workspace])
            .map(|root| IgnoreFiles::new(workspace_ignore.clone(), config, root, &global_ignores))
            .collect();
        WatchFilter {
            filesentry_ignores,
            ignore_files,
            global_ignores,
            hidden: config.hidden,
            watch_vcs: config.watch_vcs,
        }
    }

    fn ignore_path_impl(
        &self,
        path: &Path,
        is_dir: Option<bool>,
        ignore_files: &[Arc<Gitignore>],
    ) -> bool {
        if let Some(ignore) =
            IgnoreFiles::is_ignored(slice::from_ref(&self.filesentry_ignores), path, is_dir)
        {
            return ignore;
        }
        if is_hardcoded_whitelist(path) {
            return false;
        }
        if is_hardcoded_blacklist(path, is_dir.unwrap_or(false)) {
            return true;
        }
        if let Some(ignore) = IgnoreFiles::is_ignored(ignore_files, path, is_dir) {
            return ignore;
        }
        // ignore .git dircectory except .git/HEAD (and .git itself)
        if is_vcs_ignore(path, self.watch_vcs) {
            return true;
        }
        !self.hidden && is_hidden(path)
    }
}

impl filesentry::Filter for WatchFilter {
    fn ignore_path(&self, path: &Path, is_dir: Option<bool>) -> bool {
        let i = self
            .ignore_files
            .partition_point(|ignore_files| path < ignore_files.root);
        let (root, ignore_files) = self
            .ignore_files
            .get(i)
            .map_or((Path::new(""), &self.global_ignores), |files| {
                (&files.root, &files.ignores)
            });
        if path == root {
            return false;
        }
        self.ignore_path_impl(path, is_dir, ignore_files)
    }

    fn ignore_path_rec(&self, mut path: &Path, is_dir: Option<bool>) -> bool {
        let i = self
            .ignore_files
            .partition_point(|ignore_files| path < ignore_files.root);
        let (root, ignore_files) = self
            .ignore_files
            .get(i)
            .map_or((Path::new(""), &self.global_ignores), |files| {
                (&files.root, &files.ignores)
            });
        loop {
            if path == root {
                return false;
            }
            if self.ignore_path_impl(path, is_dir, ignore_files) {
                return true;
            }
            let Some(parent) = path.parent() else {
                break;
            };
            path = parent;
        }
        false
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name().is_some_and(|it| {
        it.as_encoded_bytes().first() == Some(&b'.')
        // handled by vcs ignore rules
        && it != ".git"
    })
}

// hidden directories we want to watch by default
fn is_hardcoded_whitelist(path: &Path) -> bool {
    path.ends_with(".helix")
        | path.ends_with(".github")
        | path.ends_with(".cargo")
        | path.ends_with(".envrc")
}

fn is_hardcoded_blacklist(path: &Path, is_dir: bool) -> bool {
    // don't descend into the cargo regstiry and similar
    path.parent()
        .is_some_and(|parent| parent.ends_with(".cargo"))
        && is_dir
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|it| it.to_str())
}

fn is_vcs_ignore(path: &Path, watch_vcs: bool) -> bool {
    // ignore .git directory contents except .git/HEAD (and .git itself)
    // Note: only checks immediate parent; recursive checking is done by ignore_path_rec
    if watch_vcs
        && path.parent().is_some_and(|it| it.ends_with(".git"))
        && !path.ends_with(".git/HEAD")
    {
        return true;
    }
    match file_name(path) {
        Some(".jj" | ".svn" | ".hg") => true,
        Some(".git") => !watch_vcs,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::file_watcher::{is_hardcoded_whitelist, is_hidden, is_vcs_ignore};

    #[test]
    fn test_vcs_ignore() {
        assert!(!is_vcs_ignore(Path::new(".git"), true));
        assert!(!is_vcs_ignore(Path::new(".git/HEAD"), true));
        assert!(is_vcs_ignore(Path::new(".git/foo"), true));
        // Note: .git/foo/bar is NOT caught by is_vcs_ignore (only checks immediate parent)
        // but it IS caught by ignore_path_rec which checks ancestors recursively
        assert!(!is_vcs_ignore(Path::new(".git/foo/bar"), true));
        assert!(!is_vcs_ignore(Path::new(".foo"), true));
        assert!(is_vcs_ignore(Path::new(".jj"), true));
        assert!(is_vcs_ignore(Path::new(".svn"), true));
        assert!(is_vcs_ignore(Path::new(".hg"), true));
    }

    #[test]
    fn test_hidden() {
        assert!(is_hidden(Path::new(".foo")));
        // handled by vcs ignore rules
        assert!(!is_hidden(Path::new(".git")));
    }

    #[test]
    fn test_whitelist() {
        // Note: .git is NOT in whitelist - it has special handling in is_vcs_ignore and is_hidden
        assert!(is_hardcoded_whitelist(Path::new(".helix")));
        assert!(is_hardcoded_whitelist(Path::new(".github")));
        assert!(!is_hardcoded_whitelist(Path::new(".githup")));
    }
}
