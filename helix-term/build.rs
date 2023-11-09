use helix_loader::grammar::{build_grammars, fetch_grammars};

fn main() {
    fetch_grammars().expect("Failed to fetch tree-sitter grammars");

    let wasm_build = std::matches!(std::env::var("CARGO_CFG_TARGET_ARCH"), Ok(s) if s == "wasm32");
    if !wasm_build && std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }
}
