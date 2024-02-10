use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn insert_mode_cursor_position() -> anyhow::Result<()> {
    test(TestCase {
        in_text: String::new(),
        in_selection: Selection::single(0, 0),
        in_keys: "i".into(),
        out_text: String::new(),
        out_selection: Selection::single(0, 0),
    })
    .await?;

    test(("#[\n|]#", "i", "#[|\n]#")).await?;
    test(("#[\n|]#", "i<esc>", "#[|\n]#")).await?;
    test(("#[\n|]#", "i<esc>i", "#[|\n]#")).await?;

    Ok(())
}

/// Range direction is preserved when escaping insert mode to normal
#[tokio::test(flavor = "multi_thread")]
async fn insert_to_normal_mode_cursor_position() -> anyhow::Result<()> {
    test(("#[f|]#oo\n", "vll<A-;><esc>", "#[|foo]#\n")).await?;
    test((
        indoc! {"\
                #[f|]#oo
                #(b|)#ar"
        },
        "vll<A-;><esc>",
        indoc! {"\
                #[|foo]#
                #(|bar)#"
        },
    ))
    .await?;

    test((
        indoc! {"\
                #[f|]#oo
                #(b|)#ar"
        },
        "a",
        indoc! {"\
                #[fo|]#o
                #(ba|)#r"
        },
    ))
    .await?;

    test((
        indoc! {"\
                #[f|]#oo
                #(b|)#ar"
        },
        "a<esc>",
        indoc! {"\
                #[f|]#oo
                #(b|)#ar"
        },
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn surround_by_character() -> anyhow::Result<()> {
    // Only pairs matching the passed character count
    test((
        "(so [many {go#[o|]#d} text] here)",
        "mi{",
        "(so [many {#[good|]#} text] here)",
    ))
    .await?;
    test((
        "(so [many {go#[o|]#d} text] here)",
        "mi[",
        "(so [#[many {good} text|]#] here)",
    ))
    .await?;
    test((
        "(so [many {go#[o|]#d} text] here)",
        "mi(",
        "(#[so [many {good} text] here|]#)",
    ))
    .await?;

    // Works with characters that aren't pairs too
    test((
        "'so 'many 'go#[o|]#d' text' here'",
        "mi'",
        "'so 'many '#[good|]#' text' here'",
    ))
    .await?;
    test((
        "'so 'many 'go#[o|]#d' text' here'",
        "2mi'",
        "'so '#[many 'good' text|]#' here'",
    ))
    .await?;
    test((
        "'so \"many 'go#[o|]#d' text\" here'",
        "mi\"",
        "'so \"#[many 'good' text|]#\" here'",
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn surround_inside_pair() -> anyhow::Result<()> {
    // Works at first character of buffer
    // TODO: Adjust test when opening pair failure is fixed
    test(("#[(|]#something)", "mim", "#[(|]#something)")).await?;

    // Inside a valid pair selects pair
    test(("some (#[t|]#ext) here", "mim", "some (#[text|]#) here")).await?;

    // On pair character selects pair
    // TODO: Opening pair character is a known failure case that needs addressing
    // test(("some #[(|]#text) here", "mim", "some (#[text|]#) here")).await?;
    test(("some (text#[)|]# here", "mim", "some (#[text|]#) here")).await?;

    // No valid pair does nothing
    test(("so#[m|]#e (text) here", "mim", "so#[m|]#e (text) here")).await?;

    // Count skips to outer pairs
    test((
        "(so (many (go#[o|]#d) text) here)",
        "1mim",
        "(so (many (#[good|]#) text) here)",
    ))
    .await?;
    test((
        "(so (many (go#[o|]#d) text) here)",
        "2mim",
        "(so (#[many (good) text|]#) here)",
    ))
    .await?;
    test((
        "(so (many (go#[o|]#d) text) here)",
        "3mim",
        "(#[so (many (good) text) here|]#)",
    ))
    .await?;

    // Matching pairs outside selection don't match
    test((
        "((so)((many) go#[o|]#d (text))(here))",
        "mim",
        "((so)(#[(many) good (text)|]#)(here))",
    ))
    .await?;
    test((
        "((so)((many) go#[o|]#d (text))(here))",
        "2mim",
        "(#[(so)((many) good (text))(here)|]#)",
    ))
    .await?;

    // Works with mixed braces
    test((
        "(so [many {go#[o|]#d} text] here)",
        "mim",
        "(so [many {#[good|]#} text] here)",
    ))
    .await?;
    test((
        "(so [many {go#[o|]#d} text] here)",
        "2mim",
        "(so [#[many {good} text|]#] here)",
    ))
    .await?;
    test((
        "(so [many {go#[o|]#d} text] here)",
        "3mim",
        "(#[so [many {good} text] here|]#)",
    ))
    .await?;

    // Selection direction is preserved
    test((
        "(so [many {go#[|od]#} text] here)",
        "mim",
        "(so [many {#[|good]#} text] here)",
    ))
    .await?;
    test((
        "(so [many {go#[|od]#} text] here)",
        "2mim",
        "(so [#[|many {good} text]#] here)",
    ))
    .await?;
    test((
        "(so [many {go#[|od]#} text] here)",
        "3mim",
        "(#[|so [many {good} text] here]#)",
    ))
    .await?;

    // Only pairs outside of full selection range are considered
    test((
        "(so (many (go#[od) |]#text) here)",
        "mim",
        "(so (#[many (good) text|]#) here)",
    ))
    .await?;
    test((
        "(so (many#[ (go|]#od) text) here)",
        "mim",
        "(so (#[many (good) text|]#) here)",
    ))
    .await?;
    test((
        "(so#[ (many (go|]#od) text) here)",
        "mim",
        "(#[so (many (good) text) here|]#)",
    ))
    .await?;
    test((
        "(so (many (go#[od) text) |]#here)",
        "mim",
        "(#[so (many (good) text) here|]#)",
    ))
    .await?;

    // Works with multiple cursors
    test((
        "(so (many (good) text) #[he|]#re\nso (many (good) text) #(|he)#re)",
        "mim",
        "(#[so (many (good) text) here\nso (many (good) text) here|]#)",
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn surround_around_pair() -> anyhow::Result<()> {
    // Works at first character of buffer
    // TODO: Adjust test when opening pair failure is fixed
    test(("#[(|]#something)", "mam", "#[(|]#something)")).await?;

    // Inside a valid pair selects pair
    test(("some (#[t|]#ext) here", "mam", "some #[(text)|]# here")).await?;

    // On pair character selects pair
    // TODO: Opening pair character is a known failure case that needs addressing
    // test(("some #[(|]#text) here", "mam", "some #[(text)|]# here")).await?;
    test(("some (text#[)|]# here", "mam", "some #[(text)|]# here")).await?;

    // No valid pair does nothing
    test(("so#[m|]#e (text) here", "mam", "so#[m|]#e (text) here")).await?;

    // Count skips to outer pairs
    test((
        "(so (many (go#[o|]#d) text) here)",
        "1mam",
        "(so (many #[(good)|]# text) here)",
    ))
    .await?;
    test((
        "(so (many (go#[o|]#d) text) here)",
        "2mam",
        "(so #[(many (good) text)|]# here)",
    ))
    .await?;
    test((
        "(so (many (go#[o|]#d) text) here)",
        "3mam",
        "#[(so (many (good) text) here)|]#",
    ))
    .await?;

    // Matching pairs outside selection don't match
    test((
        "((so)((many) go#[o|]#d (text))(here))",
        "mam",
        "((so)#[((many) good (text))|]#(here))",
    ))
    .await?;
    test((
        "((so)((many) go#[o|]#d (text))(here))",
        "2mam",
        "#[((so)((many) good (text))(here))|]#",
    ))
    .await?;

    // Works with mixed braces
    test((
        "(so [many {go#[o|]#d} text] here)",
        "mam",
        "(so [many #[{good}|]# text] here)",
    ))
    .await?;
    test((
        "(so [many {go#[o|]#d} text] here)",
        "2mam",
        "(so #[[many {good} text]|]# here)",
    ))
    .await?;
    test((
        "(so [many {go#[o|]#d} text] here)",
        "3mam",
        "#[(so [many {good} text] here)|]#",
    ))
    .await?;

    // Selection direction is preserved
    test((
        "(so [many {go#[|od]#} text] here)",
        "mam",
        "(so [many #[|{good}]# text] here)",
    ))
    .await?;
    test((
        "(so [many {go#[|od]#} text] here)",
        "2mam",
        "(so #[|[many {good} text]]# here)",
    ))
    .await?;
    test((
        "(so [many {go#[|od]#} text] here)",
        "3mam",
        "#[|(so [many {good} text] here)]#",
    ))
    .await?;

    // Only pairs outside of full selection range are considered
    test((
        "(so (many (go#[od) |]#text) here)",
        "mam",
        "(so #[(many (good) text)|]# here)",
    ))
    .await?;
    test((
        "(so (many#[ (go|]#od) text) here)",
        "mam",
        "(so #[(many (good) text)|]# here)",
    ))
    .await?;
    test((
        "(so#[ (many (go|]#od) text) here)",
        "mam",
        "#[(so (many (good) text) here)|]#",
    ))
    .await?;
    test((
        "(so (many (go#[od) text) |]#here)",
        "mam",
        "#[(so (many (good) text) here)|]#",
    ))
    .await?;

    // Works with multiple cursors
    test((
        "(so (many (good) text) #[he|]#re\nso (many (good) text) #(|he)#re)",
        "mam",
        "#[(so (many (good) text) here\nso (many (good) text) here)|]#",
    ))
    .await?;

    Ok(())
}

/// Ensure the very initial cursor in an opened file is the width of
/// the first grapheme
#[tokio::test(flavor = "multi_thread")]
async fn cursor_position_newly_opened_file() -> anyhow::Result<()> {
    let test = |content: &str, expected_sel: Selection| -> anyhow::Result<()> {
        let file = helpers::temp_file_with_contents(content)?;
        let mut app = helpers::AppBuilder::new()
            .with_file(file.path(), None)
            .build()?;

        let (view, doc) = helix_view::current!(app.editor);
        let sel = doc.selection(view.id).clone();
        assert_eq!(expected_sel, sel);

        Ok(())
    };

    test("foo", Selection::single(0, 1))?;
    test("👨‍👩‍👧‍👦 foo", Selection::single(0, 7))?;
    test("", Selection::single(0, 0))?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn cursor_position_append_eof() -> anyhow::Result<()> {
    // Selection is forwards
    test((
        "#[foo|]#",
        "abar<esc>",
        helpers::platform_line("#[foobar|]#\n"),
    ))
    .await?;

    // Selection is backwards
    test((
        "#[|foo]#",
        "abar<esc>",
        helpers::platform_line("#[foobar|]#\n"),
    ))
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn select_mode_tree_sitter_next_function_is_union_of_objects() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.rs", None),
        (
            helpers::platform_line(indoc! {"\
                #[/|]#// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
            "}),
            "]fv]f",
            helpers::platform_line(indoc! {"\
                /// Increments
                #[fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }|]#
            "}),
        ),
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn select_mode_tree_sitter_prev_function_unselects_object() -> anyhow::Result<()> {
    test_with_config(
        AppBuilder::new().with_file("foo.rs", None),
        (
            helpers::platform_line(indoc! {"\
                /// Increments
                #[fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }|]#
            "}),
            "v[f",
            helpers::platform_line(indoc! {"\
                /// Increments
                #[fn inc(x: usize) -> usize { x + 1 }|]#
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
            "}),
        ),
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn select_mode_tree_sitter_prev_function_goes_backwards_to_object() -> anyhow::Result<()> {
    // Note: the anchor stays put and the head moves back.
    test_with_config(
        AppBuilder::new().with_file("foo.rs", None),
        (
            helpers::platform_line(indoc! {"\
                /// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                #[fn ident(x: usize) -> usize { x }|]#
            "}),
            "v[f",
            helpers::platform_line(indoc! {"\
                /// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                #[|fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                ]#fn ident(x: usize) -> usize { x }
            "}),
        ),
    )
    .await?;

    test_with_config(
        AppBuilder::new().with_file("foo.rs", None),
        (
            helpers::platform_line(indoc! {"\
                /// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                #[fn ident(x: usize) -> usize { x }|]#
            "}),
            "v[f[f",
            helpers::platform_line(indoc! {"\
                /// Increments
                #[|fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                ]#fn ident(x: usize) -> usize { x }
            "}),
        ),
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn find_char_line_ending() -> anyhow::Result<()> {
    test((
        helpers::platform_line(indoc! {
            "\
            one
            #[|t]#wo
            three"
        }),
        "T<ret>gll2f<ret>",
        helpers::platform_line(indoc! {
            "\
            one
            two#[
            |]#three"
        }),
    ))
    .await?;

    test((
        helpers::platform_line(indoc! {
            "\
            #[|o]#ne
            two
            three"
        }),
        "f<ret>2t<ret>ghT<ret>F<ret>",
        helpers::platform_line(indoc! {
            "\
            one#[|
            t]#wo
            three"
        }),
    ))
    .await?;

    Ok(())
}
