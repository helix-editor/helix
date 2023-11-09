use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let wasm_build = std::matches!(std::env::var("CARGO_CFG_TARGET_ARCH"), Ok(s) if s == "wasm32");

    if wasm_build {
        let paths = fs::read_dir("../runtime/themes/").unwrap();

        let mut themes = String::new();
        themes.push_str(
            "pub fn themes() -> Vec<String> {
    vec![\n",
        );

        let mut get_theme_fn = String::new();
        get_theme_fn.push_str(
            "fn get_theme(name: &str) -> Option<&str> {
    match name {\n",
        );

        for path in paths {
            if let Ok(path) = path {
                let mut name = path.file_name().into_string().unwrap();
                if name.ends_with("toml") {
                    name.truncate(name.len() - 5);
                    themes.push_str(&format!("        \"{}\".to_string(),\n", name));

                    let content = fs::read_to_string(path.path()).unwrap();
                    let theme = &format!("        \"{}\" => Some(r##\"{}\"##),\n", name, content);
                    get_theme_fn.push_str(theme);
                }
            }
        }
        themes.push_str(
            "    ]\n
}\n",
        );
        get_theme_fn.push_str(
            "        _ => None,
    }
}",
        );

        themes.push_str(&get_theme_fn);

        let out_dir = env::var_os("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join("themes.rs");
        fs::write(&dest_path, &themes).unwrap();
        println!("cargo:rerun-if-changed=../runtime/themes/");
    }
}
