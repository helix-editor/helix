use std::{fs::File, io::Write, path::Path, process::Command};

use tempfile::TempDir;

use crate::{DiffProvider, Hg};

fn exec_hg_cmd(args: &str, hg_dir: &Path) {
    let res = Command::new("hg")
        .arg("--cwd")
        .arg(hg_dir)
        .args(args.split_whitespace())
        .env("HGPLAIN", "")
        .env("HGRCPATH", "")
        .output()
        .unwrap_or_else(|_| panic!("`hg {args}` failed"));
    if !res.status.success() {
        println!("{}", String::from_utf8_lossy(&res.stdout));
        eprintln!("{}", String::from_utf8_lossy(&res.stderr));
        panic!("`hg {args}` failed (see output above)")
    }
}

fn create_commit(repo: &Path, add_modified: bool) {
    if add_modified {
        exec_hg_cmd("add", repo);
    }
    exec_hg_cmd("--config ui.username=foo commit -m message", repo);
}

fn empty_hg_repo() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp dir for hg testing");
    exec_hg_cmd("init", tmp.path());
    tmp
}

#[test]
fn missing_file() {
    let temp_hg = empty_hg_repo();
    let file = temp_hg.path().join("file.txt");
    File::create(&file).unwrap().write_all(b"foo").unwrap();

    assert!(Hg.get_diff_base(&file).is_err());
}

#[test]
fn unmodified_file() {
    let temp_hg = empty_hg_repo();
    let file = temp_hg.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    create_commit(temp_hg.path(), true);
    assert_eq!(Hg.get_diff_base(&file).unwrap(), Vec::from(contents));
}

#[test]
fn modified_file() {
    let temp_hg = empty_hg_repo();
    let file = temp_hg.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    create_commit(temp_hg.path(), true);
    File::create(&file).unwrap().write_all(b"bar").unwrap();

    assert_eq!(Hg.get_diff_base(&file).unwrap(), Vec::from(contents));
}

/// Test that `get_file_head` does not return content for a directory.
/// This is important to correctly cover cases where a directory is removed and replaced by a file.
/// If the contents of the directory object were returned a diff between a path and the directory children would be produced.
#[test]
fn directory() {
    let temp_hg = empty_hg_repo();
    let dir = temp_hg.path().join("file.txt");
    std::fs::create_dir(&dir).expect("");
    let file = dir.join("file.txt");
    let contents = b"foo".as_slice();
    File::create(file).unwrap().write_all(contents).unwrap();

    create_commit(temp_hg.path(), true);

    std::fs::remove_dir_all(&dir).unwrap();
    File::create(&dir).unwrap().write_all(b"bar").unwrap();
    assert!(Hg.get_diff_base(&dir).is_err());
}

/// Test that `get_file_head` does not return content for a symlink.
/// This is important to correctly cover cases where a symlink is removed and replaced by a file.
/// If the contents of the symlink object were returned a diff between a path and the actual file would be produced (bad ui).
#[cfg(any(unix, windows))]
#[test]
fn symlink() {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    #[cfg(not(unix))]
    use std::os::windows::fs::symlink_file as symlink;
    let temp_hg = empty_hg_repo();
    let file = temp_hg.path().join("file.txt");
    let contents = b"foo".as_slice();
    File::create(&file).unwrap().write_all(contents).unwrap();
    let file_link = temp_hg.path().join("file_link.txt");
    symlink("file.txt", &file_link).unwrap();

    create_commit(temp_hg.path(), true);
    assert!(Hg.get_diff_base(&file_link).is_err());
    assert_eq!(Hg.get_diff_base(&file).unwrap(), Vec::from(contents));
}
