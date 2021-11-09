use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!(
        "cargo:rustc-env=VERSION_WITH_GIT_HASH={}",
        format!("{} ({})", env!("CARGO_PKG_VERSION"), &git_hash[..8])
    );
}
