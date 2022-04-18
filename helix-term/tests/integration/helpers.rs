use std::io::Write;

use crossterm::event::{Event, KeyEvent};
use helix_core::{test, Selection, Transaction};
use helix_term::{application::Application, args::Args, config::Config};
use helix_view::{doc, input::parse_macro};

#[derive(Clone, Debug)]
pub struct TestCase {
    pub in_text: String,
    pub in_selection: Selection,
    pub in_keys: String,
    pub out_text: String,
    pub out_selection: Selection,
}

impl<S: Into<String>> From<(S, S, S)> for TestCase {
    fn from((input, keys, output): (S, S, S)) -> Self {
        let (in_text, in_selection) = test::print(&input.into());
        let (out_text, out_selection) = test::print(&output.into());

        TestCase {
            in_text,
            in_selection,
            in_keys: keys.into(),
            out_text,
            out_selection,
        }
    }
}

pub fn test_key_sequence<T: Into<TestCase>>(
    app: Option<Application>,
    test_case: T,
    test_fn: &dyn Fn(&mut Application),
) -> anyhow::Result<()> {
    let test_case = test_case.into();
    let mut app =
        app.unwrap_or_else(|| Application::new(Args::default(), Config::default()).unwrap());

    let (view, doc) = helix_view::current!(app.editor);
    let sel = doc.selection(view.id).clone();

    // replace the initial text with the input text
    doc.apply(
        &Transaction::change_by_selection(&doc.text(), &sel, |_| {
            (0, doc.text().len_chars(), Some((&test_case.in_text).into()))
        })
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
pub fn test_key_sequence_text_result<T: Into<TestCase>>(
    args: Args,
    config: Config,
    test_case: T,
) -> anyhow::Result<()> {
    let test_case = test_case.into();
    let app = Application::new(args, config).unwrap();

    test_key_sequence(Some(app), test_case.clone(), &|app| {
        let doc = doc!(app.editor);
        assert_eq!(&test_case.out_text, doc.text());

        let mut selections: Vec<_> = doc.selections().values().cloned().collect();
        assert_eq!(1, selections.len());

        let sel = selections.pop().unwrap();
        assert_eq!(test_case.out_selection, sel);
    })?;

    Ok(())
}

pub fn temp_file_with_contents<S: AsRef<str>>(content: S) -> tempfile::NamedTempFile {
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    temp_file
        .as_file_mut()
        .write_all(content.as_ref().as_bytes())
        .unwrap();
    temp_file.flush().unwrap();
    temp_file.as_file_mut().sync_all().unwrap();
    temp_file
}
