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

fn exec_git_stdout(args: &str, git_dir: &Path) -> String {
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

    String::from_utf8(res.stdout).unwrap()
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

    assert!(git::get_diff_base(git::get_diff_base(&file, None)file, None, true).is_err());
}

#[test]
fn unmodified_file() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    create_commit(temp_git.path(), true);
    assert_eq!(
        git::get_diff_base(git::get_diff_base(&file, None)file, None, true).unwrap(),
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
        git::get_diff_base(git::get_diff_base(&file, None)file, None, true).unwrap(),
        Vec::from(contents)
    );
}

#[test]
fn diff_base_from_branch() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let main_contents = b"main".as_slice();
    let feature_contents = b"feature".as_slice();
    File::create(&file)
        .unwrap()
        .write_all(main_contents)
        .unwrap();
    create_commit(temp_git.path(), true);

    exec_git_cmd("checkout -b feature", temp_git.path());
    File::create(&file)
        .unwrap()
        .write_all(feature_contents)
        .unwrap();
    create_commit(temp_git.path(), true);

    assert_eq!(
        git::get_diff_base(&file, Some("main")).unwrap(),
        Vec::from(main_contents)
    );
    assert_eq!(
        git::get_diff_base(git::get_diff_base(&file, None)file, None, true).unwrap(),
        Vec::from(feature_contents)
    );
}

#[test]
fn diff_base_from_commit_sha() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    let main_contents = b"main".as_slice();
    File::create(&file)
        .unwrap()
        .write_all(main_contents)
        .unwrap();
    create_commit(temp_git.path(), true);
    let main_commit = exec_git_stdout("rev-parse HEAD", temp_git.path());

    exec_git_cmd("checkout -b feature", temp_git.path());
    File::create(&file).unwrap().write_all(b"feature").unwrap();
    create_commit(temp_git.path(), true);

    assert_eq!(
        git::get_diff_base(&file, Some(main_commit.trim())).unwrap(),
        Vec::from(main_contents)
    );
}

#[test]
fn file_missing_in_selected_base_is_empty() {
    let temp_git = empty_git_repo();
    exec_git_cmd("commit --allow-empty -m root", temp_git.path());

    exec_git_cmd("checkout -b feature", temp_git.path());
    let file = temp_git.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"feature").unwrap();
    create_commit(temp_git.path(), true);

    assert_eq!(git::get_diff_base(&file, Some("main")).unwrap(), Vec::new());
}

#[test]
fn invalid_diff_base_revision() {
    let temp_git = empty_git_repo();
    let file = temp_git.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"main").unwrap();
    create_commit(temp_git.path(), true);

    assert!(git::get_diff_base(&file, Some("does-not-exist")).is_err());
    assert!(git::ensure_diff_base(&file, "does-not-exist").is_err());
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
    assert!(git::get_diff_base(&dir, None).is_err());
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

    assert_eq!(git::get_diff_base(&file_link, None).unwrap(), contents);
    assert_eq!(git::get_diff_base(git::get_diff_base(&file, None)file, None, true).unwrap(), contents);
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

    assert_eq!(git::get_diff_base(&file_link, None).unwrap(), contents);
    assert_eq!(git::get_diff_base(git::get_diff_base(&file, None)file, None, true).unwrap(), contents);
}
