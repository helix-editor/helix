use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use std::sync::mpsc::channel;

fn collect_tree_sitter_dirs(ignore: &[String]) -> Vec<String> {
    let mut dirs = Vec::new();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("languages");
    for entry in fs::read_dir(path).unwrap().flatten() {
        let path = entry.path();
        let dir = path.file_name().unwrap().to_str().unwrap().to_string();
        if !ignore.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

#[cfg(unix)]
const DYLIB_EXTENSION: &str = "so";

#[cfg(windows)]
const DYLIB_EXTENSION: &str = "dll";

// const BUILD_TARGET: &'static str = env!("BUILD_TARGET");

use anyhow::{anyhow, Context};
use std::{path::Path, process::Command};

fn build_library(src_path: &Path, language: &str) -> Result<()> {
    let header_path = src_path;
    // let grammar_path = src_path.join("grammar.json");
    let parser_path = src_path.join("parser.c");
    let mut scanner_path = src_path.join("scanner.c");

    let scanner_path = if scanner_path.exists() {
        Some(scanner_path)
    } else {
        scanner_path.set_extension("cc");
        if scanner_path.exists() {
            Some(scanner_path)
        } else {
            None
        }
    };
    let parser_lib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../runtime/grammars");
    let mut library_path = parser_lib_path.join(language);
    library_path.set_extension(DYLIB_EXTENSION);

    let recompile = needs_recompile(&library_path, &parser_path, &scanner_path)
        .with_context(|| "Failed to compare source and binary timestamps")?;

    if !recompile {
        return Ok(());
    }
    let mut config = cc::Build::new();
    config.cpp(true).opt_level(2).cargo_metadata(false);
    // .target(BUILD_TARGET)
    // .host(BUILD_TARGET);
    let compiler = config.get_compiler();
    let mut command = Command::new(compiler.path());
    command.current_dir(src_path);
    for (key, value) in compiler.env() {
        command.env(key, value);
    }

    if cfg!(windows) {
        command
            .args(&["/nologo", "/LD", "/I"])
            .arg(header_path)
            .arg("/Od");
        if let Some(scanner_path) = scanner_path.as_ref() {
            command.arg(scanner_path);
        }

        command
            .arg(parser_path)
            .arg("/link")
            .arg(format!("/out:{}", library_path.to_str().unwrap()));
    } else {
        command
            .arg("-shared")
            .arg("-fPIC")
            .arg("-fno-exceptions")
            .arg("-g")
            .arg("-I")
            .arg(header_path)
            .arg("-o")
            .arg(&library_path)
            .arg("-O2");
        if let Some(scanner_path) = scanner_path.as_ref() {
            if scanner_path.extension() == Some("c".as_ref()) {
                command.arg("-xc").arg("-std=c99").arg(scanner_path);
            } else {
                command.arg(scanner_path);
            }
        }
        command.arg("-xc").arg(parser_path);
    }

    let output = command
        .output()
        .with_context(|| "Failed to execute C compiler")?;
    if !output.status.success() {
        return Err(anyhow!(
            "Parser compilation failed.\nStdout: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}
fn needs_recompile(
    lib_path: &Path,
    parser_c_path: &Path,
    scanner_path: &Option<PathBuf>,
) -> Result<bool> {
    if !lib_path.exists() {
        return Ok(true);
    }
    let lib_mtime = mtime(lib_path)?;
    if mtime(parser_c_path)? > lib_mtime {
        return Ok(true);
    }
    if let Some(scanner_path) = scanner_path {
        if mtime(scanner_path)? > lib_mtime {
            return Ok(true);
        }
    }
    Ok(false)
}

fn mtime(path: &Path) -> Result<SystemTime> {
    Ok(fs::metadata(path)?.modified()?)
}

// fn build_c(files: Vec<String>, language: &str) {
//     let mut build = cc::Build::new();
//     for file in files {
//         build
//             .file(&file)
//             .include(PathBuf::from(file).parent().unwrap())
//             .pic(true)
//             .warnings(false);
//     }
//     build.compile(&format!("tree-sitter-{}-c", language));
// }

// fn build_cpp(files: Vec<String>, language: &str) {
//     let mut build = cc::Build::new();

//     let flag = if build.get_compiler().is_like_msvc() {
//         "/std:c++17"
//     } else {
//         "-std=c++14"
//     };

//     for file in files {
//         build
//             .file(&file)
//             .include(PathBuf::from(file).parent().unwrap())
//             .pic(true)
//             .warnings(false)
//             .cpp(true)
//             .flag_if_supported(flag);
//     }
//     build.compile(&format!("tree-sitter-{}-cpp", language));
// }

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

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("languages")
        .join(dir)
        .join("src");
    build_library(&path, language).unwrap();
}

fn main() {
    let ignore = vec![
        "tree-sitter-typescript".to_string(),
        "tree-sitter-haskell".to_string(), // aarch64 failures: https://github.com/tree-sitter/tree-sitter-haskell/issues/34
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

    // build_dir("tree-sitter-typescript/tsx", "tsx");
    // build_dir("tree-sitter-typescript/typescript", "typescript");
}
