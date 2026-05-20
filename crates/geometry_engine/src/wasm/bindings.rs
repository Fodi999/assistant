//! wasm_bindgen точки входа (заглушка — заполнить при сборке WASM таргета).
//!
//! Пример использования из JS:
//! ```js
//! import init, { extrude_json } from './geometry_engine.js';
//! await init();
//! const mesh = JSON.parse(extrude_json(JSON.stringify({ depth: 0.1, bevel: 0, profile: [...] })));
//! ```

// TODO: подключить wasm-bindgen когда будет WASM build target
// #[wasm_bindgen]
// pub fn extrude_json(input: &str) -> String { ... }
