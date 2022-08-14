use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::SystemTime;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::channel,
};
use tree_sitter::Language;

#[cfg(unix)]
const DYLIB_EXTENSION: &str = "so";

#[cfg(windows)]
const DYLIB_EXTENSION: &str = "dll";

#[cfg(target_arch = "wasm32")]
const DYLIB_EXTENSION: &str = "wasm";

#[derive(Debug, Serialize, Deserialize)]
struct Configuration {
    #[serde(rename = "use-grammars")]
    pub grammar_selection: Option<GrammarSelection>,
    pub grammar: Vec<GrammarConfiguration>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", untagged)]
pub enum GrammarSelection {
    Only { only: HashSet<String> },
    Except { except: HashSet<String> },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrammarConfiguration {
    #[serde(rename = "name")]
    pub grammar_id: String,
    pub source: GrammarSource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", untagged)]
pub enum GrammarSource {
    Local {
        path: String,
    },
    Git {
        #[serde(rename = "git")]
        remote: String,
        #[serde(rename = "rev")]
        revision: String,
        subpath: Option<String>,
    },
}

const BUILD_TARGET: &str = env!("BUILD_TARGET");
const REMOTE_NAME: &str = "origin";

#[cfg(target_arch = "wasm32")]
pub fn get_language(name: &str) -> Result<Language> {
    unimplemented!()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_language(name: &str) -> Result<Language> {
    use libloading::{Library, Symbol};
    let name = name.to_ascii_lowercase();
    let mut library_path = crate::runtime_dir().join("grammars").join(&name);
    library_path.set_extension(DYLIB_EXTENSION);

    let library = unsafe { Library::new(&library_path) }
        .with_context(|| format!("Error opening dynamic library {:?}", library_path))?;
    let language_fn_name = format!("tree_sitter_{}", name.replace('-', "_"));
    let language = unsafe {
        let language_fn: Symbol<unsafe extern "C" fn() -> Language> = library
            .get(language_fn_name.as_bytes())
            .with_context(|| format!("Failed to load symbol {}", language_fn_name))?;
        language_fn()
    };
    std::mem::forget(library);
    Ok(language)
}

pub fn fetch_grammars() -> Result<()> {
    // We do not need to fetch local grammars.
    let mut grammars = get_grammar_configs()?;
    grammars.retain(|grammar| !matches!(grammar.source, GrammarSource::Local { .. }));

    run_parallel(grammars, fetch_grammar, "fetch")
}

pub fn build_grammars(target: Option<String>) -> Result<()> {
    run_parallel(
        get_grammar_configs()?,
        move |grammar| build_grammar(grammar, target.as_deref()),
        "build",
    )
}

// Returns the set of grammar configurations the user requests.
// Grammars are configured in the default and user `languages.toml` and are
// merged. The `grammar_selection` key of the config is then used to filter
// down all grammars into a subset of the user's choosing.
fn get_grammar_configs() -> Result<Vec<GrammarConfiguration>> {
    let config: Configuration = crate::config::user_lang_config()
        .context("Could not parse languages.toml")?
        .try_into()?;

    let grammars = match config.grammar_selection {
        Some(GrammarSelection::Only { only: selections }) => config
            .grammar
            .into_iter()
            .filter(|grammar| selections.contains(&grammar.grammar_id))
            .collect(),
        Some(GrammarSelection::Except { except: rejections }) => config
            .grammar
            .into_iter()
            .filter(|grammar| !rejections.contains(&grammar.grammar_id))
            .collect(),
        None => config.grammar,
    };

    Ok(grammars)
}

fn run_parallel<F>(grammars: Vec<GrammarConfiguration>, job: F, action: &'static str) -> Result<()>
where
    F: Fn(GrammarConfiguration) -> Result<()> + std::marker::Send + 'static + Clone,
{
    let pool = threadpool::Builder::new().build();
    let (tx, rx) = channel();

    for grammar in grammars {
        let tx = tx.clone();
        let job = job.clone();

        pool.execute(move || {
            // Ignore any SendErrors, if any job in another thread has encountered an
            // error the Receiver will be closed causing this send to fail.
            let _ = tx.send(job(grammar));
        });
    }

    drop(tx);

    // TODO: print all failures instead of the first one found.
    rx.iter()
        .find(|result| result.is_err())
        .map(|err| err.with_context(|| format!("Failed to {} some grammar(s)", action)))
        .unwrap_or(Ok(()))
}

fn fetch_grammar(grammar: GrammarConfiguration) -> Result<()> {
    if let GrammarSource::Git {
        remote, revision, ..
    } = grammar.source
    {
        let grammar_dir = crate::runtime_dir()
            .join("grammars")
            .join("sources")
            .join(&grammar.grammar_id);

        fs::create_dir_all(&grammar_dir).context(format!(
            "Could not create grammar directory {:?}",
            grammar_dir
        ))?;

        // create the grammar dir contains a git directory
        if !grammar_dir.join(".git").is_dir() {
            git(&grammar_dir, ["init"])?;
        }

        // ensure the remote matches the configured remote
        if get_remote_url(&grammar_dir).map_or(true, |s| s != remote) {
            set_remote(&grammar_dir, &remote)?;
        }

        // ensure the revision matches the configured revision
        if get_revision(&grammar_dir).map_or(true, |s| s != revision) {
            // Fetch the exact revision from the remote.
            // Supported by server-side git since v2.5.0 (July 2015),
            // enabled by default on major git hosts.
            git(
                &grammar_dir,
                ["fetch", "--depth", "1", REMOTE_NAME, &revision],
            )?;
            git(&grammar_dir, ["checkout", &revision])?;

            println!(
                "Grammar '{}' checked out at '{}'.",
                grammar.grammar_id, revision
            );
        } else {
            println!("Grammar '{}' is already up to date.", grammar.grammar_id);
        }
    }

    Ok(())
}

// Sets the remote for a repository to the given URL, creating the remote if
// it does not yet exist.
fn set_remote(repository_dir: &Path, remote_url: &str) -> Result<String> {
    git(
        repository_dir,
        ["remote", "set-url", REMOTE_NAME, remote_url],
    )
    .or_else(|_| git(repository_dir, ["remote", "add", REMOTE_NAME, remote_url]))
}

fn get_remote_url(repository_dir: &Path) -> Option<String> {
    git(repository_dir, ["remote", "get-url", REMOTE_NAME]).ok()
}

fn get_revision(repository_dir: &Path) -> Option<String> {
    git(repository_dir, ["rev-parse", "HEAD"]).ok()
}

// A wrapper around 'git' commands which returns stdout in success and a
// helpful error message showing the command, stdout, and stderr in error.
fn git<I, S>(repository_dir: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(repository_dir)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned())
    } else {
        // TODO: figure out how to display the git command using `args`
        Err(anyhow!(
            "Git command failed.\nStdout: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

fn build_grammar(grammar: GrammarConfiguration, target: Option<&str>) -> Result<()> {
    let grammar_dir = if let GrammarSource::Local { path } = &grammar.source {
        PathBuf::from(&path)
    } else {
        crate::runtime_dir()
            .join("grammars")
            .join("sources")
            .join(&grammar.grammar_id)
    };

    let grammar_dir_entries = grammar_dir.read_dir().with_context(|| {
        format!(
            "Failed to read directory {:?}. Did you use 'hx --grammar fetch'?",
            grammar_dir
        )
    })?;

    if grammar_dir_entries.count() == 0 {
        return Err(anyhow!(
            "Directory {:?} is empty. Did you use 'hx --grammar fetch'?",
            grammar_dir
        ));
    };

    let path = match &grammar.source {
        GrammarSource::Git {
            subpath: Some(subpath),
            ..
        } => grammar_dir.join(subpath),
        _ => grammar_dir,
    }
    .join("src");

    build_tree_sitter_library(&path, grammar, target)
}

fn build_tree_sitter_library(
    src_path: &Path,
    grammar: GrammarConfiguration,
    target: Option<&str>,
) -> Result<()> {
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
    let parser_lib_path = crate::runtime_dir().join("grammars");
    let mut library_path = parser_lib_path.join(&grammar.grammar_id);
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
        .opt_level(3)
        .cargo_metadata(false)
        .host(BUILD_TARGET)
        .target(target.unwrap_or(BUILD_TARGET));
    let compiler = config.get_compiler();
    let mut command = Command::new(compiler.path());
    command.current_dir(src_path);
    for (key, value) in compiler.env() {
        command.env(key, value);
    }
    command.args(compiler.args());

    if cfg!(all(windows, target_env = "msvc")) {
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
            .arg("-O3");
        if let Some(scanner_path) = scanner_path.as_ref() {
            if scanner_path.extension() == Some("c".as_ref()) {
                command.arg("-xc").arg("-std=c99").arg(scanner_path);
            } else {
                command.arg(scanner_path);
            }
        }
        command.arg("-xc").arg(parser_path);
        if cfg!(all(
            unix,
            not(any(target_os = "macos", target_os = "illumos"))
        )) {
            command.arg("-Wl,-z,relro,-z,now");
        }
    }

    let output = command
        .output()
        .context("Failed to execute C/C++ compiler")?;
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

/// Gives the contents of a file from a language's `runtime/queries/<lang>`
/// directory
pub fn load_runtime_file(language: &str, filename: &str) -> Result<String, std::io::Error> {
    let path = crate::RUNTIME_DIR
        .join("queries")
        .join(language)
        .join(filename);
    std::fs::read_to_string(&path)
}
