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

use helix_core::{state::coords_at_pos, state::Mode, State};
use smol::prelude::*;
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

use tui::backend::CrosstermBackend;
use tui::buffer::Buffer as Surface;
use tui::layout::Rect;
use tui::style::Style;

type Terminal = tui::Terminal<CrosstermBackend<std::io::Stdout>>;

static EX: smol::Executor = smol::Executor::new();

pub struct Editor {
    terminal: Terminal,
    state: Option<State>,
    first_line: u16,
    size: (u16, u16),
}

impl Editor {
    pub fn new(mut args: Args) -> Result<Self, Error> {
        let backend = CrosstermBackend::new(stdout());

        let mut terminal = Terminal::new(backend)?;

        let mut editor = Editor {
            terminal,
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
                let area = Rect::new(0, 0, self.size.0, self.size.1);
                let mut surface = Surface::empty(area);

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

                    surface.set_string(2, n as u16, line, Style::default());
                    // execute!(
                    //     stdout,
                    //     SetForegroundColor(Color::Reset),
                    //     cursor::MoveTo(2, n as u16),
                    //     Print(line)
                    // );
                }

                // iterate over selections and render them
                let select = Style::default().bg(tui::style::Color::LightBlue);
                let text = state.doc.slice(..);
                for range in state.selection.ranges() {
                    // get terminal coords for x,y for each range pos
                    // TODO: this won't work with multiline
                    let (y1, x1) = coords_at_pos(&text, range.from());
                    let (y2, x2) = coords_at_pos(&text, range.to());
                    let area = Rect::new(
                        (x1 + 2) as u16,
                        y1 as u16,
                        (x2 - x1 + 1) as u16,
                        (y2 - y1 + 1) as u16,
                    );
                    surface.set_style(area, select);

                    // TODO: don't highlight next char in append mode
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

                use tui::backend::Backend;
                // TODO: double buffer and diff here
                let empty = Surface::empty(area);
                self.terminal
                    .backend_mut()
                    .draw(empty.diff(&surface).into_iter());

                // set cursor shape
                match state.mode {
                    Mode::Insert => write!(stdout, "\x1B[6 q"),
                    Mode::Normal => write!(stdout, "\x1B[2 q"),
                };

                // render the cursor
                let pos = state.selection.cursor();
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
                                    } => helix_core::commands::insert_char(state, c),
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
