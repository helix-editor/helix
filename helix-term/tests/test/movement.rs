use super::*;

#[tokio::test]
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
#[tokio::test]
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

/// Ensure the very initial cursor in an opened file is the width of
/// the first grapheme
#[tokio::test]
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

#[tokio::test]
async fn cursor_position_append_eof() -> anyhow::Result<()> {
    // Selection is fowards
    test((
        "#[foo|]#",
        "abar<esc>",
        helpers::platform_line("#[foobar|]#\n").as_ref(),
    ))
    .await?;

    // Selection is backwards
    test((
        "#[|foo]#",
        "abar<esc>",
        helpers::platform_line("#[foobar|]#\n").as_ref(),
    ))
    .await?;

    Ok(())
}

#[tokio::test]
async fn select_mode_tree_sitter_next_function_is_union_of_objects() -> anyhow::Result<()> {
    test_with_config(
        Args {
            files: vec![(PathBuf::from("foo.rs"), Position::default())],
            ..Default::default()
        },
        Config::default(),
        helpers::test_syntax_conf(None),
        (
            helpers::platform_line(indoc! {"\
                #[/|]#// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
            "})
            .as_ref(),
            "]fv]f",
            helpers::platform_line(indoc! {"\
                /// Increments
                #[fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }|]#
            "})
            .as_ref(),
        ),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn select_mode_tree_sitter_prev_function_unselects_object() -> anyhow::Result<()> {
    test_with_config(
        Args {
            files: vec![(PathBuf::from("foo.rs"), Position::default())],
            ..Default::default()
        },
        Config::default(),
        helpers::test_syntax_conf(None),
        (
            helpers::platform_line(indoc! {"\
                /// Increments
                #[fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }|]#
            "})
            .as_ref(),
            "v[f",
            helpers::platform_line(indoc! {"\
                /// Increments
                #[fn inc(x: usize) -> usize { x + 1 }|]#
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
            "})
            .as_ref(),
        ),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn select_mode_tree_sitter_prev_function_goes_backwards_to_object() -> anyhow::Result<()> {
    // Note: the anchor stays put and the head moves back.
    test_with_config(
        Args {
            files: vec![(PathBuf::from("foo.rs"), Position::default())],
            ..Default::default()
        },
        Config::default(),
        helpers::test_syntax_conf(None),
        (
            helpers::platform_line(indoc! {"\
                /// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                #[fn ident(x: usize) -> usize { x }|]#
            "})
            .as_ref(),
            "v[f",
            helpers::platform_line(indoc! {"\
                /// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                #[|fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                ]#fn ident(x: usize) -> usize { x }
            "})
            .as_ref(),
        ),
    )
    .await?;

    test_with_config(
        Args {
            files: vec![(PathBuf::from("foo.rs"), Position::default())],
            ..Default::default()
        },
        Config::default(),
        helpers::test_syntax_conf(None),
        (
            helpers::platform_line(indoc! {"\
                /// Increments
                fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                #[fn ident(x: usize) -> usize { x }|]#
            "})
            .as_ref(),
            "v[f[f",
            helpers::platform_line(indoc! {"\
                /// Increments
                #[|fn inc(x: usize) -> usize { x + 1 }
                /// Decrements
                fn dec(x: usize) -> usize { x - 1 }
                /// Identity
                ]#fn ident(x: usize) -> usize { x }
            "})
            .as_ref(),
        ),
    )
    .await?;

    Ok(())
}
