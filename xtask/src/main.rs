mod docgen;
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

    pub fn querycheck() -> Result<(), DynError> {
        use helix_core::{syntax, tree_sitter::Query};
        use helix_loader::grammar::get_language;
        use helix_loader::ts_probe::TsFeature;
        for language_config in syntax::LanguageConfigurations::default().language {
            for ts_feature in TsFeature::all() {
                // TODO: do language name and grammar name discrepancies exist? 
                let language_name = &language_config.language_id;
                let grammar_name = language_config.grammar.as_ref().unwrap_or(language_name);
                if let Ok(treesitter_parser) = get_language(grammar_name) {
                    let query_feature_file_name = ts_feature.runtime_filename();
                    let query_file_text_contents = syntax::read_query(language_name, query_feature_file_name);
                    if !query_file_text_contents.is_empty() {
                        if let Err(err) = Query::new(treesitter_parser, &query_file_text_contents) {
                            return Err(format!("Failed to parse {query_feature_file_name} queries for {language_name}: {err}").into());
                        }
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
            "query-check" => tasks::querycheck()?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
