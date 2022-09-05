use crate::path;
use crate::DynError;
use helix_view::theme::Loader;
use helix_view::theme::Modifier;
use helix_view::Theme;

struct Rule {
    fg: Option<&'static str>,
    bg: Option<&'static str>,
    check_both: bool,
}

enum Require {
    Existence(Rule),
    Difference(&'static str, &'static str),
}

// Placed in an fn here, so it's the first thing you see
fn get_rules() -> Vec<Require> {
    vec![
        // Check for ui.selection, which is required
        Require::Existence(Rule::has_either("ui.selection")),
        // Check for planned readable text
        Require::Existence(Rule::has_fg("ui.text")),
        Require::Existence(Rule::has_bg("ui.background")),
        // Check for complete editor.statusline bare minimum
        Require::Existence(Rule::has_both("ui.statusline")),
        Require::Existence(Rule::has_both("ui.statusline.inactive")),
        // Check for editor.color-modes
        Require::Existence(Rule::has_either("ui.statusline.normal")),
        Require::Existence(Rule::has_either("ui.statusline.insert")),
        Require::Existence(Rule::has_either("ui.statusline.select")),
        Require::Difference("ui.statusline.normal", "ui.statusline.insert"),
        Require::Difference("ui.statusline.normal", "ui.statusline.select"),
        // Check for editor.cursorline
        Require::Existence(Rule::has_bg("ui.cursorline.primary")),
        // Check for editor.whitespace
        Require::Existence(Rule::has_fg("ui.virtual.whitespace")),
        // Check fir rulers
        Require::Existence(Rule::has_either("ui.virtual.indent-guide")),
        // Check for editor.rulers
        Require::Existence(Rule::has_either("ui.virtual.ruler")),
        // Check for menus and prompts
        Require::Existence(Rule::has_both("ui.menu")),
        Require::Existence(Rule::has_both("ui.help")),
        Require::Existence(Rule::has_bg("ui.popup")),
        Require::Existence(Rule::has_either("ui.window")),
        // Check for visible cursor
        Require::Existence(Rule::has_bg("ui.cursor.primary")),
        Require::Existence(Rule::has_either("ui.cursor.match")),
    ]
}

impl Rule {
    fn has_bg(bg: &'static str) -> Rule {
        Rule {
            fg: None,
            bg: Some(bg),
            check_both: true,
        }
    }
    fn has_fg(fg: &'static str) -> Rule {
        Rule {
            fg: Some(fg),
            bg: None,
            check_both: true,
        }
    }
    fn has_either(item: &'static str) -> Rule {
        Rule {
            fg: Some(item),
            bg: Some(item),
            check_both: false,
        }
    }
    fn has_both(item: &'static str) -> Rule {
        Rule {
            fg: Some(item),
            bg: Some(item),
            check_both: true,
        }
    }
    fn found_fg(&self, theme: &Theme) -> bool {
        if let Some(fg) = &self.fg {
            if theme.get(fg).fg.is_none() && theme.get(fg).add_modifier == Modifier::empty() {
                return false;
            }
        }
        return true;
    }
    fn found_bg(&self, theme: &Theme) -> bool {
        if let Some(bg) = &self.bg {
            if theme.get(bg).bg.is_none() && theme.get(bg).add_modifier == Modifier::empty() {
                return false;
            }
        }
        return true;
    }
    fn rule_name(&self) -> &'static str {
        if self.fg.is_some() {
            self.fg.unwrap()
        } else if self.bg.is_some() {
            self.bg.unwrap()
        } else {
            "LINTER_ERROR_NO_RULE"
        }
    }

    fn validate(&self, theme: &Theme, messages: &mut Vec<String>) {
        let found_fg = self.found_fg(theme);
        let found_bg = self.found_bg(theme);

        if !self.check_both && (found_fg || found_bg) {
            return;
        }
        if !found_fg || !found_bg {
            let mut missing = vec![];
            if !found_fg {
                missing.push("`fg`");
            }
            if !found_bg {
                missing.push("`bg`");
            }
            let entry = if !self.check_both && !found_fg && !found_bg {
                missing.join(" or ")
            } else {
                missing.join(" and ")
            };
            messages.push(format!(
                "$THEME: missing {} for `{}`",
                entry,
                self.rule_name()
            ))
        }
    }
}

pub fn lint(file: String) -> Result<(), DynError> {
    if file.contains("base16") {
        println!("Skipping base16: {}", file);
        return Ok(());
    }
    let path = path::themes().join(file.clone() + ".toml");
    let theme = std::fs::read(&path).unwrap();
    let theme: Theme = toml::from_slice(&theme).expect("Failed to parse theme");

    let mut messages: Vec<String> = vec![];
    get_rules().iter().for_each(|lint| match lint {
        Require::Existence(rule) => rule.validate(&theme, &mut messages),
        Require::Difference(a, b) => {
            let theme_a = theme.get(a);
            let theme_b = theme.get(b);
            if theme_a == theme_b {
                messages.push(format!("$THEME: `{}` and `{}` cannot be equal", a, b));
            }
        }
    });

    if messages.len() > 0 {
        messages.iter().for_each(|m| {
            let theme = file.clone();
            let message = m.replace("$THEME", theme.as_str());
            println!("{}", message);
        });
        Err(messages.len().to_string().into())
    } else {
        Ok(())
    }
}

pub fn lint_all() -> Result<(), DynError> {
    let files = Loader::read_names(path::themes().as_path());
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
