use clap::ArgMatches as Args;

use helix_view::{document::Mode, Document, Editor, Theme, View};

use crate::compositor::{Component, Compositor, EventResult};
use crate::editor_view::EditorView;
use crate::prompt::Prompt;

use log::{debug, info};

use std::{
    borrow::Cow,
    io::{self, stdout, Stdout, Write},
    path::PathBuf,
    time::Duration,
};

use smol::prelude::*;

use anyhow::Error;

use crossterm::{
    cursor,
    event::{read, Event, EventStream, KeyCode, KeyEvent},
    execute, queue,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};

use tui::{
    backend::CrosstermBackend,
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Modifier, Style},
};

type Terminal = crate::terminal::Terminal<CrosstermBackend<std::io::Stdout>>;

pub struct Application {
    compositor: Compositor,
    editor: Editor,
    terminal: Terminal,

    executor: &'static smol::Executor<'static>,
    language_server: helix_lsp::Client,
}

// TODO: temp
#[inline(always)]
pub fn text_color() -> Style {
    return Style::default().fg(Color::Rgb(219, 191, 239)); // lilac
}

// pub fn render_cursor(&mut self, view: &View, prompt: Option<&Prompt>, viewport: Rect) {
//     let mut stdout = stdout();
//     match view.doc.mode() {
//         Mode::Insert => write!(stdout, "\x1B[6 q"),
//         mode => write!(stdout, "\x1B[2 q"),
//     };
//     let pos = if let Some(prompt) = prompt {
//         Position::new(self.size.0 as usize, 2 + prompt.cursor)
//     } else {
//         let cursor = view.doc.state.selection().cursor();

//         let mut pos = view
//             .screen_coords_at_pos(&view.doc.text().slice(..), cursor)
//             .expect("Cursor is out of bounds.");
//         pos.col += viewport.x as usize;
//         pos.row += viewport.y as usize;
//         pos
//     };

//     execute!(stdout, cursor::MoveTo(pos.col as u16, pos.row as u16));
// }

impl Application {
    pub fn new(mut args: Args, executor: &'static smol::Executor<'static>) -> Result<Self, Error> {
        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend)?;
        let mut editor = Editor::new();
        let size = terminal.size()?;

        if let Some(file) = args.values_of_t::<PathBuf>("files").unwrap().pop() {
            editor.open(file, (size.width, size.height))?;
        }

        let mut compositor = Compositor::new();
        compositor.push(Box::new(EditorView::new()));

        let language_server = helix_lsp::Client::start(&executor, "rust-analyzer", &[]);

        let mut app = Self {
            editor,
            terminal,
            // TODO; move to state
            compositor,

            executor,
            language_server,
        };

        Ok(app)
    }

    fn render(&mut self) {
        let executor = &self.executor;
        let editor = &mut self.editor;
        let compositor = &self.compositor;

        // TODO: should be unnecessary
        // self.terminal.autoresize();
        let mut cx = crate::compositor::Context { editor, executor };
        let area = self.terminal.size().unwrap();
        compositor.render(area, self.terminal.current_buffer_mut(), &mut cx);

        self.terminal.draw();
    }

    pub async fn event_loop(&mut self) {
        let mut reader = EventStream::new();

        // initialize lsp
        self.language_server.initialize().await.unwrap();
        // TODO: temp
        // self.language_server
        //     .text_document_did_open(&cx.editor.view().unwrap().doc)
        //     .await
        //     .unwrap();

        self.render();

        loop {
            if self.editor.should_close {
                break;
            }

            use futures_util::{select, FutureExt};
            select! {
                event = reader.next().fuse() => {
                    self.handle_terminal_events(event)
                }
                call = self.language_server.incoming.next().fuse() => {
                    self.handle_language_server_message(call).await
                }
            }
        }
    }

    pub fn handle_terminal_events(&mut self, event: Option<Result<Event, crossterm::ErrorKind>>) {
        let mut cx = crate::compositor::Context {
            editor: &mut self.editor,
            executor: &self.executor,
        };
        // Handle key events
        let should_redraw = match event {
            Some(Ok(Event::Resize(width, height))) => {
                self.terminal.resize(Rect::new(0, 0, width, height));

                self.compositor
                    .handle_event(Event::Resize(width, height), &mut cx)
            }
            Some(Ok(event)) => self.compositor.handle_event(event, &mut cx),
            Some(Err(x)) => panic!(x),
            None => panic!(),
        };

        if should_redraw {
            self.render();
            // calling render twice here fixes it for some reason
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
                        let view: Option<&mut helix_view::View> = None;
                        // TODO:
                        // let view = self
                        //     .editor
                        //     .views
                        //     .iter_mut()
                        //     .find(|view| view.doc.path == path);

                        if let Some(view) = view {
                            let doc = view.doc.text().slice(..);
                            let diagnostics = params
                                .diagnostics
                                .into_iter()
                                .map(|diagnostic| {
                                    use helix_lsp::util::lsp_pos_to_pos;
                                    let start = lsp_pos_to_pos(&doc, diagnostic.range.start);
                                    let end = lsp_pos_to_pos(&doc, diagnostic.range.end);

                                    helix_core::Diagnostic {
                                        range: (start, end),
                                        line: diagnostic.range.start.line as usize,
                                        message: diagnostic.message,
                                        // severity
                                        // code
                                        // source
                                    }
                                })
                                .collect();

                            view.doc.diagnostics = diagnostics;

                            // TODO: we want to process all the events in queue, then render. publishDiagnostic tends to send a whole bunch of events
                            self.render();
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Some(Call::MethodCall(call)) => {
                debug!("Method not found {}", call.method);

                self.language_server.reply(
                    call.id,
                    // TODO: make a Into trait that can cast to Err(jsonrpc::Error)
                    Err(helix_lsp::jsonrpc::Error {
                        code: helix_lsp::jsonrpc::ErrorCode::MethodNotFound,
                        message: "Method not found".to_string(),
                        data: None,
                    }),
                );
            }
            _ => unreachable!(),
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        enable_raw_mode()?;

        let mut stdout = stdout();

        execute!(stdout, terminal::EnterAlternateScreen)?;

        // Exit the alternate screen and disable raw mode before panicking
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            execute!(std::io::stdout(), terminal::LeaveAlternateScreen);
            disable_raw_mode();
            hook(info);
        }));

        self.event_loop().await;

        // reset cursor shape
        write!(stdout, "\x1B[2 q");

        execute!(stdout, terminal::LeaveAlternateScreen)?;

        disable_raw_mode()?;

        Ok(())
    }
}

// TODO: language configs:
// tabSize, fileExtension etc, mapping to tree sitter parser
// themes:
// map tree sitter highlights to color values
//
// TODO: expand highlight thing so we're able to render only viewport range
// TODO: async: maybe pre-cache scopes as empty so we render all graphemes initially as regular
////text until calc finishes
// TODO: scope matching: biggest union match? [string] & [html, string], [string, html] & [ string, html]
// can do this by sorting our theme matches based on array len (longest first) then stopping at the
// first rule that matches (rule.all(|scope| scopes.contains(scope)))
