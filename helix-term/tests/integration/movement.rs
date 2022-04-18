use helix_term::application::Application;

use super::*;

#[tokio::test]
async fn insert_mode_cursor_position() -> anyhow::Result<()> {
    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        TestCase {
            in_text: String::new(),
            in_selection: Selection::single(0, 0),
            in_keys: "i".into(),
            out_text: String::new(),
            out_selection: Selection::single(0, 0),
        },
    )?;

    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        ("#[\n|]#", "i", "#[|\n]#"),
    )?;

    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        ("#[\n|]#", "i<esc>", "#[|\n]#"),
    )?;

    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        ("#[\n|]#", "i<esc>i", "#[|\n]#"),
    )?;

    Ok(())
}

/// Range direction is preserved when escaping insert mode to normal
#[tokio::test]
async fn insert_to_normal_mode_cursor_position() -> anyhow::Result<()> {
    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        ("#[f|]#oo\n", "vll<A-;><esc>", "#[|foo]#\n"),
    )?;

    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        (
            indoc! {"\
                #[f|]#oo
                #(b|)#ar"
            },
            "vll<A-;><esc>",
            indoc! {"\
                #[|foo]#
                #(|bar)#"
            },
        ),
    )?;

    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        (
            indoc! {"\
                #[f|]#oo
                #(b|)#ar"
            },
            "a",
            indoc! {"\
                #[fo|]#o
                #(ba|)#r"
            },
        ),
    )?;

    test_key_sequence_text_result(
        Args::default(),
        Config::default(),
        (
            indoc! {"\
                #[f|]#oo
                #(b|)#ar"
            },
            "a<esc>",
            indoc! {"\
                #[f|]#oo
                #(b|)#ar"
            },
        ),
    )?;

    Ok(())
}

/// Ensure the very initial cursor in an opened file is the width of
/// the first grapheme
#[tokio::test]
async fn cursor_position_newly_opened_file() -> anyhow::Result<()> {
    let test = |content: &str, expected_sel: Selection| {
        let file = helpers::temp_file_with_contents(content);

        let mut app = Application::new(
            Args {
                files: vec![(file.path().to_path_buf(), Position::default())],
                ..Default::default()
            },
            Config::default(),
        )
        .unwrap();

        let (view, doc) = helix_view::current!(app.editor);
        let sel = doc.selection(view.id).clone();
        assert_eq!(expected_sel, sel);
    };

    test("foo", Selection::single(0, 1));
    test("👨‍👩‍👧‍👦 foo", Selection::single(0, 7));
    test("", Selection::single(0, 0));

    Ok(())
}
