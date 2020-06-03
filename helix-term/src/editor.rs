use std::io::{self, stdout, Write};

use crossterm::{
    cursor,
    cursor::position,
    event::{self, read, Event, KeyCode, KeyEvent},
    execute, style,
    terminal::{self, disable_raw_mode, enable_raw_mode},
    Result,
};

const HELP: &str = r#"
 - Use q to quit
 - Move cursor with h, j, k, l
"#;

pub struct Editor {}

impl Editor {
    pub fn read_char() -> Result<char> {
        loop {
            if let Ok(Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            })) = event::read()
            {
                return Ok(c);
            }
        }
    }

    pub fn print_events() -> Result<()> {
        loop {
            // Handle key events
            match Editor::read_char()? {
                'h' => execute!(io::stdout(), cursor::MoveLeft(1))?,
                'j' => execute!(io::stdout(), cursor::MoveDown(1))?,
                'k' => execute!(io::stdout(), cursor::MoveUp(1))?,
                'l' => execute!(io::stdout(), cursor::MoveRight(1))?,
                'q' => {
                    execute!(
                        io::stdout(),
                        style::ResetColor,
                        cursor::Show,
                        terminal::LeaveAlternateScreen
                    )?;
                    break;
                }
                _ => println!("use 'q' to quit."),
            }
        }

        Ok(())
    }

    pub fn run() -> Result<()> {
        enable_raw_mode()?;

        // used for clearing the screen
        execute!(io::stdout(), terminal::EnterAlternateScreen)?;
        println!("{}", HELP);
        let mut stdout = stdout();
        if let Err(e) = Editor::print_events() {
            println!("Error: {:?}\r", e);
        }

        disable_raw_mode()
    }
}
