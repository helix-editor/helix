use helix_loader::grammar::{build_grammars, fetch_grammars};

fn main() {
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }

    // link icon to windows executable
    if cfg!(target_os = "windows") {
        // fetch manifest dir from env var set by Cargo:
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by Cargo");

        // link against the helix-icon-windows library from contrib dir:
        println!("cargo:rustc-link-search=native={}", manifest_dir.replace("helix-term", "contrib"));
        println!("cargo:rustc-link-lib=dylib=helix-icon-windows");
    }
}
