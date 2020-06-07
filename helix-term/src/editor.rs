use crossterm::{
    cursor,
    cursor::position,
    event::{self, read, Event, EventStream, KeyCode, KeyEvent},
    execute, queue, style,
    terminal::{self, disable_raw_mode, enable_raw_mode},
    Result,
};
use futures::{future::FutureExt, select, StreamExt};
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

const HELP: &str = r#"
 - Use q to quit
 - Move cursor with h, j, k, l
"#;

pub struct Editor {
    file: PathBuf,
}

impl Editor {
    pub async fn print_events() {
        let mut reader = EventStream::new();
        loop {
            // Handle key events
            let mut event = reader.next().await;
            match event {
                Some(Ok(x)) => {
                    if let Event::Key(KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    }) = x
                    {
                        match c {
                            'h' => execute!(io::stdout(), cursor::MoveLeft(1)).unwrap(),
                            'j' => execute!(io::stdout(), cursor::MoveDown(1)).unwrap(),
                            'k' => execute!(io::stdout(), cursor::MoveUp(1)).unwrap(),
                            'l' => execute!(io::stdout(), cursor::MoveRight(1)).unwrap(),
                            'q' => {
                                execute!(
                                    io::stdout(),
                                    style::ResetColor,
                                    cursor::Show,
                                    terminal::LeaveAlternateScreen
                                );
                                break;
                            }
                            _ => println!("{:?}", x),
                        }
                    }
                }
                Some(Err(x)) => panic!(x),
                None => break,
            }
        }
    }

    pub fn run() -> Result<()> {
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
        smol::block_on(Editor::print_events());

        // Send a shutdown signal.
        drop(s);

        execute!(stdout, terminal::LeaveAlternateScreen)?;

        // Wait for threads to finish.
        for t in threads {
            t.join().unwrap();
        }

        disable_raw_mode()
    }
}
