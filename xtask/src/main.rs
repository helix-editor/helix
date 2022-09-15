use std::{env, error::Error};

type DynError = Box<dyn Error>;

pub mod helpers {
    use std::path::{Path, PathBuf};

    use crate::path;
    use helix_core::syntax::Configuration as LangConfig;
    use helix_term::health::TsFeature;

    /// Get the list of languages that support a particular tree-sitter
    /// based feature.
    pub fn ts_lang_support(feat: TsFeature) -> Vec<String> {
        let queries_dir = path::ts_queries();

        find_files(&queries_dir, feat.runtime_filename())
            .iter()
            .map(|f| {
                // .../helix/runtime/queries/python/highlights.scm
                let tail = f.strip_prefix(&queries_dir).unwrap(); // python/highlights.scm
                let lang = tail.components().next().unwrap(); // python
                lang.as_os_str().to_string_lossy().to_string()
            })
            .collect()
    }

    /// Get the list of languages that have any form of tree-sitter
    /// queries defined in the runtime directory.
    pub fn langs_with_ts_queries() -> Vec<String> {
        std::fs::read_dir(path::ts_queries())
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                entry
                    .file_type()
                    .ok()?
                    .is_dir()
                    .then(|| entry.file_name().to_string_lossy().to_string())
            })
            .collect()
    }

    // naive implementation, but suffices for our needs
    pub fn find_files(dir: &Path, filename: &str) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_dir() {
                    Some(find_files(&path, filename))
                } else {
                    (path.file_name()?.to_string_lossy() == filename).then(|| vec![path])
                }
            })
            .flatten()
            .collect()
    }

    pub fn lang_config() -> LangConfig {
        let bytes = std::fs::read(path::lang_config()).unwrap();
        toml::from_slice(&bytes).unwrap()
    }
}

pub mod md_gen {
    use crate::DynError;

    use crate::helpers;
    use crate::path;
    use helix_term::commands::Category;
    use helix_term::{
        commands::{Req, TYPABLE_COMMAND_LIST},
        health::TsFeature,
        keymap::{self, KeyTrie, KeyTrieNode, Keymap, MappableCommand},
    };
    use helix_view::{document::Mode, input::KeyEvent};
    use std::collections::HashSet;
    use std::fs;
    use std::fs::read_to_string;

    pub const TYPABLE_COMMANDS_MD_OUTPUT: &str = "typable-cmd.md";
    pub const COMMANDS_MD_OUTPUT: &str = "keymap.md";
    pub const LANG_SUPPORT_MD_OUTPUT: &str = "lang-support.md";

    fn md_table_heading(cols: &[String]) -> String {
        let mut header = String::new();
        header += &md_table_row(cols);
        header += &md_table_row(&vec!["---".to_string(); cols.len()]);
        header
    }

    fn md_heading(heading: &str, level: usize) -> String {
        let mut string = "#".repeat(level);
        string += " ";
        string += heading;
        string += "\n";
        string
    }

    fn md_table_row(cols: &[String]) -> String {
        format!("| {} |\n", cols.join(" | "))
    }

    /// Markdown monospace. Escapes ` and | to make sure it works for keys.
    fn md_mono(s: &str) -> String {
        if s.contains('`') {
            format!("`` {} ``", s)
        } else if s.contains('|') {
            format!("<code>{}</code>", s.replace('|', "&#124;"))
        } else {
            format!("`{}`", s)
        }
    }

    /// Markdown anchors for partial imports.
    /// See <https://rust-lang.github.io/mdBook/format/mdbook.html#including-portions-of-a-file>
    fn md_anchor(md: &str, label: &str) -> String {
        format!("ANCHOR: {0}\n{1}\nANCHOR_END: {0}\n", label, md)
    }

    fn md_paragraph(md: &str) -> String {
        format!("{}\n\n", md)
    }

    /// Markdown representing a key event
    fn md_key(key: &KeyEvent) -> String {
        md_mono(&key.to_string())
    }

    /// Markdown representing a list of key events
    fn md_keys(keys: &[KeyEvent]) -> String {
        keys.iter().map(md_key).collect::<Vec<_>>().join(", ")
    }

    /// Markdown link to section in same doc
    fn md_link(name: &str) -> String {
        let lower = name.to_ascii_lowercase();
        let link = lower.replace(' ', "-").replace('(', "").replace(')', "");
        format!("[{}](#{})", name, link)
    }

    /// Markdown description for entering a mode
    fn md_enter_mode(node: &KeyTrieNode) -> String {
        let name = node.name();
        let lower = name.to_ascii_lowercase();
        let link = lower.replace(' ', "-").replace('(', "").replace(')', "");
        match node.is_sticky {
            true => format!("Enter sticky [{} mode](#{})", lower, link),
            false => format!("Enter [{} mode](#{})", lower, link),
        }
    }

    /// Markdown string description of a command
    fn md_description(command: &MappableCommand) -> String {
        match command {
            MappableCommand::Typable { .. } => unreachable!(),
            MappableCommand::Static {
                name: _,
                fun: _,
                category: _,
                doc,
                requirements,
            } => {
                let mut description = doc.trim().to_string();
                for req in *requirements {
                    let str = match req {
                        Req::Lsp => " (**LSP**)",
                        Req::TreeSitter => " (**TS**)",
                        Req::Dap => " (**DAP**)",
                    };

                    description.push_str(str)
                }
                description
            }
        }
    }

    /// Markdown table of contents
    fn md_toc(table: &Vec<(String, usize)>) -> String {
        let mut md = String::new();
        for (name, depth) in table {
            let mut line = "  ".repeat(*depth);
            line.push_str("- ");
            line.push_str(&md_link(name));
            line.push('\n');
            md.push_str(&line);
        }

        md
    }

    fn get_mode_description(mode: &str) -> (Option<String>, Option<String>) {
        let path = path::book_modes()
            .join(mode.to_lowercase().replace(' ', "_"))
            .with_extension("md");

        match read_to_string(path) {
            Ok(string) => {
                let mut split = string.splitn(2, "\n\n\n\n");
                (
                    split.next().map(|s| s.trim().to_owned()),
                    split.next().map(|s| s.trim().to_owned()),
                )
            }
            Err(_) => (None, None),
        }
    }

    /// Generate markdown for keymap section, including headings description,
    /// tips, and adding it to table of contents
    fn md_keymap_section(
        name: &str,
        table: &mut Vec<(String, usize)>,
        level: usize,
        inner: &str,
    ) -> String {
        let mut md = String::new();
        md.push_str(&md_heading(name, level + 2));
        table.push((name.to_owned(), level));

        let (desc, tip) = get_mode_description(name);
        if let Some(desc) = desc {
            md.push_str(&md_paragraph(&desc));
        }

        md.push_str(inner);

        if let Some(tip) = tip {
            md.push('\n');
            md.push_str(&md_paragraph(&tip));
        }
        md
    }

    fn gen_keymap(
        keymap: &KeyTrieNode,
        name: &str,
        level: usize,
        commands_handled: &mut HashSet<String>,
        table: &mut Vec<(String, usize)>,
        separate_categories: bool,
    ) -> String {
        let items = unify(keymap);

        if separate_categories {
            let categories = get_categories(&items);
            let mut md = String::new();
            md.push_str(&md_keymap_section(name, table, level, ""));
            for category in categories {
                let iterator = items.iter().filter(|i| i.1.category() == category);
                let category_name = &category.to_string();
                md.push_str(&keymap_section(
                    iterator,
                    category_name,
                    commands_handled,
                    table,
                    level + 1,
                ));
            }
            md
        } else {
            keymap_section(items.iter(), name, commands_handled, table, level)
        }
    }

    fn keymap_section<'a>(
        items: impl Iterator<Item = &'a (Vec<KeyEvent>, &'a KeyTrie)>,
        name: &str,
        commands_handled: &mut HashSet<String>,
        table: &mut Vec<(String, usize)>,
        level: usize,
    ) -> String {
        let mut md = String::new();
        let mut sub_modes = Vec::new();
        let mut inner = String::new();
        inner.push_str(&md_table_heading(&[
            "Key".to_owned(),
            "Description".to_owned(),
            "Command".to_owned(),
        ]));
        for (keys, trie) in items {
            let (description, command) = match trie {
                KeyTrie::Leaf(command) => {
                    commands_handled.insert(command.name().to_owned());
                    (md_description(command), md_mono(command.name()))
                }
                KeyTrie::Sequence(_) => unreachable!(),
                KeyTrie::Node(node) => {
                    sub_modes.push(node);
                    (md_enter_mode(node), "".to_string())
                }
            };
            inner.push_str(&md_table_row(&[md_keys(keys), description, command]));
        }

        md.push_str(&md_keymap_section(name, table, level, &inner));

        for mode in sub_modes {
            // If this mode wasn't added yet
            if !table.iter().any(|i| i.0 == mode.name()) {
                let text = gen_keymap(mode, mode.name(), level + 1, commands_handled, table, false);
                md.push_str(&text);
            }
        }
        md
    }

    /// Get categories present, in the order that they appear. Each category
    /// will only appear once in the result.
    fn get_categories(items: &Vec<(Vec<KeyEvent>, &KeyTrie)>) -> Vec<Category> {
        let mut v = Vec::new();
        let mut categories = HashSet::new();
        for (_, trie) in items {
            let category = trie.category();
            let new = categories.insert(category);
            if new {
                v.push(category)
            }
        }
        v
    }

    /// Unify keys that have the same result
    fn unify(keymap: &KeyTrieNode) -> Vec<(Vec<KeyEvent>, &KeyTrie)> {
        let mut handled_indexes = HashSet::new();
        let keys = keymap.order();
        let num_keys = keys.len();
        let mut items = Vec::new();
        for i in 0..num_keys {
            if !handled_indexes.contains(&i) {
                handled_indexes.insert(i);
                let key = keys[i];
                let mut v = vec![key];
                let value = keymap.get(&key).unwrap();
                for (j, other_key) in keys.iter().enumerate().skip(i + 1) {
                    let other = keymap.get(other_key).unwrap();
                    if other == value {
                        if let KeyTrie::Node(node) = value {
                            if let KeyTrie::Node(other_node) = other {
                                if other_node.is_sticky != node.is_sticky {
                                    // Don't unify keys if their nodes have different stickyness
                                    continue;
                                }
                            }
                        }
                        handled_indexes.insert(j);
                        v.push(*other_key);
                    }
                }

                items.push((v, value));
            }
        }
        items
    }

    pub fn commands() -> Result<String, DynError> {
        let mut md = String::new();

        let default_keymap = keymap::default::default();

        let modes = [
            (default_keymap.get(&Mode::Normal).unwrap(), "Normal", true),
            (default_keymap.get(&Mode::Insert).unwrap(), "Insert", false),
            (
                &Keymap::new(keymap::default::select_changes()),
                "Select",
                false,
            ),
        ];

        let mut mapped = HashSet::new();
        // table of contents
        let mut table = Vec::new();
        for mode in modes {
            let text = gen_keymap(mode.0, mode.1, 0, &mut mapped, &mut table, mode.2);
            md.push_str(&text);
        }

        let mut unmapped = String::new();
        let name = "Unmapped Commands";

        unmapped.push_str(&md_table_heading(&[
            "Command".to_owned(),
            "Description".to_owned(),
        ]));
        for command in MappableCommand::STATIC_COMMAND_LIST {
            if !mapped.contains(command.name()) {
                unmapped.push_str(&md_table_row(&[
                    md_mono(command.name()),
                    md_description(command),
                ]))
            }
        }

        md.push_str(&md_keymap_section(name, &mut table, 0, &unmapped));

        let toc = md_toc(&table);
        let mut md = md_anchor(&md, "all");
        let toc = md_anchor(&toc, "toc");
        md.push_str(&toc);

        Ok(md)
    }

    pub fn typable_commands() -> Result<String, DynError> {
        let mut md = String::new();
        md.push_str(&md_table_heading(&[
            "Name".to_owned(),
            "Description".to_owned(),
        ]));

        let cmdify = |s: &str| format!("`:{}`", s);

        for cmd in TYPABLE_COMMAND_LIST {
            let names = std::iter::once(&cmd.name)
                .chain(cmd.aliases.iter())
                .map(|a| cmdify(a))
                .collect::<Vec<_>>()
                .join(", ");

            let doc = cmd.doc.replace('\n', "<br>");

            md.push_str(&md_table_row(&[names.to_owned(), doc.to_owned()]));
        }

        Ok(md)
    }

    pub fn lang_features() -> Result<String, DynError> {
        let mut md = String::new();
        let ts_features = TsFeature::all();

        let mut cols = vec!["Language".to_owned()];
        cols.append(
            &mut ts_features
                .iter()
                .map(|t| t.long_title().to_string())
                .collect::<Vec<_>>(),
        );
        cols.push("Default LSP".to_owned());

        md.push_str(&md_table_heading(&cols));
        let config = helpers::lang_config();

        let mut langs = config
            .language
            .iter()
            .map(|l| l.language_id.clone())
            .collect::<Vec<_>>();
        langs.sort_unstable();

        let mut ts_features_to_langs = Vec::new();
        for &feat in ts_features {
            ts_features_to_langs.push((feat, helpers::ts_lang_support(feat)));
        }

        let mut row = Vec::new();
        for lang in langs {
            let lc = config
                .language
                .iter()
                .find(|l| l.language_id == lang)
                .unwrap(); // lang comes from config
            row.push(lc.language_id.clone());

            for (_feat, support_list) in &ts_features_to_langs {
                row.push(
                    if support_list.contains(&lang) {
                        "✓"
                    } else {
                        ""
                    }
                    .to_owned(),
                );
            }
            row.push(
                lc.language_server
                    .as_ref()
                    .map(|s| s.command.clone())
                    .map(|c| md_mono(&c))
                    .unwrap_or_default(),
            );

            md.push_str(&md_table_row(&row));
            row.clear();
        }

        Ok(md)
    }

    pub fn write(filename: &str, data: &str) {
        let error = format!("Could not write to {}", filename);
        let path = path::book_gen().join(filename);
        fs::write(path, data).expect(&error);
    }
}

pub mod path {
    use std::path::{Path, PathBuf};

    pub fn project_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    pub fn book_gen() -> PathBuf {
        project_root().join("book/src/generated/")
    }

    pub fn book_modes() -> PathBuf {
        project_root().join("book/src/modes/")
    }

    pub fn ts_queries() -> PathBuf {
        project_root().join("runtime/queries")
    }

    pub fn lang_config() -> PathBuf {
        project_root().join("languages.toml")
    }
}

pub mod tasks {
    use crate::md_gen;
    use crate::DynError;

    pub fn docgen() -> Result<(), DynError> {
        use md_gen::*;
        write(TYPABLE_COMMANDS_MD_OUTPUT, &typable_commands()?);
        write(COMMANDS_MD_OUTPUT, &commands()?);
        write(LANG_SUPPORT_MD_OUTPUT, &lang_features()?);
        Ok(())
    }

    pub fn query_check() -> Result<(), String> {
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
            let language_name = language.language_id.to_ascii_lowercase();
            let grammar_name = language.grammar.unwrap_or(language.language_id);
            for query_file in query_files {
                let language = get_language(&grammar_name);
                let query_text = read_query(&language_name, query_file);
                if !query_text.is_empty() {
                    if let Ok(language) = language {
                        if let Err(reason) = Query::new(language, &query_text) {
                            return Err(format!(
                                "Failed to parse {} queries for {}: {}",
                                query_file, language_name, reason
                            ));
                        }
                    }
                }
            }
        }

        println!("Query check succeeded");

        Ok(())
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask docgen`.

    Tasks:
        docgen: Generate files to be included in the mdbook output.
        query-check: Check that tree-sitter queries are valid.
"
        );
    }
}

fn main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task {
        None => tasks::print_help(),
        Some(t) => match t.as_str() {
            "docgen" => tasks::docgen()?,
            "query-check" => tasks::query_check()?,
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };
    Ok(())
}
