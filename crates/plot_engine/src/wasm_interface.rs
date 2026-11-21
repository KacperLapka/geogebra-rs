//! Thin JavaScript-facing WASM API.
//!
//! Each export delegates to `crate::graph_state`; all validation and state
//! changes live there so the renderer and the JS API share one code path.

use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::graph_state;

/// Updates the graph using the default `[-10, 10]` x range.
#[wasm_bindgen(js_name = "updateGraph")]
pub fn update_graph(function: &str) -> Result<(), JsValue> {
    graph_state::update_graph(function)
}

/// Pans and shifts the active graph.
#[wasm_bindgen(js_name = "addOffset")]
pub fn add_offset(x: f32, y: f32) -> Result<(), JsValue> {
    graph_state::add_offset(x, y)
}

/// Scales the visible x range of the active graph.
#[wasm_bindgen(js_name = "zoom")]
pub fn zoom(factor: f32) -> Result<(), JsValue> {
    graph_state::zoom(factor)
}

/// Replaces the graph with a function evaluated over `[start, end]`.
#[wasm_bindgen(js_name = "updateGraphWithRange")]
pub fn update_graph_with_range(function: &str, start: f64, end: f64) -> Result<(), JsValue> {
    graph_state::update_graph_with_range(function, start, end)
}
