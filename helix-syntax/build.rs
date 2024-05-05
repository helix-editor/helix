use std::path::PathBuf;
use std::{env, fs};

fn main() {
    if env::var_os("DISABLED_TS_BUILD").is_some() {
        return;
    }
    let mut config = cc::Build::new();

    let manifest_path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let include_path = manifest_path.join("../vendor/tree-sitter/include");
    let src_path = manifest_path.join("../vendor/tree-sitter/src");
    for entry in fs::read_dir(&src_path).unwrap() {
        let entry = entry.unwrap();
        let path = src_path.join(entry.file_name());
        println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
    }

    config
        .flag_if_supported("-std=c11")
        .flag_if_supported("-fvisibility=hidden")
        .flag_if_supported("-Wshadow")
        .flag_if_supported("-Wno-unused-parameter")
        .include(&src_path)
        .include(&include_path)
        .file(src_path.join("lib.c"))
        .compile("tree-sitter");
}
