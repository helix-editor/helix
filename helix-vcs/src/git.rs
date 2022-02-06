use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
};

use git2::{Oid, Repository};
use similar::DiffTag;

use crate::{LineDiff, LineDiffs, RepoRoot};

pub struct Git {
    repo: Repository,
    /// Absolute path to root of the repo
    root: RepoRoot,
    head: Oid,

    /// A cache mapping absolute file paths to file contents
    /// in the HEAD commit.
    head_cache: HashMap<PathBuf, String>,
}

impl std::fmt::Debug for Git {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Git").field("root", &self.root).finish()
    }
}

impl Git {
    pub fn head_commit_id(repo: &Repository) -> Option<Oid> {
        repo.head()
            .and_then(|gitref| gitref.peel_to_commit())
            .map(|commit| commit.id())
            .ok()
    }

    pub fn discover_from_path(file: &Path) -> Option<Self> {
        let repo = Repository::discover(file).ok()?;
        let root = repo.workdir()?.to_path_buf();
        let head_oid = Self::head_commit_id(&repo)?;
        Some(Self {
            repo,
            root,
            head: head_oid,
            head_cache: HashMap::new(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn relative_to_root<'p>(&self, path: &'p Path) -> Option<&'p Path> {
        path.strip_prefix(&self.root).ok()
    }

    pub fn read_file_from_head(&mut self, file: &Path) -> Option<&str> {
        let current_head = Self::head_commit_id(&self.repo)?;
        // TODO: Check cache validity on events like WindowChange
        // instead of on every keypress ? Will require hooks.
        if current_head != self.head {
            self.head_cache.clear();
            self.head = current_head;
        }

        if !self.head_cache.contains_key(file) {
            let relative = self.relative_to_root(file)?;
            let revision = &format!("HEAD:{}", relative.display());
            let object = self.repo.revparse_single(revision).ok()?;
            let blob = object.peel_to_blob().ok()?;
            let contents = std::str::from_utf8(blob.content()).ok()?;
            self.head_cache
                .insert(file.to_path_buf(), contents.to_string());
        }

        self.head_cache.get(file).map(|s| s.as_str())
    }

    pub fn line_diff_with_head(&mut self, file: &Path, contents: &str) -> LineDiffs {
        let base = match self.read_file_from_head(file) {
            Some(b) => b,
            None => return LineDiffs::new(),
        };
        let mut config = similar::TextDiff::configure();
        config.timeout(std::time::Duration::from_millis(250));

        let mut line_diffs: LineDiffs = HashMap::new();

        let mut mark_lines = |range: Range<usize>, change: LineDiff| {
            for line in range {
                line_diffs.insert(line, change);
            }
        };

        let diff = config.diff_lines(base, contents);
        for op in diff.ops() {
            let (tag, _, line_range) = op.as_tag_tuple();
            let start = line_range.start;
            match tag {
                DiffTag::Insert => mark_lines(line_range, LineDiff::Added),
                DiffTag::Replace => mark_lines(line_range, LineDiff::Modified),
                DiffTag::Delete => mark_lines(start..start + 1, LineDiff::Deleted),
                DiffTag::Equal => (),
            }
        }

        line_diffs
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs::{self, File},
        process::Command,
    };

    use tempfile::TempDir;

    use super::*;

    fn empty_git_repo() -> TempDir {
        let tmp = tempfile::tempdir().expect("Could not create temp dir for git testing");
        exec_git_cmd("init", tmp.path());
        tmp
    }

    fn exec_git_cmd(args: &str, git_dir: &Path) {
        Command::new("git")
            .arg("-C")
            .arg(git_dir) // execute the git command in this directory
            .args(args.split_whitespace())
            .status()
            .expect(&format!("`git {args}` failed"))
            .success()
            .then(|| ())
            .expect(&format!("`git {args}` failed"));
    }

    #[test]
    fn test_cannot_discover_bare_git_repo() {
        let temp_git = empty_git_repo();
        let file = temp_git.path().join("file.txt");
        File::create(&file).expect("Could not create file");

        assert!(Git::discover_from_path(&file).is_none());
    }

    #[test]
    fn test_discover_git_repo() {
        let temp_git = empty_git_repo();
        let file = temp_git.path().join("file.txt");
        File::create(&file).expect("Could not create file");
        exec_git_cmd("add file.txt", temp_git.path());
        exec_git_cmd("commit -m message", temp_git.path());

        let root = Git::discover_from_path(&file).map(|g| g.root().to_owned());
        assert_eq!(Some(temp_git.path().to_owned()), root);
    }

    #[test]
    fn test_read_file_from_head() {
        let tmp_repo = empty_git_repo();
        let git_dir = tmp_repo.path();
        let file = git_dir.join("file.txt");

        let contents = r#"
            a file with unnecessary
            indent and text.
        "#;
        fs::write(&file, contents).expect("Could not write to file");
        exec_git_cmd("add file.txt", git_dir);
        exec_git_cmd("commit -m message", git_dir);

        let mut git = Git::discover_from_path(&file).unwrap();
        assert_eq!(
            Some(contents),
            git.read_file_from_head(&file),
            "Wrong blob contents from HEAD on clean index"
        );

        fs::write(&file, "new text").expect("Could not write to file");
        assert_eq!(
            Some(contents),
            git.read_file_from_head(&file),
            "Wrong blob contents from HEAD when index is dirty"
        );
    }
}
