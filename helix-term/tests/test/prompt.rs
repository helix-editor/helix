use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_history_completion() -> anyhow::Result<()> {
    test_key_sequence(
        &mut AppBuilder::new().build()?,
        Some(":asdf<ret>:theme d<C-n><tab>"),
        Some(&|app| {
            assert!(!app.editor.is_err());
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_history_search() -> anyhow::Result<()> {
    test_key_sequence(
        &mut AppBuilder::new().build()?,
        Some(":show<minus>directory<ret>:show<minus>clipboard<minus>provider<ret>:new<ret>:bc<ret>:sh<up><up><ret>"),
        Some(&|app| {
            assert!(&app
                .editor
                .get_status()
                .unwrap()
                .0
                .starts_with("Current working dir"));
        }),
        false,
    )
    .await?;

    Ok(())
}
