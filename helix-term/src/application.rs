use helix_core::{
    config::{default_syntax_loader, user_syntax_loader},
    pos_at_coords, syntax, Selection,
};
use helix_dap::{self as dap, Payload, Request};
use helix_lsp::{lsp, util::lsp_pos_to_pos, LspProgressMap};
use helix_view::{editor::Breakpoint, theme, Editor};
use serde_json::json;

use crate::{
    args::Args,
    commands::{align_view, apply_workspace_edit, fetch_stack_trace, Align},
    compositor::Compositor,
    config::Config,
    job::Jobs,
    ui::{self, overlay::overlayed},
};

use log::{error, warn};
use std::{
    io::{stdin, stdout, Write},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Error;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
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


fn default_theme(theme_loader: Arc<theme::Loader>, true_color: bool) -> (String, theme::Theme) {
    if true_color {
        (String::from("default"), theme_loader.default())
    } else {
        (String::from("base16_default"), theme_loader.base16_default())
    }
}

pub struct Application {
    compositor: Compositor,
    editor: Editor,

    // TODO: share an ArcSwap with Editor?
    config: Config,

    #[allow(dead_code)]
    theme_loader: Arc<theme::Loader>,
    #[allow(dead_code)]
    syn_loader: Arc<syntax::Loader>,

    signals: Signals,
    jobs: Jobs,
    lsp_progress: LspProgressMap,
}

impl Application {
    pub fn new(args: Args, mut config: Config) -> Result<Self, Error> {
        use helix_view::editor::Action;
        let mut compositor = Compositor::new()?;
        let size = compositor.size();

        let conf_dir = helix_loader::config_dir();

        let theme_loader =
            std::sync::Arc::new(theme::Loader::new(&conf_dir, &helix_loader::runtime_dir()));

        let true_color = config.editor.true_color || crate::true_color();

        let (theme_name, theme) = match config.theme.as_ref() {
            Some(theme_name) => {
                match theme_loader
                    .load(theme_name)
                    .map_err(|e| {
                        log::warn!("failed to load theme `{}` - {}", theme_name, e);
                        e
                    })
                    .ok()
                    .filter(|theme| (true_color || theme.is_16_color())) {
                        Some(theme) => (theme_name.clone(), theme),
                        None => default_theme(theme_loader.clone(), true_color)
                    }

            }
            None => {
                default_theme(theme_loader.clone(), true_color)
            }
        };

        let syn_loader_conf = user_syntax_loader().unwrap_or_else(|err| {
            eprintln!("Bad language config: {}", err);
            eprintln!("Press <ENTER> to continue with default language config");
            use std::io::Read;
            // This waits for an enter press.
            let _ = std::io::stdin().read(&mut []);
            default_syntax_loader()
        });
        let syn_loader = std::sync::Arc::new(syntax::Loader::new(syn_loader_conf));

        let mut editor = Editor::new(
            size,
            theme_loader.clone(),
            syn_loader.clone(),
            config.editor.clone(),
            theme_name.clone(),
        );

        let editor_view = Box::new(ui::EditorView::new(std::mem::take(&mut config.keys)));
        compositor.push(editor_view);

        if args.load_tutor {
            let path = helix_loader::runtime_dir().join("tutor.txt");
            editor.open(path, Action::VerticalSplit)?;
            // Unset path to prevent accidentally saving to the original tutor file.
            doc_mut!(editor).set_path(None)?;
        } else if args.edit_config {
            let path = conf_dir.join("config.toml");
            editor.open(path, Action::VerticalSplit)?;
        } else if !args.files.is_empty() {
            let first = &args.files[0].0; // we know it's not empty
            if first.is_dir() {
                std::env::set_current_dir(&first)?;
                editor.new_file(Action::VerticalSplit);
                let picker = ui::file_picker(".".into(), &config.editor);
                compositor.push(Box::new(overlayed(picker)));
            } else {
                let nr_of_files = args.files.len();
                editor.open(first.to_path_buf(), Action::VerticalSplit)?;
                for (file, pos) in args.files {
                    if file.is_dir() {
                        return Err(anyhow::anyhow!(
                            "expected a path to file, found a directory. (to open a directory pass it as first argument)"
                        ));
                    } else {
                        let doc_id = editor.open(file, Action::Load)?;
                        // with Action::Load all documents have the same view
                        let view_id = editor.tree.focus;
                        let doc = editor.document_mut(doc_id).unwrap();
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
        } else if stdin().is_tty() {
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

        editor.set_theme(theme, theme_name.clone());

        #[cfg(windows)]
        let signals = futures_util::stream::empty();
        #[cfg(not(windows))]
        let signals = Signals::new(&[signal::SIGTSTP, signal::SIGCONT])?;

        let app = Self {
            compositor,
            editor,

            config,

            theme_loader,
            syn_loader,

            signals,
            jobs: Jobs::new(),
            lsp_progress: LspProgressMap::new(),
        };

        Ok(app)
    }

    fn render(&mut self) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            jobs: &mut self.jobs,
            scroll: None,
        };

        self.compositor.render(&mut cx);
    }

    pub async fn event_loop(&mut self) {
        let mut reader = EventStream::new();
        let mut last_render = Instant::now();
        let deadline = Duration::from_secs(1) / 60;

        self.render();

        loop {
            if self.editor.should_close() {
                break;
            }

            use futures_util::StreamExt;

            tokio::select! {
                biased;

                event = reader.next() => {
                    self.handle_terminal_events(event)
                }
                Some(signal) = self.signals.next() => {
                    self.handle_signals(signal).await;
                }
                Some((id, call)) = self.editor.language_servers.incoming.next() => {
                    self.handle_language_server_message(call, id).await;
                    // limit render calls for fast language server messages
                    let last = self.editor.language_servers.incoming.is_empty();
                    if last || last_render.elapsed() > deadline {
                        self.render();
                        last_render = Instant::now();
                    }
                }
                Some(payload) = self.editor.debugger_events.next() => {
                    self.handle_debugger_message(payload).await;
                }
                Some(callback) = self.jobs.futures.next() => {
                    self.jobs.handle_callback(&mut self.editor, &mut self.compositor, callback);
                    self.render();
                }
                Some(callback) = self.jobs.wait_futures.next() => {
                    self.jobs.handle_callback(&mut self.editor, &mut self.compositor, callback);
                    self.render();
                }
                _ = &mut self.editor.idle_timer => {
                    // idle timeout
                    self.editor.clear_idle_timer();
                    self.handle_idle_timeout();
                }
            }
        }
    }

    #[cfg(windows)]
    // no signal handling available on windows
    pub async fn handle_signals(&mut self, _signal: ()) {}

    #[cfg(not(windows))]
    pub async fn handle_signals(&mut self, signal: i32) {
        use helix_view::graphics::Rect;
        match signal {
            signal::SIGTSTP => {
                self.compositor.save_cursor();
                self.restore_term().unwrap();
                low_level::emulate_default_handler(signal::SIGTSTP).unwrap();
            }
            signal::SIGCONT => {
                self.claim_term().await.unwrap();
                // redraw the terminal
                let Rect { width, height, .. } = self.compositor.size();
                self.compositor.resize(width, height);
                self.compositor.load_cursor();
                self.render();
            }
            _ => unreachable!(),
        }
    }

    pub fn handle_idle_timeout(&mut self) {
        use crate::compositor::EventResult;
        let editor_view = self
            .compositor
            .find::<ui::EditorView>()
            .expect("expected at least one EditorView");

        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            jobs: &mut self.jobs,
            scroll: None,
        };
        if let EventResult::Consumed(_) = editor_view.handle_idle_timeout(&mut cx) {
            self.render();
        }
    }

    pub fn handle_terminal_events(&mut self, event: Option<Result<Event, crossterm::ErrorKind>>) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            jobs: &mut self.jobs,
            scroll: None,
        };
        // Handle key events
        let should_redraw = match event {
            Some(Ok(Event::Resize(width, height))) => {
                self.compositor.resize(width, height);

                self.compositor
                    .handle_event(Event::Resize(width, height), &mut cx)
            }
            Some(Ok(event)) => self.compositor.handle_event(event, &mut cx),
            Some(Err(x)) => panic!("{}", x),
            None => panic!(),
        };

        if should_redraw && !self.editor.should_close() {
            self.render();
        }
    }

    pub async fn handle_debugger_message(&mut self, payload: helix_dap::Payload) {
        use crate::commands::dap::{breakpoints_changed, select_thread_id};
        use dap::requests::RunInTerminal;
        use helix_dap::{events, Event};

        let debugger = match self.editor.debugger.as_mut() {
            Some(debugger) => debugger,
            None => return,
        };
        match payload {
            Payload::Event(ev) => match *ev {
                Event::Stopped(events::Stopped {
                    thread_id,
                    description,
                    text,
                    reason,
                    all_threads_stopped,
                    ..
                }) => {
                    let all_threads_stopped = all_threads_stopped.unwrap_or_default();

                    if all_threads_stopped {
                        if let Ok(response) = debugger.request::<dap::requests::Threads>(()).await {
                            for thread in response.threads {
                                fetch_stack_trace(debugger, thread.id).await;
                            }
                            select_thread_id(
                                &mut self.editor,
                                thread_id.unwrap_or_default(),
                                false,
                            )
                            .await;
                        }
                    } else if let Some(thread_id) = thread_id {
                        debugger.thread_states.insert(thread_id, reason.clone()); // TODO: dap uses "type" || "reason" here

                        // whichever thread stops is made "current" (if no previously selected thread).
                        select_thread_id(&mut self.editor, thread_id, false).await;
                    }

                    let scope = match thread_id {
                        Some(id) => format!("Thread {}", id),
                        None => "Target".to_owned(),
                    };

                    let mut status = format!("{} stopped because of {}", scope, reason);
                    if let Some(desc) = description {
                        status.push_str(&format!(" {}", desc));
                    }
                    if let Some(text) = text {
                        status.push_str(&format!(" {}", text));
                    }
                    if all_threads_stopped {
                        status.push_str(" (all threads stopped)");
                    }

                    self.editor.set_status(status);
                }
                Event::Continued(events::Continued { thread_id, .. }) => {
                    debugger
                        .thread_states
                        .insert(thread_id, "running".to_owned());
                    if debugger.thread_id == Some(thread_id) {
                        debugger.resume_application();
                    }
                }
                Event::Thread(_) => {
                    // TODO: update thread_states, make threads request
                }
                Event::Breakpoint(events::Breakpoint { reason, breakpoint }) => {
                    match &reason[..] {
                        "new" => {
                            if let Some(source) = breakpoint.source {
                                self.editor
                                    .breakpoints
                                    .entry(source.path.unwrap()) // TODO: no unwraps
                                    .or_default()
                                    .push(Breakpoint {
                                        id: breakpoint.id,
                                        verified: breakpoint.verified,
                                        message: breakpoint.message,
                                        line: breakpoint.line.unwrap().saturating_sub(1), // TODO: no unwrap
                                        column: breakpoint.column,
                                        ..Default::default()
                                    });
                            }
                        }
                        "changed" => {
                            for breakpoints in self.editor.breakpoints.values_mut() {
                                if let Some(i) =
                                    breakpoints.iter().position(|b| b.id == breakpoint.id)
                                {
                                    breakpoints[i].verified = breakpoint.verified;
                                    breakpoints[i].message = breakpoint.message.clone();
                                    breakpoints[i].line =
                                        breakpoint.line.unwrap().saturating_sub(1); // TODO: no unwrap
                                    breakpoints[i].column = breakpoint.column;
                                }
                            }
                        }
                        "removed" => {
                            for breakpoints in self.editor.breakpoints.values_mut() {
                                if let Some(i) =
                                    breakpoints.iter().position(|b| b.id == breakpoint.id)
                                {
                                    breakpoints.remove(i);
                                }
                            }
                        }
                        reason => {
                            warn!("Unknown breakpoint event: {}", reason);
                        }
                    }
                }
                Event::Output(events::Output {
                    category, output, ..
                }) => {
                    let prefix = match category {
                        Some(category) => {
                            if &category == "telemetry" {
                                return;
                            }
                            format!("Debug ({}):", category)
                        }
                        None => "Debug:".to_owned(),
                    };

                    log::info!("{}", output);
                    self.editor.set_status(format!("{} {}", prefix, output));
                }
                Event::Initialized => {
                    // send existing breakpoints
                    for (path, breakpoints) in &mut self.editor.breakpoints {
                        // TODO: call futures in parallel, await all
                        let _ = breakpoints_changed(debugger, path.clone(), breakpoints);
                    }
                    // TODO: fetch breakpoints (in case we're attaching)

                    if debugger.configuration_done().await.is_ok() {
                        self.editor.set_status("Debugged application started");
                    }; // TODO: do we need to handle error?
                }
                ev => {
                    log::warn!("Unhandled event {:?}", ev);
                    return; // return early to skip render
                }
            },
            Payload::Response(_) => unreachable!(),
            Payload::Request(request) => match request.command.as_str() {
                RunInTerminal::COMMAND => {
                    let arguments: dap::requests::RunInTerminalArguments =
                        serde_json::from_value(request.arguments.unwrap_or_default()).unwrap();
                    // TODO: no unwrap

                    let process = std::process::Command::new("tmux")
                        .arg("split-window")
                        .arg(arguments.args.join(" "))
                        .spawn()
                        .unwrap();

                    let _ = debugger
                        .reply(
                            request.seq,
                            dap::requests::RunInTerminal::COMMAND,
                            serde_json::to_value(dap::requests::RunInTerminalResponse {
                                process_id: Some(process.id()),
                                shell_process_id: None,
                            })
                            .map_err(|e| e.into()),
                        )
                        .await;
                }
                _ => log::error!("DAP reverse request not implemented: {:?}", request),
            },
        }
        self.render();
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
                    Some(notification) => notification,
                    None => return,
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
                            let language_id =
                                doc.language_id().map(ToOwned::to_owned).unwrap_or_default();

                            tokio::spawn(language_server.text_document_did_open(
                                doc.url().unwrap(),
                                doc.version(),
                                doc.text(),
                                language_id,
                            ));
                        }
                    }
                    Notification::PublishDiagnostics(params) => {
                        let path = params.uri.to_file_path().unwrap();
                        let doc = self.editor.document_by_path_mut(&path);

                        if let Some(doc) = doc {
                            let lang_conf = doc.language_config();
                            let text = doc.text();

                            let diagnostics = params
                                .diagnostics
                                .into_iter()
                                .filter_map(|diagnostic| {
                                    use helix_core::{
                                        diagnostic::{Range, Severity::*},
                                        Diagnostic,
                                    };
                                    use lsp::DiagnosticSeverity;

                                    let language_server = doc.language_server().unwrap();

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

                                    Some(Diagnostic {
                                        range: Range { start, end },
                                        line: diagnostic.range.start.line as usize,
                                        message: diagnostic.message,
                                        severity,
                                        // code
                                        // source
                                    })
                                })
                                .collect();

                            doc.set_diagnostics(diagnostics);
                        }
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

                        if self.config.lsp.display_messages {
                            self.editor.set_status(status);
                        }
                    }
                    Notification::ProgressMessage(_params) => {
                        // do nothing
                    }
                }
            }
            Call::MethodCall(helix_lsp::jsonrpc::MethodCall {
                method, params, id, ..
            }) => {
                let call = match MethodCall::parse(&method, params) {
                    Some(call) => call,
                    None => {
                        error!("Method not found {}", method);
                        return;
                    }
                };

                match call {
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
                        let language_server =
                            match self.editor.language_servers.get_by_id(server_id) {
                                Some(language_server) => language_server,
                                None => {
                                    warn!("can't find language server with id `{}`", server_id);
                                    return;
                                }
                            };

                        tokio::spawn(language_server.reply(id, Ok(serde_json::Value::Null)));
                    }
                    MethodCall::ApplyWorkspaceEdit(params) => {
                        apply_workspace_edit(
                            &mut self.editor,
                            helix_lsp::OffsetEncoding::Utf8,
                            &params.edit,
                        );

                        let language_server =
                            match self.editor.language_servers.get_by_id(server_id) {
                                Some(language_server) => language_server,
                                None => {
                                    warn!("can't find language server with id `{}`", server_id);
                                    return;
                                }
                            };

                        tokio::spawn(language_server.reply(
                            id,
                            Ok(json!(lsp::ApplyWorkspaceEditResponse {
                                applied: true,
                                failure_reason: None,
                                failed_change: None,
                            })),
                        ));
                    }
                    MethodCall::WorkspaceConfiguration(params) => {
                        let language_server =
                            match self.editor.language_servers.get_by_id(server_id) {
                                Some(language_server) => language_server,
                                None => {
                                    warn!("can't find language server with id `{}`", server_id);
                                    return;
                                }
                            };
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
                                    None => language_server.config()?,
                                };
                                if let Some(section) = item.section.as_ref() {
                                    for part in section.split('.') {
                                        config = config.get(part)?;
                                    }
                                }
                                Some(config)
                            })
                            .collect();
                        tokio::spawn(language_server.reply(id, Ok(json!(result))));
                    }
                }
            }
            e => unreachable!("{:?}", e),
        }
    }

    async fn claim_term(&mut self) -> Result<(), Error> {
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, terminal::EnterAlternateScreen)?;
        if self.config.editor.mouse {
            execute!(stdout, EnableMouseCapture)?;
        }
        Ok(())
    }

    fn restore_term(&mut self) -> Result<(), Error> {
        let mut stdout = stdout();
        // reset cursor shape
        write!(stdout, "\x1B[2 q")?;
        // Ignore errors on disabling, this might trigger on windows if we call
        // disable without calling enable previously
        let _ = execute!(stdout, DisableMouseCapture);
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<i32, Error> {
        self.claim_term().await?;

        // Exit the alternate screen and disable raw mode before panicking
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // We can't handle errors properly inside this closure.  And it's
            // probably not a good idea to `unwrap()` inside a panic handler.
            // So we just ignore the `Result`s.
            let _ = execute!(std::io::stdout(), DisableMouseCapture);
            let _ = execute!(std::io::stdout(), terminal::LeaveAlternateScreen);
            let _ = terminal::disable_raw_mode();
            hook(info);
        }));

        self.event_loop().await;

        self.jobs.finish().await;

        if self.editor.close_language_servers(None).await.is_err() {
            log::error!("Timed out waiting for language servers to shutdown");
        };

        self.restore_term()?;

        Ok(self.editor.exit_code)
    }
}
