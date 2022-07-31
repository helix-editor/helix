use crate::path;
use crate::DynError;
use helix_view::theme::Style;
use helix_view::Theme;

pub fn lint_all() -> Result<(), DynError> {
    let files = std::fs::read_dir(path::themes())
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_string_lossy().to_string();
            if path.is_dir() || name.contains("README") {
                None
            } else {
                Some(name)
            }
        })
        .collect::<Vec<String>>();
    let mut errors = vec![];
    let files_count = files.len();
    println!("{{");
    files
        .into_iter()
        .for_each(|path| match lint(path.replace(".toml", "")) {
            Err(err) => {
                let errs: String = err.to_string();
                errors.push(errs)
            }
            _ => return,
        });
    println!(
        "\"status\":\"{} of {} themes had issues\"}}",
        errors.len(),
        files_count
    );
    if errors.len() > 0 {
        Err(errors.join(" ").into())
    } else {
        Ok(())
    }
}

pub fn lint(file: String) -> Result<(), DynError> {
    let path = path::themes().join(file.clone() + ".toml");
    let theme = std::fs::read(&path).unwrap();
    let theme: Theme = toml::from_slice(&theme).expect("Failed to parse theme");
    let check = vec![
        vec!["ui.background", "ui.background.separator"],
        vec![
            "ui.cursor",
            "ui.cursor.insert",
            "ui.cursor.select",
            "ui.cursor.match",
            "ui.cursor.primary",
        ],
        vec!["ui.linenr", "ui.linenr.selected"],
        vec![
            "ui.statusline",
            "ui.statusline.inactive",
            "ui.statusline.normal",
            "ui.statusline.insert",
            "ui.statusline.select",
            "ui.statusline.separator",
        ],
        vec!["ui.popup", "ui.popup.info"],
        vec!["ui.window"],
        vec!["ui.help"],
        vec!["ui.text"],
        vec!["ui.text.focus", "ui.text.info"],
        vec![
            "ui.virtual",
            "ui.virtual.ruler",
            "ui.virtual.whitespace",
            "ui.virtual.indent-guide",
        ],
        vec!["ui.menu", "ui.menu.selected", "ui.menu.scroll"],
        vec!["ui.selection", "ui.selection.primary"],
        vec![
            "ui.cursorline",
            "ui.cursorline.primary",
            "ui.cursorline.secondary",
        ],
        vec!["warning"],
        vec!["error"],
        vec!["info"],
        vec!["hint"],
        vec![
            "diagnostic",
            "diagnostic.hint",
            "diagnostic.info",
            "diagnostic.warning",
            "diagnostic.error",
        ],
        vec!["markup.raw.inline", "markup.heading"],
    ];
    struct ScopeWithError {
        error: bool,
        scope: String,
    }

    let lint_errors: Vec<String> = check
        .into_iter()
        .map(|oneof| {
            oneof.into_iter().fold(
                ScopeWithError {
                    error: false,
                    scope: String::new(),
                },
                |mut acc, path| {
                    let style = theme.get(path);
                    if !style.eq(&Style::default()) {
                        acc.error = true;
                    } else if acc.scope.len() == 0 {
                        acc.scope = path.to_string();
                    }
                    acc
                },
            )
        })
        .filter_map(|s| if !s.error { Some(s.scope) } else { None })
        .collect();

    if lint_errors.len() > 0 {
        print!("{:?}:", file);
        print_json_arr(lint_errors);
        println!(",");
        Err(path.to_string_lossy().to_string().into())
    } else {
        Ok(())
    }
}
fn print_json_arr(arr: Vec<String>) {
    println!("[");
    let mut first = true;
    for err in arr {
        println!("\t{}\"{}\"", if first { "" } else { "," }, err);
        first = false;
    }
    println!("]");
}
