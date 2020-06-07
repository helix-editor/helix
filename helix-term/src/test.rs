//! Demonstrates how to read events asynchronously with async-std.
//!
//! cargo run --features="event-stream" --example event-stream-async-std

use std::{
    io::{stdout, Write},
    time::Duration,
};

use futures::{future::FutureExt, select, StreamExt};
use smol::Timer;
// use futures_timer::Delay;

use crossterm::{
    cursor::position,
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
    Result,
};

const HELP: &str = r#"EventStream based on futures::Stream with async-std
 - Keyboard, mouse and terminal resize events enabled
 - Prints "." every second if there's no event
 - Hit "c" to print current cursor position
 - Use Esc to quit
"#;

async fn print_events() {
    let mut reader = EventStream::new();

    loop {
        let mut delay = Timer::after(Duration::from_millis(1_000)).fuse();
        let mut event = reader.next().fuse();

        select! {
            _ = delay => { println!(".\r"); },
                    maybe_event = event => {
                          match maybe_event {
                    Some(Ok(event)) => {
                        println!("Event::{:?}\r", event);

                        if event == Event::Key(KeyCode::Char('c').into()) {
                            println!("Cursor position: {:?}\r", position());
                        
                        }

                            println!("test");

                        if event == Event::Key(KeyCode::Esc.into()) {
                            break;
                        }
                    }
                    Some(Err(e)) => println!("Error: {:?}\r", e),
                    None => break,
                }
            }
        };
    }
}

fn main() -> Result<()> {
    println!("{}", HELP);

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnableMouseCapture)?;

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
    smol::block_on(print_events());

    // Send a shutdown signal.
    drop(s);

    // Wait for threads to finish.
    for t in threads {
        t.join().unwrap();
    }

    execute!(stdout, DisableMouseCapture)?;

    disable_raw_mode()
}
