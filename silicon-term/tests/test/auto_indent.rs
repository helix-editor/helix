use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent_c() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.c", None),
        // switches to append mode?
        (
            "void foo() {#[|}]#",
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
