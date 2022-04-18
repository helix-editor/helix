use helix_loader::grammar::{build_grammars, fetch_grammars};
use std::borrow::Cow;
use std::process::Command;

const VERSION: &str = include_str!("../VERSION");

// mirror struct for helix_term::Config
// original can't be used because it's not built yet
use serde::Deserialize;
#[derive(Deserialize)]
struct BuildConfig {
    paths: helix_loader::Paths,
}

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

    std::env::set_current_dir("..").unwrap();
    let config = std::env::var("HELIX_CONFIG").unwrap();
    let config = std::fs::read_to_string(config).unwrap();
    let paths = toml::from_str::<BuildConfig>(&config).unwrap().paths;
    helix_loader::init_paths(paths).unwrap();
    std::env::set_current_dir("./helix-term").unwrap();

    let grammar_dir = helix_loader::grammar_dir();
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars().expect("Failed to compile tree-sitter grammars");
    }

    println!("cargo:rerun-if-changed={}", grammar_dir.display());
    println!("cargo:rerun-if-changed=../VERSION");

    println!("cargo:rustc-env=VERSION_AND_GIT_HASH={}", version);
}
