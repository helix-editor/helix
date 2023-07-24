mod validation_context;

use super::{platform_line, AppBuilder, TestApplication, TIMEOUT};
use anyhow::bail;
use crossterm::event::Event;
use helix_core::{
    test::{self, Content},
    Transaction,
};
use helix_view::{input::parse_macro, input::KeyEvent};
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;
use tui::backend::TerminalEventResult;
use validation_context::ValidationContext;

#[macro_export]
macro_rules! test {
    ($config:expr, ($($arg_1:tt)*), ($($arg_2:tt)*), ($($arg_3:tt)*)) => {
        $crate::test::helpers::test_harness::test_with_config($config, (
            $crate::test::helpers::platform_line(&indoc::formatdoc!($($arg_1)*)),
            format!($($arg_2)*),
            $crate::test::helpers::platform_line(&indoc::formatdoc!($($arg_3)*))
        ))
    };
    ($arg_1:tt, $arg_2:tt, $arg_3:tt) => {
        $crate::test!(
            $crate::test::helpers::AppBuilder::default(),
            $arg_1,
            $arg_2,
            $arg_3
        )
    };
}

pub async fn test<T: Into<TestCaseSpec>>(test_case: T) -> anyhow::Result<()> {
    test_with_config(AppBuilder::default(), test_case).await
}

/// Use this for very simple test cases where there is one input
/// document, selection, and sequence of key presses, and you just
/// want to verify the resulting document and selection.
pub async fn test_with_config<T: Into<TestCaseSpec>>(
    app_builder: AppBuilder,
    spec: T,
) -> anyhow::Result<()> {
    let spec = spec.into();
    let input = spec.input.clone();
    let mut test_harness = TestHarness::default().push_test_case(TestCase {
        spec,
        validation_fn: Box::new(|cx| {
            cx.assert_eq_selection();
            cx.assert_eq_text_current();
            cx.assert_app_is_ok();
        }),
    });
    test_harness.app_builder = app_builder;

    let mut active_test_harness = ActiveTestHarness::from(test_harness);

    // replace the initial text with the input text
    let (view, doc) = helix_view::current!(active_test_harness.app.0.editor);
    doc.apply(
        &Transaction::change_by_selection(doc.text(), &doc.selection(view.id).clone(), |_| {
            (0, doc.text().len_chars(), Some((&input.text).into()))
        })
        .with_selection(input.selection),
        view.id,
    );

    active_test_harness.finish().await
}

#[derive(Debug, Default)]
pub struct TestCaseSpec {
    input: Content,
    key_events: Vec<KeyEvent>,
    expected: Content,
}

impl<I, K, O> From<(I, K, O)> for TestCaseSpec
where
    I: AsRef<str>,
    K: AsRef<str>,
    O: AsRef<str>,
{
    fn from((input, keys, expected): (I, K, O)) -> Self {
        Self {
            input: test::print(input.as_ref()).into(),
            key_events: parse_macro(keys.as_ref()).unwrap(),
            expected: test::print(expected.as_ref()).into(),
        }
    }
}

pub struct TestCase {
    spec: TestCaseSpec,
    validation_fn: Box<dyn Fn(ValidationContext)>,
}

impl TestCase {
    pub fn with_keys(mut self, str: &str) -> Self {
        self.spec.key_events = parse_macro(str).unwrap();
        self
    }

    pub fn with_expected_text(mut self, str: &str) -> Self {
        self.spec.expected.text = platform_line(str);
        self
    }

    pub fn with_validation_fn(mut self, f: Box<dyn Fn(ValidationContext)>) -> Self {
        self.validation_fn = f;
        self
    }
}

impl Default for TestCase {
    fn default() -> Self {
        Self {
            spec: Default::default(),
            validation_fn: Box::new(|_| {}),
        }
    }
}

#[derive(Default)]
pub struct TestHarness {
    app_builder: AppBuilder,
    test_cases: Vec<TestCase>,
    should_exit: bool,
}

impl TestHarness {
    pub fn with_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.app_builder = self.app_builder.with_file(path);
        self
    }

    pub fn push_test_case(mut self, test_case: TestCase) -> Self {
        self.test_cases.push(test_case);
        self
    }

    pub fn should_exit(mut self) -> Self {
        self.should_exit = true;
        self
    }

    pub async fn run(self) -> anyhow::Result<()> {
        ActiveTestHarness::from(self).finish().await
    }
}

pub struct ActiveTestHarness {
    pub app: TestApplication,
    event_stream_tx: UnboundedSender<TerminalEventResult>,
    test_cases: Vec<TestCase>,
    should_exit: bool,
}

impl From<TestHarness> for ActiveTestHarness {
    fn from(test_harness: TestHarness) -> Self {
        let (app, tx) = test_harness.app_builder.build().unwrap();
        Self {
            app,
            event_stream_tx: tx,
            test_cases: test_harness.test_cases,
            should_exit: test_harness.should_exit,
        }
    }
}

impl ActiveTestHarness {
    pub async fn finish(mut self) -> anyhow::Result<()> {
        for (input_index, test_case) in self.test_cases.iter().enumerate() {
            // TEMP: event_loop call will otherwise stall
            if test_case.spec.key_events.is_empty() {
                continue;
            }

            for key_event in test_case.spec.key_events.iter() {
                let key = Event::Key((*key_event).into());
                self.event_stream_tx.send(Ok(key))?;
            }

            let app_exited = self.app.tick().await;

            if app_exited {
                if input_index < self.test_cases.len() - 1 {
                    bail!("Application exited before all test cases were run.");
                }

                if !self.should_exit {
                    bail!("Application wasn't expected to exit.");
                }
            }

            (test_case.validation_fn)(ValidationContext {
                spec: &test_case.spec,
                app: &self.app,
            })
        }

        if !self.should_exit {
            // Workaround for sending close event.
            for key_event in parse_macro("<esc>:q!<ret>")?.into_iter() {
                self.event_stream_tx
                    .send(Ok(Event::Key(key_event.into())))?;
            }

            tokio::time::timeout(TIMEOUT, self.app.tick()).await?;
        }

        // Close
        {
            let close_errs = self.app.close().await;

            if close_errs.is_empty() {
                return Ok(());
            }

            for err in close_errs {
                log::error!("Close error: {}", err);
            }

            bail!("Error closing app");
        }
    }
}
