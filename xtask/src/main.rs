mod docgen;
mod helpers;
mod path;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tasks {
    use crate::DynError;

    pub fn docgen() -> Result<(), DynError> {
        use crate::docgen::*;
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(STATIC_COMMANDS_MD_OUTPUT, &static_commands()?);
        write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
        Ok(())
    }

    pub fn querycheck() -> Result<(), DynError> {
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
            let language_name = &language.language_id;
            let grammar_name = language.grammar.as_ref().unwrap_or(language_name);
            for query_file in query_files {
                let language = get_language(grammar_name);
                let query_text = read_query(language_name, query_file);
                if let Ok(lang) = language {
                    if !query_text.is_empty() {
                        if let Err(reason) = Query::new(&lang, &query_text) {
                            return Err(format!(
                                "Failed to parse {} queries for {}: {}",
                                query_file, language_name, reason
                            )
                            .into());
                        }
                    }
                }
            }
        }

        println!("Query check succeeded");

        Ok(())
    }

    pub fn themecheck() -> Result<(), DynError> {
        use helix_view::theme::Loader;

        let theme_names = [
            vec!["default".to_string(), "base16_default".to_string()],
            Loader::read_names(&crate::path::themes()),
        ]
        .concat();
        let loader = Loader::new(&[crate::path::runtime()]);
        let mut errors_present = false;

        for name in theme_names {
            let (_, warnings) = loader.load_with_warnings(&name).unwrap();

            if !warnings.is_empty() {
                errors_present = true;
                println!("Theme '{name}' loaded with errors:");
                for warning in warnings {
                    println!("\t* {}", warning);
                }
            }
        }

        match errors_present {
            true => Err("Errors found when loading bundled themes".into()),
            false => {
                println!("Theme check successful!");
                Ok(())
            }
        }
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
        query-check: Check that tree-sitter queries are valid.
        theme-check: Check that theme files in runtime/themes are valid.
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
            "query-check" => tasks::querycheck()?,
            "theme-check" => tasks::themecheck()?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
