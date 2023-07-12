use std::time::Duration;

use anyhow::bail;
use crossterm::event::{Event, KeyEvent};
use helix_core::{test, Selection, Transaction};
use helix_view::{current_ref, doc, input::parse_macro};
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::{AppBuilder, TestApplication};

#[derive(Clone, Debug)]
pub struct TestCase {
    pub in_text: String,
    pub in_selection: Selection,
    pub in_keys: String,
    pub out_text: String,
    pub out_selection: Selection,
}

impl<I, K, O> From<(I, K, O)> for TestCase
where
    I: AsRef<str>,
    K: Into<String>,
    O: AsRef<str>,
{
    fn from((input, keys, output): (I, K, O)) -> Self {
        let (in_text, in_selection) = test::print(input.as_ref());
        let (out_text, out_selection) = test::print(output.as_ref());

        TestCase {
            in_text,
            in_selection,
            in_keys: keys.into(),
            out_text,
            out_selection,
        }
    }
}

#[macro_export]
macro_rules! test_case {
    (($($arg_1:tt)*), ($($arg_2:tt)*), ($($arg_3:tt)*)) => {
        $crate::test::helpers::test_harness::test((
            $crate::test::helpers::platform_line(&indoc::formatdoc!($($arg_1)*)),
            format!($($arg_2)*),
            $crate::test::helpers::platform_line(&indoc::formatdoc!($($arg_3)*))
        ))
    };
}

pub async fn test<T: Into<TestCase>>(test_case: T) -> anyhow::Result<()> {
    test_with_config(AppBuilder::default(), test_case).await
}

/// Use this for very simple test cases where there is one input
/// document, selection, and sequence of key presses, and you just
/// want to verify the resulting document and selection.
pub async fn test_with_config<T: Into<TestCase>>(
    app_builder: AppBuilder,
    test_case: T,
) -> anyhow::Result<()> {
    let test_case = test_case.into();

    test_key_sequence_with_input_text(
        app_builder,
        test_case.clone(),
        &|app| {
            let doc = doc!(app.editor);
            assert_eq!(&test_case.out_text, doc.text());

            let mut selections: Vec<_> = doc.selections().values().cloned().collect();
            assert_eq!(1, selections.len());

            let sel = selections.pop().unwrap();
            assert_eq!(test_case.out_selection, sel);
        },
        false,
    )
    .await
}

pub async fn test_key_sequence_with_input_text<T: Into<TestCase>>(
    app_builder: AppBuilder,
    test_case: T,
    test_fn: &dyn Fn(&TestApplication),
    should_exit: bool,
) -> anyhow::Result<()> {
    let test_case = test_case.into();
    let mut app = app_builder.build()?;
    let (view, doc) = helix_view::current!(app.editor);
    let sel = doc.selection(view.id).clone();

    // replace the initial text with the input text
    let transaction = Transaction::change_by_selection(doc.text(), &sel, |_| {
        (0, doc.text().len_chars(), Some((&test_case.in_text).into()))
    })
    .with_selection(test_case.in_selection.clone());

    doc.apply(&transaction, view.id);

    test_key_sequences(
        &mut app,
        &[(Some(&test_case.in_keys), Some(test_fn))],
        should_exit,
    )
    .await
}

#[allow(clippy::type_complexity)]
pub async fn test_key_sequences(
    app: &mut TestApplication,
    inputs: &[(Option<&str>, Option<&dyn Fn(&TestApplication)>)],
    should_exit: bool,
) -> anyhow::Result<()> {
    const TIMEOUT: Duration = Duration::from_millis(500);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut rx_stream = UnboundedReceiverStream::new(rx);

    for (input_index, (in_keys, test_fn)) in inputs.iter().enumerate() {
        let (view, doc) = current_ref!(app.editor);
        let state = test::plain(doc.text().slice(..), doc.selection(view.id));

        log::debug!("executing test with document state:\n\n-----\n\n{}", state);

        if let Some(in_keys) = in_keys {
            for key_event in parse_macro(in_keys)?.into_iter() {
                let key = Event::Key(KeyEvent::from(key_event));
                log::trace!("sending key: {:?}", key);
                tx.send(Ok(key))?;
            }
        }

        let app_exited = !app.event_loop(&mut rx_stream).await;

        if !app_exited {
            let (view, doc) = current_ref!(app.editor);
            let state = test::plain(doc.text().slice(..), doc.selection(view.id));

            log::debug!(
                "finished running test with document state:\n\n-----\n\n{}",
                state
            );
        }

        if app_exited {
            if input_index < inputs.len() - 1 {
                bail!("Application exited before all test functions could run");
            }

            if !should_exit {
                bail!("Application wans't expected not to exit.");
            }
        }

        if let Some(test) = test_fn {
            test(app);
        };
    }

    if !should_exit {
        for key_event in parse_macro("<esc>:q!<ret>")?.into_iter() {
            tx.send(Ok(Event::Key(KeyEvent::from(key_event))))?;
        }

        let event_loop = app.event_loop(&mut rx_stream);
        tokio::time::timeout(TIMEOUT, event_loop).await?;
    }

    let close_errs = app.close().await;

    if close_errs.is_empty() {
        return Ok(());
    }

    for err in close_errs {
        log::error!("Close error: {}", err);
    }

    bail!("Error closing app");
}
