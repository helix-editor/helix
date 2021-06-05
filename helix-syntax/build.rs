use std::fs;
use std::path::PathBuf;

use std::sync::mpsc::channel;

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
            .warnings(false);
    }
    build.compile(&format!("tree-sitter-{}-c", language));
}

fn build_cpp(files: Vec<String>, language: &str) {
    let mut build = cc::Build::new();

    let flag = if build.get_compiler().is_like_msvc() {
        "/std:c++17"
    } else {
        "-std=c++14"
    };

    for file in files {
        build
            .file(&file)
            .include(PathBuf::from(file).parent().unwrap())
            .pic(true)
            .warnings(false)
            .cpp(true)
            .flag_if_supported(flag);
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
    let ignore = vec![
        "tree-sitter-typescript".to_string(),
        ".DS_Store".to_string(),
    ];
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
    pool.join();
    // drop(tx);
    assert_eq!(rx.try_iter().sum::<usize>(), n_jobs);

    build_dir("tree-sitter-typescript/tsx", "tsx");
    build_dir("tree-sitter-typescript/typescript", "typescript");
}
