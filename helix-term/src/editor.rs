use crate::{keymap, Args};
use anyhow::Error;
use crossterm::{
    cursor,
    cursor::position,
    event::{self, read, Event, EventStream, KeyCode, KeyEvent},
    execute, queue,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use futures::{future::FutureExt, select, StreamExt};
use helix_core::{state::coords_at_pos, state::Mode, State};
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

static EX: smol::Executor = smol::Executor::new();

pub struct Editor {
    state: Option<State>,
    first_line: u16,
    size: (u16, u16),
}

impl Editor {
    pub fn new(mut args: Args) -> Result<Self, Error> {
        let mut editor = Editor {
            state: None,
            first_line: 0,
            size: terminal::size().unwrap(),
        };

        if let Some(file) = args.files.pop() {
            editor.open(file)?;
        }

        Ok(editor)
    }

    pub fn open(&mut self, path: PathBuf) -> Result<(), Error> {
        self.state = Some(State::load(path)?);
        Ok(())
    }

    fn render(&mut self) {
        match &self.state {
            Some(state) => {
                let lines = state
                    .doc
                    .lines_at(self.first_line as usize)
                    .take(self.size.1 as usize)
                    .map(|x| x.as_str().unwrap());

                let mut stdout = stdout();

                for (n, line) in lines.enumerate() {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::DarkCyan),
                        cursor::MoveTo(0, n as u16),
                        Print((n + 1).to_string())
                    );
                    execute!(
                        stdout,
                        SetForegroundColor(Color::Reset),
                        cursor::MoveTo(2, n as u16),
                        Print(line)
                    );
                }

                let mode = match state.mode {
                    Mode::Insert => "INS",
                    Mode::Normal => "NOR",
                };

                execute!(
                    stdout,
                    SetForegroundColor(Color::Reset),
                    cursor::MoveTo(0, self.size.1),
                    Print(mode)
                );

                // set cursor shape
                match state.mode {
                    Mode::Insert => write!(stdout, "\x1B[6 q"),
                    Mode::Normal => write!(stdout, "\x1B[2 q"),
                };

                // render the cursor
                let pos = state.selection.primary().head;
                let coords = coords_at_pos(&state.doc.slice(..), pos);
                execute!(
                    stdout,
                    cursor::MoveTo((coords.1 + 2) as u16, coords.0 as u16)
                );
            }
            None => (),
        }
    }

    pub async fn print_events(&mut self) {
        let mut reader = EventStream::new();
        let keymap = keymap::default();

        self.render();

        loop {
            // Handle key events
            let mut event = reader.next().await;
            match event {
                // TODO: handle resize events
                Some(Ok(Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }))) => {
                    break;
                }
                Some(Ok(Event::Key(event))) => {
                    if let Some(state) = &mut self.state {
                        match state.mode {
                            Mode::Insert => {
                                match event {
                                    KeyEvent {
                                        code: KeyCode::Esc, ..
                                    } => helix_core::commands::normal_mode(state, 1),
                                    KeyEvent {
                                        code: KeyCode::Char(c),
                                        ..
                                    } => helix_core::commands::insert(state, c),
                                    _ => (), // skip
                                }
                                self.render();
                            }
                            Mode::Normal => {
                                // TODO: handle modes and sequences (`gg`)
                                if let Some(command) = keymap.get(&event) {
                                    // TODO: handle count other than 1
                                    command(state, 1);

                                    self.render();
                                }
                            }
                        }
                    }
                }
                Some(Ok(_)) => {
                    // unhandled event
                }
                Some(Err(x)) => panic!(x),
                None => break,
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        enable_raw_mode()?;

        let mut stdout = stdout();

        execute!(stdout, terminal::EnterAlternateScreen)?;

        self.print_events().await;

        // reset cursor shape
        write!(stdout, "\x1B[2 q");

        execute!(stdout, terminal::LeaveAlternateScreen)?;

        disable_raw_mode()?;

        Ok(())
    }
}
