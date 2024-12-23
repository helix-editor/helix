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
        let (_, warnings) = loader.load_with_warnings(&name).unwrap();

        if !warnings.is_empty() {
            errors_present = true;
            println!("Theme '{name}' loaded with errors:");
            for warning in warnings {
                println!("\t* {}", warning);
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
