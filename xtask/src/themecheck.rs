use std::{collections::HashSet, error::Error};

use helix_view::theme::Loader;
use once_cell::sync::Lazy;

use crate::path;

pub fn themecheck() -> Result<(), Box<dyn Error>> {
    let loader = Loader::new(
        path::themes().parent().unwrap(),
        path::themes().parent().unwrap(),
    );

    let mut error_ct = 0;
    for theme in Loader::read_names(path::themes().as_path()) {
        for warning in themecheck_theme(&theme, &loader)? {
            println!("{}: {}", theme, warning);
            error_ct += 1;
        }
    }

    if error_ct > 0 {
        return Err(format!("found {} theme errors", error_ct).into());
    }

    Ok(())
}

fn themecheck_theme(theme: &str, loader: &Loader) -> Result<Vec<String>, Box<dyn Error>> {
    let (_, mut warnings) = loader.load_with_warnings(theme)?;

    warnings.retain(|m| !is_known_issue(theme, m));

    Ok(warnings)
}

fn is_known_issue(theme: &str, message: &str) -> bool {
    static ISSUES: Lazy<HashSet<(&str, &str)>> = Lazy::new(|| {
        let mut known_issues = HashSet::new();
        for issue in KNOWN_ISSUES {
            known_issues.insert(issue);
        }
        known_issues
    });

    ISSUES.contains(&(theme, message))
}

// Theme issues that existed before the implementation of this check. You
// should not add to this list, but rather fix theme errors before committing.
const KNOWN_ISSUES: [(&str, &str); 4] = [
    (
        "monokai_aqua",
        "error loading color 'light-black': malformed hexcode: light-black",
    ),
    (
        "monokai_aqua",
        "error loading color 'purple': malformed hexcode: purple",
    ),
    (
        "papercolor-light",
        "error loading color 'indent': malformed hexcode: indent",
    ),
    (
        "emacs",
        "error loading color 'highlight': malformed hexcode: highlight",
    ),
];
