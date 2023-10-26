pub mod addons;
pub mod crossterm;

use js_sys::Function;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, KeyboardEvent};

#[wasm_bindgen(module = "xterm")]
extern "C" {

    #[wasm_bindgen(js_name = "ITerminalOptions")]
    pub type TerminalOptions;

    #[wasm_bindgen(method, setter, js_name = "fontSize")]
    pub fn set_font_size(this: &TerminalOptions, val: u32);

    #[wasm_bindgen(method, setter, js_name = "scrollback")]
    pub fn set_scrollback(this: &TerminalOptions, val: u32);

    #[wasm_bindgen(method, setter, js_name = "theme")]
    pub fn set_theme(this: &TerminalOptions, val: &Theme);

    // ========================================================================

    #[wasm_bindgen(js_name = "ITheme")]
    pub type Theme;

    #[wasm_bindgen(method, setter, js_name = "foreground")]
    pub fn set_foreground(this: &Theme, val: &str);

    #[wasm_bindgen(method, setter, js_name = "background")]
    pub fn set_background(this: &Theme, val: &str);

    // ========================================================================

    #[wasm_bindgen(js_name = "IDisposable")]
    pub type Disposable;

    #[wasm_bindgen(method, js_name = "dispose")]
    pub fn dispose(this: &Disposable);

    // ========================================================================

    #[wasm_bindgen(js_name = "IEvent")]
    pub type Event;

    //   export interface Event<T, U = void> {
    //     (listener: (arg1: T, arg2: U) => any): Disposable;
    //   }

    // ========================================================================

    #[wasm_bindgen(extends = Disposable)]
    pub type Terminal;

    #[wasm_bindgen(constructor)]
    pub fn construct(options: Option<&TerminalOptions>) -> Terminal;

    #[wasm_bindgen(method, getter, js_name = "rows")]
    pub fn get_rows(this: &Terminal) -> u32;

    #[wasm_bindgen(method, getter, js_name = "cols")]
    pub fn get_cols(this: &Terminal) -> u32;

    // ========================================================================

    #[wasm_bindgen(method, js_name = "onBinary")]
    pub fn on_binary(
        this: &Terminal,
        f: &Function, // Event<&str>
    ) -> Disposable;

    //---------------

    #[wasm_bindgen(method, js_name = "onCursorMove")]
    pub fn on_cursor_move(
        this: &Terminal,
        f: &Function, // Event<void>
    ) -> Disposable;

    //---------------

    #[wasm_bindgen(method, js_name = "onData")]
    pub fn on_data(
        this: &Terminal,
        f: &Function, // Event<&str>
    ) -> Disposable;

    //---------------

    pub type OnKeyEvent;

    #[wasm_bindgen(method, getter, js_name = "key")]
    pub fn key(this: &OnKeyEvent) -> String;

    #[wasm_bindgen(method, getter, js_name = "domEvent")]
    pub fn dom_event(this: &OnKeyEvent) -> KeyboardEvent;

    #[wasm_bindgen(method, js_name = "onKey")]
    pub fn on_key(
        this: &Terminal,
        f: &Function, // Event<{key: &str, dom_event: KeyboardEvent}>
    ) -> Disposable;

    //---------------

    #[wasm_bindgen(method, js_name = "onResize")]
    pub fn on_resize(
        this: &Terminal,
        f: &Function, // Event<{cols: u32, rows: u32}>
    ) -> Disposable;

    //---------------

    #[wasm_bindgen(method, js_name = "open")]
    pub fn open(this: &Terminal, parent: HtmlElement);

    #[wasm_bindgen(method, js_name = "attachCustomKeyEventHandler")]
    pub fn attach_custom_key_event_handler(
        this: &Terminal,
        custom_key_event_handler: &Function, // (event: KeyboardEvent) => bool
    );

    #[wasm_bindgen(method, js_name = "clear")]
    pub fn clear(this: &Terminal);

    #[wasm_bindgen(method, js_name = "reset")]
    pub fn reset(this: &Terminal);

    #[wasm_bindgen(method, js_name = "focus")]
    pub fn focus(this: &Terminal);

    // ----------

    #[wasm_bindgen(method, js_name = "write")]
    pub fn write(this: &Terminal, data: &str);

    #[wasm_bindgen(method, js_name = "write")]
    pub fn write_utf8(this: &Terminal, data: &[u8]);

    #[wasm_bindgen(method, js_name = "write")]
    pub fn write_callback(this: &Terminal, data: &str, callback: &Function); // () => void

    #[wasm_bindgen(method, js_name = "write")]
    pub fn write_utf8_callback(this: &Terminal, data: &[u8], callback: &Function); // () => void

    // ----------

    #[wasm_bindgen(method, js_name = "writeln")]
    pub fn writeln(this: &Terminal, data: &str);

    #[wasm_bindgen(method, js_name = "writeln")]
    pub fn writeln_utf8(this: &Terminal, data: &[u8]);

    #[wasm_bindgen(method, js_name = "writeln")]
    pub fn writeln_with_callback(this: &Terminal, data: &str, callback: Option<&Function>); // () => void

    #[wasm_bindgen(method, js_name = "writeln")]
    pub fn writeln_utf8_with_callback(this: &Terminal, data: &[u8], callback: Option<&Function>); // () => void

    // ----------

    #[wasm_bindgen(method, js_name = "loadAddon")]
    pub fn load_addon(this: &Terminal, addon: TerminalAddon);

    // ========================================================================

    #[wasm_bindgen(extends = Disposable, js_name = "ITerminalAddon")]
    pub type TerminalAddon;

    #[wasm_bindgen(method)]
    pub fn activate(this: &TerminalAddon, terminal: Terminal);

}

impl Terminal {
    pub fn new(options: &TerminalOptions) -> Self {
        Terminal::construct(Some(&options))
    }
}

impl TerminalOptions {
    pub fn new() -> Self {
        js_sys::Object::new().unchecked_into()
    }
}

impl Theme {
    pub fn new() -> Self {
        js_sys::Object::new().unchecked_into()
    }
}
