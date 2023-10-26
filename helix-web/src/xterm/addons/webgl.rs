use crate::xterm::{Terminal, TerminalAddon};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen(module = "xterm-addon-webgl")]
extern "C" {

    #[wasm_bindgen(extends = TerminalAddon)]
    pub type WebglAddon;

    #[wasm_bindgen(method, setter, js_name = "textureAtlas")]
    pub fn set_texture_atlas(this: &WebglAddon, val: &HtmlCanvasElement);

    #[wasm_bindgen(constructor)]
    pub fn new(preserve_drawing_buffer: Option<bool>) -> WebglAddon;

    #[wasm_bindgen(method, js_name = "activate")]
    pub fn activate(this: &WebglAddon, terminal: Terminal);

    #[wasm_bindgen(method, js_name = "dispose")]
    pub fn dispose(this: &WebglAddon);

}
