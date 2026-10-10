//! WASM bindings for yaml-schema: validate YAML documents against YAML schemas in the browser.
//!
//! Build with `wasm-pack build crates/yaml-schema-wasm --target web --out-dir pkg`.

use wasm_bindgen::prelude::*;

pub mod api;
pub mod result;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}
