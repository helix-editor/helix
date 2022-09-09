use super::*;

use helix_term::application::Application;

#[tokio::test]
async fn test_history_completion() -> anyhow::Result<()> {
    test_key_sequence(
        &mut Application::new(Args::default(), Config::default())?,
        Some(":asdf<ret>:theme d<C-n><tab>"),
        Some(&|app| {
            assert!(!app.editor.is_err());
        }),
        false,
    )
    .await?;

    Ok(())
}
