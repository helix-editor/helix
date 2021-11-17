use std::env;

pub mod md_gen {
    use super::path;
    use std::fs;

    use helix_term::commands::cmd::TYPABLE_COMMAND_LIST;

    pub const TYPABLE_COMMANDS_MD_OUTPUT: &str = "typable-cmd.md";

    pub fn typable_commands() -> String {
        let mut md = String::new();
        md.push_str("| Name | Description |\n");
        md.push_str("| ---  | ---         |\n");

        let cmdify = |s: &str| format!("`:{}`", s);

        for cmd in TYPABLE_COMMAND_LIST {
            let names = std::iter::once(&cmd.name)
                .chain(cmd.aliases.iter())
                .map(|a| cmdify(a))
                .collect::<Vec<_>>()
                .join(", ");

            let entry = format!("| {} | {} |\n", names, cmd.doc);
            md.push_str(&entry);
        }

        md
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
}

pub mod tasks {
    use super::md_gen;

    pub fn bookgen() {
        md_gen::write(
            md_gen::TYPABLE_COMMANDS_MD_OUTPUT,
            &md_gen::typable_commands(),
        );
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask bookgen`.

    Tasks:
        bookgen: Generate files to be included in the mdbook output.
"
        );
    }
}

fn main() -> Result<(), String> {
    let task = env::args().nth(1);
    match task {
        None => tasks::print_help(),
        Some(t) => match t.as_str() {
            "bookgen" => tasks::bookgen(),
            invalid => return Err(format!("Invalid task name: {}", invalid)),
        },
    };
    Ok(())
}
