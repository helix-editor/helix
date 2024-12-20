use helix_core::unicode::segmentation::UnicodeSegmentation;
use helix_loader::grammar::git;

use crate::{helpers::lang_config_grammars, DynError};

pub fn grammar_check() -> Result<(), DynError> {
    let pool = threadpool::Builder::new().build();
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    let current_dir = std::sync::Arc::new(std::env::current_dir().unwrap().as_path().to_owned());

    for language in lang_config_grammars().grammar {
        let tx = tx.clone();
        let current_dir = std::sync::Arc::clone(&current_dir);

        pool.execute(move || {
            let bold_green = "\x1b[1;32m";
            let reset = "\x1b[0m";
            let blue = "\x1b[34m";
            let dark = "\x1b[35m";

            if let Ok(result) = git(&current_dir, ["ls-remote", &language.source.git]) {
                let latest_commit = result.split_word_bounds().next().unwrap();

                let current_commit = language.source.rev;

                let updates_available = current_commit != latest_commit;

                let repo = language.source.git;
                let name = language.name;

                let link = if repo.starts_with("https://github.com") {
                    let url = format!("{blue}\u{1b}]8;;{}/compare{current_commit}...{latest_commit}\u{1b}\\{}\u{1b}]8;;\u{1b}\\{reset}", repo, "[View Diff]");
                    url
                } else {
                    let url = format!("{dark}\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\{reset}", repo, "[Repo Link]");
                    url
                };


                let status = if updates_available {
                    format!("{bold_green}Updates available{reset}")
                } else {
                    "Up to date".into()
                };

                let out = if updates_available {
                    format!(
                        "{status} {link} {name} ",
                    )
                } else {
                    format!("{status}                    {name}")
                };
                let _ = tx.send(out);
            }
        });
    }

    drop(tx);

    println!("\n\n");

    for msg in rx.iter() {
        println!("    {msg}");
    }

    Ok(())
}
