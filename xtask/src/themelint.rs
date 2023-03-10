use crate::path;
use crate::DynError;
use helix_view::{theme::Modifier, Theme};

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
        Require::Existence(Rule::has_either("ui.selection.primary")),
        Require::Difference("ui.selection", "ui.selection.primary"),
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
        // Check for general ui.virtual (such as inlay-hint)
        Require::Existence(Rule::has_fg("ui.virtual")),
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
        true
    }
    fn found_bg(&self, theme: &Theme) -> bool {
        if let Some(bg) = &self.bg {
            if theme.get(bg).bg.is_none() && theme.get(bg).add_modifier == Modifier::empty() {
                return false;
            }
        }
        true
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

    fn check_difference(
        theme: &Theme,
        a: &'static str,
        b: &'static str,
        messages: &mut Vec<String>,
    ) {
        let theme_a = theme.get(a);
        let theme_b = theme.get(b);
        if theme_a == theme_b {
            messages.push(format!("$THEME: `{}` and `{}` cannot be equal", a, b));
        }
    }

    fn check_existence(rule: &Rule, theme: &Theme, messages: &mut Vec<String>) {
        let found_fg = rule.found_fg(theme);
        let found_bg = rule.found_bg(theme);

        if !rule.check_both && (found_fg || found_bg) {
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
            let entry = if !rule.check_both && !found_fg && !found_bg {
                missing.join(" or ")
            } else {
                missing.join(" and ")
            };
            messages.push(format!(
                "$THEME: missing {} for `{}`",
                entry,
                rule.rule_name()
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
    let theme = std::fs::read_to_string(path).unwrap();
    let theme: Theme = toml::from_str(&theme).expect("Failed to parse theme");

    let mut messages: Vec<String> = vec![];
    get_rules().iter().for_each(|lint| match lint {
        Require::Existence(rule) => Rule::check_existence(rule, &theme, &mut messages),
        Require::Difference(a, b) => Rule::check_difference(&theme, a, b, &mut messages),
    });

    if !messages.is_empty() {
        messages.iter().for_each(|m| {
            let theme = file.clone();
            let message = m.replace("$THEME", theme.as_str());
            println!("{}", message);
        });
        Err(format!("{} has issues", file).into())
    } else {
        Ok(())
    }
}

pub fn lint_all() -> Result<(), DynError> {
    let files = helix_loader::read_toml_names(path::themes().as_path());
    let files_count = files.len();
    let ok_files_count = files
        .into_iter()
        .filter_map(|path| lint(path.replace(".toml", "")).ok())
        .count();

    if files_count != ok_files_count {
        Err(format!(
            "{} of {} themes had issues",
            files_count - ok_files_count,
            files_count
        )
        .into())
    } else {
        Ok(())
    }
}
