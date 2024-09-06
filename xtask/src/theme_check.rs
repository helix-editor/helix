use helix_view::theme::Loader;

use crate::{path, DynError};

pub fn theme_check() -> Result<(), DynError> {
    let theme_names = [
        vec!["default".to_string(), "base16_default".to_string()],
        Loader::read_names(&path::themes()),
    ]
    .concat();
    let loader = Loader::new(&[path::runtime()]);
    let mut failures_found = false;

    for name in theme_names {
        let (_, validation_failures) = loader.load(&name).unwrap();

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
