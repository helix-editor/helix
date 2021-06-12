use helix_lsp::lsp;
use helix_view::{document::Mode, Document, Editor, Theme, View};

use crate::{args::Args, compositor::Compositor, ui};

use log::{error, info};

use std::{
    future::Future,
    io::{self, stdout, Stdout, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::Error;

use crossterm::{
    event::{Event, EventStream},
    execute, terminal,
};

use tui::layout::Rect;

use futures_util::stream::FuturesUnordered;
use std::pin::Pin;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type LspCallback =
    BoxFuture<Result<Box<dyn FnOnce(&mut Editor, &mut Compositor) + Send>, anyhow::Error>>;

pub type LspCallbacks = FuturesUnordered<LspCallback>;
pub type LspCallbackWrapper = Box<dyn FnOnce(&mut Editor, &mut Compositor) + Send>;

pub struct Application {
    compositor: Compositor,
    editor: Editor,

    callbacks: LspCallbacks,
}

impl Application {
    pub fn new(mut args: Args) -> Result<Self, Error> {
        use helix_view::editor::Action;
        let mut compositor = Compositor::new()?;
        let size = compositor.size();
        let mut editor = Editor::new(size);

        compositor.push(Box::new(ui::EditorView::new()));

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

        let mut app = Self {
            compositor,
            editor,

            callbacks: FuturesUnordered::new(),
        };

        Ok(app)
    }

    fn render(&mut self) {
        let editor = &mut self.editor;
        let compositor = &mut self.compositor;
        let callbacks = &mut self.callbacks;

        let mut cx = crate::compositor::Context {
            editor,
            callbacks,
            scroll: None,
        };

        compositor.render(&mut cx);
    }

    pub async fn event_loop(&mut self) {
        let mut reader = EventStream::new();

        self.render();

        loop {
            if self.editor.should_close() {
                break;
            }

            use futures_util::StreamExt;

            tokio::select! {
                event = reader.next() => {
                    self.handle_terminal_events(event)
                }
                Some(call) = self.editor.language_servers.incoming.next() => {
                    self.handle_language_server_message(call).await
                }
                Some(callback) = &mut self.callbacks.next() => {
                    self.handle_language_server_callback(callback)
                }
            }
        }
    }
    pub fn handle_language_server_callback(
        &mut self,
        callback: Result<LspCallbackWrapper, anyhow::Error>,
    ) {
        if let Ok(callback) = callback {
            // TODO: handle Err()
            callback(&mut self.editor, &mut self.compositor);
            self.render();
        }
    }

    pub fn handle_terminal_events(&mut self, event: Option<Result<Event, crossterm::ErrorKind>>) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            callbacks: &mut self.callbacks,
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

    pub async fn handle_language_server_message(&mut self, call: helix_lsp::Call) {
        use helix_lsp::{Call, Notification};
        match call {
            Call::Notification(helix_lsp::jsonrpc::Notification { method, params, .. }) => {
                let notification = match Notification::parse(&method, params) {
                    Some(notification) => notification,
                    None => return,
                };

                // TODO: parse should return Result/Option
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
                                        diagnostic::{Range, Severity, Severity::*},
                                        Diagnostic,
                                    };
                                    use helix_lsp::{lsp, util::lsp_pos_to_pos};
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
                            // TODO: we want to process all the events in queue, then render. publishDiagnostic tends to send a whole bunch of events
                            self.render();
                        }
                    }
                    Notification::ShowMessage(params) => {
                        log::warn!("unhandled window/showMessage: {:?}", params);
                    }
                    Notification::LogMessage(params) => {
                        log::warn!("unhandled window/logMessage: {:?}", params);
                    }
                    Notification::ProgressMessage(params) => {
                        let token = match params.token {
                            lsp::NumberOrString::Number(n) => n.to_string(),
                            lsp::NumberOrString::String(s) => s,
                        };
                        let msg = {
                            let lsp::ProgressParamsValue::WorkDone(work) = params.value;
                            let parts = match work {
                                lsp::WorkDoneProgress::Begin(lsp::WorkDoneProgressBegin {
                                    title,
                                    message,
                                    percentage,
                                    ..
                                }) => (Some(title), message, percentage.map(|n| n.to_string())),
                                lsp::WorkDoneProgress::Report(lsp::WorkDoneProgressReport {
                                    message,
                                    percentage,
                                    ..
                                }) => (None, message, percentage.map(|n| n.to_string())),
                                lsp::WorkDoneProgress::End(lsp::WorkDoneProgressEnd {
                                    message,
                                }) => {
                                    if let Some(message) = message {
                                        (None, Some(message), None)
                                    } else {
                                        self.editor.clear_status();
                                        return;
                                    }
                                }
                            };
                            match parts {
                                (Some(title), Some(message), Some(percentage)) => {
                                    format!("{}% {} - {}", percentage, title, message)
                                }
                                (Some(title), None, Some(percentage)) => {
                                    format!("{}% {}", percentage, title)
                                }
                                (Some(title), Some(message), None) => {
                                    format!("{} - {}", title, message)
                                }
                                (None, Some(message), Some(percentage)) => {
                                    format!("{}% {}", percentage, message)
                                }
                                (Some(title), None, None) => title,
                                (None, Some(message), None) => message,
                                (None, None, Some(percentage)) => format!("{}%", percentage),
                                (None, None, None) => "".into(),
                            }
                        };
                        let status = format!("[{}] {}", token, msg);
                        self.editor.set_status(status);
                        self.render();
                    }
                    _ => unreachable!(),
                }
            }
            Call::MethodCall(call) => {
                error!("Method not found {}", call.method);

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

        // reset cursor shape
        write!(stdout, "\x1B[2 q");

        execute!(stdout, terminal::LeaveAlternateScreen)?;

        terminal::disable_raw_mode()?;

        Ok(())
    }
}
