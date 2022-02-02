// Writes the TARGET environment variable available to build scripts so that
// it is available to the xtask by reading from the environment.
// See: https://stackoverflow.com/a/51311222/7232773
fn main() {
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );

    println!("cargo:rustc-env=HOST={}", std::env::var("HOST").unwrap());
}
