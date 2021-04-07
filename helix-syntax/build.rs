use std::path::PathBuf;
use std::{env, fs};

use std::sync::mpsc::channel;

fn get_opt_level() -> u32 {
    env::var("OPT_LEVEL").unwrap().parse::<u32>().unwrap()
}

fn get_debug() -> bool {
    env::var("DEBUG").unwrap() == "true"
}

fn collect_tree_sitter_dirs(ignore: &[String]) -> Vec<String> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir("languages").unwrap().flatten() {
        let path = entry.path();
        let dir = path.file_name().unwrap().to_str().unwrap().to_string();
        if !ignore.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

fn collect_src_files(dir: &str) -> (Vec<String>, Vec<String>) {
    eprintln!("Collect files for {}", dir);

    let mut c_files = Vec::new();
    let mut cpp_files = Vec::new();
    let path = PathBuf::from("languages").join(&dir).join("src");
    for entry in fs::read_dir(path).unwrap().flatten() {
        let path = entry.path();
        if path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("binding")
        {
            continue;
        }
        if let Some(ext) = path.extension() {
            if ext == "c" {
                c_files.push(path.to_str().unwrap().to_string());
            } else if ext == "cc" || ext == "cpp" || ext == "cxx" {
                cpp_files.push(path.to_str().unwrap().to_string());
            }
        }
    }
    (c_files, cpp_files)
}

fn build_c(files: Vec<String>, language: &str) {
    let mut build = cc::Build::new();
    for file in files {
        build
            .file(&file)
            .include(PathBuf::from(file).parent().unwrap())
            .pic(true)
            .opt_level(get_opt_level())
            .debug(get_debug())
            .warnings(false)
            .flag_if_supported("-std=c99");
    }
    build.compile(&format!("tree-sitter-{}-c", language));
}

fn build_cpp(files: Vec<String>, language: &str) {
    let mut build = cc::Build::new();
    for file in files {
        build
            .file(&file)
            .include(PathBuf::from(file).parent().unwrap())
            .pic(true)
            .opt_level(get_opt_level())
            .debug(get_debug())
            .warnings(false)
            .cpp(true);
    }
    build.compile(&format!("tree-sitter-{}-cpp", language));
}

fn build_dir(dir: &str, language: &str) {
    println!("Build language {}", language);
    if PathBuf::from("languages")
        .join(dir)
        .read_dir()
        .unwrap()
        .next()
        .is_none()
    {
        eprintln!(
            "The directory {} is empty, did you use 'git clone --recursive'?",
            dir
        );
        eprintln!("You can fix in using 'git submodule init && git submodule update --recursive'.");
        std::process::exit(1);
    }
    let (c, cpp) = collect_src_files(dir);
    if !c.is_empty() {
        build_c(c, language);
    }
    if !cpp.is_empty() {
        build_cpp(cpp, language);
    }
}

fn main() {
    let ignore = vec!["tree-sitter-typescript".to_string()];
    let dirs = collect_tree_sitter_dirs(&ignore);

    let mut n_jobs = 0;
    let pool = threadpool::Builder::new().build(); // by going through the builder, it'll use num_cpus
    let (tx, rx) = channel();

    for dir in dirs {
        let tx = tx.clone();
        n_jobs += 1;

        pool.execute(move || {
            let language = &dir[12..]; // skip tree-sitter- prefix
            build_dir(&dir, language);

            // report progress
            tx.send(1).unwrap();
        });
    }
    assert_eq!(rx.iter().take(n_jobs).sum::<usize>(), n_jobs);

    build_dir("tree-sitter-typescript/tsx", "tsx");
    build_dir("tree-sitter-typescript/typescript", "typescript");
}
