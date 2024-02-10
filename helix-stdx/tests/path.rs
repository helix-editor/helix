#![cfg(windows)]

use std::{
    env::set_current_dir,
    error::Error,
    path::{Component, Path, PathBuf},
};

use helix_stdx::path;
use tempfile::Builder;

// Paths on Windows are almost always case-insensitive.
// Normalization should return the original path.
// E.g. mkdir `CaSe`, normalize(`case`) = `CaSe`.
#[test]
fn test_case_folding_windows() -> Result<(), Box<dyn Error>> {
    // tmp/root/case
    let tmp_prefix = std::env::temp_dir();
    set_current_dir(&tmp_prefix)?;

    let root = Builder::new().prefix("root-").tempdir()?;
    let case = Builder::new().prefix("CaSe-").tempdir_in(&root)?;

    let root_without_prefix = root.path().strip_prefix(&tmp_prefix)?;

    let lowercase_case = format!(
        "case-{}",
        case.path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .split_at(5)
            .1
    );
    let test_path = root_without_prefix.join(lowercase_case);
    assert_eq!(
        path::normalize(&test_path),
        case.path().strip_prefix(&tmp_prefix)?
    );

    Ok(())
}

#[test]
fn test_normalize_path() -> Result<(), Box<dyn Error>> {
    /*
    tmp/root/
    ├── link -> dir1/orig_file
    ├── dir1/
    │   └── orig_file
    └── dir2/
        └── dir_link -> ../dir1/
    */

    let tmp_prefix = std::env::temp_dir();
    set_current_dir(&tmp_prefix)?;

    // Create a tree structure as shown above
    let root = Builder::new().prefix("root-").tempdir()?;
    let dir1 = Builder::new().prefix("dir1-").tempdir_in(&root)?;
    let orig_file = Builder::new().prefix("orig_file-").tempfile_in(&dir1)?;
    let dir2 = Builder::new().prefix("dir2-").tempdir_in(&root)?;

    // Create path and delete existing file
    let dir_link = Builder::new()
        .prefix("dir_link-")
        .tempfile_in(&dir2)?
        .path()
        .to_owned();
    let link = Builder::new()
        .prefix("link-")
        .tempfile_in(&root)?
        .path()
        .to_owned();

    use std::os::windows;
    windows::fs::symlink_dir(&dir1, &dir_link)?;
    windows::fs::symlink_file(&orig_file, &link)?;

    // root/link
    let path = link.strip_prefix(&tmp_prefix)?;
    assert_eq!(
        path::normalize(path),
        path,
        "input {:?} and symlink last component shouldn't be resolved",
        path
    );

    // root/dir2/dir_link/orig_file/../..
    let path = dir_link
        .strip_prefix(&tmp_prefix)
        .unwrap()
        .join(orig_file.path().file_name().unwrap())
        .join(Component::ParentDir)
        .join(Component::ParentDir);
    let expected = dir_link
        .strip_prefix(&tmp_prefix)
        .unwrap()
        .join(Component::ParentDir);
    assert_eq!(
        path::normalize(&path),
        expected,
        "input {:?} and \"..\" should not erase the simlink that goes ahead",
        &path
    );

    // root/link/.././../dir2/../
    let path = link
        .strip_prefix(&tmp_prefix)
        .unwrap()
        .join(Component::ParentDir)
        .join(Component::CurDir)
        .join(Component::ParentDir)
        .join(dir2.path().file_name().unwrap())
        .join(Component::ParentDir);
    let expected = link
        .strip_prefix(&tmp_prefix)
        .unwrap()
        .join(Component::ParentDir)
        .join(Component::ParentDir);
    assert_eq!(path::normalize(&path), expected, "input {:?}", &path);

    Ok(())
}
