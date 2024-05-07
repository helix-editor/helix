mod codegen;
mod docgen;
mod helpers;
mod path;
mod querycheck;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tasks {
    use crate::codegen::code_gen;
    use crate::docgen::{lang_features, typable_commands, write};
    use crate::docgen::{LANG_SUPPORT_MD_OUTPUT, TYPABLE_COMMANDS_MD_OUTPUT};
    use crate::querycheck::query_check;
    use crate::DynError;

    pub fn docgen() -> Result<(), DynError> {
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
        Ok(())
    }

    pub fn querycheck() -> Result<(), DynError> {
        query_check()
    }

    pub fn codegen() {
        code_gen()
    }

    pub fn install_steel() {
        code_gen();
        std::process::Command::new("cargo")
            .args(["install", "--path", "helix-term", "--locked", "--force"])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
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
            "query-check" => tasks::querycheck()?,
            "code-gen" => tasks::codegen(),
            "steel" => tasks::install_steel(),
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
