use std::{io::Write, time::Duration};

use anyhow::bail;
use crossterm::event::{Event, KeyEvent};
use helix_core::{test, Selection, Transaction};
use helix_term::{application::Application, args::Args, config::Config};
use helix_view::{doc, input::parse_macro};
use tokio_stream::wrappers::UnboundedReceiverStream;

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

#[inline]
pub async fn test_key_sequence(
    app: &mut Application,
    in_keys: Option<&str>,
    test_fn: Option<&dyn Fn(&Application)>,
) -> anyhow::Result<()> {
    test_key_sequences(app, vec![(in_keys, test_fn)]).await
}

pub async fn test_key_sequences(
    app: &mut Application,
    inputs: Vec<(Option<&str>, Option<&dyn Fn(&Application)>)>,
) -> anyhow::Result<()> {
    const TIMEOUT: Duration = Duration::from_millis(500);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut rx_stream = UnboundedReceiverStream::new(rx);

    for (in_keys, test_fn) in inputs {
        if let Some(in_keys) = in_keys {
            for key_event in parse_macro(&in_keys)?.into_iter() {
                tx.send(Ok(Event::Key(KeyEvent::from(key_event))))?;
            }
        }

        if !app.event_loop_until_idle(&mut rx_stream).await {
            bail!("application exited before test function could run");
        }

        if let Some(test) = test_fn {
            test(app);
        };
    }

    for key_event in parse_macro("<esc>:q!<ret>")?.into_iter() {
        tx.send(Ok(Event::Key(KeyEvent::from(key_event))))?;
    }

    let event_loop = app.event_loop(&mut rx_stream);
    tokio::time::timeout(TIMEOUT, event_loop).await?;
    app.close().await?;

    Ok(())
}

pub async fn test_key_sequence_with_input_text<T: Into<TestCase>>(
    app: Option<Application>,
    test_case: T,
    test_fn: &dyn Fn(&Application),
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

    test_key_sequence(&mut app, Some(&test_case.in_keys), Some(test_fn)).await
}

/// Use this for very simple test cases where there is one input
/// document, selection, and sequence of key presses, and you just
/// want to verify the resulting document and selection.
pub async fn test_key_sequence_text_result<T: Into<TestCase>>(
    args: Args,
    config: Config,
    test_case: T,
) -> anyhow::Result<()> {
    let test_case = test_case.into();
    let app = Application::new(args, config).unwrap();

    test_key_sequence_with_input_text(Some(app), test_case.clone(), &|app| {
        let doc = doc!(app.editor);
        assert_eq!(&test_case.out_text, doc.text());

        let mut selections: Vec<_> = doc.selections().values().cloned().collect();
        assert_eq!(1, selections.len());

        let sel = selections.pop().unwrap();
        assert_eq!(test_case.out_selection, sel);
    })
    .await
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
