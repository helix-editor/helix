use helix_view::theme::Loader;

use crate::{path, DynError};

pub fn theme_check() -> Result<(), DynError> {
    let theme_names = [
        vec!["default".to_string(), "base16_default".to_string()],
        Loader::read_names(&path::themes()),
    ]
    .concat();
    let loader = Loader::new(&[path::runtime()]);
    let mut errors_present = false;

    for name in theme_names {
        let (_, load_errors) = loader.load(&name).unwrap();

        if !load_errors.is_empty() {
            errors_present = true;
            println!("Theme '{name}' loaded with errors:");
            for error in load_errors {
                println!("\t* {}", error);
            }
        }
    }

    match errors_present {
        true => Err("Errors found when loading bundled themes".into()),
        false => {
            println!("Theme check successful!");
            Ok(())
        }
    }
}
