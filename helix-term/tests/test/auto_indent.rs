use crate::test::helpers::{platform_line, test_harness::test_with_config, AppBuilder};
use indoc::indoc;

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent_c() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::default().with_file("foo.c", None),
        // switches to append mode?
        (
            platform_line("void foo() {#[|}]#"),
            "i<ret><esc>",
            platform_line(indoc! {"\
                void foo() {
                  #[|\n]#\
                }
            "}),
        ),
    )
    .await
}
