use anyhow::{anyhow, Context, Result};
use std::fs;
use std::time::SystemTime;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::channel,
};

use helix_core::syntax::{GrammarConfiguration, GrammarSource, DYLIB_EXTENSION};

const BUILD_TARGET: &str = env!("BUILD_TARGET");
const REMOTE_NAME: &str = "origin";

pub fn fetch_grammars() -> Result<()> {
    run_parallel(get_grammar_configs(), fetch_grammar, "fetch")
}

pub fn build_grammars() -> Result<()> {
    run_parallel(get_grammar_configs(), build_grammar, "build")
}

fn run_parallel<F>(grammars: Vec<GrammarConfiguration>, job: F, action: &'static str) -> Result<()>
where
    F: Fn(GrammarConfiguration) -> Result<()> + std::marker::Send + 'static + Copy,
{
    let mut n_jobs = 0;
    let pool = threadpool::Builder::new().build();
    let (tx, rx) = channel();

    for grammar in grammars {
        let tx = tx.clone();
        n_jobs += 1;

        pool.execute(move || {
            let grammar_id = grammar.grammar_id.clone();
            job(grammar).unwrap_or_else(|err| {
                eprintln!("Failed to {} grammar '{}'\n{}", action, grammar_id, err)
            });

            // report progress
            tx.send(1).unwrap();
        });
    }
    pool.join();

    if rx.try_iter().sum::<usize>() == n_jobs {
        Ok(())
    } else {
        Err(anyhow!("Failed to {} some grammar(s).", action))
    }
}

fn fetch_grammar(grammar: GrammarConfiguration) -> Result<()> {
    if let GrammarSource::Git { remote, revision } = grammar.source {
        let grammar_dir = helix_core::runtime_dir()
            .join("grammars/sources")
            .join(grammar.grammar_id.clone());

        fs::create_dir_all(grammar_dir.clone()).expect("Could not create grammar directory");

        // create the grammar dir contains a git directory
        if !grammar_dir.join(".git").is_dir() {
            git(&grammar_dir, ["init"])?;
        }

        // ensure the remote matches the configured remote
        if get_remote_url(&grammar_dir).map_or(true, |s| s.trim_end() != remote) {
            set_remote(&grammar_dir, &remote)?;
        }

        // ensure the revision matches the configured revision
        if get_revision(&grammar_dir).map_or(true, |s| s.trim_end() != revision) {
            // Fetch the exact revision from the remote.
            // Supported by server-side git since v2.5.0 (July 2015),
            // enabled by default on major git hosts.
            git(&grammar_dir, ["fetch", REMOTE_NAME, &revision])?;
            git(&grammar_dir, ["checkout", &revision])?;

            println!(
                "Grammar '{}' checked out at '{}'.",
                grammar.grammar_id, revision
            );
            Ok(())
        } else {
            println!("Grammar '{}' is already up to date.", grammar.grammar_id);
            Ok(())
        }
    } else {
        println!("Skipping local grammar '{}'", grammar.grammar_id);
        Ok(())
    }
}

// Sets the remote for a repository to the given URL, creating the remote if
// it does not yet exist.
fn set_remote(repository: &Path, remote_url: &str) -> Result<String> {
    git(repository, ["remote", "set-url", REMOTE_NAME, remote_url])
        .or_else(|_| git(repository, ["remote", "add", REMOTE_NAME, remote_url]))
}

fn get_remote_url(repository: &Path) -> Option<String> {
    git(repository, ["remote", "get-url", REMOTE_NAME]).ok()
}

fn get_revision(repository: &Path) -> Option<String> {
    git(repository, ["rev-parse", "HEAD"]).ok()
}

// A wrapper around 'git' commands which returns stdout in success and a
// helpful error message showing the command, stdout, and stderr in error.
fn git<I, S>(repository: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(repository)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        // TODO: figure out how to display the git command using `args`
        Err(anyhow!(
            "Git command failed.\nStdout: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

fn build_grammar(grammar: GrammarConfiguration) -> Result<()> {
    let grammar_dir = if let GrammarSource::Local { ref path } = grammar.source {
        PathBuf::from(path)
    } else {
        helix_core::runtime_dir()
            .join("grammars/sources")
            .join(grammar.grammar_id.clone())
    };

    grammar_dir.read_dir().with_context(|| {
        format!(
            "The directory {:?} is empty, you probably need to use 'hx --fetch-grammars'?",
            grammar_dir
        )
    })?;

    let path = match grammar.path {
        Some(ref subpath) => grammar_dir.join(subpath),
        None => grammar_dir,
    }
    .join("src");

    build_tree_sitter_library(&path, grammar)
}

// Returns the set of grammar configurations the user requests.
// Grammars are configured in the default and user `languages.toml` and are
// merged. The `grammar_selection` key of the config is then used to filter
// down all grammars into a subset of the user's choosing.
fn get_grammar_configs() -> Vec<GrammarConfiguration> {
    let config = helix_core::config::user_syntax_loader().expect("Could not parse languages.toml");

    config.grammar
}

fn build_tree_sitter_library(src_path: &Path, grammar: GrammarConfiguration) -> Result<()> {
    let header_path = src_path;
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
    let parser_lib_path = helix_core::runtime_dir().join("../runtime/grammars");
    let mut library_path = parser_lib_path.join(grammar.grammar_id.clone());
    library_path.set_extension(DYLIB_EXTENSION);

    let recompile = needs_recompile(&library_path, &parser_path, &scanner_path)
        .context("Failed to compare source and binary timestamps")?;

    if !recompile {
        println!("Grammar '{}' is already built.", grammar.grammar_id);
        return Ok(());
    }

    println!("Building grammar '{}'", grammar.grammar_id);

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

    let output = command.output().context("Failed to execute C compiler")?;
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
