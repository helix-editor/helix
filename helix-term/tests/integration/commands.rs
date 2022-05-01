use helix_core::diagnostic::Severity;
use helix_term::application::Application;

use super::*;

#[tokio::test]
async fn test_write_quit_fail() -> anyhow::Result<()> {
    test_key_sequence(
        &mut Application::new(
            Args {
                files: vec![(PathBuf::from("/foo"), Position::default())],
                ..Default::default()
            },
            Config::default(),
        )?,
        "ihello<esc>:wq<ret>",
        Some(&|app| {
            assert_eq!(&Severity::Error, app.editor.get_status().unwrap().1);
        }),
        None,
    )
    .await?;

    Ok(())
}
