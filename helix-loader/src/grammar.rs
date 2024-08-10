use std::{fmt, path::PathBuf};

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use tree_sitter::Language;

/// The location of a skidder repo.
///
/// These can either be local paths or git repositories to fetch.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Repository {
    Local {
        path: PathBuf,
    },
    Git {
        name: String,
        remote: String,
        branch: String,
    },
}

impl From<Repository> for skidder::Repo {
    fn from(repo: Repository) -> Self {
        match repo {
            Repository::Local { path } => skidder::Repo::Local { path },
            Repository::Git {
                name,
                remote,
                branch,
            } => skidder::Repo::Git {
                name,
                remote,
                branch,
            },
        }
    }
}

impl From<skidder::Repo> for Repository {
    fn from(repo: skidder::Repo) -> Self {
        match repo {
            skidder::Repo::Local { path } => Repository::Local { path },
            skidder::Repo::Git {
                name,
                remote,
                branch,
            } => Repository::Git {
                name,
                remote,
                branch,
            },
        }
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local { path } => write!(f, "Local repository at '{}'", path.display()),
            Self::Git {
                name,
                remote,
                branch,
            } => write!(
                f,
                "Remote repository named '{name}' at '{remote}' on branch '{branch}'"
            ),
        }
    }
}

#[derive(Debug)]
pub struct Loader {
    config: skidder::Config,
}

impl Loader {
    pub fn new(sources: &[Repository]) -> Self {
        let mut repos: Vec<_> = sources.iter().cloned().map(Into::into).collect();
        repos.push(skidder::Repo::Git {
            // TODO: better name
            name: "upstream".to_string(),
            remote: "https://github.com/helix-editor/tree-sitter-grammars".to_string(),
            // TODO: versioned branches, figure out whether to merge symbols & rainbows
            // queries or not.
            branch: "rainbows-and-symbols".to_string(),
        });
        if let Some(default_repo) = option_env!("HELIX_DEFAULT_LANGUAGE_SUPPORT_REPO") {
            repos.push(skidder::Repo::Local {
                path: default_repo.into(),
            });
        }

        // TODO: be able to compile in a repo too with lower precedence than the default
        // git one.

        Self {
            config: skidder::Config {
                repos,
                index: crate::language_support_dir(),
                verbose: true,
            },
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_language(&self, _name: &str) -> Result<Language> {
        unimplemented!()
    }

    pub fn get_language(&self, name: &str) -> Result<Language> {
        use libloading::{Library, Symbol};

        let Some((grammar, library_path)) = self.config.compiled_parser_path(name) else {
            bail!(
                "No parser file found for '{}' in any language support repo",
                name
            );
        };

        let library = unsafe { Library::new(&library_path) }.map_err(|err| {
            anyhow::anyhow!(
                "Error opening dynamic library {}: {err}",
                library_path.display()
            )
        })?;
        let language_fn_name = format!("tree_sitter_{}", grammar.replace(['-', '.'], "_"));
        let language = unsafe {
            let language_fn: Symbol<unsafe extern "C" fn() -> Language> = library
                .get(language_fn_name.as_bytes())
                .with_context(|| format!("Failed to load symbol {}", language_fn_name))?;
            language_fn()
        };
        std::mem::forget(library);
        Ok(language)
    }

    pub fn read_grammar_file(&self, grammar: &str, file: &str) -> Result<String> {
        let Some(grammar_dir) = self.config.grammar_dir(grammar) else {
            bail!(
                "No file '{}' found for grammar '{}' in any language support repo",
                file,
                grammar
            );
        };
        let path = grammar_dir.join(file);
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read file {}", path.display()))?;
        Ok(contents)
    }

    pub fn repository_dirs(&self) -> impl Iterator<Item = (Repository, PathBuf)> + '_ {
        self.config
            .repos
            .iter()
            .map(|repo| (repo.clone().into(), repo.dir(&self.config)))
    }
}

pub fn update_grammars(config: &Loader) -> Result<()> {
    println!("Fetching language support...");
    skidder::fetch(&config.config, true)?;
    println!("Building tree-sitter parsers...");
    skidder::build_all_grammars(&config.config, false, None)?;
    println!("Language support updated successfully");
    Ok(())
}
