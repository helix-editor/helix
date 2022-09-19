use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod helpers {
    use std::path::{Path, PathBuf};

    use crate::path;
    use helix_core::syntax::Configuration as LangConfig;
    use helix_term::health::TsFeature;

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
    use helix_term::commands::TYPABLE_COMMAND_LIST;
    use helix_term::health::TsFeature;
    use std::fs;

    pub const TYPABLE_COMMANDS_MD_OUTPUT: &str = "typable-cmd.md";
    pub const LANG_SUPPORT_MD_OUTPUT: &str = "lang-support.md";

    fn md_table_heading(cols: &[String]) -> String {
        let mut header = String::new();
        header += &md_table_row(cols);
        header += &md_table_row(&vec!["---".to_string(); cols.len()]);
        header
    }

    fn md_table_row(cols: &[String]) -> String {
        format!("| {} |\n", cols.join(" | "))
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

            let doc = cmd.doc.replace('\n', "<br>");

            md.push_str(&md_table_row(&[names.to_owned(), doc.to_owned()]));
        }

        Ok(md)
    }

    pub fn lang_features() -> Result<String, DynError> {
        let mut md = String::new();
        let ts_features = TsFeature::all();

        let mut cols = vec!["Language".to_owned()];
        cols.append(
            &mut ts_features
                .iter()
                .map(|t| t.long_title().to_string())
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

    pub fn book_gen_theme_previews() -> PathBuf {
        book_gen().join("themes")
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
    use crate::DynError;

    pub fn docgen() -> Result<(), DynError> {
        use md_gen::*;
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
        Ok(())
    }

    pub fn query_check() -> Result<(), String> {
        use crate::helpers::lang_config;
        use helix_core::{syntax::read_query, tree_sitter::Query};
        use helix_loader::grammar::get_language;

        let query_files = [
            "highlights.scm",
            "locals.scm",
            "injections.scm",
            "textobjects.scm",
            "indents.scm",
        ];

        for language in lang_config().language {
            let language_name = language.language_id.to_ascii_lowercase();
            let grammar_name = language.grammar.unwrap_or(language.language_id);
            for query_file in query_files {
                let language = get_language(&grammar_name);
                let query_text = read_query(&language_name, query_file);
                if !query_text.is_empty() && language.is_ok() {
                    if let Err(reason) = Query::new(language.unwrap(), &query_text) {
                        return Err(format!(
                            "Failed to parse {} queries for {}: {}",
                            query_file, language_name, reason
                        ));
                    }
                }
            }
        }

        println!("Query check succeeded");

        Ok(())
    }

    pub fn gen_theme_previews() -> Result<(), DynError> {
        use crossterm::event::EventStream;
        use helix_term::{application::Application, args::Args, config::Config};
        use helix_view::theme;

        let themes =
            theme::Loader::new(&helix_loader::config_dir(), &helix_loader::runtime_dir()).names();

        let rt = tokio::runtime::Runtime::new().unwrap();

        for theme in themes {
            rt.block_on(async {
                // create an application instance with a nice soucefile and the current theme
                let args = Args {
                    files: vec![("helix-core/src/register.rs".into(), (16, 15).into())],
                    ..Default::default()
                };
                let config = Config {
                    theme: Some(theme.clone()),
                    ..Default::default()
                };
                let mut app = Application::new(args, config).unwrap();

                let tmp_file = temp_file::empty();

                // redirect stdout to some temp file
                let opts = std::fs::OpenOptions::new()
                    .truncate(true)
                    .write(true)
                    .create(true)
                    .open(tmp_file.path())
                    .unwrap();
                let _redirect = gag::Redirect::stdout(opts).unwrap();

                // render the editor and stop the application after some time
                let mut stream = EventStream::new();
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
                    _ = app.run(&mut stream) => {}
                }

                // convert the captured ANSI sequence to html using the external `aha` tool and
                // capture stdout
                let html_out = tokio::process::Command::new("aha")
                    .arg("-l")
                    .arg("-f")
                    .arg(tmp_file.path())
                    .output()
                    .await
                    .unwrap();

                let out_path = crate::path::book_gen_theme_previews();
                let _ = tokio::fs::create_dir(&out_path).await;

                // dump `aha` output to the final html file
                tokio::fs::write(out_path.join(format!("{}.html", theme)), html_out.stdout)
                    .await
                    .unwrap();
            });
        }
        Ok(())
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
        query-check: Check that tree-sitter queries are valid.
        gen-theme-previews: Make a snapshot of a theme
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
            "query-check" => tasks::query_check()?,
            "gen-theme-previews" => tasks::gen_theme_previews()?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
