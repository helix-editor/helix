use crossterm::{
    cursor,
    cursor::position,
    event::{self, read, Event, EventStream, KeyCode, KeyEvent},
    execute, queue,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use futures::{future::FutureExt, select, StreamExt};
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Error;

use crate::{keymap, Args};
use helix_core::{state::coords_at_pos, Buffer, State};

pub struct BufferComponent<'a> {
    x: u16,
    y: u16,
    contents: Vec<&'a str>,
}

impl BufferComponent<'_> {
    pub fn render(&self) {
        let mut line_count = 0;
        for line in &self.contents {
            execute!(
                stdout(),
                SetForegroundColor(Color::DarkCyan),
                cursor::MoveTo(self.x, self.y + line_count),
                Print((line_count + 1).to_string())
            );
            execute!(
                stdout(),
                SetForegroundColor(Color::Reset),
                cursor::MoveTo(self.x + 2, self.y + line_count),
                Print(line)
            );
            line_count += 1;
        }
    }
}

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
        let buffer = Buffer::load(path)?;
        let state = State::new(buffer);
        self.state = Some(state);
        Ok(())
    }

    fn render(&mut self) {
        // TODO:

        match &self.state {
            Some(s) => {
                let view = BufferComponent {
                    x: 0,
                    y: self.first_line,
                    contents: s
                        .file()
                        .lines_at(self.first_line as usize)
                        .take(self.size.1 as usize)
                        .map(|x| x.as_str().unwrap())
                        .collect::<Vec<&str>>(),
                };
                view.render();
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
                    // TODO: handle modes and sequences (`gg`)
                    if let Some(command) = keymap.get(&event) {
                        if let Some(state) = &mut self.state {
                            // TODO: handle count other than 1
                            command(state, 1);
                            self.render();
                            // render the cursor
                            let pos = self.state.as_ref().unwrap().selection.primary().head;
                            let coords = coords_at_pos(
                                &self.state.as_ref().unwrap().doc.contents.slice(..),
                                pos,
                            );
                            execute!(
                                stdout(),
                                cursor::MoveTo((coords.1 + 2) as u16, coords.0 as u16)
                            );
                        }
                    }
                }
                Some(Ok(_)) => {
                    // unhandled event
                    ()
                }
                Some(Err(x)) => panic!(x),
                None => break,
            }
        }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        enable_raw_mode()?;

        let mut stdout = stdout();

        execute!(stdout, terminal::EnterAlternateScreen)?;

        use std::thread;

        // Same number of threads as there are CPU cores.
        let num_threads = num_cpus::get().max(1);

        // A channel that sends the shutdown signal.
        let (s, r) = piper::chan::<()>(0);
        let mut threads = Vec::new();

        // Create an executor thread pool.
        for _ in 0..num_threads {
            // Spawn an executor thread that waits for the shutdown signal.
            let r = r.clone();
            threads.push(thread::spawn(move || smol::run(r.recv())));
        }

        // No need to `run()`, now we can just block on the main future.
        smol::block_on(self.print_events());

        // Send a shutdown signal.
        drop(s);

        execute!(stdout, terminal::LeaveAlternateScreen)?;

        // Wait for threads to finish.
        for t in threads {
            t.join().unwrap();
        }

        disable_raw_mode()?;

        Ok(())
    }
}
