use helix_core::unicode::segmentation::UnicodeSegmentation;
use helix_loader::grammar::git;

use crate::{helpers::lang_config_raw, DynError};

pub fn treesittere() -> Result<(), DynError> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    for language in lang_config_raw().grammar {
        let bold_green = "\x1b[1;32m";
        let reset = "\x1b[0m";
        let underline = "\x1b[4m";
        let blue = "\x1b[34m";

        let result = git(current_dir.as_path(), ["ls-remote", &language.source.git])?;

        let latest_commit = result.split_word_bounds().next().unwrap();

        let current_commit = language.source.rev;

        let status = if current_commit == latest_commit {
            "Up to date       ".into()
        } else {
            format!("{bold_green}Updates available{reset}")
        };
        let repo = language.source.git;
        let name = language.name;

        let link =
            format!("{underline}{blue}{repo}/compare/{current_commit}...{latest_commit}{reset}");

        println!("    {status} {name}\n    {link}\n");
    }

    println!("Query check succeeded");

    Ok(())
}

pub fn query_check() -> Result<(), DynError> {
    use crate::helpers::lang_config;
    use helix_core::{syntax::read_query, tree_sitter::Query};
    use helix_loader::grammar::get_language;

    let query_files = [
        "highlights.scm",
        "locals.scm",
        "injections.scm",
        "textobjects.scm",
        "indents.scm",
    ];

    for language in lang_config().language {
        let language_name = &language.language_id;
        let grammar_name = language.grammar.as_ref().unwrap_or(language_name);
        for query_file in query_files {
            let language = get_language(grammar_name);
            let query_text = read_query(language_name, query_file);
            if let Ok(lang) = language {
                if !query_text.is_empty() {
                    if let Err(reason) = Query::new(&lang, &query_text) {
                        return Err(format!(
                            "Failed to parse {} queries for {}: {}",
                            query_file, language_name, reason
                        )
                        .into());
                    }
                }
            }
        }
    }

    println!("Query check succeeded");

    Ok(())
}
