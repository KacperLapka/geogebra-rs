use math_engine::Mathematician;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

pub struct GraphState {
    pub changed: bool,
    pub function: Mathematician,
}

#[wasm_bindgen(js_name = "updateGraph")]
pub fn update_graph(function: &str) -> Result<(), JsValue> {
    update_graph_with_range(function, -10.0, 10.0)
}

#[wasm_bindgen(js_name = "updateGraphWithRange")]
pub fn update_graph_with_range(function: &str, start: f64, end: f64) -> Result<(), JsValue> {
    let mut r = Ok(());
    crate::GRAPH_STATE.with_borrow_mut(|state| {
        state.function = Mathematician::new(function.to_owned());
        state.function.set_start(start);
        state.function.set_end(end);
        let _ = state.function.calc_derivatives();
        state.changed = true;
        if let Err(error) = state.function.to_points() {
            r = Err(error);
        }
    });
    r
}
