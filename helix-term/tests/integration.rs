use helix_term::{application::Application, args::Args, config::Config};
use helix_view::current;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[tokio::test]
async fn it_works() {
    let args = Args::default();
    let config = Config::default();
    let mut app = Application::new(args, config).unwrap();

    let inputs = &['i', 'h', 'e', 'l', 'l', 'o', ' ', 'w', 'o', 'r', 'l', 'd'];

    for input in inputs {
        // TODO: use input.parse::<KeyEvent>
        app.handle_terminal_events(Ok(Event::Key(KeyEvent {
            code: KeyCode::Char(*input),
            modifiers: KeyModifiers::NONE,
        })));
    }

    let (_, doc) = current!(app.editor);
    assert_eq!(doc.text(), "hello world\n");
}
