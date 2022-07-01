use super::*;

#[tokio::test]
async fn auto_indent_c() -> anyhow::Result<()> {
    test_with_config(
        Args {
            files: vec![(PathBuf::from("foo.c"), Position::default())],
            ..Default::default()
        },
        Config::default(),
        // switches to append mode?
        (
            helpers::platform_line("void foo() {#[|}]#").as_ref(),
            "i<ret><esc>",
            helpers::platform_line(indoc! {"\
                void foo() {
                  #[|\n]#\
                }
            "})
            .as_ref(),
        ),
    )
    .await?;

    Ok(())
}
