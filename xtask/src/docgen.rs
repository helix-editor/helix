use crate::helpers;
use crate::path;
use crate::DynError;

use helix_term::commands::MappableCommand;
use helix_term::commands::TYPABLE_COMMAND_LIST;
use helix_term::health::TsFeature;
use helix_view::document::Mode;

use std::collections::HashSet;
use std::fs;

pub const TYPABLE_COMMANDS_MD_OUTPUT: &str = "typable-cmd.md";
pub const STATIC_COMMANDS_MD_OUTPUT: &str = "static-cmd.md";
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

pub fn static_commands() -> Result<String, DynError> {
    let mut md = String::new();
    let keymap = helix_term::keymap::default();
    let keymaps = [
        ("normal", keymap[&Mode::Normal].reverse_map()),
        ("select", keymap[&Mode::Select].reverse_map()),
        ("insert", keymap[&Mode::Insert].reverse_map()),
    ];

    md.push_str(&md_table_heading(&[
        "Name".to_owned(),
        "Description".to_owned(),
        "Default keybinds".to_owned(),
    ]));

    for cmd in MappableCommand::STATIC_COMMAND_LIST {
        let keymap_strings: Vec<_> = keymaps
            .iter()
            .map(|(mode, keymap)| {
                let bindings = keymap
                    .get(cmd.name())
                    .map(|bindings| {
                        let mut bind_strings: Vec<_> = bindings
                            .iter()
                            .map(|bind| {
                                let keys = &bind
                                    .iter()
                                    .map(|key| key.key_sequence_format())
                                    .collect::<String>()
                                    // escape | so it doesn't get rendered as a column separator
                                    .replace('|', "\\|");
                                format!("`` {} ``", keys)
                            })
                            .collect();
                        // sort for stable output. sorting by length puts simple
                        // keybindings first and groups similar keys together
                        bind_strings.sort_by_key(|s| (s.len(), s.to_owned()));
                        bind_strings.join(", ")
                    })
                    .unwrap_or_default();

                (mode, bindings)
            })
            .collect();

        let keymap_string = keymap_strings
            .iter()
            .filter(|(_, bindings)| !bindings.is_empty())
            .map(|(mode, bindings)| format!("{}: {}", mode, bindings))
            .collect::<Vec<_>>()
            .join(", ");

        md.push_str(&md_table_row(&[
            md_mono(cmd.name()),
            cmd.doc().to_owned(),
            keymap_string,
        ]));
    }

    Ok(md)
}

pub fn lang_features() -> Result<String, DynError> {
    let mut md = String::new();
    let ts_features = TsFeature::all();

    let mut cols = vec!["Language".to_owned()];
    cols.append(
        &mut ts_features
            .iter()
            .map(|t| t.long_title().to_string())
            .collect::<Vec<_>>(),
    );
    cols.push("Default language servers".to_owned());

    md.push_str(&md_table_heading(&cols));
    let config = helpers::lang_config();

    let mut langs = config
        .language
        .iter()
        .map(|l| l.language_id.clone())
        .collect::<Vec<_>>();
    langs.sort_unstable();

    let mut ts_features_to_langs = Vec::new();
    for &feat in ts_features {
        ts_features_to_langs.push((feat, helpers::ts_lang_support(feat)));
    }

    let mut row = Vec::new();
    for lang in langs {
        let lc = config
            .language
            .iter()
            .find(|l| l.language_id == lang)
            .unwrap(); // lang comes from config
        row.push(lc.language_id.clone());

        for (_feat, support_list) in &ts_features_to_langs {
            row.push(
                if support_list.contains(&lang) {
                    "✓"
                } else {
                    ""
                }
                .to_owned(),
            );
        }
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
