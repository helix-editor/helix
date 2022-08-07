use helix_loader::grammar::{build_grammars, fetch_grammars};
use std::borrow::Cow;
use std::process::Command;

const VERSION: &str = include_str!("../VERSION");

fn main() {
    let git_hash = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok());

    let version: Cow<_> = match git_hash {
        Some(git_hash) => format!("{} ({})", VERSION, &git_hash[..8]).into(),
        None => VERSION.into(),
    };

    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }

    println!("cargo:rerun-if-changed=../runtime/grammars/");
    println!("cargo:rerun-if-changed=../VERSION");

    println!("cargo:rustc-env=VERSION_AND_GIT_HASH={}", version);
}
