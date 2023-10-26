use rs_xterm_js::{
    addons::{fit::FitAddon, webgl::WebglAddon},
    Terminal, TerminalOptions, Theme,
};
use wasm_bindgen::JsCast;

pub fn spawn_terminal() -> Terminal {
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
    terminal
}
