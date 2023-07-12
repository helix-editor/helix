use crate::test::helpers::{test_harness::test_key_sequences, AppBuilder};

#[tokio::test(flavor = "multi_thread")]
async fn test_history_completion() -> anyhow::Result<()> {
    test_key_sequences(
        &mut AppBuilder::default().build()?,
        &[(
            Some(":asdf<ret>:theme d<C-n><tab>"),
            Some(&|app| {
                assert!(!app.editor.is_err());
            }),
        )],
        false,
    )
    .await
}
