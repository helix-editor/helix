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

#[tokio::test(flavor = "multi_thread")]
async fn insert_newline_continue_line_comment() -> anyhow::Result<()> {
    // `insert_newline` continues a single line comment
    test((
        indoc! {"\
            // Hello world!#[|
            ]#
            "},
        ":lang rust<ret>i<ret>",
        indoc! {"\
            // Hello world!
            // #[|
            ]#
            "},
    ))
    .await?;

    // The comment is not continued if the cursor is before the comment token. (Note that we
    // are entering insert-mode with `I`.)
    test((
        indoc! {"\
            // Hello world!#[|
            ]#
            "},
        ":lang rust<ret>I<ret>",
        indoc! {"\
            \n#[/|]#/ Hello world!
            "},
    ))
    .await?;

    // `insert_newline` again clears the whitespace on the first continued comment and continues
    // the comment again.
    test((
        indoc! {"\
            // Hello world!
            // #[|
            ]#
            "},
        ":lang rust<ret>i<ret>",
        indoc! {"\
            // Hello world!
            //
            // #[|
            ]#
            "},
    ))
    .await?;

    // Line comment continuation and trailing whitespace is also trimmed when using
    // `insert_newline` in the middle of a comment.
    test((
        indoc! {"\
            //·hello····#[|·]#····world
            "}
        .replace('·', " "),
        ":lang rust<ret>i<ret>",
        indoc! {"\
            //·hello
            //·#[|·]#····world
            "}
        .replace('·', " "),
    ))
    .await?;

    Ok(())
}
