mod docgen;
mod helpers;
mod paths;
mod themelint;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tasks {
    use crate::docgen::*;
    use crate::themelint::*;
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

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
        themelint <theme>: Report errors for <theme>, or all themes if no theme is specified.
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
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
