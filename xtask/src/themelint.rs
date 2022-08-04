use crate::path;
use crate::DynError;
use helix_view::theme::Color;
use helix_view::theme::Style;
use helix_view::Theme;

const CONTRAST_REQUIRED: f64 = 2.;

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

    let lumin_color = |c: Color| -> i64 {
        println!("{:?}", c);
        match c {
            Color::Black => 1,
            _ => 1000,
        }
    };
    let lumin = |r: u8, g: u8, b: u8| -> f64 {
        (0.2126 * r as f64) + (0.7152 * g as f64) + (0.0722 * b as f64) * 1000. + 1.
    };
    let get_fg = |s: Style| -> Color { s.fg.expect("Must have fg") };
    let get_bg = |s: Style| -> Color { s.bg.expect("Must have bg") };
    let lint_rules = vec![
        (("ui.text", get_fg), ("ui.background", get_bg)),
        (("ui.statusline", get_fg), ("ui.statusline", get_bg)),
    ];

    let mut lint_warnings = vec![];
    lint_rules.into_iter().for_each(|rule| {
        let from_rule = rule.0;
        let from_color = from_rule.1(theme.get(from_rule.0));
        let to_rule = rule.1;
        let to_color = to_rule.1(theme.get(to_rule.0));
        let from_lumin = if let Color::Rgb(r, g, b) = from_color {
            lumin(r, g, b) as i64
        } else {
            lumin_color(from_color)
        };
        let to_lumin = if let Color::Rgb(r, g, b) = to_color {
            lumin(r, g, b) as i64
        } else {
            lumin_color(from_color)
        };
        let contrast = std::cmp::max(from_lumin, to_lumin) as f64
            / (std::cmp::min(from_lumin, to_lumin) as f64 + 0.000001);
        let mut message = String::from(contrast.to_string());
        if from_lumin == 0 {
            message.push_str("from0");
        }
        if to_lumin == 0 {
            message.push_str("to0");
        }
        if from_lumin != 0 && to_lumin != 0 && contrast < CONTRAST_REQUIRED {
            message.push_str("LOW")
        }
        lint_warnings.push(message);
    });
    println!("{:?}", lint_warnings);
    struct ScopeWithError {
        error: bool,
        scope: String,
        messages: Vec<String>,
    }

    let lint_errors: Vec<String> = check
        .into_iter()
        .map(|oneof| {
            oneof.into_iter().fold(
                ScopeWithError {
                    error: false,
                    scope: String::new(),
                    messages: Vec::new(),
                },
                |mut acc, path| {
                    let style = theme.get(path);
                    if style.eq(&Style::default()) {
                        acc.error = true;
                    }
                    if acc.scope.len() == 0 {
                        acc.scope = path.to_string();
                    }

                    acc
                },
            )
        })
        .filter_map(|s| if s.error { Some(s.scope) } else { None })
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
