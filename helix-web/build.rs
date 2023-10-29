use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let tutor = fs::read_to_string("../runtime/tutor").unwrap();
    let tutor = format!("const TUTOR: &str = r##\"{}\"##;", tutor);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tutor.rs");
    fs::write(&dest_path, &tutor).unwrap();
    println!("cargo:rerun-if-changed=tutor.rs");
}
