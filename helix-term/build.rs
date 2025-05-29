use helix_loader::grammar::{build_grammars, fetch_grammars};

use winresource::WindowsResource;

fn main() {
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = WindowsResource::new();
        res.set_icon("../contrib/helix-256p.ico");
        res.compile().expect("Failed to build Windows resource");
    }
}
