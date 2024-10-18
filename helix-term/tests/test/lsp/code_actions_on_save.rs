use helix_term::application::Application;
use std::{
    io::Read,
    path::{Path, PathBuf},
};

use super::*;

// Give time to send textDocument/didOpen notification
const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

// Gopls does not use native line endings so set default line ending
// to LF to avoid issues in Windows tests.
const DEFAULT_LINE_ENDING: helix_view::editor::LineEndingConfig =
    helix_view::editor::LineEndingConfig::LF;

// Check that we have gopls available while also allowing
// for gopls to initialize
fn assert_gopls(app: &Application, path: &Path) {
    let doc = app.editor.document_by_path(path);
    let mut ls = None;
    let mut initialized = false;
    if let Some(doc) = doc {
        for _ in 0..10 {
            ls = doc.language_servers().find(|s| s.name() == "gopls");

            if let Some(gopls) = ls {
                if gopls.is_initialized() {
                    initialized = true;
                    break;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    assert!(
        doc.is_some(),
        "doc not found {:?} in {:?}",
        path,
        app.editor
            .documents
            .iter()
            .filter_map(|(_, d)| d.path())
            .collect::<Vec<&PathBuf>>()
    );
    assert!(ls.is_some(), "gopls language server not found");
    assert!(initialized, "gopls language server not initialized in time");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_organize_imports_go() -> anyhow::Result<()> {
    let lang_conf = indoc! {r#"
            [[language]]
            name = "go"
            code-actions-on-save = [{ code-action = "source.organizeImports", enabled = true }]
            indent = { tab-width = 4, unit = " " }
        "#};

    let text = indoc! {r#"
            #[p|]#ackage main

            import "fmt"

            import "path"

            func main() {
             fmt.Println("a")
                path.Join("b")
            }
        "#};

    let dir = tempfile::Builder::new().tempdir()?;
    let mut file = tempfile::Builder::new().suffix(".go").tempfile_in(&dir)?;
    let mut app = helpers::AppBuilder::new()
        .with_config(Config {
            editor: helix_view::editor::Config {
                idle_timeout: IDLE_TIMEOUT,
                default_line_ending: DEFAULT_LINE_ENDING,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_lang_loader(helpers::test_syntax_loader(Some(lang_conf.into())))
        .with_file(file.path(), None)
        .with_input_text(text)
        .build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                None,
                Some(&|app| {
                    assert_gopls(app, file.path());
                }),
            ),
            (Some(":w<ret>"), None),
        ],
        false,
    )
    .await?;

    assert_file_has_content(
        &mut file,
        "package main\n\nimport (\n\t\"fmt\"\n\t\"path\"\n)\n\nfunc main() {\n\tfmt.Println(\"a\")\n\tpath.Join(\"b\")\n}\n"
    )?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_organize_imports_go_write_all_quit() -> anyhow::Result<()> {
    let lang_conf = indoc! {r#"
            [[language]]
            name = "go"
            code-actions-on-save = [{ code-action = "source.organizeImports", enabled = true }]
        "#};

    let text = indoc! {r#"
            #[p|]#ackage main

            import "path"
            import "fmt"

            func main() {
             fmt.Println("a")
                path.Join("b")
            }
        "#};

    let dir = tempfile::Builder::new().tempdir()?;
    let mut file1 = tempfile::Builder::new().suffix(".go").tempfile_in(&dir)?;
    let mut file2 = tempfile::Builder::new().suffix(".go").tempfile_in(&dir)?;
    let mut app = helpers::AppBuilder::new()
        .with_config(Config {
            editor: helix_view::editor::Config {
                idle_timeout: IDLE_TIMEOUT,
                default_line_ending: DEFAULT_LINE_ENDING,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_lang_loader(helpers::test_syntax_loader(Some(lang_conf.into())))
        .with_file(file1.path(), None)
        .with_input_text(text)
        .build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                Some(&format!(
                    ":o {}<ret>ipackage main<ret>import \"fmt\"<ret>func test()<ret><esc>",
                    file2.path().to_string_lossy(),
                )),
                None,
            ),
            (
                None,
                Some(&|app| {
                    assert_gopls(app, file1.path());
                    assert_gopls(app, file2.path());
                }),
            ),
            (Some(":wqa<ret>"), None),
        ],
        true,
    )
    .await?;

    assert_file_has_content(
        &mut file1,
        "package main\n\nimport (\n\t\"fmt\"\n\t\"path\"\n)\n\nfunc main() {\n\tfmt.Println(\"a\")\n\tpath.Join(\"b\")\n}\n",
    )?;

    assert_file_has_content(&mut file2, "package main\n\nfunc test()\n")?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_code_action_go() -> anyhow::Result<()> {
    let lang_conf = indoc! {r#"
            [[language]]
            name = "go"
            code-actions-on-save = [{ code-action = "source.invalid", enabled = true }]
        "#};

    let text = indoc! {r#"
            #[p|]#ackage main

            import "fmt"

            import "path"

            func main() {
                fmt.Println("a")
                path.Join("b")
            }
        "#};

    let dir = tempfile::Builder::new().tempdir()?;
    let mut file = tempfile::Builder::new().suffix(".go").tempfile_in(&dir)?;
    let mut app = helpers::AppBuilder::new()
        .with_config(Config {
            editor: helix_view::editor::Config {
                idle_timeout: IDLE_TIMEOUT,
                default_line_ending: DEFAULT_LINE_ENDING,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_lang_loader(helpers::test_syntax_loader(Some(lang_conf.into())))
        .with_file(file.path(), None)
        .with_input_text(text)
        .build()?;

    test_key_sequences(
        &mut app,
        vec![
            (
                None,
                Some(&|app| {
                    assert_gopls(app, file.path());
                }),
            ),
            (
                Some(":w<ret>"),
                Some(&|app| {
                    assert!(!app.editor.is_err(), "error: {:?}", app.editor.get_status());
                }),
            ),
        ],
        false,
    )
    .await?;

    reload_file(&mut file).unwrap();
    let mut file_content = String::new();
    file.as_file_mut().read_to_string(&mut file_content)?;

    assert_file_has_content(
        &mut file,
        "package main\n\nimport \"fmt\"\n\nimport \"path\"\n\nfunc main() {\n\tfmt.Println(\"a\")\n\tpath.Join(\"b\")\n}\n",
    )?;

    Ok(())
}
