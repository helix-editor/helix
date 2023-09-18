use std::borrow::Cow;
use std::path::Path;
use std::process::Command;

const VERSION: &str = include_str!("../VERSION");

fn main() {
    let git_hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok());

    let version: Cow<_> = match &git_hash {
        Some(git_hash) => format!("{} ({})", VERSION, &git_hash[..8]).into(),
        None => VERSION.into(),
    };

    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        std::env::var("TARGET").unwrap()
    );

    println!("cargo:rerun-if-changed=../VERSION");
    println!("cargo:rustc-env=VERSION_AND_GIT_HASH={}", version);

    if git_hash.is_none() {
        return;
    }

    // we need to revparse because the git dir could be anywhere if you are
    // using detached worktrees but there is no good way to obtain an OsString
    // from command output so for now we can't accept non-utf8 paths here
    // probably rare enouch where it doesn't matter tough we could use gitoxide
    // here but that would be make it a hard dependency and slow compile times
    let Some(git_dir): Option<String> = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok())
    else {
        return;
    };
    // If heads starts pointing at something else (different branch)
    // we need to return
    let head = Path::new(&git_dir).join("HEAD");
    if head.exists() {
        println!("cargo:rerun-if-changed={}", head.display());
    }
    // if the thing head points to (branch) itself changes
    // we need to return
    let Some(head_ref): Option<String> = Command::new("git")
        .args(["symbolic-ref", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok())
    else {
        return;
    };
    let head_ref = Path::new(&git_dir).join(head_ref);
    if head_ref.exists() {
        println!("cargo:rerun-if-changed={}", head_ref.display());
    }
}
