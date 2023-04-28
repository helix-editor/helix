use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn replace_matching_double_quotes_with_treesitter() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.rs", None),
        (
            helpers::platform_line(r#"#["|]#hello world""#),
            r#"mr"{"#,
            helpers::platform_line(r#"#[{|]#hello world}"#),
        ),
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_matching_double_quotes_with_treesitter() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.rs", None),
        (
            helpers::platform_line(r#"#["|]#hello world""#),
            r#"md""#,
            helpers::platform_line(r#"#[h|]#ello world"#),
        ),
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn replace_matching_single_quotes_with_treesitter() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.rs", None),
        (
            helpers::platform_line(r#"#['|]#h'"#),
            r#"mr'""#,
            helpers::platform_line(r#"#["|]#h""#),
        ),
    )
    .await?;

    Ok(())
}
