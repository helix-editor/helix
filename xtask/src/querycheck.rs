use crate::DynError;

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
        "rainbows.scm",
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
