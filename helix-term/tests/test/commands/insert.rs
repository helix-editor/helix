use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn change_line_above_comment() -> anyhow::Result<()> {
    // <https://github.com/helix-editor/helix/issues/12570>
    test((
        indoc! {"\
        #[fn main() {}
        |]#// a comment
        "},
        ":lang rust<ret>c",
        indoc! {"\
        #[
        |]#// a comment
        "},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_newline_many_selections() -> anyhow::Result<()> {
    test((
        indoc! {"\
            #(|o)#ne
            #(|t)#wo
            #[|t]#hree
            "},
        "i<ret>",
        indoc! {"\
            \n#(|o)#ne

            #(|t)#wo

            #[|t]#hree
            "},
    ))
    .await?;

    // In this case the global offset that adjusts selections for inserted and deleted text
    // should become negative because more text is deleted than is inserted.
    test((
        indoc! {"\
            #[|🏴‍☠️]#          #(|🏴‍☠️)#          #(|🏴‍☠️)#
            #(|🏴‍☠️)#          #(|🏴‍☠️)#          #(|🏴‍☠️)#
            "},
        "i<ret>",
        indoc! {"\
            \n#[|🏴‍☠️]#
            #(|🏴‍☠️)#
            #(|🏴‍☠️)#

            #(|🏴‍☠️)#
            #(|🏴‍☠️)#
            #(|🏴‍☠️)#
            "},
    ))
    .await?;

    // <https://github.com/helix-editor/helix/issues/12495>
    test((
        indoc! {"\
            id #(|1)#,Item #(|1)#,cost #(|1)#,location #(|1)#
            id #(|2)#,Item #(|2)#,cost #(|2)#,location #(|2)#
            id #(|1)##(|0)#,Item #(|1)##(|0)#,cost #(|1)##(|0)#,location #(|1)##[|0]#"},
        "i<ret>",
        indoc! {"\
            id
            #(|1)#,Item
            #(|1)#,cost
            #(|1)#,location
            #(|1)#
            id
            #(|2)#,Item
            #(|2)#,cost
            #(|2)#,location
            #(|2)#
            id
            #(|1)#
            #(|0)#,Item
            #(|1)#
            #(|0)#,cost
            #(|1)#
            #(|0)#,location
            #(|1)#
            #[|0]#"},
    ))
    .await?;

    // <https://github.com/helix-editor/helix/issues/12461>
    test((
        indoc! {"\
            real R〉 #(||)# 〈real R〉 @ 〈real R〉
            #(||)# 〈real R〉 + 〈ureal R〉 i #(||)# 〈real R〉 - 〈ureal R〉 i
            #(||)# 〈real R〉 + i #(||)# 〈real R〉 - i #(||)# 〈real R〉 〈infnan〉 i
            #(||)# + 〈ureal R〉 i #(||)# - 〈ureal R〉 i
            #(||)# 〈infnan〉 i #(||)# + i #[||]# - i"},
        "i<ret>",
        indoc! {"\
            real R〉
            #(||)# 〈real R〉 @ 〈real R〉

            #(||)# 〈real R〉 + 〈ureal R〉 i
            #(||)# 〈real R〉 - 〈ureal R〉 i

            #(||)# 〈real R〉 + i
            #(||)# 〈real R〉 - i
            #(||)# 〈real R〉 〈infnan〉 i

            #(||)# + 〈ureal R〉 i
            #(||)# - 〈ureal R〉 i

            #(||)# 〈infnan〉 i
            #(||)# + i
            #[||]# - i"},
    ))
    .await?;

    Ok(())
}

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
async fn insert_newline_trim_whitespace_to_previous_selection() -> anyhow::Result<()> {
    test((
        indoc! {"\"#[a|]# #(a|)# #(a|)#\""},
        "c<ret>",
        indoc! {"\"\n#[\n|]##(\n|)##(\"|)#"},
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

    // Comment continuation should work on multiple selections.
    // <https://github.com/helix-editor/helix/issues/12539>
    test((
        indoc! {"\
            ///·Docs#[|·]#
            pub·struct·A;

            ///·Docs#(|·)#
            pub·struct·B;
            "}
        .replace('·', " "),
        ":lang rust<ret>i<ret><ret>",
        indoc! {"\
            ///·Docs
            ///
            ///·#[|·]#
            pub·struct·A;

            ///·Docs
            ///
            ///·#(|·)#
            pub·struct·B;
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

#[tokio::test(flavor = "multi_thread")]
async fn test_open_above_with_multiple_cursors() -> anyhow::Result<()> {
    // the primary cursor is also in the top line
    test((
        indoc! {"#[H|]#elix
            #(i|)#s
            #(c|)#ool"},
        "O",
        indoc! {
            "#[\n|]#
            Helix
            #(\n|)#
            is
            #(\n|)#
            cool
            "
        },
    ))
    .await?;

    // now with some additional indentation
    test((
        indoc! {"····#[H|]#elix
            ····#(i|)#s
            ····#(c|)#ool"}
        .replace("·", " "),
        ":indent-style 4<ret>O",
        indoc! {
            "····#[\n|]#
            ····Helix
            ····#(\n|)#
            ····is
            ····#(\n|)#
            ····cool
            "
        }
        .replace("·", " "),
    ))
    .await?;

    // the first line is within a comment, the second not.
    // However, if we open above, the first newly added line should start within a comment
    // while the other should be a normal line
    test((
        indoc! {"fn main() {
                // #[VIP|]# comment
                l#(e|)#t yes = false;
            }"},
        ":lang rust<ret>O",
        indoc! {"fn main() {
                // #[\n|]#
                // VIP comment
                #(\n|)#
                let yes = false;
            }"},
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_open_below_with_multiple_cursors() -> anyhow::Result<()> {
    // the primary cursor is also in the top line
    test((
        indoc! {"#[H|]#elix
            #(i|)#s
            #(c|)#ool"},
        "o",
        indoc! {"Helix
            #[\n|]#
            is
            #(\n|)#
            cool
            #(\n|)#
            "
        },
    ))
    .await?;

    // now with some additional indentation
    test((
        indoc! {"····#[H|]#elix
            ····#(i|)#s
            ····#(c|)#ool"}
        .replace("·", " "),
        ":indent-style 4<ret>o",
        indoc! {
            "····Helix
            ····#[\n|]#
            ····is
            ····#(\n|)#
            ····cool
            ····#(\n|)#
            "
        }
        .replace("·", " "),
    ))
    .await?;

    // the first line is within a comment, the second not.
    // However, if we open below, the first newly added line should start within a comment
    // while the other should be a normal line
    test((
        indoc! {"fn main() {
                // #[VIP|]# comment
                l#(e|)#t yes = false;
            }"},
        ":lang rust<ret>o",
        indoc! {"fn main() {
                // VIP comment
                // #[\n|]#
                let yes = false;
                #(\n|)#
            }"},
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

#[tokio::test(flavor = "multi_thread")]
async fn try_restore_indent() -> anyhow::Result<()> {
    // Assert that `helix_view::editor::try_restore_indent` handles line endings correctly
    // endings.
    test((
        indoc! {"\
        if true #[|{]#
        }
        "},
        // `try_restore_indent` should remove the indentation when adding a blank line.
        ":lang rust<ret>o<esc>",
        indoc! {"\
        if true {
        #[
        |]#}
        "},
    ))
    .await?;

    Ok(())
}

// Tests being able to jump in insert mode, then undo the write performed by the jump
// https://github.com/helix-editor/helix/issues/13480
#[tokio::test(flavor = "multi_thread")]
async fn test_jump_undo_redo() -> anyhow::Result<()> {
    use helix_core::hashmap;
    use helix_term::keymap;
    use helix_view::document::Mode;

    let mut config = Config::default();
    config.keys.insert(
        Mode::Insert,
        keymap!({"Insert Mode"
            "C-i" => goto_file_start,
            "C-o" => goto_file_end,
        }),
    );

    // Undo
    test_with_config(
        AppBuilder::new().with_config(config.clone()),
        ("#[|]#", "iworld<C-i>Hello, <esc>u", "#[w|]#orld"),
    )
    .await?;

    // Redo
    test_with_config(
        AppBuilder::new().with_config(config),
        (
            "#[|]#",
            "iworld<C-i>Hello, <esc>ui<C-o><esc>U",
            "Hello, #[w|]#orld",
        ),
    )
    .await?;
    Ok(())
}
