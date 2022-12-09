use super::*;
use helix_term::application::Application;
use helix_view::{doc, editor};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn words_completion_basic() -> anyhow::Result<()> {
    let mut app = Application::new(
        Args::default(),
        Config {
            editor: editor::Config {
                completion_trigger_len: 1,
                ..editor::Config::default()
            },
            ..Config::default()
        },
        test_syntax_conf(None),
    )?;

    // completion without language server
    test_key_sequences(
        &mut app,
        vec![
            // doc1
            (Some("ihello<esc>"), None),
            // doc2
            (Some(":new<ret>iworld<esc>"), None),
            // doc3
            (Some(":new<ret>"), None),
            // h|
            (Some("ih"), None),
            // next step to allow completion done and <tab> to choose first option `hello` from doc1
            (
                Some("<tab>"),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert_eq!(helpers::platform_line("hello\n"), doc.text().to_string());
                }),
            ),
            // w|
            (Some(" w"), None),
            // next step to allow completion done and <tab> to choose first option `world` from doc2
            (
                Some("<tab>"),
                Some(&|app| {
                    let doc = doc!(app.editor);
                    assert_eq!(
                        helpers::platform_line("hello world\n"),
                        doc.text().to_string()
                    );
                }),
            ),
        ],
        false,
    )
    .await?;

    // TODO completion with language server

    Ok(())
}
