use helix_core::{
    diagnostic::Severity,
    indent::{treesitter_indent_for_pos, IndentStyle},
    syntax::{Configuration, IndentationConfiguration, LanguageConfiguration, Loader},
    Syntax,
};
use once_cell::sync::OnceCell;
use std::path::PathBuf;

#[test]
fn test_treesitter_indent_rust() {
    test_treesitter_indent("rust.rs", "source.rust");
}
#[test]
fn test_treesitter_indent_rust_2() {
    test_treesitter_indent("indent.rs", "source.rust");
    // TODO Use commands.rs as indentation test.
    // Currently this fails because we can't align the parameters of a closure yet
    // test_treesitter_indent("commands.rs", "source.rust");
}

fn test_treesitter_indent(file_name: &str, lang_scope: &str) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data/indent");
    path.push(file_name);
    let file = std::fs::File::open(path).unwrap();
    let doc = ropey::Rope::from_reader(file).unwrap();

    let loader = Loader::new(Configuration {
        language: vec![LanguageConfiguration {
            scope: "source.rust".to_string(),
            file_types: vec!["rs".to_string()],
            shebangs: vec![],
            language_id: "Rust".to_string(),
            highlight_config: OnceCell::new(),
            config: None,
            //
            injection_regex: None,
            roots: vec![],
            comment_token: None,
            auto_format: false,
            diagnostic_severity: Severity::Warning,
            grammar: None,
            language_server: None,
            indent: Some(IndentationConfiguration {
                tab_width: 4,
                unit: String::from("    "),
            }),
            indent_query: OnceCell::new(),
            textobject_query: OnceCell::new(),
            debugger: None,
            auto_pairs: None,
        }],
    });

    // set runtime path so we can find the queries
    let mut runtime = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    runtime.push("../runtime");
    std::env::set_var("HELIX_RUNTIME", runtime.to_str().unwrap());

    let language_config = loader.language_config_for_scope(lang_scope).unwrap();
    let highlight_config = language_config.highlight_config(&[]).unwrap();
    let syntax = Syntax::new(&doc, highlight_config, std::sync::Arc::new(loader));
    let indent_query = language_config.indent_query().unwrap();
    let text = doc.slice(..);

    for i in 0..doc.len_lines() {
        let line = text.line(i);
        if let Some(pos) = helix_core::find_first_non_whitespace_char(line) {
            let suggested_indent = treesitter_indent_for_pos(
                indent_query,
                &syntax,
                &IndentStyle::Spaces(4),
                text,
                i,
                text.line_to_char(i) + pos,
                false,
            )
            .unwrap();
            assert!(
                line.get_slice(..pos).map_or(false, |s| s == suggested_indent),
                "Wrong indentation on line {}:\n\"{}\" (original line)\n\"{}\" (suggested indentation)\n",
                i+1,
                line.slice(..line.len_chars()-1),
                suggested_indent,
            );
        }
    }
}
