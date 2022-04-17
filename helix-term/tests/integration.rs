#[cfg(feature = "integration")]
mod integration {
    mod helpers;

    use std::path::PathBuf;

    use helix_core::{syntax::AutoPairConfig, Position, Selection};
    use helix_term::{args::Args, config::Config};

    use indoc::indoc;

    use self::helpers::*;

    #[tokio::test]
    async fn hello_world() -> anyhow::Result<()> {
        test_key_sequence_text_result(
            Args::default(),
            Config::default(),
            ("#[\n|]#", "ihello world<esc>", "hello world#[|\n]#"),
        )?;

        Ok(())
    }

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

    #[tokio::test]
    async fn auto_pairs_basic() -> anyhow::Result<()> {
        test_key_sequence_text_result(
            Args::default(),
            Config::default(),
            ("#[\n|]#", "i(<esc>", "(#[|)]#\n"),
        )?;

        test_key_sequence_text_result(
            Args::default(),
            Config {
                editor: helix_view::editor::Config {
                    auto_pairs: AutoPairConfig::Enable(false),
                    ..Default::default()
                },
                ..Default::default()
            },
            ("#[\n|]#", "i(<esc>", "(#[|\n]#"),
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn auto_indent_c() -> anyhow::Result<()> {
        test_key_sequence_text_result(
            Args {
                files: vec![(PathBuf::from("foo.c"), Position::default())],
                ..Default::default()
            },
            Config::default(),
            // switches to append mode?
            (
                "void foo() {#[|}]#\n",
                "i<ret><esc>",
                indoc! {"\
                    void foo() {
                      #[|\n]#\
                    }
                "},
            ),
        )?;

        Ok(())
    }
}
