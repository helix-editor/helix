use std::fs;

use helix_view::{theme::Loader, Theme};

use crate::{path, DynError};

pub fn theme_check() -> Result<(), DynError> {
    let theme_names = Loader::read_names(&path::themes());
    let mut failures_found = false;

    for name in theme_names {
        let path = path::themes().join(format!("{name}.toml"));
        let content = fs::read_to_string(path).unwrap();
        let toml = toml::from_str(&content).unwrap();
        let (_, validation_failures) = Theme::from_keys(toml);

        if !validation_failures.is_empty() {
            failures_found = true;
            println!("Theme '{name}' loaded with warnings:");
            for failure in validation_failures {
                println!("\t* {failure}");
            }
        }
    }

    match failures_found {
        true => Err("Validation failures found in bundled themes".into()),
        false => {
            println!("Theme check successful!");
            Ok(())
        }
    }
}
