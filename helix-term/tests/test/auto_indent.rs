use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent_c() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.c", None),
        // switches to append mode?
        (
            helpers::platform_line("void foo() {#[|}]#"),
            "i<ret><esc>",
            helpers::platform_line(indoc! {"\
                void foo() {
                  #[|\n]#\
                }
            "}),
        ),
    )
    .await?;

    Ok(())
}
