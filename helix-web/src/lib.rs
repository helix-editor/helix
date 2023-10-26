mod backend;
mod utils;

use backend::XTermJsBackend;
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

    // TODO(wasm32) use tokio rt?
    // let rt = Builder::new_current_thread().build().unwrap();

    // rt.block_on(async {

    let mut app = Application::new(
        Args::default(),
        Config::default(),
        helix_core::config::default_syntax_loader(),
        XTermJsBackend::new(120, 150),
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

    // });
}
