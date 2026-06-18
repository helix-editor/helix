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

fn git_output(args: &str, git_dir: &Path) -> String {
    let res = Command::new("git")
        .arg("-C")
        .arg(git_dir)
        .args(args.split_whitespace())
        .env_remove("GIT_DIR")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "false")
        .output()
        .unwrap_or_else(|_| panic!("`git {args}` failed"));
    if !res.status.success() {
        println!("{}", String::from_utf8_lossy(&res.stdout));
        eprintln!("{}", String::from_utf8_lossy(&res.stderr));
        panic!("`git {args}` failed (see output above)")
    }
    String::from_utf8_lossy(&res.stdout).trim().to_string()
}

fn create_commit(repo: &Path, add_modified: bool) {
    if add_modified {
        exec_git_cmd("add -A", repo);
    }
    exec_git_cmd("commit -m message", repo);
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

    assert!(git::get_diff_base(&file).is_err());
}

#[test]
fn unmodified_file() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    create_commit(temp_git.path(), true);
    assert_eq!(git::get_diff_base(&file).unwrap(), Vec::from(contents));
}

#[test]
fn modified_file() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    create_commit(temp_git.path(), true);
    File::create(&file).unwrap().write_all(b"bar").unwrap();

    assert_eq!(git::get_diff_base(&file).unwrap(), Vec::from(contents));
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
    assert!(git::get_diff_base(&dir).is_err());
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

    assert_eq!(git::get_diff_base(&file_link).unwrap(), contents);
    assert_eq!(git::get_diff_base(&file).unwrap(), contents);
}

#[test]
fn branches_lists_current_and_local_branches() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"main").unwrap();
    create_commit(temp_git.path(), true);
    exec_git_cmd("branch feature", temp_git.path());

    let repository = git::repository(temp_git.path()).unwrap();
    let work_dir = std::fs::canonicalize(temp_git.path()).unwrap();
    assert_eq!(repository.work_dir(), work_dir.as_path());

    let branches = repository.branches().unwrap();
    let main = branches
        .iter()
        .find(|branch| branch.name() == "main")
        .unwrap();
    assert!(main.is_current());
    assert_eq!(main.kind(), crate::BranchKind::Local);

    let feature = branches
        .iter()
        .find(|branch| branch.name() == "feature")
        .unwrap();
    assert!(!feature.is_current());
    assert_eq!(feature.kind(), crate::BranchKind::Local);
}

#[test]
fn repository_errors_outside_git_worktree() {
    let temp_dir = tempfile::tempdir().expect("create temp dir outside git");
    let err = git::repository(temp_dir.path()).unwrap_err();

    assert!(
        format!("{err:#}").contains("current directory is not inside a git worktree"),
        "{err:#}"
    );
}

#[test]
fn switch_branch_switches_clean_worktree() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"main").unwrap();
    create_commit(temp_git.path(), true);
    exec_git_cmd("branch feature", temp_git.path());

    let repository = git::repository(temp_git.path()).unwrap();
    let branches = repository.branches().unwrap();
    let feature = branches
        .iter()
        .find(|branch| branch.name() == "feature")
        .unwrap();

    repository.switch_branch(feature).unwrap();

    assert_eq!(
        git_output("branch --show-current", temp_git.path()),
        "feature"
    );
}

#[test]
fn switch_branch_tracks_remote_branch() {
    let remote = tempfile::tempdir().expect("create temp dir for remote");
    exec_git_cmd("init --bare", remote.path());

    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"main").unwrap();
    create_commit(temp_git.path(), true);
    exec_git_cmd(
        &format!("remote add origin {}", remote.path().display()),
        temp_git.path(),
    );
    exec_git_cmd("push -u origin main", temp_git.path());
    exec_git_cmd("branch feature", temp_git.path());
    exec_git_cmd("push origin feature", temp_git.path());
    exec_git_cmd("branch -D feature", temp_git.path());

    let repository = git::repository(temp_git.path()).unwrap();
    let branches = repository.branches().unwrap();
    let feature = branches
        .iter()
        .find(|branch| branch.name() == "origin/feature")
        .unwrap();
    assert_eq!(feature.kind(), crate::BranchKind::Remote);

    repository.switch_branch(feature).unwrap();

    assert_eq!(
        git_output("branch --show-current", temp_git.path()),
        "feature"
    );
    assert_eq!(
        git_output(
            "rev-parse --abbrev-ref --symbolic-full-name @{u}",
            temp_git.path()
        ),
        "origin/feature"
    );
}

#[test]
fn switch_branch_refuses_dirty_worktree() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"main").unwrap();
    create_commit(temp_git.path(), true);
    exec_git_cmd("branch feature", temp_git.path());

    File::create(&file).unwrap().write_all(b"dirty").unwrap();

    let repository = git::repository(temp_git.path()).unwrap();
    let branches = repository.branches().unwrap();
    let feature = branches
        .iter()
        .find(|branch| branch.name() == "feature")
        .unwrap();
    let err = repository.switch_branch(feature).unwrap_err();

    assert!(err.to_string().contains("uncommitted changes"));
    assert_eq!(git_output("branch --show-current", temp_git.path()), "main");
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

    assert_eq!(git::get_diff_base(&file_link).unwrap(), contents);
    assert_eq!(git::get_diff_base(&file).unwrap(), contents);
}
