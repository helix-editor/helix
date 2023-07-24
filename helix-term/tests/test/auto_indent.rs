use crate::{test, test::helpers::AppBuilder};

#[tokio::test(flavor = "multi_thread")]
async fn auto_indent_c() -> anyhow::Result<()> {
    test!(
        AppBuilder::default().with_file("foo.c"),
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
