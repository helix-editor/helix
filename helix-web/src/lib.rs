mod backend;
mod utils;
#[path = "crossterm/mod.rs"]
mod xtct;

use backend::spawn_terminal;
use crossterm::event;
use helix_term::{application::Application, args::Args, config::Config};
use wasm_bindgen::prelude::*;
use xtct::XtermJsCrosstermBackend;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub async fn main() {
    utils::set_panic_hook();
    utils::set_logging(log::Level::Debug);

    let terminal = spawn_terminal();
    let term_ref = &terminal;
    let write: XtermJsCrosstermBackend = term_ref.into();

    let config = Config::default();
    let mut app = Application::new_with_write(
        Args::default(),
        config,
        helix_core::config::default_syntax_loader(),
        write,
    )
    .unwrap();

    app.run(&mut event::EventStream::new(&terminal))
        .await
        .unwrap();
}
