use helix_core::syntax;
use helix_lsp::{lsp, util::lsp_pos_to_pos, LspProgressMap};
use helix_view::{theme, Editor};

use crate::{args::Args, compositor::Compositor, config::Config, job::Jobs, ui};

use log::error;

use std::{
    io::{stdout, Write},
    sync::Arc,
};

use anyhow::Error;

use crossterm::{
    event::{Event, EventStream},
    execute, terminal,
};

pub struct Application {
    compositor: Compositor,
    editor: Editor,

    // TODO should be separate to take only part of the config
    config: Config,

    theme_loader: Arc<theme::Loader>,
    syn_loader: Arc<syntax::Loader>,

    jobs: Jobs,
    lsp_progress: LspProgressMap,
}

impl Application {
    pub fn new(mut args: Args, mut config: Config) -> Result<Self, Error> {
        use helix_view::editor::Action;
        let mut compositor = Compositor::new()?;
        let size = compositor.size();

        let conf_dir = helix_core::config_dir();

        let theme_loader =
            std::sync::Arc::new(theme::Loader::new(&conf_dir, &helix_core::runtime_dir()));

        // load $HOME/.config/helix/languages.toml, fallback to default config
        let lang_conf = std::fs::read(conf_dir.join("languages.toml"));
        let lang_conf = lang_conf
            .as_deref()
            .unwrap_or(include_bytes!("../../languages.toml"));

        let theme = if let Some(theme) = &config.theme {
            match theme_loader.load(theme) {
                Ok(theme) => theme,
                Err(e) => {
                    log::warn!("failed to load theme `{}` - {}", theme, e);
                    theme_loader.default()
                }
            }
        } else {
            theme_loader.default()
        };

        let syn_loader_conf = toml::from_slice(lang_conf).expect("Could not parse languages.toml");
        let syn_loader = std::sync::Arc::new(syntax::Loader::new(syn_loader_conf));

        let mut editor = Editor::new(size, theme_loader.clone(), syn_loader.clone());

        let mut editor_view = Box::new(ui::EditorView::new(std::mem::take(&mut config.keys)));
        compositor.push(editor_view);

        if !args.files.is_empty() {
            let first = &args.files[0]; // we know it's not empty
            if first.is_dir() {
                editor.new_file(Action::VerticalSplit);
                compositor.push(Box::new(ui::file_picker(first.clone())));
            } else {
                for file in args.files {
                    if file.is_dir() {
                        return Err(anyhow::anyhow!(
                            "expected a path to file, found a directory. (to open a directory pass it as first argument)"
                        ));
                    } else {
                        editor.open(file, Action::VerticalSplit)?;
                    }
                }
            }
        } else {
            editor.new_file(Action::VerticalSplit);
        }

        editor.set_theme(theme);

        let mut app = Self {
            compositor,
            editor,

            config,

            theme_loader,
            syn_loader,

            jobs: Jobs::new(),
            lsp_progress: LspProgressMap::new(),
        };

        Ok(app)
    }

    fn render(&mut self) {
        let editor = &mut self.editor;
        let compositor = &mut self.compositor;
        let jobs = &mut self.jobs;

        let mut cx = crate::compositor::Context {
            editor,
            jobs,
            scroll: None,
        };

        compositor.render(&mut cx);
    }

    pub async fn event_loop(&mut self) {
        let mut reader = EventStream::new();

        self.render();

        loop {
            if self.editor.should_close() {
                self.jobs.finish();
                break;
            }

            use futures_util::{FutureExt, StreamExt};

            tokio::select! {
                event = reader.next() => {
                    self.handle_terminal_events(event)
                }
                Some((id, call)) = self.editor.language_servers.incoming.next() => {
                    self.handle_language_server_message(call, id).await;

                    // eagerly process any other available notifications/calls
                    let now = std::time::Instant::now();
                    let deadline = std::time::Duration::from_millis(10);
                    while let Some(Some((id, call))) = self.editor.language_servers.incoming.next().now_or_never() {
                       self.handle_language_server_message(call, id).await;

                       if now.elapsed() > deadline { // use a deadline so we don't block too long
                           break;
                       }
                    }
                    self.render();
                }
                Some(callback) = self.jobs.next_job() => {
                    self.jobs.handle_callback(&mut self.editor, &mut self.compositor, callback);
                    self.render();
                }
            }
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

    pub async fn handle_language_server_message(
        &mut self,
        call: helix_lsp::Call,
        server_id: usize,
    ) {
        use helix_lsp::{Call, MethodCall, Notification};
        let editor_view = self
            .compositor
            .find(std::any::type_name::<ui::EditorView>())
            .expect("expected at least one EditorView");
        let editor_view = editor_view
            .as_any_mut()
            .downcast_mut::<ui::EditorView>()
            .unwrap();

        match call {
            Call::Notification(helix_lsp::jsonrpc::Notification { method, params, .. }) => {
                let notification = match Notification::parse(&method, params) {
                    Some(notification) => notification,
                    None => return,
                };

                match notification {
                    Notification::PublishDiagnostics(params) => {
                        let path = Some(params.uri.to_file_path().unwrap());

                        let doc = self
                            .editor
                            .documents
                            .iter_mut()
                            .find(|(_, doc)| doc.path() == path.as_ref());

                        if let Some((_, doc)) = doc {
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

                                    Some(Diagnostic {
                                        range: Range { start, end },
                                        line: diagnostic.range.start.line as usize,
                                        message: diagnostic.message,
                                        severity: diagnostic.severity.map(
                                            |severity| match severity {
                                                DiagnosticSeverity::Error => Error,
                                                DiagnosticSeverity::Warning => Warning,
                                                DiagnosticSeverity::Information => Info,
                                                DiagnosticSeverity::Hint => Hint,
                                            },
                                        ),
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
                        log::warn!("unhandled window/logMessage: {:?}", params);
                    }
                    Notification::ProgressMessage(params) => {
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
                    _ => unreachable!(),
                }
            }
            Call::MethodCall(helix_lsp::jsonrpc::MethodCall {
                method,
                params,
                jsonrpc,
                id,
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

                        let spinner = editor_view.spinners_mut().get_or_create(server_id);
                        if spinner.is_stopped() {
                            spinner.start();
                        }

                        let doc = self.editor.documents().find(|doc| {
                            doc.language_server()
                                .map(|server| server.id() == server_id)
                                .unwrap_or_default()
                        });
                        match doc {
                            Some(doc) => {
                                // it's ok to unwrap, we check for the language server before
                                let server = doc.language_server().unwrap();
                                tokio::spawn(server.reply(id, Ok(serde_json::Value::Null)));
                            }
                            None => {
                                if let Some(server) =
                                    self.editor.language_servers.get_by_id(server_id)
                                {
                                    log::warn!(
                                        "missing document with language server id `{}`",
                                        server_id
                                    );
                                    tokio::spawn(server.reply(
                                        id,
                                        Err(helix_lsp::jsonrpc::Error {
                                            code: helix_lsp::jsonrpc::ErrorCode::InternalError,
                                            message: "document missing".to_string(),
                                            data: None,
                                        }),
                                    ));
                                } else {
                                    log::warn!(
                                        "can't find language server with id `{}`",
                                        server_id
                                    );
                                }
                            }
                        }
                    }
                }
                // self.language_server.reply(
                //     call.id,
                //     // TODO: make a Into trait that can cast to Err(jsonrpc::Error)
                //     Err(helix_lsp::jsonrpc::Error {
                //         code: helix_lsp::jsonrpc::ErrorCode::MethodNotFound,
                //         message: "Method not found".to_string(),
                //         data: None,
                //     }),
                // );
            }
            e => unreachable!("{:?}", e),
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        terminal::enable_raw_mode()?;

        let mut stdout = stdout();

        execute!(stdout, terminal::EnterAlternateScreen)?;

        // Exit the alternate screen and disable raw mode before panicking
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            execute!(std::io::stdout(), terminal::LeaveAlternateScreen);
            terminal::disable_raw_mode();
            hook(info);
        }));

        self.event_loop().await;

        self.editor.close_language_servers(None).await;

        // reset cursor shape
        write!(stdout, "\x1B[2 q");

        execute!(stdout, terminal::LeaveAlternateScreen)?;

        terminal::disable_raw_mode()?;

        Ok(())
    }
}
