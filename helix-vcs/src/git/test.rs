use std::{fs::File, io::Write, path::Path, process::Command};

use tempfile::TempDir;

use crate::git;

fn exec_git_cmd(args: &str, git_dir: &Path) {
    let res = Command::new("git")
        .arg("-C")
        .arg(git_dir) // execute the git command in this directory
        .args(args.split_whitespace())
        .env_remove("GIT_DIR")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "false")
        .env("GIT_AUTHOR_DATE", "2000-01-01 00:00:00 +0000")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_AUTHOR_NAME", "author")
        .env("GIT_COMMITTER_DATE", "2000-01-02 00:00:00 +0000")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_COMMITTER_NAME", "committer")
        .env("GIT_CONFIG_COUNT", "2")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .env("GIT_CONFIG_KEY_1", "init.defaultBranch")
        .env("GIT_CONFIG_VALUE_1", "main")
        .output()
        .unwrap_or_else(|_| panic!("`git {args}` failed"));
    if !res.status.success() {
        println!("{}", String::from_utf8_lossy(&res.stdout));
        eprintln!("{}", String::from_utf8_lossy(&res.stderr));
        panic!("`git {args}` failed (see output above)")
    }
}

fn create_commit(repo: &Path, add_modified: bool) {
    if add_modified {
        exec_git_cmd("add -A", repo);
    }
    exec_git_cmd("commit -m message", repo);
}

/// Writes `contents` to `file` and commits it, so tests can build up a history.
fn commit_file(repo: &Path, file: &Path, contents: &[u8]) {
    File::create(file).unwrap().write_all(contents).unwrap();
    create_commit(repo, true);
}

fn empty_git_repo() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp dir for git testing");
    exec_git_cmd("init", tmp.path());
    exec_git_cmd("config user.email test@helix.org", tmp.path());
    exec_git_cmd("config user.name helix-test", tmp.path());
    tmp
}

#[test]
fn missing_file() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"foo").unwrap();

    assert!(git::get_diff_base(&file, "HEAD", true).is_err());
}

#[test]
fn unmodified_file() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    create_commit(temp_git.path(), true);
    assert_eq!(
        git::get_diff_base(&file, "HEAD", true).unwrap().content,
        Vec::from(contents)
    );
}

#[test]
fn modified_file() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    create_commit(temp_git.path(), true);
    File::create(&file).unwrap().write_all(b"bar").unwrap();

    assert_eq!(
        git::get_diff_base(&file, "HEAD", true).unwrap().content,
        Vec::from(contents)
    );
}

/// Test that `get_file_head` does not return content for a directory.
/// This is important to correctly cover cases where a directory is removed and replaced by a file.
/// If the contents of the directory object were returned a diff between a path and the directory children would be produced.
#[test]
fn directory() {
    let temp_git = empty_git_repo();
    let dir = temp_git.path().join("file.txt");
    std::fs::create_dir(&dir).expect("");
    let file = dir.join("file.txt");
    let contents = b"foo".as_slice();
    File::create(file).unwrap().write_all(contents).unwrap();

    create_commit(temp_git.path(), true);

    std::fs::remove_dir_all(&dir).unwrap();
    File::create(&dir).unwrap().write_all(b"bar").unwrap();
    assert!(git::get_diff_base(&dir, "HEAD", true).is_err());
}

/// Test that `get_diff_base` resolves symlinks so that the same diff base is
/// used as the target file.
///
/// This is important to correctly cover cases where a symlink is removed and
/// replaced by a file. If the contents of the symlink object were returned
/// a diff between a literal file path and the actual file content would be
/// produced (bad ui).
#[cfg(any(unix, windows))]
#[test]
fn symlink() {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    #[cfg(not(unix))]
    use std::os::windows::fs::symlink_file as symlink;

    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let contents = Vec::from(b"foo");
    File::create(&file).unwrap().write_all(&contents).unwrap();
    let file_link = temp_git.path().join("file_link.txt");

    symlink("file.txt", &file_link).unwrap();
    create_commit(temp_git.path(), true);

    assert_eq!(
        git::get_diff_base(&file_link, "HEAD", true)
            .unwrap()
            .content,
        contents
    );
    assert_eq!(
        git::get_diff_base(&file, "HEAD", true).unwrap().content,
        contents
    );
}

/// Test that `get_diff_base` returns content when the file is a symlink to
/// another file that is in a git repo, but the symlink itself is not.
#[cfg(any(unix, windows))]
#[test]
fn symlink_to_git_repo() {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    #[cfg(not(unix))]
    use std::os::windows::fs::symlink_file as symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let temp_git = empty_git_repo();

    let file = temp_git.path().join("file.txt");
    let contents = Vec::from(b"foo");
    File::create(&file).unwrap().write_all(&contents).unwrap();
    create_commit(temp_git.path(), true);

    let file_link = temp_dir.path().join("file_link.txt");
    symlink(&file, &file_link).unwrap();

    assert_eq!(
        git::get_diff_base(&file_link, "HEAD", true)
            .unwrap()
            .content,
        contents
    );
    assert_eq!(
        git::get_diff_base(&file, "HEAD", true).unwrap().content,
        contents
    );
}

#[test]
fn diff_base_at_revision() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");

    commit_file(temp_git.path(), &file, b"first");
    commit_file(temp_git.path(), &file, b"second");

    assert_eq!(
        git::get_diff_base(&file, "HEAD~1", true).unwrap().content,
        b"first".to_vec()
    );
    assert_eq!(
        git::get_diff_base(&file, "HEAD", true).unwrap().content,
        b"second".to_vec()
    );
}

#[test]
fn diff_base_at_branch() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");

    commit_file(temp_git.path(), &file, b"on main");
    exec_git_cmd("checkout -b feature", temp_git.path());
    commit_file(temp_git.path(), &file, b"on feature");

    assert_eq!(
        git::get_diff_base(&file, "main", true).unwrap().content,
        b"on main".to_vec()
    );
    assert_eq!(
        git::get_diff_base(&file, "HEAD", true).unwrap().content,
        b"on feature".to_vec()
    );
}

/// A trailing `...` must diff against the merge base rather than the tip, so that commits
/// landed on the base branch *after* branching don't show up as changes of your own.
#[test]
fn diff_base_merge_base() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let other = temp_git.path().join("other.txt");

    // A (merge base) on main, then C on main and E on the feature branch.
    commit_file(temp_git.path(), &file, b"base");
    exec_git_cmd("checkout -b feature", temp_git.path());
    exec_git_cmd("checkout main", temp_git.path());
    commit_file(temp_git.path(), &file, b"moved on after branching");
    // Touch a second file so `main` and `feature` really have diverged.
    commit_file(temp_git.path(), &other, b"unrelated");
    exec_git_cmd("checkout feature", temp_git.path());
    commit_file(temp_git.path(), &file, b"my change");

    // Tip of `main` includes the commits made after the branch point ...
    assert_eq!(
        git::get_diff_base(&file, "main", true).unwrap().content,
        b"moved on after branching".to_vec()
    );
    // ... while the merge base does not.
    assert_eq!(
        git::get_diff_base(&file, "main...", true).unwrap().content,
        b"base".to_vec()
    );
}

/// An annotated tag resolves to a tag object rather than a commit, so the merge-base form has
/// to peel it first. A lightweight tag (as used by `revisions` below) would not catch this.
#[test]
fn diff_base_merge_base_with_annotated_tag() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");

    commit_file(temp_git.path(), &file, b"tagged");
    exec_git_cmd("tag -a v1 -m release", temp_git.path());
    commit_file(temp_git.path(), &file, b"later");

    assert_eq!(
        git::get_diff_base(&file, "v1", true).unwrap().content,
        b"tagged".to_vec()
    );
    let diff_base = git::get_diff_base(&file, "v1...", true).unwrap();
    assert_eq!(diff_base.content, b"tagged".to_vec());
    assert!(!diff_base.used_fallback, "should not have fallen back");
}

/// `<rev>..` is git's two-dot form; with the working tree as the other side it means the same
/// as `<rev>`. `...<rev>` is the other spelling of the merge-base form.
#[test]
fn diff_base_range_spellings() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");

    commit_file(temp_git.path(), &file, b"base");
    exec_git_cmd("checkout -b feature", temp_git.path());
    exec_git_cmd("checkout main", temp_git.path());
    commit_file(temp_git.path(), &file, b"moved on after branching");
    exec_git_cmd("checkout feature", temp_git.path());
    commit_file(temp_git.path(), &file, b"my change");

    for spec in ["main", "main.."] {
        let diff_base = git::get_diff_base(&file, spec, true).unwrap();
        assert_eq!(
            diff_base.content,
            b"moved on after branching".to_vec(),
            "{spec} should resolve to the tip of main"
        );
        assert!(!diff_base.used_fallback, "{spec} should not fall back");
    }

    let diff_base = git::get_diff_base(&file, "main...", true).unwrap();
    assert_eq!(
        diff_base.content,
        b"base".to_vec(),
        "main... should resolve to the merge base"
    );
    assert!(!diff_base.used_fallback, "main... should not fall back");

    // `...main` picks the same merge base, but in git it means "what main did since
    // diverging" — the opposite of what the gutter shows. Rejecting it beats silently
    // reinterpreting it.
    assert!(
        git::get_diff_base(&file, "...main", true)
            .unwrap()
            .used_fallback,
        "...main should not be silently accepted"
    );
}

/// A file added on the branch is absent from the merge base, but `git diff main...HEAD` still
/// shows it as an addition. An empty base reproduces that instead of dropping the gutter.
#[test]
fn diff_base_file_added_on_branch() {
    let temp_git = empty_git_repo();
    let existing = temp_git.path().join("existing.txt");
    let added = temp_git.path().join("added.txt");

    commit_file(temp_git.path(), &existing, b"base");
    exec_git_cmd("checkout -b feature", temp_git.path());
    commit_file(temp_git.path(), &added, b"brand new");

    // Tracked in HEAD but not in the base revision: empty base, so the whole file is an add.
    assert_eq!(
        git::get_diff_base(&added, "main", true).unwrap().content,
        Vec::<u8>::new()
    );

    // Genuinely untracked files still produce no diff base at all, as against HEAD.
    let untracked = temp_git.path().join("untracked.txt");
    File::create(&untracked).unwrap().write_all(b"x").unwrap();
    assert!(git::get_diff_base(&untracked, "main", true).is_err());
    assert!(git::get_diff_base(&untracked, "HEAD", true).is_err());
}

#[test]
fn unresolvable_revision_falls_back_to_head() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    commit_file(temp_git.path(), &file, b"foo");

    let diff_base = git::get_diff_base(&file, "no-such-revision", true).unwrap();
    assert_eq!(diff_base.content, b"foo".to_vec());
    assert!(diff_base.used_fallback);
}

#[test]
fn revisions() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    commit_file(temp_git.path(), &file, b"foo");
    exec_git_cmd("branch feature", temp_git.path());
    exec_git_cmd("tag v1", temp_git.path());
    // `refs/stash` is not a plausible diff base and must not be offered.
    File::create(&file).unwrap().write_all(b"bar").unwrap();
    exec_git_cmd("stash", temp_git.path());

    let revisions = git::get_revisions(temp_git.path(), true).unwrap();

    assert!(revisions.contains(&"HEAD".to_string()));
    assert!(revisions.contains(&"main".to_string()));
    assert!(revisions.contains(&"feature".to_string()));
    assert!(revisions.contains(&"v1".to_string()));
    assert!(!revisions.iter().any(|rev| rev.contains("stash")));
}
