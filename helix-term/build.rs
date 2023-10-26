use helix_loader::grammar::{build_grammars, fetch_grammars};

fn main() {
    let wasm32_build = match std::env::var("CARGO_CFG_TARGET_ARCH") {
        Ok(s) => s == "wasm32",
        _ => false,
    };
    if wasm32_build {
        return;
    }
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }
}
