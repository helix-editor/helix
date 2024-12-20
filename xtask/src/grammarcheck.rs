use helix_core::unicode::segmentation::UnicodeSegmentation;
use helix_loader::grammar::git;

use crate::{helpers::lang_config_raw, DynError};

pub fn grammar_check() -> Result<(), DynError> {
    let pool = threadpool::Builder::new().build();
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    let current_dir = std::sync::Arc::new(
        std::env::current_dir()
            .expect("Failed to get current directory")
            .as_path()
            .to_owned(),
    );

    for language in lang_config_raw().grammar {
        let tx = tx.clone();
        let current_dir = std::sync::Arc::clone(&current_dir);

        pool.execute(move || {
            let bold_green = "\x1b[1;32m";
            let reset = "\x1b[0m";
            let underline = "\x1b[4m";
            let blue = "\x1b[34m";

            if let Ok(result) = git(&current_dir, ["ls-remote", &language.source.git]) {
                let latest_commit = result.split_word_bounds().next().unwrap();

                let current_commit = language.source.rev;

                let updates_available = current_commit == latest_commit;

                let repo = language.source.git;
                let name = language.name;

                let link = if repo.starts_with("https://github.com") {
                    format!(
                        "{underline}{blue}{repo}/compare/{current_commit}...{latest_commit}{reset}"
                    )
                } else {
                    format!("{underline}{blue}{repo}{reset}")
                };

                let out = if updates_available {
                    format!("    {bold_green}Updates available{reset} {name} {link}")
                } else {
                    format!("    Up to date        {name}")
                };
                let _ = tx.send(out);
            }
        });
    }

    drop(tx);

    for msg in rx.iter() {
        println!("{msg}");
    }

    Ok(())
}
