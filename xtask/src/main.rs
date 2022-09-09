mod docgen;
mod helpers;
mod path;
mod themelint;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tasks {
    use crate::docgen::{lang_features, typable_commands, write};
    use crate::docgen::{LANG_SUPPORT_MD_OUTPUT, TYPABLE_COMMANDS_MD_OUTPUT};
    use crate::themelint::{lint, lint_all};
    use crate::DynError;

    pub fn docgen() -> Result<(), DynError> {
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
        Ok(())
    }


    pub fn themelint(file: Option<String>) -> Result<(), DynError> {
        match file {
            Some(file) => lint(file),
            None => lint_all(),
        }
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

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
        themelint <theme>: Report errors for <theme>, or all themes if no theme is specified.
        query-check: Check that tree-sitter queries are valid.
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
            "themelint" => tasks::themelint(env::args().nth(2))?,
            "query-check" => tasks::query_check()?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
