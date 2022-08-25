use crate::path;
use crate::DynError;
use helix_view::Theme;

struct Rule {
    fg: Option<String>,
    bg: Option<String>,
}

// Placed in an fn here, so it's the first thing you see
fn get_rules() -> Vec<Rule> {
    vec![
        Rule::has_fg_bg("ui.text".into(), "ui.background".into()),
        Rule::has_both("ui.statusline".into()),
        Rule::has_bg("ui.virtual.ruler".into()),
    ]
}

impl Rule {
    fn has_fg_bg(fg: String, bg: String) -> Rule {
        Rule {
            fg: Some(fg),
            bg: Some(bg),
        }
    }
    fn has_bg(bg: String) -> Rule {
        Rule {
            fg: None,
            bg: Some(bg),
        }
    }
    fn has_both(item: String) -> Rule {
        Rule {
            fg: Some(item.clone()),
            bg: Some(item),
        }
    }
    fn validate(&self, theme: &Theme, messages: &mut Vec<String>) {
        if let Some(fg) = &self.fg {
            if theme.get(fg).fg.is_none() {
                messages.push(format!("{}.fg", fg.clone()));
            }
        }
        if let Some(bg) = &self.bg {
            if theme.get(bg).bg.is_none() {
                messages.push(format!("{}.bg", bg.clone()));
            }
        }
    }
}

pub fn lint(file: String) -> Result<(), DynError> {
    let path = path::themes().join(file.clone() + ".toml");
    let theme = std::fs::read(&path).unwrap();
    let theme: Theme = toml::from_slice(&theme).expect("Failed to parse theme");

    let mut messages: Vec<String> = vec![];
    get_rules()
        .iter()
        .for_each(|rule| rule.validate(&theme, &mut messages));

    if messages.len() > 0 {
        Err(messages
            .iter()
            .map(|m| {
                let mut msg = file.clone();
                msg.push_str(".");
                msg.push_str(m);
                msg
            })
            .collect::<Vec<String>>()
            .join(" ")
            .into())
    } else {
        Ok(())
    }
}

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
