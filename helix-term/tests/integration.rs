#[cfg(feature = "integration")]
mod integration {
    use std::path::PathBuf;

    use helix_core::{syntax::AutoPairConfig, Position, Selection, Tendril, Transaction};
    use helix_term::{application::Application, args::Args, config::Config};
    use helix_view::{current, doc, input::parse_macro};

    use crossterm::event::{Event, KeyEvent};
    use indoc::indoc;

    pub struct TestCase {
        pub in_text: String,
        pub in_selection: Selection,
        pub in_keys: String,
        pub out_text: String,
        pub out_selection: Selection,
    }

    fn test_key_sequence(
        app: Option<Application>,
        test_case: &TestCase,
        test_fn: &dyn Fn(&mut Application),
    ) -> anyhow::Result<()> {
        let mut app =
            app.unwrap_or_else(|| Application::new(Args::default(), Config::default()).unwrap());

        let (view, doc) = current!(app.editor);

        doc.apply(
            &Transaction::insert(
                doc.text(),
                &Selection::single(1, 0),
                Tendril::from(&test_case.in_text),
            )
            .with_selection(test_case.in_selection.clone()),
            view.id,
        );

        let input_keys = parse_macro(&test_case.in_keys)?
            .into_iter()
            .map(|key_event| Event::Key(KeyEvent::from(key_event)));

        for key in input_keys {
            app.handle_terminal_events(Ok(key));
        }

        test_fn(&mut app);

        Ok(())
    }

    /// Use this for very simple test cases where there is one input
    /// document, selection, and sequence of key presses, and you just
    /// want to verify the resulting document and selection.
    fn test_key_sequence_text_result(
        args: Args,
        config: Config,
        test_case: TestCase,
    ) -> anyhow::Result<()> {
        let app = Application::new(args, config).unwrap();

        test_key_sequence(Some(app), &test_case, &|app| {
            let doc = doc!(app.editor);
            assert_eq!(&test_case.out_text, doc.text());

            let mut selections: Vec<_> = doc.selections().values().cloned().collect();
            assert_eq!(1, selections.len());

            let sel = selections.pop().unwrap();
            assert_eq!(test_case.out_selection, sel);
        })?;

        Ok(())
    }

    #[tokio::test]
    async fn hello_world() -> anyhow::Result<()> {
        test_key_sequence_text_result(
            Args::default(),
            Config::default(),
            TestCase {
                in_text: String::new(),
                in_selection: Selection::single(0, 1),
                // TODO: fix incorrect selection on new doc
                in_keys: String::from("ihello world<esc>hl"),
                out_text: String::from("hello world\n"),
                out_selection: Selection::single(11, 12),
            },
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn auto_pairs_basic() -> anyhow::Result<()> {
        test_key_sequence_text_result(
            Args::default(),
            Config::default(),
            TestCase {
                in_text: String::new(),
                in_selection: Selection::single(0, 1),
                in_keys: String::from("i(<esc>hl"),
                out_text: String::from("()\n"),
                out_selection: Selection::single(1, 2),
            },
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
            TestCase {
                in_text: String::new(),
                in_selection: Selection::single(0, 1),
                in_keys: String::from("i(<esc>hl"),
                out_text: String::from("(\n"),
                out_selection: Selection::single(1, 2),
            },
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn auto_indent_rs() -> anyhow::Result<()> {
        test_key_sequence_text_result(
            Args {
                files: vec![(PathBuf::from("foo.c"), Position::default())],
                ..Default::default()
            },
            Config::default(),
            TestCase {
                in_text: String::from("void foo() {}"),
                in_selection: Selection::single(12, 13),
                in_keys: String::from("i<ret><esc>"),
                out_text: String::from(indoc! {r#"
                    void foo() {
                      
                    }
                "#}),
                out_selection: Selection::single(15, 16),
            },
        )?;

        Ok(())
    }
}
