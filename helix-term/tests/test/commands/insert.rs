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

/// NOTE: Language is set to markdown to check if the indentation is correct for the new line
#[tokio::test(flavor = "multi_thread")]
async fn test_open_above() -> anyhow::Result<()> {
    // `O` is pressed in the first line
    test((
        indoc! {"Helix #[is|]# cool"},
        ":lang markdown<ret>O",
        indoc! {"\
            #[\n|]#
            Helix is cool
        "},
    ))
    .await?;

    // `O` is pressed in the first line, but the current line has some indentation
    test((
        indoc! {"\
            ··This line has 2 spaces in front of it#[\n|]#
        "}
        .replace('·', " "),
        ":lang markdown<ret>Oa",
        indoc! {"\
            ··a#[\n|]#
            ··This line has 2 spaces in front of it
        "}
        .replace('·', " "),
    ))
    .await?;

    // `O` is pressed but *not* in the first line
    test((
        indoc! {"\
            I use
            b#[t|]#w.
        "},
        ":lang markdown<ret>Oarch",
        indoc! {"\
            I use
            arch#[\n|]#
            btw.
        "},
    ))
    .await?;

    // `O` is pressed but *not* in the first line and the line has some indentation
    test((
        indoc! {"\
            I use
            ····b#[t|]#w.
        "}
        .replace("·", " "),
        ":lang markdown<ret>Ohelix",
        indoc! {"\
            I use
            ····helix#[\n|]#
            ····btw.
        "}
        .replace("·", " "),
    ))
    .await?;

    Ok(())
}

/// NOTE: To make the `open_above` comment-aware, we're setting the language for each test to rust.
#[tokio::test(flavor = "multi_thread")]
async fn test_open_above_with_comments() -> anyhow::Result<()> {
    // `O` is pressed in the first line inside a line comment
    test((
        indoc! {"// a commen#[t|]#"},
        ":lang rust<ret>O",
        indoc! {"\
            // #[\n|]#
            // a comment
        "},
    ))
    .await?;

    // `O` is pressed in the first line inside a line comment, but with indentation
    test((
        indoc! {"····// a comm#[e|]#nt"}.replace("·", " "),
        ":lang rust<ret>O",
        indoc! {"\
            ····// #[\n|]#
            ····// a comment
        "}
        .replace("·", " "),
    ))
    .await?;

    // `O` is pressed but not in the first line but inside a line comment
    test((
        indoc! {"\
            fn main() { }
            // yeetus deletus#[\n|]#
        "},
        ":lang rust<ret>O",
        indoc! {"\
            fn main() { }
            // #[\n|]#
            // yeetus deletus
        "},
    ))
    .await?;

    // `O` is pressed but not in the first line but inside a line comment and with indentation
    test((
        indoc! {"\
            fn main() { }
            ····// yeetus deletus#[\n|]#
        "}
        .replace("·", " "),
        ":lang rust<ret>O",
        indoc! {"\
            fn main() { }
            ····// #[\n|]#
            ····// yeetus deletus
        "}
        .replace("·", " "),
    ))
    .await?;

    Ok(())
}
