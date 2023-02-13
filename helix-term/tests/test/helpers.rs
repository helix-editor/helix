use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::bail;
use crossterm::event::{Event, KeyEvent};
use helix_core::{diagnostic::Severity, test, Selection, Transaction};
use helix_term::{application::Application, args::Args, config::Config};
use helix_view::{doc, input::parse_macro, Editor};
use tempfile::NamedTempFile;
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
    should_exit: bool,
) -> anyhow::Result<()> {
    test_key_sequences(app, vec![(in_keys, test_fn)], should_exit).await
}

#[allow(clippy::type_complexity)]
pub async fn test_key_sequences(
    app: &mut Application,
    inputs: Vec<(Option<&str>, Option<&dyn Fn(&Application)>)>,
    should_exit: bool,
) -> anyhow::Result<()> {
    const TIMEOUT: Duration = Duration::from_millis(500);
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut rx_stream = UnboundedReceiverStream::new(rx);
    let num_inputs = inputs.len();

    for (i, (in_keys, test_fn)) in inputs.into_iter().enumerate() {
        if let Some(in_keys) = in_keys {
            for key_event in parse_macro(in_keys)?.into_iter() {
                let key = Event::Key(KeyEvent::from(key_event));
                log::trace!("sending key: {:?}", key);
                tx.send(Ok(key))?;
            }
        }

        let app_exited = !app.event_loop_until_idle(&mut rx_stream).await;

        // the app should not exit from any test until the last one
        if i < num_inputs - 1 && app_exited {
            bail!("application exited before test function could run");
        }

        // verify if it exited on the last iteration if it should have and
        // the inverse
        if i == num_inputs - 1 && app_exited != should_exit {
            bail!("expected app to exit: {} != {}", should_exit, app_exited);
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

    let errs = app.close().await;

    if !errs.is_empty() {
        log::error!("Errors closing app");

        for err in errs {
            log::error!("{}", err);
        }

        bail!("Error closing app");
    }

    Ok(())
}

pub async fn test_key_sequence_with_input_text<T: Into<TestCase>>(
    app: Option<Application>,
    test_case: T,
    test_fn: &dyn Fn(&Application),
    should_exit: bool,
) -> anyhow::Result<()> {
    let test_case = test_case.into();
    let mut app = match app {
        Some(app) => app,
        None => Application::new(Args::default(), test_config(), test_syntax_conf(None))?,
    };

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
    args: Args,
    config: Config,
    syn_conf: helix_core::syntax::Configuration,
    test_case: T,
) -> anyhow::Result<()> {
    let test_case = test_case.into();
    let app = Application::new(args, config, syn_conf)?;

    test_key_sequence_with_input_text(
        Some(app),
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
    test_with_config(
        Args::default(),
        test_config(),
        test_syntax_conf(None),
        test_case,
    )
    .await
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

/// Generates a config with defaults more suitable for integration tests
pub fn test_config() -> Config {
    let mut config = Config::default();
    config.editor.lsp.enable = false;
    config
}

/// Replaces all LF chars with the system's appropriate line feed
/// character, and if one doesn't exist already, appends the system's
/// appropriate line ending to the end of a string.
pub fn platform_line(input: &str) -> String {
    let line_end = helix_core::DEFAULT_LINE_ENDING.as_str();

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
            config: Config::default(),
            syn_conf: test_syntax_conf(None),
            input: None,
        }
    }
}

impl AppBuilder {
    pub fn new() -> Self {
        AppBuilder::default()
    }

    pub fn with_file<P: Into<PathBuf>>(
        mut self,
        path: P,
        pos: Option<helix_core::Position>,
    ) -> Self {
        self.args.files.push((path.into(), pos.unwrap_or_default()));
        self
    }

    // Remove this attribute once `with_config` is used in a test:
    #[allow(dead_code)]
    pub fn with_config(mut self, config: Config) -> Self {
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

    pub fn build(self) -> anyhow::Result<Application> {
        let mut app = Application::new(self.args, self.config, self.syn_conf)?;

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

pub async fn run_event_loop_until_idle(app: &mut Application) {
    let (_, rx) = tokio::sync::mpsc::unbounded_channel();
    let mut rx_stream = UnboundedReceiverStream::new(rx);
    app.event_loop_until_idle(&mut rx_stream).await;
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
