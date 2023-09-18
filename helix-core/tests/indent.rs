use helix_core::{
    indent::{indent_level_for_line, treesitter_indent_for_pos, IndentStyle},
    syntax::{Configuration, Loader},
    Syntax,
};
use ropey::Rope;
use std::{ops::Range, path::PathBuf, process::Command};

#[test]
fn test_treesitter_indent_rust() {
    standard_treesitter_test("rust.rs", "source.rust");
}

#[test]
fn test_treesitter_indent_cpp() {
    standard_treesitter_test("cpp.cpp", "source.cpp");
}

#[test]
fn test_treesitter_indent_rust_helix() {
    // We pin a specific git revision to prevent unrelated changes from causing the indent tests to fail.
    // Ideally, someone updates this once in a while and fixes any errors that occur.
    let rev = "af382768cdaf89ff547dbd8f644a1bddd90e7c8f";
    let files = Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            "--full-tree",
            rev,
            "helix-term/src",
        ])
        .output()
        .unwrap();
    let files = String::from_utf8(files.stdout).unwrap();

    let ignored_files = vec![
        // Contains many macros that tree-sitter does not parse in a meaningful way and is otherwise not very interesting
        "helix-term/src/health.rs",
    ];

    for file in files.split_whitespace() {
        if ignored_files.contains(&file) {
            continue;
        }
        let ignored_lines: Vec<Range<usize>> = match file {
            "helix-term/src/application.rs" => vec![
                // We can't handle complicated indent rules inside macros (`json!` in this case) since
                // the tree-sitter grammar only parses them as `token_tree` and `identifier` nodes.
                1045..1051,
            ],
            "helix-term/src/commands.rs" => vec![
                // This is broken because of the current handling of `call_expression`
                // (i.e. having an indent query for it but outdenting again in specific cases).
                // The indent query is needed to correctly handle multi-line arguments in function calls
                // inside indented `field_expression` nodes (which occurs fairly often).
                //
                // Once we have the `@indent.always` capture type, it might be possible to just have an indent
                // capture for the `arguments` field of a call expression. That could enable us to correctly
                // handle this.
                2226..2230,
            ],
            "helix-term/src/commands/dap.rs" => vec![
                // Complex `format!` macro
                46..52,
            ],
            "helix-term/src/commands/lsp.rs" => vec![
                // Macro
                624..627,
                // Return type declaration of a closure. `cargo fmt` adds an additional space here,
                // which we cannot (yet) model with our indent queries.
                878..879,
                // Same as in `helix-term/src/commands.rs`
                1335..1343,
            ],
            "helix-term/src/config.rs" => vec![
                // Multiline string
                146..152,
            ],
            "helix-term/src/keymap.rs" => vec![
                // Complex macro (see above)
                456..470,
                // Multiline string without indent
                563..567,
            ],
            "helix-term/src/main.rs" => vec![
                // Multiline string
                44..70,
            ],
            "helix-term/src/ui/completion.rs" => vec![
                // Macro
                218..232,
            ],
            "helix-term/src/ui/editor.rs" => vec![
                // The chained function calls here are not indented, probably because of the comment
                // in between. Since `cargo fmt` doesn't even attempt to format it, there's probably
                // no point in trying to indent this correctly.
                342..350,
            ],
            "helix-term/src/ui/lsp.rs" => vec![
                // Macro
                56..61,
            ],
            "helix-term/src/ui/statusline.rs" => vec![
                // Same as in `helix-term/src/commands.rs`
                436..442,
                450..456,
            ],
            _ => Vec::new(),
        };

        let git_object = rev.to_string() + ":" + file;
        let content = Command::new("git")
            .args(["cat-file", "blob", &git_object])
            .output()
            .unwrap();
        let doc = Rope::from_reader(&mut content.stdout.as_slice()).unwrap();
        test_treesitter_indent(file, doc, "source.rust", ignored_lines);
    }
}

#[test]
fn test_indent_level_for_line_with_spaces() {
    let tab_width: usize = 4;
    let indent_width: usize = 4;

    let line = ropey::Rope::from_str("        Indented with 8 spaces");

    let indent_level = indent_level_for_line(line.slice(0..), tab_width, indent_width);
    assert_eq!(indent_level, 2)
}

#[test]
fn test_indent_level_for_line_with_tabs() {
    let tab_width: usize = 4;
    let indent_width: usize = 4;

    let line = ropey::Rope::from_str("\t\tIndented with 2 tabs");

    let indent_level = indent_level_for_line(line.slice(0..), tab_width, indent_width);
    assert_eq!(indent_level, 2)
}

#[test]
fn test_indent_level_for_line_with_spaces_and_tabs() {
    let tab_width: usize = 4;
    let indent_width: usize = 4;

    let line = ropey::Rope::from_str("   \t \tIndented with mix of spaces and tabs");

    let indent_level = indent_level_for_line(line.slice(0..), tab_width, indent_width);
    assert_eq!(indent_level, 2)
}

fn indent_tests_dir() -> PathBuf {
    let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_dir.push("tests/data/indent");
    test_dir
}

fn indent_test_path(name: &str) -> PathBuf {
    let mut path = indent_tests_dir();
    path.push(name);
    path
}

fn indent_tests_config() -> Configuration {
    let mut config_path = indent_tests_dir();
    config_path.push("languages.toml");
    let config = std::fs::read_to_string(config_path).unwrap();
    toml::from_str(&config).unwrap()
}

fn standard_treesitter_test(file_name: &str, lang_scope: &str) {
    let test_path = indent_test_path(file_name);
    let test_file = std::fs::File::open(test_path).unwrap();
    let doc = ropey::Rope::from_reader(test_file).unwrap();
    test_treesitter_indent(file_name, doc, lang_scope, Vec::new())
}

/// Test that all the lines in the given file are indented as expected.
/// ignored_lines is a list of (1-indexed) line ranges that are excluded from this test.
fn test_treesitter_indent(
    test_name: &str,
    doc: Rope,
    lang_scope: &str,
    ignored_lines: Vec<std::ops::Range<usize>>,
) {
    let loader = Loader::new(indent_tests_config());

    // set runtime path so we can find the queries
    let mut runtime = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    runtime.push("../runtime");
    std::env::set_var("HELIX_RUNTIME", runtime.to_str().unwrap());

    let language_config = loader.language_config_for_scope(lang_scope).unwrap();
    let indent_style = IndentStyle::from_str(&language_config.indent.as_ref().unwrap().unit);
    let highlight_config = language_config.highlight_config(&[]).unwrap();
    let text = doc.slice(..);
    let syntax = Syntax::new(text, highlight_config, std::sync::Arc::new(loader)).unwrap();
    let indent_query = language_config.indent_query().unwrap();

    for i in 0..doc.len_lines() {
        let line = text.line(i);
        if ignored_lines.iter().any(|range| range.contains(&(i + 1))) {
            continue;
        }
        if let Some(pos) = helix_core::find_first_non_whitespace_char(line) {
            let tab_width: usize = 4;
            let suggested_indent = treesitter_indent_for_pos(
                indent_query,
                &syntax,
                &indent_style,
                tab_width,
                indent_style.indent_width(tab_width),
                text,
                i,
                text.line_to_char(i) + pos,
                false,
            )
            .unwrap();
            assert!(
                line.get_slice(..pos).map_or(false, |s| s == suggested_indent),
                "Wrong indentation for file {:?} on line {}:\n\"{}\" (original line)\n\"{}\" (suggested indentation)\n",
                test_name,
                i+1,
                line.slice(..line.len_chars()-1),
                suggested_indent,
            );
        }
    }
}
