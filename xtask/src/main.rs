mod docgen;
mod helpers;
mod path;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tasks {
    use crate::DynError;
    use std::collections::HashSet;

    pub fn docgen() -> Result<(), DynError> {
        use crate::docgen::*;
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(STATIC_COMMANDS_MD_OUTPUT, &static_commands()?);
        write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
        Ok(())
    }

    pub fn querycheck(languages: impl Iterator<Item = String>) -> Result<(), DynError> {
        use helix_core::syntax::LanguageData;

        let languages_to_check: HashSet<_> = languages.collect();
        let loader = helix_core::config::default_lang_loader();
        for (_language, lang_data) in loader.languages() {
            if !languages_to_check.is_empty()
                && !languages_to_check.contains(&lang_data.config().language_id)
            {
                continue;
            }
            let config = lang_data.config();
            let Some(syntax_config) = LanguageData::compile_syntax_config(config, &loader)? else {
                continue;
            };
            let grammar = syntax_config.grammar;
            LanguageData::compile_indent_query(grammar, config)?;
            LanguageData::compile_textobject_query(grammar, config)?;
            LanguageData::compile_rainbow_query(grammar, config)?;
        }

        println!("Query check succeeded");

        Ok(())
    }

    pub fn themecheck(themes: impl Iterator<Item = String>) -> Result<(), DynError> {
        use helix_view::theme::Loader;

        let themes_to_check: HashSet<_> = themes.collect();

        let theme_names = [
            vec!["default".to_string(), "base16_default".to_string()],
            Loader::read_names(&crate::path::themes()),
        ]
        .concat();
        let loader = Loader::new(&[crate::path::runtime()]);
        let mut errors_present = false;

        for name in theme_names {
            if !themes_to_check.is_empty() && !themes_to_check.contains(&name) {
                continue;
            }

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
        docgen                     Generate files to be included in the mdbook output.
        query-check [languages]    Check that tree-sitter queries are valid for the given
                                   languages, or all languages if none are specified.
        theme-check [themes]       Check that the theme files in runtime/themes/ are valid for the
                                   given themes, or all themes if none are specified.
"
        );
    }
}

fn main() -> Result<(), DynError> {
    let mut args = env::args().skip(1);
    let task = args.next();
    match task {
        None => tasks::print_help(),
        Some(t) => match t.as_str() {
            "docgen" => tasks::docgen()?,
            "query-check" => tasks::querycheck(args)?,
            "theme-check" => tasks::themecheck(args)?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
