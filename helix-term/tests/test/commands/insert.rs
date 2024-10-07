use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn insert_newline_trim_trailing_whitespace() -> anyhow::Result<()> {
    // Trailing whitespace is trimmed.
    test((
        indoc! {"\
            hello·······#[|
            ]#world
            "}
        .replace('·', " "),
        "i<ret>",
        indoc! {"\
            hello
            #[|
            ]#world
            "}
        .replace('·', " "),
    ))
    .await?;

    // Whitespace that would become trailing is trimmed too.
    test((
        indoc! {"\
            hello········#[|w]#orld
            "}
        .replace('·', " "),
        "i<ret>",
        indoc! {"\
            hello
            #[|w]#orld
            "}
        .replace('·', " "),
    ))
    .await?;

    // Only whitespace before the cursor is trimmed.
    test((
        indoc! {"\
            hello········#[|·]#····world
            "}
        .replace('·', " "),
        "i<ret>",
        indoc! {"\
            hello
            #[|·]#····world
            "}
        .replace('·', " "),
    ))
    .await?;

    Ok(())
}
