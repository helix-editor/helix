use std::collections::HashSet;

use crate::path;
use crate::DynError;
use helix_view::theme::Loader;
use helix_view::theme::Modifier;
use helix_view::Theme;

use once_cell::sync::Lazy;

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

    // Here we load the theme directly without inheritence in order to ensure it has
    // the required values itself.
    let path = path::themes().join(file.clone() + ".toml");
    let theme = std::fs::read_to_string(path).unwrap();
    let theme: Theme = toml::from_str(&theme).expect("Failed to parse theme");

    let mut messages: Vec<String> = vec![];
    get_rules().iter().for_each(|lint| match lint {
        Require::Existence(rule) => Rule::check_existence(rule, &theme, &mut messages),
        Require::Difference(a, b) => Rule::check_difference(&theme, a, b, &mut messages),
    });

    // We also check the theme loader warnings and fail on those.
    let loader = Loader::new(
        path::themes().parent().unwrap(),
        path::themes().parent().unwrap(),
    );
    let (_, warnings) = loader.load_with_warnings(&file)?;
    for warning in warnings {
        messages.push(format!("$THEME: warning: {}", warning));
    }

    messages
        .iter_mut()
        .for_each(|m| *m = m.replace("$THEME", &file));

    messages.retain(|m| !is_known_issue(m));

    if !messages.is_empty() {
        messages.iter().for_each(|m| println!("{}", m));
        Err(format!("{} has issues", file).into())
    } else {
        Ok(())
    }
}

pub fn lint_all() -> Result<(), DynError> {
    let theme_names = Loader::read_names(path::themes().as_path());
    let theme_count = theme_names.len();
    let ok_themes = theme_names
        .into_iter()
        .filter_map(|path| lint(path).ok())
        .count();

    if theme_count != ok_themes {
        Err(format!(
            "{} of {} themes had issues",
            theme_count - ok_themes,
            theme_count
        )
        .into())
    } else {
        Ok(())
    }
}

fn is_known_issue(message: &str) -> bool {
    static ISSUES: Lazy<HashSet<&str>> = Lazy::new(|| {
        let mut known_issues = HashSet::new();
        for issue in KNOWN_ISSUES {
            known_issues.insert(issue);
        }
        known_issues
    });

    ISSUES.contains(message)
}

// Allow all known errors so that we can integrate this tool into CI.
const KNOWN_ISSUES: [&str; 519] = [
    // Pre-existing theme errors prior to addition of theme warnings.
    "rasmus: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ayu_evolve: missing `fg` or `bg` for `ui.selection`",
    "ayu_evolve: missing `fg` or `bg` for `ui.selection.primary`",
    "ayu_evolve: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ayu_evolve: missing `fg` for `ui.text`",
    "ayu_evolve: missing `bg` for `ui.background`",
    "ayu_evolve: missing `fg` and `bg` for `ui.statusline`",
    "ayu_evolve: missing `fg` and `bg` for `ui.statusline.inactive`",
    "ayu_evolve: missing `fg` or `bg` for `ui.statusline.normal`",
    "ayu_evolve: missing `fg` or `bg` for `ui.statusline.insert`",
    "ayu_evolve: missing `fg` or `bg` for `ui.statusline.select`",
    "ayu_evolve: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "ayu_evolve: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "ayu_evolve: missing `bg` for `ui.cursorline.primary`",
    "ayu_evolve: missing `fg` for `ui.virtual.whitespace`",
    "ayu_evolve: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "ayu_evolve: missing `fg` or `bg` for `ui.virtual.ruler`",
    "ayu_evolve: missing `fg` and `bg` for `ui.menu`",
    "ayu_evolve: missing `fg` and `bg` for `ui.help`",
    "ayu_evolve: missing `bg` for `ui.popup`",
    "ayu_evolve: missing `fg` or `bg` for `ui.window`",
    "ayu_evolve: missing `bg` for `ui.cursor.primary`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.selection`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.selection.primary`",
    "catppuccin_frappe: `ui.selection` and `ui.selection.primary` cannot be equal",
    "catppuccin_frappe: missing `fg` for `ui.text`",
    "catppuccin_frappe: missing `bg` for `ui.background`",
    "catppuccin_frappe: missing `fg` and `bg` for `ui.statusline`",
    "catppuccin_frappe: missing `fg` and `bg` for `ui.statusline.inactive`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.statusline.normal`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.statusline.insert`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.statusline.select`",
    "catppuccin_frappe: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "catppuccin_frappe: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "catppuccin_frappe: missing `bg` for `ui.cursorline.primary`",
    "catppuccin_frappe: missing `fg` for `ui.virtual.whitespace`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.virtual.ruler`",
    "catppuccin_frappe: missing `fg` and `bg` for `ui.menu`",
    "catppuccin_frappe: missing `fg` and `bg` for `ui.help`",
    "catppuccin_frappe: missing `bg` for `ui.popup`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.window`",
    "catppuccin_frappe: missing `bg` for `ui.cursor.primary`",
    "catppuccin_frappe: missing `fg` or `bg` for `ui.cursor.match`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.selection`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.selection.primary`",
    "rose_pine_moon: `ui.selection` and `ui.selection.primary` cannot be equal",
    "rose_pine_moon: missing `fg` for `ui.text`",
    "rose_pine_moon: missing `bg` for `ui.background`",
    "rose_pine_moon: missing `fg` and `bg` for `ui.statusline`",
    "rose_pine_moon: missing `fg` and `bg` for `ui.statusline.inactive`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.statusline.normal`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.statusline.insert`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.statusline.select`",
    "rose_pine_moon: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "rose_pine_moon: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "rose_pine_moon: missing `bg` for `ui.cursorline.primary`",
    "rose_pine_moon: missing `fg` for `ui.virtual.whitespace`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.virtual.ruler`",
    "rose_pine_moon: missing `fg` and `bg` for `ui.menu`",
    "rose_pine_moon: missing `fg` and `bg` for `ui.help`",
    "rose_pine_moon: missing `bg` for `ui.popup`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.window`",
    "rose_pine_moon: missing `bg` for `ui.cursor.primary`",
    "rose_pine_moon: missing `fg` or `bg` for `ui.cursor.match`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.selection`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.selection.primary`",
    "github_light_colorblind: `ui.selection` and `ui.selection.primary` cannot be equal",
    "github_light_colorblind: missing `fg` for `ui.text`",
    "github_light_colorblind: missing `bg` for `ui.background`",
    "github_light_colorblind: missing `fg` and `bg` for `ui.statusline`",
    "github_light_colorblind: missing `fg` and `bg` for `ui.statusline.inactive`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.statusline.normal`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.statusline.insert`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.statusline.select`",
    "github_light_colorblind: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_light_colorblind: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "github_light_colorblind: missing `bg` for `ui.cursorline.primary`",
    "github_light_colorblind: missing `fg` for `ui.virtual.whitespace`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.virtual.ruler`",
    "github_light_colorblind: missing `fg` and `bg` for `ui.menu`",
    "github_light_colorblind: missing `fg` and `bg` for `ui.help`",
    "github_light_colorblind: missing `bg` for `ui.popup`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.window`",
    "github_light_colorblind: missing `bg` for `ui.cursor.primary`",
    "github_light_colorblind: missing `fg` or `bg` for `ui.cursor.match`",
    "hex_toxic: missing `fg` or `bg` for `ui.selection`",
    "hex_toxic: missing `fg` or `bg` for `ui.selection.primary`",
    "hex_toxic: `ui.selection` and `ui.selection.primary` cannot be equal",
    "hex_toxic: missing `fg` for `ui.text`",
    "hex_toxic: missing `bg` for `ui.background`",
    "hex_toxic: missing `fg` and `bg` for `ui.statusline`",
    "hex_toxic: missing `fg` and `bg` for `ui.statusline.inactive`",
    "hex_toxic: missing `fg` or `bg` for `ui.statusline.normal`",
    "hex_toxic: missing `fg` or `bg` for `ui.statusline.insert`",
    "hex_toxic: missing `fg` or `bg` for `ui.statusline.select`",
    "hex_toxic: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "hex_toxic: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "hex_toxic: missing `bg` for `ui.cursorline.primary`",
    "hex_toxic: missing `fg` for `ui.virtual.whitespace`",
    "hex_toxic: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "hex_toxic: missing `fg` or `bg` for `ui.virtual.ruler`",
    "hex_toxic: missing `fg` and `bg` for `ui.menu`",
    "hex_toxic: missing `fg` and `bg` for `ui.help`",
    "hex_toxic: missing `bg` for `ui.popup`",
    "hex_toxic: missing `fg` or `bg` for `ui.window`",
    "hex_toxic: missing `bg` for `ui.cursor.primary`",
    "hex_toxic: missing `fg` or `bg` for `ui.cursor.match`",
    "zenburn: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "zenburn: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "zenburn: missing `bg` for `ui.cursorline.primary`",
    "zenburn: missing `fg` or `bg` for `ui.window`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.selection`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.selection.primary`",
    "catppuccin_macchiato: `ui.selection` and `ui.selection.primary` cannot be equal",
    "catppuccin_macchiato: missing `fg` for `ui.text`",
    "catppuccin_macchiato: missing `bg` for `ui.background`",
    "catppuccin_macchiato: missing `fg` and `bg` for `ui.statusline`",
    "catppuccin_macchiato: missing `fg` and `bg` for `ui.statusline.inactive`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.statusline.normal`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.statusline.insert`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.statusline.select`",
    "catppuccin_macchiato: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "catppuccin_macchiato: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "catppuccin_macchiato: missing `bg` for `ui.cursorline.primary`",
    "catppuccin_macchiato: missing `fg` for `ui.virtual.whitespace`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.virtual.ruler`",
    "catppuccin_macchiato: missing `fg` and `bg` for `ui.menu`",
    "catppuccin_macchiato: missing `fg` and `bg` for `ui.help`",
    "catppuccin_macchiato: missing `bg` for `ui.popup`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.window`",
    "catppuccin_macchiato: missing `bg` for `ui.cursor.primary`",
    "catppuccin_macchiato: missing `fg` or `bg` for `ui.cursor.match`",
    "hex_steel: `ui.selection` and `ui.selection.primary` cannot be equal",
    "hex_steel: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.selection`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.selection.primary`",
    "github_dark_colorblind: `ui.selection` and `ui.selection.primary` cannot be equal",
    "github_dark_colorblind: missing `fg` for `ui.text`",
    "github_dark_colorblind: missing `bg` for `ui.background`",
    "github_dark_colorblind: missing `fg` and `bg` for `ui.statusline`",
    "github_dark_colorblind: missing `fg` and `bg` for `ui.statusline.inactive`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.statusline.normal`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.statusline.insert`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.statusline.select`",
    "github_dark_colorblind: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_dark_colorblind: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "github_dark_colorblind: missing `bg` for `ui.cursorline.primary`",
    "github_dark_colorblind: missing `fg` for `ui.virtual.whitespace`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.virtual.ruler`",
    "github_dark_colorblind: missing `fg` and `bg` for `ui.menu`",
    "github_dark_colorblind: missing `fg` and `bg` for `ui.help`",
    "github_dark_colorblind: missing `bg` for `ui.popup`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.window`",
    "github_dark_colorblind: missing `bg` for `ui.cursor.primary`",
    "github_dark_colorblind: missing `fg` or `bg` for `ui.cursor.match`",
    "tokyonight: `ui.selection` and `ui.selection.primary` cannot be equal",
    "tokyonight: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "bogster: `ui.selection` and `ui.selection.primary` cannot be equal",
    "bogster: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "bogster_light: `ui.selection` and `ui.selection.primary` cannot be equal",
    "bogster_light: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "monokai_aqua: missing `fg` or `bg` for `ui.selection`",
    "monokai_aqua: missing `fg` or `bg` for `ui.selection.primary`",
    "monokai_aqua: `ui.selection` and `ui.selection.primary` cannot be equal",
    "monokai_aqua: missing `fg` for `ui.text`",
    "monokai_aqua: missing `bg` for `ui.background`",
    "monokai_aqua: missing `fg` and `bg` for `ui.statusline`",
    "monokai_aqua: missing `fg` and `bg` for `ui.statusline.inactive`",
    "monokai_aqua: missing `fg` or `bg` for `ui.statusline.select`",
    "monokai_aqua: missing `bg` for `ui.cursorline.primary`",
    "monokai_aqua: missing `fg` for `ui.virtual.whitespace`",
    "monokai_aqua: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "monokai_aqua: missing `fg` or `bg` for `ui.virtual.ruler`",
    "monokai_aqua: missing `fg` and `bg` for `ui.menu`",
    "monokai_aqua: missing `fg` and `bg` for `ui.help`",
    "monokai_aqua: missing `bg` for `ui.popup`",
    "monokai_aqua: missing `fg` or `bg` for `ui.window`",
    "monokai_aqua: missing `bg` for `ui.cursor.primary`",
    "monokai_aqua: missing `fg` or `bg` for `ui.cursor.match`",
    "monokai_pro_machine: `ui.selection` and `ui.selection.primary` cannot be equal",
    "monokai_pro_machine: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "monokai_pro_machine: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "monokai_pro_machine: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "monokai_pro: `ui.selection` and `ui.selection.primary` cannot be equal",
    "monokai_pro: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "acme: `ui.selection` and `ui.selection.primary` cannot be equal",
    "acme: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "acme: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "darcula-solid: `ui.selection` and `ui.selection.primary` cannot be equal",
    "darcula-solid: missing `fg` for `ui.text`",
    "darcula-solid: missing `bg` for `ui.background`",
    "darcula-solid: missing `fg` and `bg` for `ui.statusline`",
    "darcula-solid: missing `fg` and `bg` for `ui.statusline.inactive`",
    "darcula-solid: missing `fg` or `bg` for `ui.statusline.normal`",
    "darcula-solid: missing `fg` or `bg` for `ui.statusline.insert`",
    "darcula-solid: missing `fg` or `bg` for `ui.statusline.select`",
    "darcula-solid: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "darcula-solid: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "darcula-solid: missing `bg` for `ui.cursorline.primary`",
    "darcula-solid: missing `fg` for `ui.virtual.whitespace`",
    "darcula-solid: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "darcula-solid: missing `fg` or `bg` for `ui.virtual.ruler`",
    "darcula-solid: missing `fg` and `bg` for `ui.menu`",
    "darcula-solid: missing `fg` and `bg` for `ui.help`",
    "darcula-solid: missing `bg` for `ui.cursor.primary`",
    "darcula-solid: missing `fg` or `bg` for `ui.cursor.match`",
    "autumn: `ui.selection` and `ui.selection.primary` cannot be equal",
    "autumn: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.selection`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.selection.primary`",
    "github_dark_tritanopia: `ui.selection` and `ui.selection.primary` cannot be equal",
    "github_dark_tritanopia: missing `fg` for `ui.text`",
    "github_dark_tritanopia: missing `bg` for `ui.background`",
    "github_dark_tritanopia: missing `fg` and `bg` for `ui.statusline`",
    "github_dark_tritanopia: missing `fg` and `bg` for `ui.statusline.inactive`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.statusline.normal`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.statusline.insert`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.statusline.select`",
    "github_dark_tritanopia: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_dark_tritanopia: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "github_dark_tritanopia: missing `bg` for `ui.cursorline.primary`",
    "github_dark_tritanopia: missing `fg` for `ui.virtual.whitespace`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.virtual.ruler`",
    "github_dark_tritanopia: missing `fg` and `bg` for `ui.menu`",
    "github_dark_tritanopia: missing `fg` and `bg` for `ui.help`",
    "github_dark_tritanopia: missing `bg` for `ui.popup`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.window`",
    "github_dark_tritanopia: missing `bg` for `ui.cursor.primary`",
    "github_dark_tritanopia: missing `fg` or `bg` for `ui.cursor.match`",
    "ingrid: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ingrid: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "ingrid: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "ingrid: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "ayu_light: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ayu_light: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "ayu_light: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "ayu_light: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.selection`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.selection.primary`",
    "github_light_tritanopia: `ui.selection` and `ui.selection.primary` cannot be equal",
    "github_light_tritanopia: missing `fg` for `ui.text`",
    "github_light_tritanopia: missing `bg` for `ui.background`",
    "github_light_tritanopia: missing `fg` and `bg` for `ui.statusline`",
    "github_light_tritanopia: missing `fg` and `bg` for `ui.statusline.inactive`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.statusline.normal`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.statusline.insert`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.statusline.select`",
    "github_light_tritanopia: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_light_tritanopia: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "github_light_tritanopia: missing `bg` for `ui.cursorline.primary`",
    "github_light_tritanopia: missing `fg` for `ui.virtual.whitespace`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.virtual.ruler`",
    "github_light_tritanopia: missing `fg` and `bg` for `ui.menu`",
    "github_light_tritanopia: missing `fg` and `bg` for `ui.help`",
    "github_light_tritanopia: missing `bg` for `ui.popup`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.window`",
    "github_light_tritanopia: missing `bg` for `ui.cursor.primary`",
    "github_light_tritanopia: missing `fg` or `bg` for `ui.cursor.match`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.selection`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.selection.primary`",
    "catppuccin_latte: `ui.selection` and `ui.selection.primary` cannot be equal",
    "catppuccin_latte: missing `fg` for `ui.text`",
    "catppuccin_latte: missing `bg` for `ui.background`",
    "catppuccin_latte: missing `fg` and `bg` for `ui.statusline`",
    "catppuccin_latte: missing `fg` and `bg` for `ui.statusline.inactive`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.statusline.normal`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.statusline.insert`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.statusline.select`",
    "catppuccin_latte: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "catppuccin_latte: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "catppuccin_latte: missing `bg` for `ui.cursorline.primary`",
    "catppuccin_latte: missing `fg` for `ui.virtual.whitespace`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.virtual.ruler`",
    "catppuccin_latte: missing `fg` and `bg` for `ui.menu`",
    "catppuccin_latte: missing `fg` and `bg` for `ui.help`",
    "catppuccin_latte: missing `bg` for `ui.popup`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.window`",
    "catppuccin_latte: missing `bg` for `ui.cursor.primary`",
    "catppuccin_latte: missing `fg` or `bg` for `ui.cursor.match`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.selection`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.selection.primary`",
    "tokyonight_storm: `ui.selection` and `ui.selection.primary` cannot be equal",
    "tokyonight_storm: missing `fg` for `ui.text`",
    "tokyonight_storm: missing `bg` for `ui.background`",
    "tokyonight_storm: missing `fg` and `bg` for `ui.statusline`",
    "tokyonight_storm: missing `fg` and `bg` for `ui.statusline.inactive`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.statusline.normal`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.statusline.insert`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.statusline.select`",
    "tokyonight_storm: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "tokyonight_storm: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "tokyonight_storm: missing `bg` for `ui.cursorline.primary`",
    "tokyonight_storm: missing `fg` for `ui.virtual.whitespace`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.virtual.ruler`",
    "tokyonight_storm: missing `fg` and `bg` for `ui.menu`",
    "tokyonight_storm: missing `fg` and `bg` for `ui.help`",
    "tokyonight_storm: missing `bg` for `ui.popup`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.window`",
    "tokyonight_storm: missing `bg` for `ui.cursor.primary`",
    "tokyonight_storm: missing `fg` or `bg` for `ui.cursor.match`",
    "noctis: `ui.selection` and `ui.selection.primary` cannot be equal",
    "monokai_pro_octagon: `ui.selection` and `ui.selection.primary` cannot be equal",
    "monokai_pro_octagon: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "hex_lavender: missing `fg` or `bg` for `ui.selection`",
    "hex_lavender: missing `fg` or `bg` for `ui.selection.primary`",
    "hex_lavender: `ui.selection` and `ui.selection.primary` cannot be equal",
    "hex_lavender: missing `fg` for `ui.text`",
    "hex_lavender: missing `bg` for `ui.background`",
    "hex_lavender: missing `fg` and `bg` for `ui.statusline`",
    "hex_lavender: missing `fg` and `bg` for `ui.statusline.inactive`",
    "hex_lavender: missing `fg` or `bg` for `ui.statusline.normal`",
    "hex_lavender: missing `fg` or `bg` for `ui.statusline.insert`",
    "hex_lavender: missing `fg` or `bg` for `ui.statusline.select`",
    "hex_lavender: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "hex_lavender: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "hex_lavender: missing `bg` for `ui.cursorline.primary`",
    "hex_lavender: missing `fg` for `ui.virtual.whitespace`",
    "hex_lavender: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "hex_lavender: missing `fg` or `bg` for `ui.virtual.ruler`",
    "hex_lavender: missing `fg` and `bg` for `ui.menu`",
    "hex_lavender: missing `fg` and `bg` for `ui.help`",
    "hex_lavender: missing `bg` for `ui.popup`",
    "hex_lavender: missing `fg` or `bg` for `ui.window`",
    "hex_lavender: missing `bg` for `ui.cursor.primary`",
    "hex_lavender: missing `fg` or `bg` for `ui.cursor.match`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.selection`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.selection.primary`",
    "github_dark_dimmed: `ui.selection` and `ui.selection.primary` cannot be equal",
    "github_dark_dimmed: missing `fg` for `ui.text`",
    "github_dark_dimmed: missing `bg` for `ui.background`",
    "github_dark_dimmed: missing `fg` and `bg` for `ui.statusline`",
    "github_dark_dimmed: missing `fg` and `bg` for `ui.statusline.inactive`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.statusline.normal`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.statusline.insert`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.statusline.select`",
    "github_dark_dimmed: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_dark_dimmed: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "github_dark_dimmed: missing `bg` for `ui.cursorline.primary`",
    "github_dark_dimmed: missing `fg` for `ui.virtual.whitespace`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.virtual.ruler`",
    "github_dark_dimmed: missing `fg` and `bg` for `ui.menu`",
    "github_dark_dimmed: missing `fg` and `bg` for `ui.help`",
    "github_dark_dimmed: missing `bg` for `ui.popup`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.window`",
    "github_dark_dimmed: missing `bg` for `ui.cursor.primary`",
    "github_dark_dimmed: missing `fg` or `bg` for `ui.cursor.match`",
    "varua: `ui.selection` and `ui.selection.primary` cannot be equal",
    "varua: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "varua: missing `fg` or `bg` for `ui.virtual.ruler`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.selection`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.selection.primary`",
    "rose_pine_dawn: `ui.selection` and `ui.selection.primary` cannot be equal",
    "rose_pine_dawn: missing `fg` for `ui.text`",
    "rose_pine_dawn: missing `bg` for `ui.background`",
    "rose_pine_dawn: missing `fg` and `bg` for `ui.statusline`",
    "rose_pine_dawn: missing `fg` and `bg` for `ui.statusline.inactive`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.statusline.normal`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.statusline.insert`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.statusline.select`",
    "rose_pine_dawn: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "rose_pine_dawn: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "rose_pine_dawn: missing `bg` for `ui.cursorline.primary`",
    "rose_pine_dawn: missing `fg` for `ui.virtual.whitespace`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.virtual.ruler`",
    "rose_pine_dawn: missing `fg` and `bg` for `ui.menu`",
    "rose_pine_dawn: missing `fg` and `bg` for `ui.help`",
    "rose_pine_dawn: missing `bg` for `ui.popup`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.window`",
    "rose_pine_dawn: missing `bg` for `ui.cursor.primary`",
    "rose_pine_dawn: missing `fg` or `bg` for `ui.cursor.match`",
    "papercolor-light: `ui.selection` and `ui.selection.primary` cannot be equal",
    "papercolor-light: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "fleet_dark: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "fleet_dark: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "noctis_bordo: `ui.selection` and `ui.selection.primary` cannot be equal",
    "noctis_bordo: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "noctis_bordo: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "nord_light: `ui.selection` and `ui.selection.primary` cannot be equal",
    "serika-light: `ui.selection` and `ui.selection.primary` cannot be equal",
    "serika-light: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "serika-light: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "serika-light: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.selection`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.selection.primary`",
    "github_dark_high_contrast: `ui.selection` and `ui.selection.primary` cannot be equal",
    "github_dark_high_contrast: missing `fg` for `ui.text`",
    "github_dark_high_contrast: missing `bg` for `ui.background`",
    "github_dark_high_contrast: missing `fg` and `bg` for `ui.statusline`",
    "github_dark_high_contrast: missing `fg` and `bg` for `ui.statusline.inactive`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.statusline.normal`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.statusline.insert`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.statusline.select`",
    "github_dark_high_contrast: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_dark_high_contrast: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "github_dark_high_contrast: missing `bg` for `ui.cursorline.primary`",
    "github_dark_high_contrast: missing `fg` for `ui.virtual.whitespace`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.virtual.ruler`",
    "github_dark_high_contrast: missing `fg` and `bg` for `ui.menu`",
    "github_dark_high_contrast: missing `fg` and `bg` for `ui.help`",
    "github_dark_high_contrast: missing `bg` for `ui.popup`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.window`",
    "github_dark_high_contrast: missing `bg` for `ui.cursor.primary`",
    "github_dark_high_contrast: missing `fg` or `bg` for `ui.cursor.match`",
    "catppuccin_mocha: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ayu_mirage: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ayu_mirage: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "ayu_mirage: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "ayu_mirage: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "monokai_pro_ristretto: `ui.selection` and `ui.selection.primary` cannot be equal",
    "monokai_pro_ristretto: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "monokai_pro_ristretto: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "monokai_pro_ristretto: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "autumn_night: missing `fg` or `bg` for `ui.selection`",
    "autumn_night: missing `fg` or `bg` for `ui.selection.primary`",
    "autumn_night: `ui.selection` and `ui.selection.primary` cannot be equal",
    "autumn_night: missing `fg` for `ui.text`",
    "autumn_night: missing `bg` for `ui.background`",
    "autumn_night: missing `fg` and `bg` for `ui.statusline`",
    "autumn_night: missing `fg` and `bg` for `ui.statusline.inactive`",
    "autumn_night: missing `fg` or `bg` for `ui.statusline.normal`",
    "autumn_night: missing `fg` or `bg` for `ui.statusline.insert`",
    "autumn_night: missing `fg` or `bg` for `ui.statusline.select`",
    "autumn_night: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "autumn_night: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "autumn_night: missing `bg` for `ui.cursorline.primary`",
    "autumn_night: missing `fg` for `ui.virtual.whitespace`",
    "autumn_night: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "autumn_night: missing `fg` or `bg` for `ui.virtual.ruler`",
    "autumn_night: missing `fg` and `bg` for `ui.menu`",
    "autumn_night: missing `fg` and `bg` for `ui.help`",
    "autumn_night: missing `bg` for `ui.popup`",
    "autumn_night: missing `fg` or `bg` for `ui.window`",
    "autumn_night: missing `bg` for `ui.cursor.primary`",
    "autumn_night: missing `fg` or `bg` for `ui.cursor.match`",
    "everforest_light: `ui.selection` and `ui.selection.primary` cannot be equal",
    "mellow: `ui.selection` and `ui.selection.primary` cannot be equal",
    "gruvbox_dark_hard: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.selection`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.selection.primary`",
    "github_light_high_contrast: `ui.selection` and `ui.selection.primary` cannot be equal",
    "github_light_high_contrast: missing `fg` for `ui.text`",
    "github_light_high_contrast: missing `bg` for `ui.background`",
    "github_light_high_contrast: missing `fg` and `bg` for `ui.statusline`",
    "github_light_high_contrast: missing `fg` and `bg` for `ui.statusline.inactive`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.statusline.normal`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.statusline.insert`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.statusline.select`",
    "github_light_high_contrast: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "github_light_high_contrast: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "github_light_high_contrast: missing `bg` for `ui.cursorline.primary`",
    "github_light_high_contrast: missing `fg` for `ui.virtual.whitespace`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.virtual.ruler`",
    "github_light_high_contrast: missing `fg` and `bg` for `ui.menu`",
    "github_light_high_contrast: missing `fg` and `bg` for `ui.help`",
    "github_light_high_contrast: missing `bg` for `ui.popup`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.window`",
    "github_light_high_contrast: missing `bg` for `ui.cursor.primary`",
    "github_light_high_contrast: missing `fg` or `bg` for `ui.cursor.match`",
    "darcula: `ui.selection` and `ui.selection.primary` cannot be equal",
    "everforest_dark: `ui.selection` and `ui.selection.primary` cannot be equal",
    "serika-dark: `ui.selection` and `ui.selection.primary` cannot be equal",
    "serika-dark: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "serika-dark: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "serika-dark: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "boo_berry: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ayu_dark: `ui.selection` and `ui.selection.primary` cannot be equal",
    "ayu_dark: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "ayu_dark: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "ayu_dark: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "pop-dark: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "pop-dark: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "monokai_pro_spectrum: `ui.selection` and `ui.selection.primary` cannot be equal",
    "monokai_pro_spectrum: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "monokai_pro_spectrum: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "monokai_pro_spectrum: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "emacs: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "emacs: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "emacs: missing `fg` for `ui.virtual.whitespace`",
    "emacs: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "dracula: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "gruvbox: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "dracula_at_night: missing `fg` for `ui.virtual.whitespace`",
    "dracula_at_night: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "papercolor-dark: `ui.selection` and `ui.selection.primary` cannot be equal",
    "papercolor-dark: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "doom_acario_dark: `ui.selection` and `ui.selection.primary` cannot be equal",
    "doom_acario_dark: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "doom_acario_dark: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "doom_acario_dark: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "night_owl: `ui.selection` and `ui.selection.primary` cannot be equal",
    "night_owl: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "monokai: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "monokai: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "monokai: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "nord: `ui.selection` and `ui.selection.primary` cannot be equal",
    "nord: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "sonokai: `ui.statusline.normal` and `ui.statusline.insert` cannot be equal",
    "sonokai: `ui.statusline.normal` and `ui.statusline.select` cannot be equal",
    "sonokai: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "gruvbox_light: missing `fg` or `bg` for `ui.virtual.indent-guide`",
    "penumbra+: `ui.selection` and `ui.selection.primary` cannot be equal",
    // Existing theme warnings.
    "monokai_aqua: warning: error loading color 'light-black': malformed hexcode: light-black",
    "monokai_aqua: warning: error loading color 'light-black': malformed hexcode: light-black",
    "monokai_aqua: warning: error loading color 'purple': malformed hexcode: purple",
    "papercolor-light: warning: error loading color 'indent': malformed hexcode: indent",
    "emacs: warning: error loading color 'highlight': malformed hexcode: highlight",
];
