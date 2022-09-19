mod docgen;
mod helpers;
mod path;
mod querycheck;
mod themelint;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tasks {
    use crate::docgen::{lang_features, typable_commands, write};
    use crate::docgen::{LANG_SUPPORT_MD_OUTPUT, TYPABLE_COMMANDS_MD_OUTPUT};
    use crate::querycheck::query_check;
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
        query_check()
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
