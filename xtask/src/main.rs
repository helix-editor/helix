use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tree_sitter_grammars {
    use anyhow::{anyhow, Context, Result};
    use std::fs;
    use std::time::SystemTime;
    use std::{
        path::{Path, PathBuf},
        process::Command,
    };

    const TARGET: &str = env!("TARGET");
    const HOST: &str = env!("HOST");

    #[cfg(unix)]
    const DYLIB_EXTENSION: &str = "so";

    #[cfg(windows)]
    const DYLIB_EXTENSION: &str = "dll";

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
            .host(HOST)
            .target(TARGET);
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
}

pub mod helpers {
    use std::{
        fmt::Display,
        path::{Path, PathBuf},
    };

    use crate::path;
    use helix_core::syntax::Configuration as LangConfig;

    #[derive(Copy, Clone)]
    pub enum TsFeature {
        Highlight,
        TextObjects,
        AutoIndent,
    }

    impl TsFeature {
        pub fn all() -> &'static [Self] {
            &[Self::Highlight, Self::TextObjects, Self::AutoIndent]
        }

        pub fn runtime_filename(&self) -> &'static str {
            match *self {
                Self::Highlight => "highlights.scm",
                Self::TextObjects => "textobjects.scm",
                Self::AutoIndent => "indents.toml",
            }
        }
    }

    impl Display for TsFeature {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match *self {
                    Self::Highlight => "Syntax Highlighting",
                    Self::TextObjects => "Treesitter Textobjects",
                    Self::AutoIndent => "Auto Indent",
                }
            )
        }
    }

    /// Get the list of languages that support a particular tree-sitter
    /// based feature.
    pub fn ts_lang_support(feat: TsFeature) -> Vec<String> {
        let queries_dir = path::ts_queries();

        find_files(&queries_dir, feat.runtime_filename())
            .iter()
            .map(|f| {
                // .../helix/runtime/queries/python/highlights.scm
                let tail = f.strip_prefix(&queries_dir).unwrap(); // python/highlights.scm
                let lang = tail.components().next().unwrap(); // python
                lang.as_os_str().to_string_lossy().to_string()
            })
            .collect()
    }

    /// Get the list of languages that have any form of tree-sitter
    /// queries defined in the runtime directory.
    pub fn langs_with_ts_queries() -> Vec<String> {
        std::fs::read_dir(path::ts_queries())
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                entry
                    .file_type()
                    .ok()?
                    .is_dir()
                    .then(|| entry.file_name().to_string_lossy().to_string())
            })
            .collect()
    }

    // naive implementation, but suffices for our needs
    pub fn find_files(dir: &Path, filename: &str) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_dir() {
                    Some(find_files(&path, filename))
                } else {
                    (path.file_name()?.to_string_lossy() == filename).then(|| vec![path])
                }
            })
            .flatten()
            .collect()
    }

    pub fn lang_config() -> LangConfig {
        let bytes = std::fs::read(path::lang_config()).unwrap();
        toml::from_slice(&bytes).unwrap()
    }
}

pub mod md_gen {
    use crate::DynError;

    use crate::helpers;
    use crate::path;
    use std::fs;

    use helix_term::commands::cmd::TYPABLE_COMMAND_LIST;

    pub const TYPABLE_COMMANDS_MD_OUTPUT: &str = "typable-cmd.md";
    pub const LANG_SUPPORT_MD_OUTPUT: &str = "lang-support.md";

    fn md_table_heading(cols: &[String]) -> String {
        let mut header = String::new();
        header += &md_table_row(cols);
        header += &md_table_row(&vec!["---".to_string(); cols.len()]);
        header
    }

    fn md_table_row(cols: &[String]) -> String {
        "| ".to_owned() + &cols.join(" | ") + " |\n"
    }

    fn md_mono(s: &str) -> String {
        format!("`{}`", s)
    }

    pub fn typable_commands() -> Result<String, DynError> {
        let mut md = String::new();
        md.push_str(&md_table_heading(&[
            "Name".to_owned(),
            "Description".to_owned(),
        ]));

        let cmdify = |s: &str| format!("`:{}`", s);

        for cmd in TYPABLE_COMMAND_LIST {
            let names = std::iter::once(&cmd.name)
                .chain(cmd.aliases.iter())
                .map(|a| cmdify(a))
                .collect::<Vec<_>>()
                .join(", ");

            md.push_str(&md_table_row(&[names.to_owned(), cmd.doc.to_owned()]));
        }

        Ok(md)
    }

    pub fn lang_features() -> Result<String, DynError> {
        let mut md = String::new();
        let ts_features = helpers::TsFeature::all();

        let mut cols = vec!["Language".to_owned()];
        cols.append(
            &mut ts_features
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>(),
        );
        cols.push("Default LSP".to_owned());

        md.push_str(&md_table_heading(&cols));
        let config = helpers::lang_config();

        let mut langs = config
            .language
            .iter()
            .map(|l| l.language_id.clone())
            .collect::<Vec<_>>();
        langs.sort_unstable();

        let mut ts_features_to_langs = Vec::new();
        for &feat in ts_features {
            ts_features_to_langs.push((feat, helpers::ts_lang_support(feat)));
        }

        let mut row = Vec::new();
        for lang in langs {
            let lc = config
                .language
                .iter()
                .find(|l| l.language_id == lang)
                .unwrap(); // lang comes from config
            row.push(lc.language_id.clone());

            for (_feat, support_list) in &ts_features_to_langs {
                row.push(
                    if support_list.contains(&lang) {
                        "✓"
                    } else {
                        ""
                    }
                    .to_owned(),
                );
            }
            row.push(
                lc.language_server
                    .as_ref()
                    .map(|s| s.command.clone())
                    .map(|c| md_mono(&c))
                    .unwrap_or_default(),
            );

            md.push_str(&md_table_row(&row));
            row.clear();
        }

        Ok(md)
    }

    pub fn write(filename: &str, data: &str) {
        let error = format!("Could not write to {}", filename);
        let path = path::book_gen().join(filename);
        fs::write(path, data).expect(&error);
    }
}

pub mod path {
    use std::path::{Path, PathBuf};

    pub fn project_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    pub fn book_gen() -> PathBuf {
        project_root().join("book/src/generated/")
    }

    pub fn ts_queries() -> PathBuf {
        project_root().join("runtime/queries")
    }

    pub fn lang_config() -> PathBuf {
        project_root().join("languages.toml")
    }
}

pub mod tasks {
    use crate::md_gen;
    use crate::tree_sitter_grammars;
    use crate::DynError;
    use std::sync::mpsc::channel;

    pub fn docgen() -> Result<(), DynError> {
        use md_gen::*;
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
        Ok(())
    }

    pub fn build_grammars() -> Result<(), DynError> {
        use tree_sitter_grammars::*;

        let ignore = vec![
            "tree-sitter-typescript".to_string(),
            "tree-sitter-ocaml".to_string(),
        ];
        let dirs = collect_tree_sitter_dirs(&ignore).unwrap();

        let mut n_jobs = 0;
        let pool = threadpool::Builder::new().build(); // by going through the builder, it'll use num_cpus
        let (tx, rx) = channel();

        for dir in dirs {
            let tx = tx.clone();
            n_jobs += 1;

            pool.execute(move || {
                let language = &dir.strip_prefix("tree-sitter-").unwrap();
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
        build_dir("tree-sitter-ocaml/ocaml", "ocaml");
        build_dir("tree-sitter-ocaml/interface", "ocaml-interface");

        Ok(())
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
        build-grammars: Build tree-sitter grammars.
"
        );
    }
}

fn main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task {
        None => tasks::print_help(),
        Some(t) => match t.as_str() {
            "docgen" => tasks::docgen()?,
            "build-grammars" => tasks::build_grammars()?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
