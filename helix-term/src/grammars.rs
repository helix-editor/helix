use anyhow::{anyhow, Context, Result};
use std::fs;
use std::time::SystemTime;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use helix_core::syntax::DYLIB_EXTENSION;

const BUILD_TARGET: &str = env!("BUILD_TARGET");

pub fn collect_tree_sitter_dirs(ignore: &[String]) -> Result<Vec<String>> {
    let mut dirs = Vec::new();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../helix-syntax/languages");

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if !entry.file_type()?.is_dir() {
            continue;
        }

        let dir = path.file_name().unwrap().to_str().unwrap().to_string();

        // filter ignores
        if ignore.contains(&dir) {
            continue;
        }
        dirs.push(dir)
    }

    Ok(dirs)
}

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
    config
        .cpp(true)
        .opt_level(2)
        .cargo_metadata(false)
        .host(BUILD_TARGET)
        .target(BUILD_TARGET);
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
            .arg("/Od")
            .arg("/utf-8");
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
        if cfg!(all(unix, not(target_os = "macos"))) {
            command.arg("-Wl,-z,relro,-z,now");
        }
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

pub fn build_dir(dir: &str, language: &str) {
    println!("Build language {}", language);
    if PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../helix-syntax/languages")
        .join(dir)
        .read_dir()
        .unwrap()
        .next()
        .is_none()
    {
        eprintln!(
            "The directory {} is empty, you probably need to use './scripts/grammars sync'?",
            dir
        );
        std::process::exit(1);
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../helix-syntax/languages")
        .join(dir)
        .join("src");

    build_library(&path, language).unwrap();
}
