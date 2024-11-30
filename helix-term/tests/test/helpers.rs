use std::{
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

/// Specify how to set up the input text with line feeds
#[derive(Clone, Debug)]
pub enum LineFeedHandling {
    /// Replaces all LF chars with the system's appropriate line feed character,
    /// and if one doesn't exist already, appends the system's appropriate line
    /// ending to the end of a string.
    Native,

    /// Do not modify the input text in any way. What you give is what you test.
    AsIs,
}

impl LineFeedHandling {
    /// Apply the line feed handling to the input string, yielding a set of
    /// resulting texts with the appropriate line feed substitutions.
    pub fn apply(&self, text: &str) -> String {
        let line_end = match self {
            LineFeedHandling::Native => helix_core::NATIVE_LINE_ENDING,
            LineFeedHandling::AsIs => return text.into(),
        }
        .as_str();

        // we can assume that the source files in this code base will always
        // be LF, so indoc strings will always insert LF
        let mut output = text.replace('\n', line_end);

        if !output.ends_with(line_end) {
            output.push_str(line_end);
        }

        output
    }
}

impl Default for LineFeedHandling {
    fn default() -> Self {
        Self::Native
    }
}

#[derive(Clone, Debug)]
pub struct TestCase {
    pub in_text: String,
    pub in_selection: Selection,
    pub in_keys: String,
    pub out_text: String,
    pub out_selection: Selection,

    pub line_feed_handling: LineFeedHandling,
}

impl<S, R, V> From<(S, R, V)> for TestCase
where
    S: Into<String>,
    R: Into<String>,
    V: Into<String>,
{
    fn from((input, keys, output): (S, R, V)) -> Self {
        TestCase::from((input, keys, output, LineFeedHandling::default()))
    }
}

impl<S, R, V> From<(S, R, V, LineFeedHandling)> for TestCase
where
    S: Into<String>,
    R: Into<String>,
    V: Into<String>,
{
    fn from((input, keys, output, line_feed_handling): (S, R, V, LineFeedHandling)) -> Self {
        let (in_text, in_selection) = test::print(&line_feed_handling.apply(&input.into()));
        let (out_text, out_selection) = test::print(&line_feed_handling.apply(&output.into()));

        TestCase {
            in_text,
            in_selection,
            in_keys: keys.into(),
            out_text,
            out_selection,
            line_feed_handling,
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

        let app_exited = !app.event_loop_until_idle(&mut rx_stream).await;

        if !app_exited {
            let (view, doc) = current_ref!(app.editor);
            let state = test::plain(doc.text().slice(..), doc.selection(view.id));

            log::debug!(
                "finished running test with document state:\n\n-----\n\n{}",
                state
            );
        }

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
        None => Application::new(Args::default(), test_config(), test_syntax_loader(None))?,
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

/// Generates language config loader that merge in overrides, like a user language
/// config. The argument string must be a raw TOML document.
pub fn test_syntax_loader(overrides: Option<String>) -> helix_core::syntax::Loader {
    let mut lang = helix_loader::config::default_lang_config();

    if let Some(overrides) = overrides {
        let override_toml = toml::from_str(&overrides).unwrap();
        lang = helix_loader::merge_toml_values(lang, override_toml, 3);
    }

    helix_core::syntax::Loader::new(lang.try_into().unwrap()).unwrap()
}

/// Use this for very simple test cases where there is one input
/// document, selection, and sequence of key presses, and you just
/// want to verify the resulting document and selection.
pub async fn test_with_config<T: Into<TestCase>>(
    app_builder: AppBuilder,
    test_case: T,
) -> anyhow::Result<()> {
    let test_case = test_case.into();
    let app = app_builder.build()?;

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

/// Generates a config with defaults more suitable for integration tests
pub fn test_config() -> Config {
    Config {
        editor: test_editor_config(),
        keys: helix_term::keymap::default(),
        ..Default::default()
    }
}

pub fn test_editor_config() -> helix_view::editor::Config {
    helix_view::editor::Config {
        lsp: LspConfig {
            enable: false,
            ..Default::default()
        },
        ..Default::default()
    }
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

/// Creates a new temporary file in the directory that is set to read only. Useful for
/// testing write failures.
pub fn new_readonly_tempfile_in_dir(
    dir: impl AsRef<std::path::Path>,
) -> anyhow::Result<NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    let metadata = file.as_file().metadata()?;
    let mut perms = metadata.permissions();
    perms.set_readonly(true);
    file.as_file_mut().set_permissions(perms)?;
    Ok(file)
}
pub struct AppBuilder {
    args: Args,
    config: Config,
    syn_loader: helix_core::syntax::Loader,
    input: Option<(String, Selection)>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            args: Args::default(),
            config: test_config(),
            syn_loader: test_syntax_loader(None),
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

    pub fn with_lang_loader(mut self, syn_loader: helix_core::syntax::Loader) -> Self {
        self.syn_loader = syn_loader;
        self
    }

    pub fn build(self) -> anyhow::Result<Application> {
        if let Some(path) = &self.args.working_directory {
            bail!("Changing the working directory to {path:?} is not yet supported for integration tests");
        }

        if let Some((path, _)) = self.args.files.first().filter(|p| p.0.is_dir()) {
            bail!("Having the directory {path:?} in args.files[0] is not yet supported for integration tests");
        }

        let mut app = Application::new(self.args, self.config, self.syn_loader)?;

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

pub fn assert_file_has_content(file: &mut NamedTempFile, content: &str) -> anyhow::Result<()> {
    reload_file(file)?;

    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;
    assert_eq!(file_content, content);

    Ok(())
}

pub fn assert_status_not_error(editor: &Editor) {
    if let Some((_, sev)) = editor.get_status() {
        assert_ne!(&Severity::Error, sev);
    }
}

pub fn reload_file(file: &mut NamedTempFile) -> anyhow::Result<()> {
    let path = file.path();
    let f = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .open(&path)?;
    *file.as_file_mut() = f;
    Ok(())
}
