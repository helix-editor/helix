use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::SystemTime;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::channel,
};
use tempfile::TempPath;
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
    let mut rel_library_path = PathBuf::new().join("grammars").join(name);
    rel_library_path.set_extension(DYLIB_EXTENSION);
    let library_path = crate::runtime_file(&rel_library_path);

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

    println!("Fetching {} grammars", grammars.len());
    let results = run_parallel(grammars, fetch_grammar);

    let mut errors = Vec::new();
    let mut git_updated = Vec::new();
    let mut git_up_to_date = 0;
    let mut non_git = Vec::new();

    for (grammar_id, res) in results {
        match res {
            Ok(FetchStatus::GitUpToDate) => git_up_to_date += 1,
            Ok(FetchStatus::GitUpdated { revision }) => git_updated.push((grammar_id, revision)),
            Ok(FetchStatus::NonGit) => non_git.push(grammar_id),
            Err(e) => errors.push((grammar_id, e)),
        }
    }

    non_git.sort_unstable();
    git_updated.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    if git_up_to_date != 0 {
        println!("{} up to date git grammars", git_up_to_date);
    }

    if !non_git.is_empty() {
        println!("{} non git grammars", non_git.len());
        println!("\t{:?}", non_git);
    }

    if !git_updated.is_empty() {
        println!("{} updated grammars", git_updated.len());
        // We checked the vec is not empty, unwrapping will not panic
        let longest_id = git_updated.iter().map(|x| x.0.len()).max().unwrap();
        for (id, rev) in git_updated {
            println!(
                "\t{id:width$} now on {rev}",
                id = id,
                width = longest_id,
                rev = rev
            );
        }
    }

    if !errors.is_empty() {
        let len = errors.len();
        for (i, (grammar, error)) in errors.into_iter().enumerate() {
            println!("Failure {}/{len}: {grammar} {error}", i + 1);
        }
        bail!("{len} grammars failed to fetch");
    }

    Ok(())
}

pub fn build_grammars(target: Option<String>) -> Result<()> {
    let grammars = get_grammar_configs()?;
    println!("Building {} grammars", grammars.len());
    let results = run_parallel(grammars, move |grammar| {
        build_grammar(grammar, target.as_deref())
    });

    let mut errors = Vec::new();
    let mut already_built = 0;
    let mut built = Vec::new();

    for (grammar_id, res) in results {
        match res {
            Ok(BuildStatus::AlreadyBuilt) => already_built += 1,
            Ok(BuildStatus::Built) => built.push(grammar_id),
            Err(e) => errors.push((grammar_id, e)),
        }
    }

    built.sort_unstable();

    if already_built != 0 {
        println!("{} grammars already built", already_built);
    }

    if !built.is_empty() {
        println!("{} grammars built now", built.len());
        println!("\t{:?}", built);
    }

    if !errors.is_empty() {
        let len = errors.len();
        for (i, (grammar_id, error)) in errors.into_iter().enumerate() {
            println!("Failure {}/{len}: {grammar_id} {error}", i + 1);
        }
        bail!("{len} grammars failed to build");
    }

    Ok(())
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

fn run_parallel<F, Res>(grammars: Vec<GrammarConfiguration>, job: F) -> Vec<(String, Result<Res>)>
where
    F: Fn(GrammarConfiguration) -> Result<Res> + Send + 'static + Clone,
    Res: Send + 'static,
{
    let pool = threadpool::Builder::new().build();
    let (tx, rx) = channel();

    for grammar in grammars {
        let tx = tx.clone();
        let job = job.clone();

        pool.execute(move || {
            // Ignore any SendErrors, if any job in another thread has encountered an
            // error the Receiver will be closed causing this send to fail.
            let _ = tx.send((grammar.grammar_id.clone(), job(grammar)));
        });
    }

    drop(tx);

    rx.iter().collect()
}

enum FetchStatus {
    GitUpToDate,
    GitUpdated { revision: String },
    NonGit,
}

fn fetch_grammar(grammar: GrammarConfiguration) -> Result<FetchStatus> {
    if let GrammarSource::Git {
        remote, revision, ..
    } = grammar.source
    {
        let grammar_dir = crate::runtime_dirs()
            .first()
            .expect("No runtime directories provided") // guaranteed by post-condition
            .join("grammars")
            .join("sources")
            .join(&grammar.grammar_id);

        fs::create_dir_all(&grammar_dir).context(format!(
            "Could not create grammar directory {:?}",
            grammar_dir
        ))?;

        // create the grammar dir contains a git directory
        if !grammar_dir.join(".git").exists() {
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

            Ok(FetchStatus::GitUpdated { revision })
        } else {
            Ok(FetchStatus::GitUpToDate)
        }
    } else {
        Ok(FetchStatus::NonGit)
    }
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

enum BuildStatus {
    AlreadyBuilt,
    Built,
}

fn build_grammar(grammar: GrammarConfiguration, target: Option<&str>) -> Result<BuildStatus> {
    let grammar_dir = if let GrammarSource::Local { path } = &grammar.source {
        PathBuf::from(&path)
    } else {
        crate::runtime_dirs()
            .first()
            .expect("No runtime directories provided") // guaranteed by post-condition
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
) -> Result<BuildStatus> {
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
    let parser_lib_path = crate::runtime_dirs()
        .first()
        .expect("No runtime directories provided") // guaranteed by post-condition
        .join("grammars");
    let mut library_path = parser_lib_path.join(&grammar.grammar_id);
    library_path.set_extension(DYLIB_EXTENSION);

    // if we are running inside a buildscript emit cargo metadata
    // to detect if we are running from a buildscript check some env variables
    // that cargo only sets for build scripts
    if std::env::var("OUT_DIR").is_ok() && std::env::var("CARGO").is_ok() {
        if let Some(scanner_path) = scanner_path.as_ref().and_then(|path| path.to_str()) {
            println!("cargo:rerun-if-changed={scanner_path}");
        }
        if let Some(parser_path) = parser_path.to_str() {
            println!("cargo:rerun-if-changed={parser_path}");
        }
    }

    let recompile = needs_recompile(&library_path, &parser_path, &scanner_path)
        .context("Failed to compare source and binary timestamps")?;

    if !recompile {
        return Ok(BuildStatus::AlreadyBuilt);
    }

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
    // used to delay dropping the temporary object file until after the compilation is complete
    let _path_guard;

    if compiler.is_like_msvc() {
        command
            .args(["/nologo", "/LD", "/I"])
            .arg(header_path)
            .arg("/Od")
            .arg("/utf-8")
            .arg("/std:c11");
        if let Some(scanner_path) = scanner_path.as_ref() {
            if scanner_path.extension() == Some("c".as_ref()) {
                command.arg(scanner_path);
            } else {
                let mut cpp_command = Command::new(compiler.path());
                cpp_command.current_dir(src_path);
                for (key, value) in compiler.env() {
                    cpp_command.env(key, value);
                }
                cpp_command.args(compiler.args());
                let object_file =
                    library_path.with_file_name(format!("{}_scanner.obj", &grammar.grammar_id));
                cpp_command
                    .args(["/nologo", "/LD", "/I"])
                    .arg(header_path)
                    .arg("/Od")
                    .arg("/utf-8")
                    .arg("/std:c++14")
                    .arg(format!("/Fo{}", object_file.display()))
                    .arg("/c")
                    .arg(scanner_path);
                let output = cpp_command
                    .output()
                    .context("Failed to execute C++ compiler")?;

                if !output.status.success() {
                    return Err(anyhow!(
                        "Parser compilation failed.\nStdout: {}\nStderr: {}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                command.arg(&object_file);
                _path_guard = TempPath::from_path(object_file);
            }
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
            .arg("-I")
            .arg(header_path)
            .arg("-o")
            .arg(&library_path);

        if let Some(scanner_path) = scanner_path.as_ref() {
            if scanner_path.extension() == Some("c".as_ref()) {
                command.arg("-xc").arg("-std=c11").arg(scanner_path);
            } else {
                let mut cpp_command = Command::new(compiler.path());
                cpp_command.current_dir(src_path);
                for (key, value) in compiler.env() {
                    cpp_command.env(key, value);
                }
                cpp_command.args(compiler.args());
                let object_file =
                    library_path.with_file_name(format!("{}_scanner.o", &grammar.grammar_id));
                cpp_command
                    .arg("-fPIC")
                    .arg("-fno-exceptions")
                    .arg("-I")
                    .arg(header_path)
                    .arg("-o")
                    .arg(&object_file)
                    .arg("-std=c++14")
                    .arg("-c")
                    .arg(scanner_path);
                let output = cpp_command
                    .output()
                    .context("Failed to execute C++ compiler")?;
                if !output.status.success() {
                    return Err(anyhow!(
                        "Parser compilation failed.\nStdout: {}\nStderr: {}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }

                command.arg(&object_file);
                _path_guard = TempPath::from_path(object_file);
            }
        }
        command.arg("-xc").arg("-std=c11").arg(parser_path);
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

    Ok(BuildStatus::Built)
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
    let path = crate::runtime_file(&PathBuf::new().join("queries").join(language).join(filename));
    std::fs::read_to_string(path)
}
