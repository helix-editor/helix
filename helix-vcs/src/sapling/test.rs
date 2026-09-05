use std::{cell::RefCell, fs, path::Path, process::Output};

use tempfile::TempDir;

use super::*;

#[test]
fn status_and_conflicts() {
    let repo = Path::new("repo");
    let status = concat!(
        "M\0modified\0\0",
        "A\0added\0\0",
        "A\0copy\0original\0",
        "A\0renamed\0old\0",
        "R\0old\0\0",
        "R\0removed\0\0",
        "!\0missing\0\0",
        "?\0untracked\0\0",
        "M\0conflicted\0\0",
        "M\0resolved\0\0",
        "C\0clean\0\0",
        "I\0ignored\0\0",
    );
    assert_eq!(
        changes(repo, status.as_bytes(), b"U\0conflicted\0R\0resolved\0").unwrap(),
        vec![
            FileChange::Conflict {
                path: repo.join("conflicted")
            },
            FileChange::Modified {
                path: repo.join("modified")
            },
            FileChange::Untracked {
                path: repo.join("added")
            },
            FileChange::Untracked {
                path: repo.join("copy")
            },
            FileChange::Renamed {
                from_path: repo.join("old"),
                to_path: repo.join("renamed")
            },
            FileChange::Deleted {
                path: repo.join("removed")
            },
            FileChange::Deleted {
                path: repo.join("missing")
            },
            FileChange::Untracked {
                path: repo.join("untracked")
            },
            FileChange::Modified {
                path: repo.join("resolved")
            },
        ]
    );
    assert!(changes(repo, b"", b"").unwrap().is_empty());
}

#[test]
fn unusual_paths() {
    for name in [
        "space name",
        "line\nbreak",
        "tab\tname",
        "[glob]*",
        "-option",
        "trailing ",
        "日本語",
    ] {
        let status = format!("M\0{name}\0\0");
        assert_eq!(
            changes(Path::new("repo"), status.as_bytes(), b"").unwrap(),
            vec![FileChange::Modified {
                path: Path::new("repo").join(name)
            }],
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let changes = changes(Path::new("repo"), b"?\0non-utf8-\xff\0\0", b"").unwrap();
        assert_eq!(
            changes[0].path().as_os_str().as_bytes(),
            b"repo/non-utf8-\xff"
        );
    }
}

#[test]
fn invalid_paths_and_records() {
    for status in [
        b"M\0../outside\0\0".as_slice(),
        b"M\0/absolute\0\0",
        b"M\0\0\0",
        b"M\0file\0",
        b"MM\0file\0\0",
    ] {
        assert!(parse_status(status).is_err());
    }
}

#[test]
fn discovery_and_trust() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    for marker in [".sl", ".hg"] {
        let repo = root.join(marker.trim_start_matches('.'));
        fs::create_dir_all(repo.join(marker)).unwrap();
        fs::create_dir_all(repo.join("nested")).unwrap();
        assert_eq!(find_repo(&repo.join("nested")).unwrap(), repo);
        fs::create_dir(repo.join("nested/.git")).unwrap();
        assert!(find_repo(&repo.join("nested")).is_err());
        let file = repo.join("file");
        assert!(get_diff_base(&file, false).is_err());
        assert!(get_current_head_name(&file, false).is_err());
        assert!(for_each_changed_file(&repo, false, |_| panic!("untrusted callback")).is_err());
    }
    assert!(find_repo(&root).is_err());
    assert!(find_repo(Path::new("relative")).is_err());
}

// These tests use an actual Sapling repository. Run explicitly with:
// cargo test -p helix-vcs --features sapling sapling::test -- --include-ignored
// Keeping them ignored lets the regular unit suite run without `sl` installed.
fn sl(repo: &Path, args: &[&str]) -> Output {
    command(repo)
        .env("SL_CONFIG_PATH", "")
        .env("HGRCPATH", "")
        .args(args)
        .output()
        .expect("Sapling (`sl`) must be installed for these tests")
}

fn run(repo: &Path, args: &[&str]) -> Vec<u8> {
    let output = sl(repo, args);
    assert!(
        output.status.success(),
        "sl {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn repository() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let out = Command::new("sl")
        .env("SL_CONFIG_PATH", "")
        .env("HGRCPATH", "")
        .env("SL_PLAIN", "1")
        .args(["init", "--git"])
        .arg(temp.path())
        .output()
        .expect("Sapling (`sl`) must be installed for these tests");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    temp
}

fn commit(repo: &Path) {
    run(
        repo,
        &[
            "commit",
            "--addremove",
            "--message",
            "test",
            "--user",
            "Helix Test <test@example.com>",
        ],
    );
}

fn changed_files(repo: &Path) -> Vec<FileChange> {
    let files = RefCell::new(Vec::new());
    for_each_changed_file(repo, true, |file| {
        files.borrow_mut().push(file.unwrap());
        true
    })
    .unwrap();
    files.into_inner()
}

#[test]
#[ignore = "requires Sapling (`sl`)"]
fn committed_modified_added_and_untracked_files() {
    let temp = repository();
    let repo = temp.path().canonicalize().unwrap();
    let file = repo.join("file");
    let base = b"first\r\nsecond\n\0binary\xff";
    fs::write(&file, base).unwrap();
    fs::write(repo.join("empty"), "").unwrap();
    fs::write(repo.join(".gitignore"), "ignored\n").unwrap();
    commit(&repo);
    assert_eq!(get_diff_base(&file, true).unwrap(), base);
    assert!(get_diff_base(&repo.join("empty"), true).unwrap().is_empty());
    let registry = crate::DiffProviderRegistry::default();
    assert_eq!(registry.get_diff_base(&file, true).unwrap(), base);
    assert!(registry.get_diff_base(&file, false).is_none());
    fs::write(repo.join("ignored"), "ignored\n").unwrap();
    assert!(get_diff_base(&repo.join("ignored"), true).is_err());
    fs::write(&file, "changed\n").unwrap();
    assert_eq!(get_diff_base(&file, true).unwrap(), base);
    let added = repo.join("added");
    fs::write(&added, "new\n").unwrap();
    assert!(get_diff_base(&added, true).is_err());
    run(&repo, &["add", "added"]);
    assert!(get_diff_base(&added, true).unwrap().is_empty());
    fs::write(repo.join("untracked"), "new\n").unwrap();
    assert!(get_diff_base(&repo.join("untracked"), true).is_err());
    assert!(get_diff_base(&repo.join("does-not-exist"), true).is_err());
    fs::create_dir(repo.join("nested")).unwrap();
    let changes = changed_files(&repo.join("nested"));
    assert_eq!(changes.len(), 3);
    assert!(changes.contains(&FileChange::Modified { path: file }));
    assert!(changes.contains(&FileChange::Untracked { path: added }));
    assert!(changes.contains(&FileChange::Untracked {
        path: repo.join("untracked")
    }));
    let count = RefCell::new(0);
    for_each_changed_file(&repo, true, |_| {
        *count.borrow_mut() += 1;
        false
    })
    .unwrap();
    assert_eq!(*count.borrow(), 1);
}

#[test]
#[ignore = "requires Sapling (`sl`)"]
fn copies_renames_and_deletions() {
    let temp = repository();
    let repo = temp.path().canonicalize().unwrap();
    for file in ["original", "old", "removed", "missing"] {
        fs::write(repo.join(file), "base\n").unwrap();
    }
    commit(&repo);
    run(&repo, &["copy", "original", "copy"]);
    run(&repo, &["rename", "old", "renamed"]);
    run(&repo, &["remove", "removed"]);
    fs::remove_file(repo.join("missing")).unwrap();
    assert_eq!(get_diff_base(&repo.join("copy"), true).unwrap(), b"base\n");
    assert_eq!(
        get_diff_base(&repo.join("renamed"), true).unwrap(),
        b"base\n"
    );
    let changes = changed_files(&repo);
    assert_eq!(changes.len(), 4);
    assert!(changes.contains(&FileChange::Untracked {
        path: repo.join("copy")
    }));
    assert!(changes.contains(&FileChange::Renamed {
        from_path: repo.join("old"),
        to_path: repo.join("renamed")
    }));
    assert!(changes.contains(&FileChange::Deleted {
        path: repo.join("removed")
    }));
    assert!(changes.contains(&FileChange::Deleted {
        path: repo.join("missing")
    }));
}

#[test]
#[ignore = "requires Sapling (`sl`)"]
fn bookmark_and_commit_head() {
    let temp = repository();
    let repo = temp.path().canonicalize().unwrap();
    let file = repo.join("file");
    fs::write(&file, "base\n").unwrap();
    commit(&repo);
    let node = run(&repo, &["log", "-r", ".", "-T", "{node|short}"]);
    let head = get_current_head_name(&file, true).unwrap();
    assert_eq!(head.load().as_bytes(), node);
    run(&repo, &["bookmark", "feature"]);
    assert_eq!(
        get_current_head_name(&file, true)
            .unwrap()
            .load()
            .as_ref()
            .as_ref(),
        "feature"
    );
    run(&repo, &["bookmark", "--inactive", "feature"]);
    assert_eq!(
        get_current_head_name(&file, true)
            .unwrap()
            .load()
            .as_bytes(),
        node
    );
}

#[test]
#[ignore = "requires Sapling (`sl`)"]
fn empty_repository() {
    let temp = repository();
    let repo = temp.path().canonicalize().unwrap();
    let file = repo.join("file");
    assert!(changed_files(&repo).is_empty());
    fs::write(&file, "base\n").unwrap();
    run(&repo, &["add", "file"]);
    assert!(get_diff_base(&file, true).unwrap().is_empty());
}

#[cfg(feature = "git")]
#[test]
#[ignore = "requires Sapling (`sl`)"]
fn git_takes_precedence_in_colocated_repository() {
    let temp = repository();
    let repo = temp.path().canonicalize().unwrap();
    let file = repo.join("file");
    fs::write(&file, "Sapling base\n").unwrap();
    commit(&repo);

    fs::write(&file, "Git base\n").unwrap();
    for args in [
        vec!["init", "--initial-branch=git-first"],
        vec!["add", "file"],
        vec![
            "-c",
            "user.name=Helix Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "test",
        ],
    ] {
        let out = Command::new("git")
            .current_dir(&repo)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .args(&args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fs::write(&file, "Working copy\n").unwrap();
    assert_eq!(get_diff_base(&file, true).unwrap(), b"Sapling base\n");
    let registry = crate::DiffProviderRegistry::default();
    assert_eq!(registry.get_diff_base(&file, true).unwrap(), b"Git base\n");
    assert_eq!(
        registry
            .get_current_head_name(&file, true)
            .unwrap()
            .load()
            .as_bytes(),
        b"git-first"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "requires Sapling (`sl`)"]
fn literal_paths_symlinks_and_directory_replacement() {
    use std::os::unix::fs::symlink;
    let temp = repository();
    let repo = temp.path().canonicalize().unwrap();
    for file in [
        "a[1]",
        "a1",
        "space name",
        "-option",
        "glob:literal",
        "日本語",
    ] {
        fs::write(repo.join(file), file).unwrap();
    }
    fs::create_dir(repo.join("directory")).unwrap();
    fs::write(repo.join("directory/child"), "child").unwrap();
    symlink("a1", repo.join("link")).unwrap();
    commit(&repo);
    for file in [
        "a[1]",
        "a1",
        "space name",
        "-option",
        "glob:literal",
        "日本語",
    ] {
        assert_eq!(
            get_diff_base(&repo.join(file), true).unwrap(),
            file.as_bytes()
        );
    }
    assert_eq!(get_diff_base(&repo.join("link"), true).unwrap(), b"a1");
    fs::remove_file(repo.join("directory/child")).unwrap();
    fs::remove_dir(repo.join("directory")).unwrap();
    fs::write(repo.join("directory"), "replacement").unwrap();
    assert!(get_diff_base(&repo.join("directory"), true).is_err());
}

#[test]
#[ignore = "requires Sapling (`sl`)"]
fn unresolved_merge_is_not_duplicated() {
    let temp = repository();
    let repo = temp.path().canonicalize().unwrap();
    let file = repo.join("conflict");
    fs::write(&file, "base\n").unwrap();
    commit(&repo);
    let base = String::from_utf8(run(&repo, &["log", "-r", ".", "-T", "{node}"])).unwrap();
    fs::write(&file, "left\n").unwrap();
    commit(&repo);
    let left = String::from_utf8(run(&repo, &["log", "-r", ".", "-T", "{node}"])).unwrap();
    run(&repo, &["goto", &base]);
    fs::write(&file, "right\n").unwrap();
    commit(&repo);
    let merge = sl(
        &repo,
        &["merge", "--rev", &left, "--tool", "internal:merge"],
    );
    assert!(!merge.status.success());
    assert_eq!(
        changed_files(&repo),
        vec![FileChange::Conflict { path: file.clone() }]
    );
    assert_eq!(get_diff_base(&file, true).unwrap(), b"right\n");
    fs::write(&file, "resolved\n").unwrap();
    run(&repo, &["resolve", "--mark", "conflict"]);
    assert_eq!(
        changed_files(&repo),
        vec![FileChange::Modified { path: file }]
    );
}
