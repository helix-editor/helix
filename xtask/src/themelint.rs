use crate::path;
use crate::DynError;
use helix_view::theme::Modifier;
use helix_view::Theme;

struct Rule {
    fg: Option<String>,
    bg: Option<String>,
    check_both: bool,
}

// Placed in an fn here, so it's the first thing you see
fn get_rules() -> Vec<Rule> {
    vec![
        // Check for ui.selection, which is required
        Rule::has_either("ui.selection".into()),
        // Check for planned readable text
        Rule::has_fg_bg("ui.text".into(), "ui.background".into()),
        // Check for complete editor.statusline bare minimum
        Rule::has_both("ui.statusline".into()),
        // Check for editor.color-modes
        Rule::has_either("ui.statusline.insert".into()),
        Rule::has_either("ui.statusline.normal".into()),
        Rule::has_either("ui.statusline.select".into()),
        // Check for editor.cursorline
        Rule::has_bg("ui.cursorline".into()),
        // Check for editor.rulers
        Rule::has_bg("ui.virtual.ruler".into()),
        // Check for menus and prompts
        Rule::has_both("ui.menu".into()),
        // Check for visible cursor
        Rule::has_bg("ui.cursor.primary".into()),
        Rule::has_bg("ui.cursor.match".into()),
    ]
}

impl Rule {
    fn has_fg_bg(fg: String, bg: String) -> Rule {
        Rule {
            fg: Some(fg),
            bg: Some(bg),
            check_both: true,
        }
    }
    fn has_bg(bg: String) -> Rule {
        Rule {
            fg: None,
            bg: Some(bg),
            check_both: true,
        }
    }
    fn has_fg(fg: String) -> Rule {
        Rule {
            fg: Some(fg),
            bg: None,
            check_both: true,
        }
    }
    fn has_either(item: String) -> Rule {
        Rule {
            fg: Some(item.clone()),
            bg: Some(item),
            check_both: false,
        }
    }
    fn has_both(item: String) -> Rule {
        Rule {
            fg: Some(item.clone()),
            bg: Some(item),
            check_both: true,
        }
    }
    fn validate(&self, theme: &Theme, messages: &mut Vec<String>) {
        let mut found_fg = true;
        let mut found_bg = true;
        let mut fg_name = "";
        let mut bg_name = "";
        if let Some(fg) = &self.fg {
            fg_name = fg;
            if theme.get(fg).fg.is_none() && theme.get(fg).add_modifier == Modifier::empty() {
                found_fg = false;
            }
        }
        if let Some(bg) = &self.bg {
            bg_name = bg;
            if theme.get(bg).bg.is_none() && theme.get(bg).add_modifier == Modifier::empty() {
                found_bg = false;
            }
        }
        if self.check_both {
            if !found_fg {
                messages.push(format!("{}.fg", fg_name.clone()));
            }
            if !found_bg {
                messages.push(format!("{}.bg", bg_name.clone()));
            }
        } else {
            if !found_fg && !found_bg {
                messages.push(format!("{}", fg_name))
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
        let message: String = messages
            .iter()
            .map(|m| {
                let mut msg = file.clone();
                msg.push_str(".");
                msg.push_str(m);
                msg
            })
            .collect::<Vec<String>>()
            .join(" ")
            .into();
        println!("{}", message.replace(" ", "\n"));
        Err(messages.len().to_string().into())
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
                let errs: i32 = err.to_string().parse().expect("Errors must be integral");
                errors.push(errs)
            }
            _ => return,
        });
    println!("{} of {} themes had issues", errors.len(), files_count);
    if errors.len() > 0 {
        Err(errors.iter().sum::<i32>().to_string().into())
    } else {
        Ok(())
    }
}
