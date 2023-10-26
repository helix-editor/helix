mod utils;

use helix_term::{application::Application, args::Args, config::Config};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

// TODO(wasm32) figure if/how to use tokio runtime in wasm32
// #[tokio::main(flavor = "current_thread")]
#[wasm_bindgen(start)]
pub async fn main() {
    utils::set_panic_hook();
    utils::set_logging(log::Level::Debug);

    let _app = Application::new(
        Args::default(),
        Config::default(),
        helix_core::config::default_syntax_loader(),
    );
    alert("wow such Helix");
}
