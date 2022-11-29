use arc_swap::{access::Map, ArcSwap};
use futures_util::Stream;
use helix_core::{
    diagnostic::{DiagnosticTag, NumberOrString},
    path::get_relative_path,
    pos_at_coords, syntax, Selection,
};
use helix_lsp::{lsp, util::lsp_pos_to_pos, LspProgressMap};
use helix_view::{
    align_view,
    document::DocumentSavedEventResult,
    editor::{ConfigEvent, EditorEvent},
    graphics::Rect,
    theme,
    tree::Layout,
    Align, Editor,
};
use serde_json::json;
use tui::backend::Backend;

use crate::{
    args::Args,
    commands::apply_workspace_edit,
    compositor::{Compositor, Event},
    config::Config,
    job::Jobs,
    keymap::Keymaps,
    ui::{self, overlay::overlayed},
};

use log::{debug, error, warn};
use std::{
    io::{stdin, stdout, Write},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Error};

use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event as CrosstermEvent,
    },
    execute, terminal,
    tty::IsTty,
};
#[cfg(not(windows))]
use {
    signal_hook::{consts::signal, low_level},
    signal_hook_tokio::Signals,
};
#[cfg(windows)]
type Signals = futures_util::stream::Empty<()>;

const LSP_DEADLINE: Duration = Duration::from_millis(16);

#[cfg(not(feature = "integration"))]
use tui::backend::CrosstermBackend;

#[cfg(feature = "integration")]
use tui::backend::TestBackend;

#[cfg(not(feature = "integration"))]
type Terminal = tui::terminal::Terminal<CrosstermBackend<std::io::Stdout>>;

#[cfg(feature = "integration")]
type Terminal = tui::terminal::Terminal<TestBackend>;

pub struct Application {
    compositor: Compositor,
    terminal: Terminal,
    pub editor: Editor,

    config: Arc<ArcSwap<Config>>,

    #[allow(dead_code)]
    theme_loader: Arc<theme::Loader>,
    #[allow(dead_code)]
    syn_loader: Arc<syntax::Loader>,

    signals: Signals,
    jobs: Jobs,
    lsp_progress: LspProgressMap,
    last_render: Instant,
}

#[cfg(feature = "integration")]
fn setup_integration_logging() {
    let level = std::env::var("HELIX_LOG_LEVEL")
        .map(|lvl| lvl.parse().unwrap())
        .unwrap_or(log::LevelFilter::Info);

    // Separate file config so we can include year, month and day in file logs
    let _ = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {} [{}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply();
}

fn restore_term() -> Result<(), Error> {
    let mut stdout = stdout();
    // reset cursor shape
    write!(stdout, "\x1B[0 q")?;
    // Ignore errors on disabling, this might trigger on windows if we call
    // disable without calling enable previously
    let _ = execute!(stdout, DisableMouseCapture);
    execute!(
        stdout,
        DisableBracketedPaste,
        DisableFocusChange,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;
    Ok(())
}

impl Application {
    pub fn new(
        args: Args,
        config: Config,
        syn_loader_conf: syntax::Configuration,
    ) -> Result<Self, Error> {
        #[cfg(feature = "integration")]
        setup_integration_logging();

        use helix_view::editor::Action;

        let theme_loader = std::sync::Arc::new(theme::Loader::new(
            &helix_loader::config_dir(),
            &helix_loader::runtime_dir(),
        ));

        let true_color = config.editor.true_color || crate::true_color();
        let theme = config
            .theme
            .as_ref()
            .and_then(|theme| {
                theme_loader
                    .load(theme)
                    .map_err(|e| {
                        log::warn!("failed to load theme `{}` - {}", theme, e);
                        e
                    })
                    .ok()
                    .filter(|theme| (true_color || theme.is_16_color()))
            })
            .unwrap_or_else(|| theme_loader.default_theme(true_color));

        let syn_loader = std::sync::Arc::new(syntax::Loader::new(syn_loader_conf));

        #[cfg(not(feature = "integration"))]
        let backend = CrosstermBackend::new(stdout());

        #[cfg(feature = "integration")]
        let backend = TestBackend::new(120, 150);

        let terminal = Terminal::new(backend)?;
        let area = terminal.size().expect("couldn't get terminal size");
        let mut compositor = Compositor::new(area);
        let config = Arc::new(ArcSwap::from_pointee(config));
        let mut editor = Editor::new(
            area,
            theme_loader.clone(),
            syn_loader.clone(),
            Box::new(Map::new(Arc::clone(&config), |config: &Config| {
                &config.editor
            })),
        );

        let keys = Box::new(Map::new(Arc::clone(&config), |config: &Config| {
            &config.keys
        }));
        let editor_view = Box::new(ui::EditorView::new(Keymaps::new(keys)));
        compositor.push(editor_view);

        if args.load_tutor {
            let path = helix_loader::runtime_dir().join("tutor");
            editor.open(&path, Action::VerticalSplit)?;
            // Unset path to prevent accidentally saving to the original tutor file.
            doc_mut!(editor).set_path(None)?;
        } else if !args.files.is_empty() {
            let first = &args.files[0].0; // we know it's not empty
            if first.is_dir() {
                std::env::set_current_dir(first).context("set current dir")?;
                editor.new_file(Action::VerticalSplit);
                let picker = ui::file_picker(".".into(), &config.load().editor);
                compositor.push(Box::new(overlayed(picker)));
            } else {
                let nr_of_files = args.files.len();
                for (i, (file, pos)) in args.files.into_iter().enumerate() {
                    if file.is_dir() {
                        return Err(anyhow::anyhow!(
                            "expected a path to file, found a directory. (to open a directory pass it as first argument)"
                        ));
                    } else {
                        // If the user passes in either `--vsplit` or
                        // `--hsplit` as a command line argument, all the given
                        // files will be opened according to the selected
                        // option. If neither of those two arguments are passed
                        // in, just load the files normally.
                        let action = match args.split {
                            _ if i == 0 => Action::VerticalSplit,
                            Some(Layout::Vertical) => Action::VerticalSplit,
                            Some(Layout::Horizontal) => Action::HorizontalSplit,
                            None => Action::Load,
                        };
                        let doc_id = editor
                            .open(&file, action)
                            .context(format!("open '{}'", file.to_string_lossy()))?;
                        // with Action::Load all documents have the same view
                        // NOTE: this isn't necessarily true anymore. If
                        // `--vsplit` or `--hsplit` are used, the file which is
                        // opened last is focused on.
                        let view_id = editor.tree.focus;
                        let doc = doc_mut!(editor, &doc_id);
                        let pos = Selection::point(pos_at_coords(doc.text().slice(..), pos, true));
                        doc.set_selection(view_id, pos);
                    }
                }
                editor.set_status(format!("Loaded {} files.", nr_of_files));
                // align the view to center after all files are loaded,
                // does not affect views without pos since it is at the top
                let (view, doc) = current!(editor);
                align_view(doc, view, Align::Center);
            }
        } else if stdin().is_tty() || cfg!(feature = "integration") {
            editor.new_file(Action::VerticalSplit);
        } else if cfg!(target_os = "macos") {
            // On Linux and Windows, we allow the output of a command to be piped into the new buffer.
            // This doesn't currently work on macOS because of the following issue:
            //   https://github.com/crossterm-rs/crossterm/issues/500
            anyhow::bail!("Piping into helix-term is currently not supported on macOS");
        } else {
            editor
                .new_file_from_stdin(Action::VerticalSplit)
                .unwrap_or_else(|_| editor.new_file(Action::VerticalSplit));
        }

        editor.set_theme(theme);

        #[cfg(windows)]
        let signals = futures_util::stream::empty();
        #[cfg(not(windows))]
        let signals = Signals::new([signal::SIGTSTP, signal::SIGCONT, signal::SIGUSR1])
            .context("build signal handler")?;

        let app = Self {
            compositor,
            terminal,
            editor,

            config,

            theme_loader,
            syn_loader,

            signals,
            jobs: Jobs::new(),
            lsp_progress: LspProgressMap::new(),
            last_render: Instant::now(),
        };

        Ok(app)
    }

    #[cfg(feature = "integration")]
    fn render(&mut self) {}

    #[cfg(not(feature = "integration"))]
    fn render(&mut self) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            jobs: &mut self.jobs,
            scroll: None,
        };

        let area = self
            .terminal
            .autoresize()
            .expect("Unable to determine terminal size");

        // TODO: need to recalculate view tree if necessary

        let surface = self.terminal.current_buffer_mut();

        self.compositor.render(area, surface, &mut cx);

        let (pos, kind) = self.compositor.cursor(area, &self.editor);
        let pos = pos.map(|pos| (pos.col as u16, pos.row as u16));
        self.terminal.draw(pos, kind).unwrap();
    }

    pub async fn event_loop<S>(&mut self, input_stream: &mut S)
    where
        S: Stream<Item = crossterm::Result<crossterm::event::Event>> + Unpin,
    {
        self.render();
        self.last_render = Instant::now();

        loop {
            if !self.event_loop_until_idle(input_stream).await {
                break;
            }
        }
    }

    pub async fn event_loop_until_idle<S>(&mut self, input_stream: &mut S) -> bool
    where
        S: Stream<Item = crossterm::Result<crossterm::event::Event>> + Unpin,
    {
        loop {
            if self.editor.should_close() {
                return false;
            }

            use futures_util::StreamExt;

            tokio::select! {
                biased;

                Some(event) = input_stream.next() => {
                    self.handle_terminal_events(event);
                }
                Some(signal) = self.signals.next() => {
                    self.handle_signals(signal).await;
                }
                Some(callback) = self.jobs.futures.next() => {
                    self.jobs.handle_callback(&mut self.editor, &mut self.compositor, callback);
                    self.render();
                }
                Some(callback) = self.jobs.wait_futures.next() => {
                    self.jobs.handle_callback(&mut self.editor, &mut self.compositor, callback);
                    self.render();
                }
                event = self.editor.wait_event() => {
                    let _idle_handled = self.handle_editor_event(event).await;

                    #[cfg(feature = "integration")]
                    {
                        if _idle_handled {
                            return true;
                        }
                    }
                }
            }

            // for integration tests only, reset the idle timer after every
            // event to signal when test events are done processing
            #[cfg(feature = "integration")]
            {
                self.editor.reset_idle_timer();
            }
        }
    }

    pub fn handle_config_events(&mut self, config_event: ConfigEvent) {
        match config_event {
            ConfigEvent::Refresh => self.refresh_config(),

            // Since only the Application can make changes to Editor's config,
            // the Editor must send up a new copy of a modified config so that
            // the Application can apply it.
            ConfigEvent::Update(editor_config) => {
                let mut app_config = (*self.config.load().clone()).clone();
                app_config.editor = *editor_config;
                self.config.store(Arc::new(app_config));
            }
        }

        // Update all the relevant members in the editor after updating
        // the configuration.
        self.editor.refresh_config();
    }

    /// Refresh theme after config change
    fn refresh_theme(&mut self, config: &Config) {
        if let Some(theme) = config.theme.clone() {
            let true_color = self.true_color();
            match self.theme_loader.load(&theme) {
                Ok(theme) => {
                    if true_color || theme.is_16_color() {
                        self.editor.set_theme(theme);
                    } else {
                        self.editor
                            .set_error("theme requires truecolor support, which is not available");
                    }
                }
                Err(err) => {
                    let err_string = format!("failed to load theme `{}` - {}", theme, err);
                    self.editor.set_error(err_string);
                }
            }
        }
    }

    fn refresh_config(&mut self) {
        match Config::load_default() {
            Ok(config) => {
                self.refresh_theme(&config);

                // Store new config
                self.config.store(Arc::new(config));
            }
            Err(err) => {
                self.editor.set_error(err.to_string());
            }
        }
    }

    fn true_color(&self) -> bool {
        self.config.load().editor.true_color || crate::true_color()
    }

    #[cfg(windows)]
    // no signal handling available on windows
    pub async fn handle_signals(&mut self, _signal: ()) {}

    #[cfg(not(windows))]
    pub async fn handle_signals(&mut self, signal: i32) {
        match signal {
            signal::SIGTSTP => {
                // restore cursor
                use helix_view::graphics::CursorKind;
                self.terminal
                    .backend_mut()
                    .show_cursor(CursorKind::Block)
                    .ok();
                restore_term().unwrap();
                low_level::emulate_default_handler(signal::SIGTSTP).unwrap();
            }
            signal::SIGCONT => {
                self.claim_term().await.unwrap();
                // redraw the terminal
                let area = self.terminal.size().expect("couldn't get terminal size");
                self.compositor.resize(area);
                self.terminal.clear().expect("couldn't clear terminal");

                self.render();
            }
            signal::SIGUSR1 => {
                self.refresh_config();
                self.render();
            }
            _ => unreachable!(),
        }
    }

    pub fn handle_idle_timeout(&mut self) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            jobs: &mut self.jobs,
            scroll: None,
        };
        let should_render = self.compositor.handle_event(&Event::IdleTimeout, &mut cx);
        if should_render {
            self.render();
        }
    }

    pub fn handle_document_write(&mut self, doc_save_event: DocumentSavedEventResult) {
        let doc_save_event = match doc_save_event {
            Ok(event) => event,
            Err(err) => {
                self.editor.set_error(err.to_string());
                return;
            }
        };

        let doc = match self.editor.document_mut(doc_save_event.doc_id) {
            None => {
                warn!(
                    "received document saved event for non-existent doc id: {}",
                    doc_save_event.doc_id
                );

                return;
            }
            Some(doc) => doc,
        };

        debug!(
            "document {:?} saved with revision {}",
            doc.path(),
            doc_save_event.revision
        );

        doc.set_last_saved_revision(doc_save_event.revision);

        let lines = doc_save_event.text.len_lines();
        let bytes = doc_save_event.text.len_bytes();

        if doc.path() != Some(&doc_save_event.path) {
            if let Err(err) = doc.set_path(Some(&doc_save_event.path)) {
                log::error!(
                    "error setting path for doc '{:?}': {}",
                    doc.path(),
                    err.to_string(),
                );

                self.editor.set_error(err.to_string());
                return;
            }

            let loader = self.editor.syn_loader.clone();

            // borrowing the same doc again to get around the borrow checker
            let doc = doc_mut!(self.editor, &doc_save_event.doc_id);
            let id = doc.id();
            doc.detect_language(loader);
            let _ = self.editor.refresh_language_server(id);
        }

        // TODO: fix being overwritten by lsp
        self.editor.set_status(format!(
            "'{}' written, {}L {}B",
            get_relative_path(&doc_save_event.path).to_string_lossy(),
            lines,
            bytes
        ));
    }

    #[inline(always)]
    pub async fn handle_editor_event(&mut self, event: EditorEvent) -> bool {
        log::debug!("received editor event: {:?}", event);

        match event {
            EditorEvent::DocumentSaved(event) => {
                self.handle_document_write(event);
                self.render();
            }
            EditorEvent::ConfigEvent(event) => {
                self.handle_config_events(event);
                self.render();
            }
            EditorEvent::LanguageServerMessage((id, call)) => {
                self.handle_language_server_message(call, id).await;
                // limit render calls for fast language server messages
                let last = self.editor.language_servers.incoming.is_empty();

                if last || self.last_render.elapsed() > LSP_DEADLINE {
                    self.render();
                    self.last_render = Instant::now();
                }
            }
            EditorEvent::DebuggerEvent(payload) => {
                let needs_render = self.editor.handle_debugger_message(payload).await;
                if needs_render {
                    self.render();
                }
            }
            EditorEvent::IdleTimer => {
                self.editor.clear_idle_timer();
                self.handle_idle_timeout();

                #[cfg(feature = "integration")]
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn handle_terminal_events(&mut self, event: Result<CrosstermEvent, crossterm::ErrorKind>) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            jobs: &mut self.jobs,
            scroll: None,
        };
        // Handle key events
        let should_redraw = match event.unwrap() {
            CrosstermEvent::Resize(width, height) => {
                self.terminal
                    .resize(Rect::new(0, 0, width, height))
                    .expect("Unable to resize terminal");

                let area = self.terminal.size().expect("couldn't get terminal size");

                self.compositor.resize(area);

                self.compositor
                    .handle_event(&Event::Resize(width, height), &mut cx)
            }
            event => self.compositor.handle_event(&event.into(), &mut cx),
        };

        if should_redraw && !self.editor.should_close() {
            self.render();
        }
    }

    pub async fn handle_language_server_message(
        &mut self,
        call: helix_lsp::Call,
        server_id: usize,
    ) {
        use helix_lsp::{Call, MethodCall, Notification};

        match call {
            Call::Notification(helix_lsp::jsonrpc::Notification { method, params, .. }) => {
                let notification = match Notification::parse(&method, params) {
                    Ok(notification) => notification,
                    Err(err) => {
                        log::error!(
                            "received malformed notification from Language Server: {}",
                            err
                        );
                        return;
                    }
                };

                match notification {
                    Notification::Initialized => {
                        let language_server =
                            match self.editor.language_servers.get_by_id(server_id) {
                                Some(language_server) => language_server,
                                None => {
                                    warn!("can't find language server with id `{}`", server_id);
                                    return;
                                }
                            };

                        // Trigger a workspace/didChangeConfiguration notification after initialization.
                        // This might not be required by the spec but Neovim does this as well, so it's
                        // probably a good idea for compatibility.
                        if let Some(config) = language_server.config() {
                            tokio::spawn(language_server.did_change_configuration(config.clone()));
                        }

                        let docs = self.editor.documents().filter(|doc| {
                            doc.language_server().map(|server| server.id()) == Some(server_id)
                        });

                        // trigger textDocument/didOpen for docs that are already open
                        for doc in docs {
                            let url = match doc.url() {
                                Some(url) => url,
                                None => continue, // skip documents with no path
                            };

                            let language_id =
                                doc.language_id().map(ToOwned::to_owned).unwrap_or_default();

                            tokio::spawn(language_server.text_document_did_open(
                                url,
                                doc.version(),
                                doc.text(),
                                language_id,
                            ));
                        }
                    }
                    Notification::PublishDiagnostics(mut params) => {
                        let path = params.uri.to_file_path().unwrap();
                        let doc = self.editor.document_by_path_mut(&path);

                        if let Some(doc) = doc {
                            let lang_conf = doc.language_config();
                            let text = doc.text();

                            let diagnostics = params
                                .diagnostics
                                .iter()
                                .filter_map(|diagnostic| {
                                    use helix_core::diagnostic::{Diagnostic, Range, Severity::*};
                                    use lsp::DiagnosticSeverity;

                                    let language_server = if let Some(language_server) = doc.language_server() {
                                        language_server
                                    } else {
                                        log::warn!("Discarding diagnostic because language server is not initialized: {:?}", diagnostic);
                                        return None;
                                    };

                                    // TODO: convert inside server
                                    let start = if let Some(start) = lsp_pos_to_pos(
                                        text,
                                        diagnostic.range.start,
                                        language_server.offset_encoding(),
                                    ) {
                                        start
                                    } else {
                                        log::warn!("lsp position out of bounds - {:?}", diagnostic);
                                        return None;
                                    };

                                    let end = if let Some(end) = lsp_pos_to_pos(
                                        text,
                                        diagnostic.range.end,
                                        language_server.offset_encoding(),
                                    ) {
                                        end
                                    } else {
                                        log::warn!("lsp position out of bounds - {:?}", diagnostic);
                                        return None;
                                    };

                                    let severity =
                                        diagnostic.severity.map(|severity| match severity {
                                            DiagnosticSeverity::ERROR => Error,
                                            DiagnosticSeverity::WARNING => Warning,
                                            DiagnosticSeverity::INFORMATION => Info,
                                            DiagnosticSeverity::HINT => Hint,
                                            severity => unreachable!(
                                                "unrecognized diagnostic severity: {:?}",
                                                severity
                                            ),
                                        });

                                    if let Some(lang_conf) = lang_conf {
                                        if let Some(severity) = severity {
                                            if severity < lang_conf.diagnostic_severity {
                                                return None;
                                            }
                                        }
                                    };

                                    let code = match diagnostic.code.clone() {
                                        Some(x) => match x {
                                            lsp::NumberOrString::Number(x) => {
                                                Some(NumberOrString::Number(x))
                                            }
                                            lsp::NumberOrString::String(x) => {
                                                Some(NumberOrString::String(x))
                                            }
                                        },
                                        None => None,
                                    };

                                    let tags = if let Some(ref tags) = diagnostic.tags {
                                        let new_tags = tags.iter().filter_map(|tag| {
                                            match *tag {
                                                lsp::DiagnosticTag::DEPRECATED => Some(DiagnosticTag::Deprecated),
                                                lsp::DiagnosticTag::UNNECESSARY => Some(DiagnosticTag::Unnecessary),
                                                _ => None
                                            }
                                        }).collect();

                                        new_tags
                                    } else {
                                        Vec::new()
                                    };

                                    Some(Diagnostic {
                                        range: Range { start, end },
                                        line: diagnostic.range.start.line as usize,
                                        message: diagnostic.message.clone(),
                                        severity,
                                        code,
                                        tags,
                                        source: diagnostic.source.clone()
                                    })
                                })
                                .collect();

                            doc.set_diagnostics(diagnostics);
                        }

                        // Sort diagnostics first by severity and then by line numbers.
                        // Note: The `lsp::DiagnosticSeverity` enum is already defined in decreasing order
                        params
                            .diagnostics
                            .sort_unstable_by_key(|d| (d.severity, d.range.start));

                        // Insert the original lsp::Diagnostics here because we may have no open document
                        // for diagnosic message and so we can't calculate the exact position.
                        // When using them later in the diagnostics picker, we calculate them on-demand.
                        self.editor
                            .diagnostics
                            .insert(params.uri, params.diagnostics);
                    }
                    Notification::ShowMessage(params) => {
                        log::warn!("unhandled window/showMessage: {:?}", params);
                    }
                    Notification::LogMessage(params) => {
                        log::info!("window/logMessage: {:?}", params);
                    }
                    Notification::ProgressMessage(params)
                        if !self
                            .compositor
                            .has_component(std::any::type_name::<ui::Prompt>()) =>
                    {
                        let editor_view = self
                            .compositor
                            .find::<ui::EditorView>()
                            .expect("expected at least one EditorView");
                        let lsp::ProgressParams { token, value } = params;

                        let lsp::ProgressParamsValue::WorkDone(work) = value;
                        let parts = match &work {
                            lsp::WorkDoneProgress::Begin(lsp::WorkDoneProgressBegin {
                                title,
                                message,
                                percentage,
                                ..
                            }) => (Some(title), message, percentage),
                            lsp::WorkDoneProgress::Report(lsp::WorkDoneProgressReport {
                                message,
                                percentage,
                                ..
                            }) => (None, message, percentage),
                            lsp::WorkDoneProgress::End(lsp::WorkDoneProgressEnd { message }) => {
                                if message.is_some() {
                                    (None, message, &None)
                                } else {
                                    self.lsp_progress.end_progress(server_id, &token);
                                    if !self.lsp_progress.is_progressing(server_id) {
                                        editor_view.spinners_mut().get_or_create(server_id).stop();
                                    }
                                    self.editor.clear_status();

                                    // we want to render to clear any leftover spinners or messages
                                    return;
                                }
                            }
                        };

                        let token_d: &dyn std::fmt::Display = match &token {
                            lsp::NumberOrString::Number(n) => n,
                            lsp::NumberOrString::String(s) => s,
                        };

                        let status = match parts {
                            (Some(title), Some(message), Some(percentage)) => {
                                format!("[{}] {}% {} - {}", token_d, percentage, title, message)
                            }
                            (Some(title), None, Some(percentage)) => {
                                format!("[{}] {}% {}", token_d, percentage, title)
                            }
                            (Some(title), Some(message), None) => {
                                format!("[{}] {} - {}", token_d, title, message)
                            }
                            (None, Some(message), Some(percentage)) => {
                                format!("[{}] {}% {}", token_d, percentage, message)
                            }
                            (Some(title), None, None) => {
                                format!("[{}] {}", token_d, title)
                            }
                            (None, Some(message), None) => {
                                format!("[{}] {}", token_d, message)
                            }
                            (None, None, Some(percentage)) => {
                                format!("[{}] {}%", token_d, percentage)
                            }
                            (None, None, None) => format!("[{}]", token_d),
                        };

                        if let lsp::WorkDoneProgress::End(_) = work {
                            self.lsp_progress.end_progress(server_id, &token);
                            if !self.lsp_progress.is_progressing(server_id) {
                                editor_view.spinners_mut().get_or_create(server_id).stop();
                            }
                        } else {
                            self.lsp_progress.update(server_id, token, work);
                        }

                        if self.config.load().editor.lsp.display_messages {
                            self.editor.set_status(status);
                        }
                    }
                    Notification::ProgressMessage(_params) => {
                        // do nothing
                    }
                    Notification::Exit => {
                        self.editor.set_status("Language server exited");

                        // Clear any diagnostics for documents with this server open.
                        let urls: Vec<_> = self
                            .editor
                            .documents_mut()
                            .filter_map(|doc| {
                                if doc.language_server().map(|server| server.id())
                                    == Some(server_id)
                                {
                                    doc.set_diagnostics(Vec::new());
                                    doc.url()
                                } else {
                                    None
                                }
                            })
                            .collect();

                        for url in urls {
                            self.editor.diagnostics.remove(&url);
                        }

                        // Remove the language server from the registry.
                        self.editor.language_servers.remove_by_id(server_id);
                    }
                }
            }
            Call::MethodCall(helix_lsp::jsonrpc::MethodCall {
                method, params, id, ..
            }) => {
                let call = match MethodCall::parse(&method, params) {
                    Ok(call) => call,
                    Err(helix_lsp::Error::Unhandled) => {
                        error!("Language Server: Method not found {}", method);
                        return;
                    }
                    Err(err) => {
                        log::error!(
                            "received malformed method call from Language Server: {}: {}",
                            method,
                            err
                        );
                        return;
                    }
                };

                let reply = match call {
                    MethodCall::WorkDoneProgressCreate(params) => {
                        self.lsp_progress.create(server_id, params.token);

                        let editor_view = self
                            .compositor
                            .find::<ui::EditorView>()
                            .expect("expected at least one EditorView");
                        let spinner = editor_view.spinners_mut().get_or_create(server_id);
                        if spinner.is_stopped() {
                            spinner.start();
                        }

                        Ok(serde_json::Value::Null)
                    }
                    MethodCall::ApplyWorkspaceEdit(params) => {
                        apply_workspace_edit(
                            &mut self.editor,
                            helix_lsp::OffsetEncoding::Utf8,
                            &params.edit,
                        );

                        Ok(json!(lsp::ApplyWorkspaceEditResponse {
                            applied: true,
                            failure_reason: None,
                            failed_change: None,
                        }))
                    }
                    MethodCall::WorkspaceFolders => {
                        let language_server =
                            self.editor.language_servers.get_by_id(server_id).unwrap();

                        Ok(json!(language_server.workspace_folders()))
                    }
                    MethodCall::WorkspaceConfiguration(params) => {
                        let result: Vec<_> = params
                            .items
                            .iter()
                            .map(|item| {
                                let mut config = match &item.scope_uri {
                                    Some(scope) => {
                                        let path = scope.to_file_path().ok()?;
                                        let doc = self.editor.document_by_path(path)?;
                                        doc.language_config()?.config.as_ref()?
                                    }
                                    None => self
                                        .editor
                                        .language_servers
                                        .get_by_id(server_id)
                                        .unwrap()
                                        .config()?,
                                };
                                if let Some(section) = item.section.as_ref() {
                                    for part in section.split('.') {
                                        config = config.get(part)?;
                                    }
                                }
                                Some(config)
                            })
                            .collect();
                        Ok(json!(result))
                    }
                };

                let language_server = match self.editor.language_servers.get_by_id(server_id) {
                    Some(language_server) => language_server,
                    None => {
                        warn!("can't find language server with id `{}`", server_id);
                        return;
                    }
                };

                tokio::spawn(language_server.reply(id, reply));
            }
            Call::Invalid { id } => log::error!("LSP invalid method call id={:?}", id),
        }
    }

    async fn claim_term(&mut self) -> Result<(), Error> {
        use helix_view::graphics::CursorKind;
        terminal::enable_raw_mode()?;
        if self.terminal.cursor_kind() == CursorKind::Hidden {
            self.terminal.backend_mut().hide_cursor().ok();
        }
        let mut stdout = stdout();
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            EnableBracketedPaste,
            EnableFocusChange
        )?;
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
        if self.config.load().editor.mouse {
            execute!(stdout, EnableMouseCapture)?;
        }
        Ok(())
    }

    pub async fn run<S>(&mut self, input_stream: &mut S) -> Result<i32, Error>
    where
        S: Stream<Item = crossterm::Result<crossterm::event::Event>> + Unpin,
    {
        self.claim_term().await?;

        // Exit the alternate screen and disable raw mode before panicking
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // We can't handle errors properly inside this closure.  And it's
            // probably not a good idea to `unwrap()` inside a panic handler.
            // So we just ignore the `Result`.
            let _ = restore_term();
            hook(info);
        }));

        self.event_loop(input_stream).await;

        let close_errs = self.close().await;

        // restore cursor
        use helix_view::graphics::CursorKind;
        self.terminal
            .backend_mut()
            .show_cursor(CursorKind::Block)
            .ok();
        restore_term()?;

        for err in close_errs {
            self.editor.exit_code = 1;
            eprintln!("Error: {}", err);
        }

        Ok(self.editor.exit_code)
    }

    pub async fn close(&mut self) -> Vec<anyhow::Error> {
        // [NOTE] we intentionally do not return early for errors because we
        //        want to try to run as much cleanup as we can, regardless of
        //        errors along the way
        let mut errs = Vec::new();

        if let Err(err) = self
            .jobs
            .finish(&mut self.editor, Some(&mut self.compositor))
            .await
        {
            log::error!("Error executing job: {}", err);
            errs.push(err);
        };

        if let Err(err) = self.editor.flush_writes().await {
            log::error!("Error writing: {}", err);
            errs.push(err);
        }

        if self.editor.close_language_servers(None).await.is_err() {
            log::error!("Timed out waiting for language servers to shutdown");
            errs.push(anyhow::format_err!(
                "Timed out waiting for language servers to shutdown"
            ));
        }

        errs
    }
}
