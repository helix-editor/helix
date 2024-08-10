use crate::helpers;
use crate::path;
use crate::DynError;
use helix_term::commands::TYPABLE_COMMAND_LIST;
use std::collections::HashSet;
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

    let mut cols = vec!["Language".to_owned()];
    cols.push("Default LSP".to_owned());

    md.push_str(&md_table_heading(&cols));
    let config = helpers::lang_config();

    let mut langs = config
        .language
        .iter()
        .map(|l| l.language_id.clone())
        .collect::<Vec<_>>();
    langs.sort_unstable();

    let mut row = Vec::new();
    for lang in langs {
        let lc = config
            .language
            .iter()
            .find(|l| l.language_id == lang)
            .unwrap(); // lang comes from config
        row.push(lc.language_id.clone());

        let mut seen_commands = HashSet::new();
        let mut commands = String::new();
        for ls_config in lc
            .language_servers
            .iter()
            .filter_map(|ls| config.language_server.get(&ls.name))
        {
            let command = &ls_config.command;
            if !seen_commands.insert(command) {
                continue;
            }

            if !commands.is_empty() {
                commands.push_str(", ");
            }

            commands.push_str(&md_mono(command));
        }
        row.push(commands);

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
