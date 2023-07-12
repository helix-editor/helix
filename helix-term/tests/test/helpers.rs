use std::{
    fs::File,
    io::{Read, Write},
    mem::replace,
    path::PathBuf,
    time::Duration,
};

use anyhow::bail;
use crossterm::event::{Event, KeyEvent};
use helix_core::{diagnostic::Severity, test, Selection, Transaction};
use helix_term::{application::Application, args::Args, config::Config, keymap::merge_keys};
use helix_view::{current_ref, doc, editor::LspConfig, input::parse_macro, Editor};
use tempfile::NamedTempFile;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tui::backend::TestBackend;

pub type TestApplication = Application<TestBackend>;

#[derive(Clone, Debug)]
pub struct TestCase {
    pub in_text: String,
    pub in_selection: Selection,
    pub in_keys: String,
    pub out_text: String,
    pub out_selection: Selection,
}

impl<S, R, V> From<(S, R, V)> for TestCase
where
    S: AsRef<str>,
    R: Into<String>,
    V: AsRef<str>,
{
    fn from((input, keys, output): (S, R, V)) -> Self {
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

#[inline]
pub async fn test_key_sequence(
    app: &mut TestApplication,
    in_keys: Option<&str>,
    test_fn: Option<&dyn Fn(&TestApplication)>,
    should_exit: bool,
) -> anyhow::Result<()> {
    test_key_sequences(app, &[(in_keys, test_fn)], should_exit).await
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

    test_key_sequence(
        &mut app,
        Some(&test_case.in_keys),
        Some(test_fn),
        should_exit,
    )
    .await
}

/// Generates language configs that merge in overrides, like a user language
/// config. The argument string must be a raw TOML document.
pub fn test_syntax_conf(overrides: Option<String>) -> helix_core::syntax::Configuration {
    let mut lang = helix_loader::config::default_lang_config();

    if let Some(overrides) = overrides {
        let override_toml = toml::from_str(&overrides).unwrap();
        lang = helix_loader::merge_toml_values(lang, override_toml, 3);
    }

    lang.try_into().unwrap()
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

pub async fn test<T: Into<TestCase>>(test_case: T) -> anyhow::Result<()> {
    test_with_config(AppBuilder::default(), test_case).await
}

pub fn temp_file_with_contents<S: AsRef<str>>(
    content: S,
) -> anyhow::Result<tempfile::NamedTempFile> {
    let mut temp_file = tempfile::NamedTempFile::new()?;

    temp_file
        .as_file_mut()
        .write_all(content.as_ref().as_bytes())?;

    temp_file.flush()?;
    temp_file.as_file_mut().sync_all()?;
    Ok(temp_file)
}

/// Replaces all LF chars with the system's appropriate line feed
/// character, and if one doesn't exist already, appends the system's
/// appropriate line ending to the end of a string.
pub fn platform_line(input: &str) -> String {
    let line_end = helix_core::NATIVE_LINE_ENDING.as_str();

    // we can assume that the source files in this code base will always
    // be LF, so indoc strings will always insert LF
    let mut output = input.replace('\n', line_end);

    if !output.ends_with(line_end) {
        output.push_str(line_end);
    }

    output
}

/// Creates a new temporary file that is set to read only. Useful for
/// testing write failures.
pub fn new_readonly_tempfile() -> anyhow::Result<NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new()?;
    let metadata = file.as_file().metadata()?;
    let mut perms = metadata.permissions();
    perms.set_readonly(true);
    file.as_file_mut().set_permissions(perms)?;
    Ok(file)
}

pub struct AppBuilder {
    args: Args,
    config: Config,
    syn_conf: helix_core::syntax::Configuration,
    input: Option<(String, Selection)>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            args: Args::default(),
            config: Config {
                editor: helix_view::editor::Config {
                    lsp: LspConfig {
                        enable: false,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                keys: helix_term::keymap::default(),
                ..Default::default()
            },
            syn_conf: test_syntax_conf(None),
            input: None,
        }
    }
}

impl AppBuilder {
    pub fn with_file<P: Into<PathBuf>>(
        mut self,
        path: P,
        pos: Option<helix_core::Position>,
    ) -> Self {
        self.args.files.push((path.into(), pos.unwrap_or_default()));
        self
    }

    pub fn with_config(mut self, mut config: Config) -> Self {
        let keys = replace(&mut config.keys, helix_term::keymap::default());
        merge_keys(&mut config.keys, keys);
        self.config = config;
        self
    }

    pub fn with_input_text<S: Into<String>>(mut self, input_text: S) -> Self {
        self.input = Some(test::print(&input_text.into()));
        self
    }

    pub fn with_lang_config(mut self, syn_conf: helix_core::syntax::Configuration) -> Self {
        self.syn_conf = syn_conf;
        self
    }

    pub fn build(self) -> anyhow::Result<TestApplication> {
        // Unwrap will be error error if logging system has been
        // initialized by another test.
        let _ = helix_term::log::setup_logging(std::io::stdout(), None);

        let mut app = TestApplication::new(
            tui::backend::TestBackend::new(120, 150),
            self.args,
            self.config,
            self.syn_conf,
        )?;

        if let Some((text, selection)) = self.input {
            let (view, doc) = helix_view::current!(app.editor);
            let sel = doc.selection(view.id).clone();
            let trans = Transaction::change_by_selection(doc.text(), &sel, |_| {
                (0, doc.text().len_chars(), Some((text.clone()).into()))
            })
            .with_selection(selection);

            // replace the initial text with the input text
            doc.apply(&trans, view.id);
        }

        Ok(app)
    }
}

pub fn assert_file_has_content(file: &mut File, content: &str) -> anyhow::Result<()> {
    file.flush()?;
    file.sync_all()?;

    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;
    assert_eq!(content, file_content);

    Ok(())
}

pub fn assert_status_not_error(editor: &Editor) {
    if let Some((_, sev)) = editor.get_status() {
        assert_ne!(&Severity::Error, sev);
    }
}
