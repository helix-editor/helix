use crate::{test::helpers::AppBuilder, test_case};

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent_c() -> anyhow::Result<()> {
    test_case!(
        AppBuilder::default().with_file("foo.c", None),
        // switches to append mode?
        ("void foo() {{#[|}}]#"),
        ("i<ret><esc>"),
        ("
            void foo() {{
              #[|\n]#
            }}
        ")
    )
    .await
}
