

use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use formulac::Builder;
use num_complex::Complex;




#[wasm_bindgen]
pub struct PointResult {
    x: f64,
    y: f64,
}

#[wasm_bindgen]
impl PointResult {
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 { self.x }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 { self.y }
}


pub fn compute_given_function(expr_str: &str, start: f64, end: f64, step: f64) -> Result<Vec<(f64, f64)>, String> {

	let builder = Builder::new(expr_str,["x"]);
	let func = builder.compile()
		.expect("Failed to compile: {expr_str}");
	
	let mut current = start;
	let mut points = Vec::new();

	while current <= end + 1e-9 {
		let result = func([Complex::new(current, 0.0)]);
		points.push((current, result.re));
		current += step;
	}

	Ok(points)

}




#[wasm_bindgen]
pub fn evaluate_function(expr_str: &str, start: f64, end: f64, step: f64) -> Result<js_sys::Array, JsValue> {

	let points = compute_given_function(expr_str, start, end, step)
		.map_err(|err| JsValue::from_str(&err))?;

	let js_array = js_sys::Array::new();

	for (x, y) in points {
		let pt = PointResult {x, y};
		let js_val = JsValue::from(pt);
		js_array.push(&js_val);
	}

	Ok(js_array)
}
