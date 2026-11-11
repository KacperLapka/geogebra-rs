use formulac::Builder;
use num_complex::Complex;
use symb_anafis::diff;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
#[derive(Clone)]
pub struct PointResult {
    x: f64,
    y: f64,
}

#[wasm_bindgen]
impl PointResult {
    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 {
        self.x
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 {
        self.y
    }
}

#[wasm_bindgen]
pub struct Mathematician {
    points_normalised_function: Vec<PointResult>,
    points_normalised_derivative: Vec<PointResult>,
    points_normalised_second_derivative: Vec<PointResult>,
    function: String,
    derivative: String,
    second_derivative: String,
	start: Option<f64>,
	end: Option<f64>,
}

#[wasm_bindgen]
impl Mathematician {
    #[wasm_bindgen(constructor)]
    pub fn new(function: String) -> Mathematician {
        Mathematician {
            points_normalised_function: Vec::new(),
            points_normalised_derivative: Vec::new(),
            points_normalised_second_derivative: Vec::new(),
            function,
            derivative: String::new(),
            second_derivative: String::new(),
			start: None,
			end: None,
        }
    }

    pub fn calc_derivatives(&mut self) -> Result<(), JsValue> {
        self.derivative = compute_derivative(&self.function)?;
        self.second_derivative = compute_derivative(&self.derivative)?;
        Ok(())
    }

	
	pub fn to_points(&mut self) -> Result<(), JsValue> {
		
		let start = self.start.ok_or_else(|| JsValue::from_str("Start range has not been set!"))?;
        let end = self.end.ok_or_else(|| JsValue::from_str("End range has not been set!"))?;

		let points = compute_given_function(&self.function, start, end)
			.map_err(|err| JsValue::from_str(&err))?; 
		self.points_normalised_function = scale_points(&points, start, end);

		if !self.derivative.is_empty() {
			if let Ok(raw_der) = compute_given_function(&self.derivative, start, end) {
				self.points_normalised_derivative = scale_points(&raw_der, start, end);
			}
		}

		if !self.second_derivative.is_empty() {
			if let Ok(raw_sec) = compute_given_function(&self.second_derivative, start, end) {
				self.points_normalised_second_derivative = scale_points(&raw_sec, start, end);
			}
		}

		Ok(())
	}


	#[wasm_bindgen(getter)]
    pub fn function(&self) -> String {
        self.function.clone()
    }

	#[wasm_bindgen(getter)]
    pub fn derivative(&self) -> String {
        self.derivative.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn second_derivative(&self) -> String {
        self.second_derivative.clone()
    }

	#[wasm_bindgen(getter)]
    pub fn points_normalised_function(&self) -> Vec<PointResult> {
        self.points_normalised_function.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn points_normalised_derivative(&self) -> Vec<PointResult> {
        self.points_normalised_derivative.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn points_normalised_second_derivative(&self) -> Vec<PointResult> {
        self.points_normalised_second_derivative.clone()
    }

	#[wasm_bindgen(setter)]
    pub fn set_start(&mut self, start: f64) {
        self.start = Some(start); 
    }

    #[wasm_bindgen(setter)]
    pub fn set_end(&mut self, end: f64) {
        self.end = Some(end); 
    }

}

pub fn compute_given_function(
    expr_str: &str,
    start: f64,
    end: f64,
) -> Result<Vec<(f64, f64)>, String> {
    let builder = Builder::new(expr_str, ["x"]);
    let func = builder.compile().expect("Failed to compile: {expr_str}");

    let mut current = start;
    let mut points = Vec::new();
    let step: f64 = (end - start) / 1000.0;

	for _ in 0..1000 {
        let result = func([Complex::new(current, 0.0)]);
        // If the result is NaN or Infinity, fallback to 0.0 (or handle as needed)
        // so the array size remains strictly 1000.
        let y_val = if result.re.is_finite() { result.re } else { 0.0 };
        
        points.push((current, y_val));
        current += step;
    }
	
   // while current <= end + 1e-9 {
   //     let result = func([Complex::new(current, 0.0)]);
   //     points.push((current, result.re));
   //     current += step;
   // }

    Ok(points)
}

pub fn scale_points(points: &Vec<(f64, f64)>, start: f64, end: f64) -> Vec<PointResult> {
    if points.is_empty() {
        return Vec::new();
    }

    let scale_mult_x: f64 = 2.0 / (end - start);
    let scale_offset_x: f64 = -1.0 - (scale_mult_x * start);

    let mut min_y = points[0].1;
    let mut max_y = min_y;

    for &(_x, y) in points {
        if y < min_y {
            min_y = y;
        } else if y > max_y {
            max_y = y;
        }
    }

    let y_range = max_y - min_y;
    let (scale_mult_y, scale_offset_y) = if y_range.abs() < f64::EPSILON {
        (0.0, 0.0)
    } else {
        let mult = 2.0 / y_range;
        (mult, -1.0 - (mult * min_y))
    };

    points
        .iter()
        .map(|&(x, y)| {
            let scaled_x = (x * scale_mult_x) + scale_offset_x;
            let scaled_y = if y_range.abs() < f64::EPSILON {
                0.0
            } else {
                (y * scale_mult_y) + scale_offset_y
            };

            PointResult {
                x: scaled_x,
                y: scaled_y,
            }
        })
        .collect()
}

pub fn points_to_js_array(points: Vec<PointResult>) -> js_sys::Array {
    let js_array = js_sys::Array::new();
    for pt in points {
        let js_val = JsValue::from(pt);
        js_array.push(&js_val);
    }
    js_array
}

#[wasm_bindgen]
pub fn evaluate_function(expr_str: &str, start: f64, end: f64) -> Result<js_sys::Array, JsValue> {
    let points =
        compute_given_function(expr_str, start, end).map_err(|err| JsValue::from_str(&err))?;
    let scaled_points = scale_points(&points, start, end);
    let js_array = points_to_js_array(scaled_points);

    Ok(js_array)
}

pub fn compute_derivative(expr_str: &str) -> Result<String, JsValue> {
    let derivative = diff(expr_str.to_string(), "x".to_string(), Some(&[]), None)
        .map_err(|err| JsValue::from_str(&format!("Derivation error: {:?}", err)))?;

    Ok(derivative)
}
