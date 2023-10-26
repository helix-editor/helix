mod backend;
mod crossterm;
mod utils;

use backend::spawn_terminal;
use crossterm::XtermJsCrosstermBackend;
use helix_term::{application::Application, args::Args, config::Config};
use helix_view::{
    input::{Event, KeyEvent},
    keyboard::{KeyCode, KeyModifiers},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub async fn main() {
    utils::set_panic_hook();
    utils::set_logging(log::Level::Debug);

    let terminal = spawn_terminal();
    let write: XtermJsCrosstermBackend = (&terminal).into();

    let config = Config::default();
    let mut app = Application::new_with_write(
        Args::default(),
        config,
        helix_core::config::default_syntax_loader(),
        write,
    )
    .unwrap();

    let mut input_stream = futures_util::stream::iter(
        vec![Ok(Event::Key(KeyEvent {
            code: KeyCode::Char('i'),
            modifiers: KeyModifiers::NONE,
        }))]
        .into_iter()
        .chain("Helix web - made with ❤\n天下無敵".chars().map(|c| {
            Ok(Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
            }))
        })),
    );

    app.run(&mut input_stream).await.unwrap();

    alert("wow such Helix");
}
