use crate::xterm::{
    addons::{fit::FitAddon, webgl::WebglAddon},
    Terminal, TerminalOptions, Theme,
};
use helix_tui::backend::Backend;
use helix_view::graphics::{CursorKind, Rect};
use log::debug;
use wasm_bindgen::JsCast;

pub struct XTermJsBackend {
    terminal: Terminal,
}

impl XTermJsBackend {
    pub fn new() -> Self {
        let theme = Theme::new();
        theme.set_background("#282a36");
        let term_opts = TerminalOptions::new();
        term_opts.set_font_size(20);
        term_opts.set_scrollback(0);
        term_opts.set_theme(&theme);
        let terminal = Terminal::new(&term_opts);

        let elem = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("terminal")
            .unwrap();

        terminal.open(elem.dyn_into().unwrap());

        let addon = FitAddon::new();
        terminal.load_addon(addon.clone().dyn_into::<FitAddon>().unwrap().into());
        addon.fit();

        let addon = WebglAddon::new(None);
        terminal.load_addon(addon.clone().dyn_into::<WebglAddon>().unwrap().into());

        terminal.focus();

        let cols = terminal.get_cols() as usize;
        let rows = terminal.get_rows() as usize;
        debug!("{cols} - {rows}");

        XTermJsBackend { terminal }
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
        for c in content {
            debug!("{:?}", c);
        }
        self.terminal.writeln_utf8("draw called".as_bytes());

        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn show_cursor(&mut self, kind: CursorKind) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), std::io::Error> {
        Ok((0, 0))
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn clear(&mut self) -> Result<(), std::io::Error> {
        self.terminal.clear();
        Ok(())
    }

    fn size(&self) -> Result<Rect, std::io::Error> {
        Ok(Rect::new(
            0,
            0,
            self.terminal.get_cols() as u16,
            self.terminal.get_rows() as u16,
        ))
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}
