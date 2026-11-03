use lyon_tessellation::path::Path;
use wasm_bindgen::prelude::*;

use crate::graph;

pub struct GraphState {
    pub changed: bool,
    pub path: Path,
}

#[wasm_bindgen(
    js_name = "updateGraph",
)]
pub fn update_graph(points: &[f32]) {
    let path = graph::points_raw_to_path(&points.try_into().expect("Expected 1000 points"));
    log::info!("Graph updated with {} points", points.len());

    crate::GRAPH_STATE.with_borrow_mut(|state| {
        state.path = path;
        state.changed = true;
    });
}