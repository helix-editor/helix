use helix_loader::grammar::{build_grammars, fetch_grammars};

fn main() {
    if std::matches!(std::env::var("CARGO_CFG_TARGET_ARCH"), Ok(s) if s == "wasm32") {
        return;
    }
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }
}
