use crate::xterm::Terminal;
use crate::xterm::TerminalAddon;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen(module = "xterm-addon-fit")]
extern "C" {

    #[wasm_bindgen(extends = TerminalAddon)]
    pub type FitAddon;

    #[wasm_bindgen(constructor)]
    pub fn new() -> FitAddon;

    #[wasm_bindgen(method, js_name = "activate")]
    pub fn activate(this: &FitAddon, terminal: &Terminal);

    #[wasm_bindgen(method, js_name = "dispose")]
    pub fn dispose(this: &FitAddon);

    #[wasm_bindgen(method, js_name = "fit")]
    pub fn fit(this: &FitAddon);

    #[wasm_bindgen(method, js_name = "proposeDimensions")]
    pub fn propose_dimensions(this: &FitAddon) -> TerminalDimensions;

    // ========================================================================

    #[wasm_bindgen(js_name = "ITerminalDimensions")]
    pub type TerminalDimensions;

    #[wasm_bindgen(method, setter, js_name = "rows")]
    pub fn set_rows(this: &TerminalDimensions, val: u32);

    #[wasm_bindgen(method, setter, js_name = "cols")]
    pub fn set_cols(this: &TerminalDimensions, val: u32);
}

impl TerminalDimensions {
    pub fn new() -> Self {
        js_sys::Object::new().unchecked_into()
    }

    pub fn with_rows(&self, val: u32) -> &Self {
        self.set_rows(val);
        self
    }

    pub fn with_cols(&self, val: u32) -> &Self {
        self.set_cols(val);
        self
    }
}
