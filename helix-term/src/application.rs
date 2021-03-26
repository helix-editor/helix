use clap::ArgMatches as Args;

use helix_view::{document::Mode, Document, Editor, Theme, View};

use crate::{compositor::Compositor, ui};

use log::{error, info};

use std::{
    io::{self, stdout, Stdout, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use smol::prelude::*;

use anyhow::Error;

use crossterm::{
    event::{Event, EventStream},
    execute, terminal,
};

use tui::layout::Rect;

// use futures_util::future::BoxFuture;
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

    executor: &'static smol::Executor<'static>,
    callbacks: LspCallbacks,
}

impl Application {
    pub fn new(mut args: Args, executor: &'static smol::Executor<'static>) -> Result<Self, Error> {
        let mut compositor = Compositor::new()?;
        let size = compositor.size();
        let mut editor = Editor::new(executor, size);

        let files = args.values_of_t::<PathBuf>("files").unwrap();
        for file in files {
            use helix_view::editor::Action;
            editor.open(file, Action::HorizontalSplit)?;
        }

        compositor.push(Box::new(ui::EditorView::new()));

        let mut app = Self {
            compositor,
            editor,

            executor,
            callbacks: FuturesUnordered::new(),
        };

        Ok(app)
    }

    fn render(&mut self) {
        let executor = &self.executor;
        let editor = &mut self.editor;
        let compositor = &mut self.compositor;
        let callbacks = &mut self.callbacks;

        let mut cx = crate::compositor::Context {
            editor,
            executor,
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

            use futures_util::{select, FutureExt};
            select! {
                event = reader.next().fuse() => {
                    self.handle_terminal_events(event)
                }
                call = self.editor.language_servers.incoming.next().fuse() => {
                    self.handle_language_server_message(call).await
                }
                callback = self.callbacks.next().fuse() => {
                    self.handle_language_server_callback(callback)
                }
            }
        }
    }
    pub fn handle_language_server_callback(
        &mut self,
        callback: Option<Result<LspCallbackWrapper, anyhow::Error>>,
    ) {
        if let Some(Ok(callback)) = callback {
            // TODO: handle Err()
            callback(&mut self.editor, &mut self.compositor);
            self.render();
        }
    }

    pub fn handle_terminal_events(&mut self, event: Option<Result<Event, crossterm::ErrorKind>>) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            executor: &self.executor,
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

    pub async fn handle_language_server_message(&mut self, call: Option<helix_lsp::Call>) {
        use helix_lsp::{Call, Notification};
        match call {
            Some(Call::Notification(helix_lsp::jsonrpc::Notification {
                method, params, ..
            })) => {
                let notification = Notification::parse(&method, params);
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
                                .map(|diagnostic| {
                                    use helix_core::{
                                        diagnostic::{Range, Severity, Severity::*},
                                        Diagnostic,
                                    };
                                    use helix_lsp::{lsp, util::lsp_pos_to_pos};
                                    use lsp::DiagnosticSeverity;
                                    let start = lsp_pos_to_pos(text, diagnostic.range.start);
                                    let end = lsp_pos_to_pos(text, diagnostic.range.end);

                                    Diagnostic {
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
                                    }
                                })
                                .collect();

                            doc.diagnostics = diagnostics;
                            // TODO: we want to process all the events in queue, then render. publishDiagnostic tends to send a whole bunch of events
                            self.render();
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Some(Call::MethodCall(call)) => {
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
            None => (),
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
