use super::*;

#[tokio::test]
async fn auto_indent_c() -> anyhow::Result<()> {
    test_key_sequence_text_result(
        Args {
            files: vec![(PathBuf::from("foo.c"), Position::default())],
            ..Default::default()
        },
        Config::default(),
        // switches to append mode?
        (
            "void foo() {#[|}]#\n",
            "i<ret><esc>",
            indoc! {"\
                void foo() {
                  #[|\n]#\
                }
            "},
        ),
    )
    .await?;

    Ok(())
}
