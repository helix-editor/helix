use helix_tui::backend::Backend;
use helix_view::graphics::{CursorKind, Rect};
use log::debug;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

// #[wasm_bindgen(module = "/defined-in-js.js")]
// extern "C" {
//     #[wasm_bindgen(js_namespace = xterm)]
//     type Terminal;

//     #[wasm_bindgen(js_namespace = xterm, constructor)]
//     fn new() -> Terminal;

//     #[wasm_bindgen(method, js_class = "Terminal")]
//     fn open(this: &XTerm, element: &HtmlElement);

//     #[wasm_bindgen(method, js_class = "Terminal", js_name = cols)]
//     fn cols(this: &XTerm) -> u16;
// }

pub struct XTermJsBackend {
    width: u16,
    height: u16,
    pos: (u16, u16),
}

impl XTermJsBackend {
    pub fn new(width: u16, height: u16) -> Self {
        // let window = web_sys::window().expect("should have a window in this context");
        // let document = window.document().expect("window should have a document");
        // let xterm = Terminal::new();
        // let term_element = document
        //     .get_element_by_id("terminal")
        //     .expect("should have #terminal on the page");
        // let term_element = term_element
        //     .dyn_ref::<HtmlElement>()
        //     .expect("should have #terminal on the page");
        // xterm.open(term_element);

        XTermJsBackend {
            width,
            height,
            pos: (0, 0),
        }
    }
}

impl Backend for XTermJsBackend {
    fn claim(&mut self, config: helix_tui::terminal::Config) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn reconfigure(&mut self, config: helix_tui::terminal::Config) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn restore(&mut self, config: helix_tui::terminal::Config) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn force_restore() -> Result<(), std::io::Error> {
        Ok(())
    }

    fn draw<'a, I>(&mut self, content: I) -> Result<(), std::io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a helix_tui::buffer::Cell)>,
    {
        for (x, y, cell) in content {
            debug!("{x} - {y}: {:?}", cell);
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn show_cursor(&mut self, kind: CursorKind) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), std::io::Error> {
        Ok(self.pos)
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn size(&self) -> Result<Rect, std::io::Error> {
        Ok(Rect::new(0, 0, self.width, self.height))
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}
