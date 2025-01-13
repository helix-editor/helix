mod codegen;
mod docgen;
mod helpers;
mod path;
mod querycheck;
mod theme_check;

use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod tasks {
    use crate::codegen::code_gen;
    use crate::docgen::{lang_features, static_commands, typable_commands, write};
    use crate::docgen::{LANG_SUPPORT_MD_OUTPUT, STATIC_COMMANDS_MD_OUTPUT, TYPABLE_COMMANDS_MD_OUTPUT};
    use crate::querycheck::query_check;
    use crate::theme_check::theme_check;
    use crate::DynError;

    use std::path::{Path, PathBuf};

    pub fn docgen() -> Result<(), DynError> {
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(STATIC_COMMANDS_MD_OUTPUT, &static_commands()?);
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
        fn workspace_dir() -> PathBuf {
            let output = std::process::Command::new(env!("CARGO"))
                .arg("locate-project")
                .arg("--workspace")
                .arg("--message-format=plain")
                .output()
                .unwrap()
                .stdout;
            let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
            cargo_path.parent().unwrap().to_path_buf()
        }

        // Update the steel submodule
        std::process::Command::new("git")
            .args(["submodule", "init"])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        std::process::Command::new("git")
            .args(["submodule", "foreach", "git", "pull", "origin", "master"])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        let mut workspace_dir = workspace_dir();

        workspace_dir.push("steel");

        std::process::Command::new("cargo")
            .args(["xtask", "install"])
            .current_dir(workspace_dir)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        println!("=> Finished installing steel");

        code_gen();

        let helix_scm_path = helix_term::commands::helix_module_file();
        let steel_init_path = helix_term::commands::steel_init_file();

        if !helix_scm_path.exists() {
            std::fs::File::create(helix_scm_path).expect("Unable to create new helix.scm file!");
        }

        if !steel_init_path.exists() {
            std::fs::File::create(steel_init_path).expect("Unable to create new init.scm file!");
        }

        std::process::Command::new("cargo")
            .args(["install", "--path", "helix-term", "--locked", "--force"])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    pub fn themecheck() -> Result<(), DynError> {
        theme_check()
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
        query-check: Check that tree-sitter queries are valid.
        code-gen: Generate files associated with steel
        steel: Install steel
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
            "theme-check" => tasks::themecheck()?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
