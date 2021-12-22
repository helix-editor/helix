use std::process::Command;

fn main() {
    let git_hash = Command::new("git")
        .args(&["describe", "--dirty"])
        .output()
        .map(|x| String::from_utf8(x.stdout).ok())
        .ok()
        .flatten()
        .unwrap_or_else(|| String::from(env!("CARGO_PKG_VERSION")));
    println!("cargo:rustc-env=VERSION_AND_GIT_HASH={}", git_hash);
}
